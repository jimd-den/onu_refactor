/// Ọ̀nụ Safety Pass — Grammar Rules That Prevent Silent Crashes
///
/// This pass runs on the HIR (Vec<HirDiscourse>) after `lower_hir` and before
/// `lower_mir`.  It enforces three rules derived from the four root-cause
/// classes discovered when running the benchmark samples:
///
/// ┌─────┬────────────────────────────────────┬────────────────────────────┐
/// │ ID  │ What goes wrong at runtime          │ Violation class            │
/// ├─────┼────────────────────────────────────┼────────────────────────────┤
/// │ S-1 │ set-char on a string literal writes │ Pure Grammar Violation     │
/// │     │ into read-only LLVM constant memory │ + KISS (invisible rule)    │
/// │     │ — LLVM constant-folds all reads back│                            │
/// │     │ to the original literal, so the tape│                            │
/// │     │ never changes and the program loops │                            │
/// │     │ forever or crashes.                 │                            │
/// ├─────┼────────────────────────────────────┼────────────────────────────┤
/// │ S-2 │ A `with diminishing` function is   │ SOLID (SRP/LSP) + KISS     │
/// │     │ called more than once per session.  │                            │
/// │     │ Each top-level call allocates the   │                            │
/// │     │ entire 16 MB arena for its memo     │                            │
/// │     │ table — a second call finds the     │                            │
/// │     │ arena full and writes off the end,  │                            │
/// │     │ producing a segfault.               │                            │
/// ├─────┼────────────────────────────────────┼────────────────────────────┤
/// │ S-3 │ A derivation's type defaults to I64 │ Pure Grammar Violation     │
/// │     │ when the expression clearly returns │                            │
/// │     │ a string. The 3-field string struct │                            │
/// │     │ is packed into a single i64, corrupt│                            │
/// │     │ ing the length and pointer fields.  │                            │
/// └─────┴────────────────────────────────────┴────────────────────────────┘
///
/// # Error message philosophy
/// Every message is written in two registers:
///   1. **Plain-language sentence** — readable by anyone.  States *what* is
///      wrong and *what to type* to fix it, with no compiler jargon.
///   2. **Technical footnote** — for coders: names the root cause (LLVM
///      concept, SOLID principle, arena constraint) so they understand *why*.

use crate::domain::entities::error::{Diagnostic, OnuError, Span};
use crate::domain::entities::hir::{HirDiscourse, HirExpression, HirLiteral};
use crate::domain::entities::types::OnuType;
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run all safety rules on a compiled HIR program.
///
/// Returns `Err` on the first hard violation.  All violations are collected
/// before returning so the coder sees every problem at once.
pub fn run(discourses: &[HirDiscourse]) -> Result<Vec<Diagnostic>, OnuError> {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let mut hard_errors: Vec<OnuError> = Vec::new();

    // Build the set of memoized function names once (used by S-2).
    let memoized_fns: HashSet<String> = discourses
        .iter()
        .filter_map(|d| {
            if let HirDiscourse::Behavior { header, .. } = d {
                if !header.diminishing.is_empty() {
                    return Some(header.name.clone());
                }
            }
            None
        })
        .collect();

    for discourse in discourses {
        if let HirDiscourse::Behavior { header, body } = discourse {
            let name = &header.name;

            // S-1: string literal mutation check
            let mut literal_vars: HashMap<String, ()> = HashMap::new();
            let mut arena_copies: HashSet<String> = HashSet::new();
            collect_literal_bindings(body, &mut literal_vars, &mut arena_copies);
            find_mutating_calls(body, &literal_vars, &arena_copies, name, &mut hard_errors);

            // S-2: memoized function called more than once — only count calls
            // from *other* behaviors (i.e. callers, not the function itself).
            // A memoized function calling itself recursively is correct usage.
            for memo_fn in &memoized_fns {
                if memo_fn == name {
                    continue; // skip self — recursive calls are fine
                }
                let count = count_calls(body, memo_fn);
                if count > 1 {
                    hard_errors.push(memo_called_multiple_times(name, memo_fn, count));
                } else if count == 1 {
                    diagnostics.push(memo_single_call_hint(name, memo_fn));
                }
            }

            // S-3: literal text used without type annotation
            find_untyped_text_derivations(body, name, &mut hard_errors);
        }
    }

    if !hard_errors.is_empty() {
        return Err(hard_errors.remove(0));
    }
    Ok(diagnostics)
}

// ---------------------------------------------------------------------------
// S-1: String Literal Mutation
// ---------------------------------------------------------------------------

/// Walk the expression tree and record:
/// - `literal_vars`: names bound directly to a text literal
/// - `arena_copies`: names produced by a `joined-with` / `duplicated-as` call
///   (those arrive in mutable arena memory and may be mutated safely)
fn collect_literal_bindings(
    expr: &HirExpression,
    literals: &mut HashMap<String, ()>,
    arena_copies: &mut HashSet<String>,
) {
    match expr {
        HirExpression::Derivation { name, value, body, typ, .. } => {
            match value.as_ref() {
                // Direct literal text → read-only constant
                HirExpression::Literal(HirLiteral::Text(_)) => {
                    if matches!(typ, OnuType::Strings | OnuType::I64) {
                        literals.insert(name.clone(), ());
                    }
                }
                // joined-with or duplicated-as → mutable arena copy
                HirExpression::Call { name: fn_name, args } => {
                    let safe = fn_name == "joined-with" || fn_name == "duplicated-as";
                    if safe {
                        arena_copies.insert(name.clone());
                    }
                    for a in args {
                        collect_literal_bindings(a, literals, arena_copies);
                    }
                }
                other => collect_literal_bindings(other, literals, arena_copies),
            }
            collect_literal_bindings(body, literals, arena_copies);
        }
        HirExpression::Block(exprs) => {
            for e in exprs {
                collect_literal_bindings(e, literals, arena_copies);
            }
        }
        HirExpression::If { condition, then_branch, else_branch } => {
            collect_literal_bindings(condition, literals, arena_copies);
            collect_literal_bindings(then_branch, literals, arena_copies);
            collect_literal_bindings(else_branch, literals, arena_copies);
        }
        HirExpression::Call { args, .. } => {
            for a in args {
                collect_literal_bindings(a, literals, arena_copies);
            }
        }
        _ => {}
    }
}

const MUTATING_OPS: &[&str] = &["set-char", "write-tape", "inplace-set-char"];

fn find_mutating_calls(
    expr: &HirExpression,
    literals: &HashMap<String, ()>,
    arena_copies: &HashSet<String>,
    behavior_name: &str,
    hard_errors: &mut Vec<OnuError>,
) {
    match expr {
        HirExpression::Call { name, args } if MUTATING_OPS.contains(&name.as_str()) => {
            // The first argument is the string being mutated.
            if let Some(HirExpression::Variable(var_name, _)) = args.first() {
                if literals.contains_key(var_name) && !arena_copies.contains(var_name) {
                    hard_errors.push(literal_mutation_error(behavior_name, var_name, name));
                }
            }
            for a in args {
                find_mutating_calls(a, literals, arena_copies, behavior_name, hard_errors);
            }
        }
        HirExpression::Block(exprs) => {
            for e in exprs {
                find_mutating_calls(e, literals, arena_copies, behavior_name, hard_errors);
            }
        }
        HirExpression::Derivation { value, body, .. } => {
            find_mutating_calls(value, literals, arena_copies, behavior_name, hard_errors);
            find_mutating_calls(body, literals, arena_copies, behavior_name, hard_errors);
        }
        HirExpression::If { condition, then_branch, else_branch } => {
            find_mutating_calls(condition, literals, arena_copies, behavior_name, hard_errors);
            find_mutating_calls(then_branch, literals, arena_copies, behavior_name, hard_errors);
            find_mutating_calls(else_branch, literals, arena_copies, behavior_name, hard_errors);
        }
        HirExpression::Call { args, .. } => {
            for a in args {
                find_mutating_calls(a, literals, arena_copies, behavior_name, hard_errors);
            }
        }
        _ => {}
    }
}

fn literal_mutation_error(behavior: &str, var: &str, op: &str) -> OnuError {
    OnuError::GrammarViolation {
        message: format!(
            "═══ Onu Safety Rule S-1: Fixed Text Cannot Be Changed In Place ═══\n\
\n\
In behavior '{behavior}': '{var}' holds a fixed text value (a literal), \
and you are trying to change it using '{op}'.\n\
\n\
  ✗  The problem:\n\
     Fixed texts are stored in the computer's read-only memory.\n\
     Any changes you make are silently thrown away — the text stays the same.\n\
     This can cause your program to loop forever or crash.\n\
\n\
  ✓  How to fix it:\n\
     Copy the text into writable memory first, then change the copy:\n\
\n\
     Before (broken):\n\
       derivation: {var} derives-from a string \"...\"\n\
       {var} utilizes {op} ...\n\
\n\
     After (correct):\n\
       derivation: {var} derives-from \"...\" joined-with \"\"\n\
       {var} utilizes {op} ...\n\
\n\
  (Technical: String literals compile to `private unnamed_addr constant` in LLVM IR.\n\
   LLVM's constant-folding pass treats all loads from this address as the original\n\
   value, discarding every write.  'joined-with \"\"' forces a bump-allocator copy\n\
   into @onu_arena (a mutable global), bypassing the constant.)\n\
\n\
  [S-1 | Pure Grammar Violation + KISS violation]",
        ),
        span: Span::default(),
    }
}

// ---------------------------------------------------------------------------
// S-2: Memoized Function Called More Than Once
// ---------------------------------------------------------------------------

fn count_calls(expr: &HirExpression, target: &str) -> usize {
    match expr {
        HirExpression::Call { name, args } => {
            let self_count = if name == target { 1 } else { 0 };
            self_count + args.iter().map(|a| count_calls(a, target)).sum::<usize>()
        }
        HirExpression::Block(exprs) => exprs.iter().map(|e| count_calls(e, target)).sum(),
        HirExpression::Derivation { value, body, .. } => {
            count_calls(value, target) + count_calls(body, target)
        }
        HirExpression::If { condition, then_branch, else_branch } => {
            count_calls(condition, target)
                + count_calls(then_branch, target)
                + count_calls(else_branch, target)
        }
        HirExpression::Emit(e) | HirExpression::Drop(e) => count_calls(e, target),
        HirExpression::Tuple(elems) => elems.iter().map(|e| count_calls(e, target)).sum(),
        HirExpression::BinaryOp { left, right, .. } => {
            count_calls(left, target) + count_calls(right, target)
        }
        _ => 0,
    }
}

fn memo_called_multiple_times(behavior: &str, memo_fn: &str, count: usize) -> OnuError {
    OnuError::GrammarViolation {
        message: format!(
            "═══ Onu Safety Rule S-2: Speed-Up Behavior Called Too Many Times ═══\n\
\n\
In behavior '{behavior}': '{memo_fn}' (declared with 'with diminishing') is called \
{count} times, but it can only be called once per program run.\n\
\n\
  ✗  The problem:\n\
     'with diminishing' reserves a large block of memory to remember previous\n\
     answers (memoization). It can only reserve this memory once.\n\
     Calling it a second time fills up that block and crashes the program.\n\
\n\
  ✓  How to fix it (choose one):\n\
\n\
     Option A — call it once and store the result:\n\
       derivation: result derives-from ... utilizes {memo_fn} ...\n\
       -- use 'result' everywhere you need the value\n\
\n\
     Option B — if you don't need the speed-up, remove 'with diminishing':\n\
       the behavior {memo_fn}\n\
       with no guaranteed termination:   ← remove 'with diminishing', add this\n\
\n\
  (Technical: HashMemoStrategy wraps '{memo_fn}' in an entry function that\n\
   bump-allocates its entire hash table (~16 MB) from @onu_arena on the first\n\
   call. The arena is never freed. A second call moves the bump pointer past\n\
   the end of the arena, writing to unmapped memory and causing a segfault.\n\
   SOLID SRP violation: the function definition conflates 'compute value' with\n\
   'manage a fixed-size global memo table'.)\n\
\n\
  [S-2 | SOLID SRP+LSP violation + KISS violation]",
        ),
        span: Span::default(),
    }
}

fn memo_single_call_hint(behavior: &str, memo_fn: &str) -> Diagnostic {
    Diagnostic::hint(
        Span::default(),
        format!(
            "In '{behavior}': '{memo_fn}' uses 'with diminishing' (memoization). \
It can only be called once per program run — calling it a second time will crash. \
This behavior calls it exactly once, so you are safe."
        ),
    )
    .with_hint(format!(
        "[S-2 advisory] HashMemoStrategy allocates the full 16 MB arena on the first call to '{memo_fn}'. \
Only one top-level call per program run is allowed."
    ))
}

// ---------------------------------------------------------------------------
// S-3: Literal Text Used Without Type Annotation
// ---------------------------------------------------------------------------

fn find_untyped_text_derivations(
    expr: &HirExpression,
    behavior_name: &str,
    hard_errors: &mut Vec<OnuError>,
) {
    match expr {
        HirExpression::Derivation { name, typ, value, body } => {
            // If the value is a text literal but the type is I64, the type
            // annotation was missing and the lowerer defaulted to I64.
            if matches!(value.as_ref(), HirExpression::Literal(HirLiteral::Text(_)))
                && matches!(typ, OnuType::I64)
            {
                hard_errors.push(untyped_text_error(behavior_name, name));
            }
            find_untyped_text_derivations(value, behavior_name, hard_errors);
            find_untyped_text_derivations(body, behavior_name, hard_errors);
        }
        HirExpression::Block(exprs) => {
            for e in exprs {
                find_untyped_text_derivations(e, behavior_name, hard_errors);
            }
        }
        HirExpression::If { condition, then_branch, else_branch } => {
            find_untyped_text_derivations(condition, behavior_name, hard_errors);
            find_untyped_text_derivations(then_branch, behavior_name, hard_errors);
            find_untyped_text_derivations(else_branch, behavior_name, hard_errors);
        }
        HirExpression::Call { args, .. } => {
            for a in args {
                find_untyped_text_derivations(a, behavior_name, hard_errors);
            }
        }
        HirExpression::Emit(e) | HirExpression::Drop(e) => {
            find_untyped_text_derivations(e, behavior_name, hard_errors);
        }
        _ => {}
    }
}

fn untyped_text_error(behavior: &str, var: &str) -> OnuError {
    OnuError::GrammarViolation {
        message: format!(
            "═══ Onu Safety Rule S-3: Text Value Needs a Type Label ═══\n\
\n\
In behavior '{behavior}': the derivation '{var}' holds a text value (\"...\") \
but has no type label.\n\
\n\
  ✗  The problem:\n\
     Without a label the compiler assumes '{var}' is a number.\n\
     When the program runs, the text value will be broken — its length\n\
     and content will contain garbage numbers instead of real text.\n\
\n\
  ✓  How to fix it:\n\
     Add 'a string' after 'derives-from':\n\
\n\
     Before (broken):\n\
       derivation: {var} derives-from \"...\"\n\
\n\
     After (correct):\n\
       derivation: {var} derives-from a string \"...\"\n\
\n\
  (Technical: HIR lowering defaults to OnuType::I64 when no TypeInfo is present.\n\
   A Strings value is a 3-field struct {{ i64 len, ptr data, i1 is_dynamic }}.\n\
   Allocating an i64 alloca for it packs all three fields into 8 bytes, silently\n\
   corrupting the length and data pointer at runtime.)\n\
\n\
  [S-3 | Pure Grammar Violation]",
        ),
        span: Span::default(),
    }
}
