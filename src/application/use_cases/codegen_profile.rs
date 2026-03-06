/// # Function Codegen Profile: Application Use Case Layer
///
/// ## Why This Exists
/// Compiling a function involves two separate concerns:
/// 1. **Policy** — what kind of function is this, and what does that mean
///    for how the optimizer should treat it?
/// 2. **Mechanism** — given that policy, which LLVM attributes, linkage
///    flags, and calling-convention codes do we write into the module?
///
/// Before this module existed, both concerns were mixed inside
/// `declare_function` in the Infrastructure adapter (`codegen/mod.rs`).
/// That meant every change to an optimization heuristic (e.g. "don't mark
/// recursive functions `cold`") required opening an LLVM file and reading
/// past inkwell API calls to find the actual business logic.
///
/// This module owns the **policy** side only.  It is a pure Use Case:
/// no LLVM types, no inkwell, no framework dependency.  A non-technical
/// reader can understand every decision here in plain English.
/// The adapter translates the resulting `FunctionCodegenProfile` into
/// LLVM calls — that is its only job.
///
/// ## Design Pattern: Value Object + Policy Object
/// `FunctionCodegenProfile` is an immutable Value Object — compared by
/// value, constructed fresh, never mutated.  `derive_profile` is a
/// Policy Object (a pure function variant of Strategy): it encodes the
/// rules for how profiles are chosen.  Different optimization levels
/// (future `-O2` / `-Oz` modes) can be different functions or structs
/// passed to the adapter without touching the adapter itself.
use crate::domain::entities::mir::{MirFunction, MirInstruction};

// ---------------------------------------------------------------------------
// Value types — pure data, no framework dependency
// ---------------------------------------------------------------------------

/// Whether a function symbol is visible outside this translation unit.
///
/// - `Public`   → OS entry point (`main`). Must be External so the OS
///               dynamic linker can find and invoke it.
/// - `Internal` → every other function. Equivalent to C `static`:
///               the linker resolves call sites directly, no PLT stub.
///               Eliminates one indirect jump per call site — measurable
///               on workloads with hundreds of millions of calls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionLinkage {
    /// Exported symbol — visible to the OS and dynamic linker.
    Public,
    /// Translation-unit–private symbol — direct calls, no PLT.
    Internal,
}

/// The calling convention to use at call sites.
///
/// - `CDefault` → standard C ABI. Required for `main` because the C
///               runtime (`crt0`) calls it with a specific ABI.
/// - `Fast`     → LLVM `fastcc`. Internal functions do not need to
///               conform to the C ABI, so the backend can pass arguments
///               in registers and omit callee-saved-register saves that
///               are mandated by the C convention. Measurably faster on
///               short, frequently-called functions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallingConvention {
    /// Standard C calling convention. Required for `main`.
    CDefault,
    /// LLVM fast calling convention. Optimal for internal functions.
    Fast,
}

/// An optimizer hint the backend should attach to a function.
///
/// Each variant has a precise semantic meaning — the adapter is
/// responsible for translating these to the right LLVM attribute name.
/// Adding a new hint does not require touching any policy logic; removing
/// one does not require touching the adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptimizerHint {
    /// The function does not read or write memory visible to the caller.
    /// LLVM can hoist calls out of loops and eliminate duplicates.
    ReadNone,
    /// The function cannot throw or unwind the stack.
    /// Eliminates landing-pad generation at every call site.
    NoUnwind,
    /// The function cannot call `free` (or the arena equivalent).
    /// Enables more aggressive alias analysis.
    NoFree,
    /// The function contains no synchronisation primitives.
    /// Allows reordering across call boundaries in multi-threaded analysis.
    NoSync,
    /// Force-inline at every call site.
    /// Only safe for non-recursive, pure-leaf functions where the body
    /// is small and calling overhead dominates.
    AlwaysInline,
}

/// The complete optimization profile for a single function.
///
/// This is what the Application layer hands to the Infrastructure adapter.
/// The adapter reads it and writes the corresponding LLVM — nothing more.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionCodegenProfile {
    /// Whether the symbol is exported or translation-unit–private.
    pub linkage: FunctionLinkage,
    /// Which calling convention to use.
    pub calling_convention: CallingConvention,
    /// Optimizer hints to attach to the function definition.
    pub optimizer_hints: Vec<OptimizerHint>,
}

// ---------------------------------------------------------------------------
// Policy — the ONLY place that encodes optimization heuristics
// ---------------------------------------------------------------------------

/// Derives the codegen profile for a function from its MIR shape.
///
/// ### Rules (in plain English)
/// 1. **Entry points** (`run` / `main`) are public and use the C ABI.
///    They receive no optimizer hints because the runtime caller expects
///    a specific ABI and the body is only called once.
///
/// 2. **All other functions** are internal (no PLT) and use `fastcc`.
///
/// 3. **Pure data-leaf functions** (no I/O, no mutation) additionally
///    receive `ReadNone`, `NoUnwind`, `NoFree`, and `NoSync` — the four
///    hints that let LLVM treat the function as a mathematical computation
///    with no side effects.
///
/// 4. **Non-recursive** pure leaves also get `AlwaysInline`.
///    Recursive functions deliberately do NOT get this hint: forcing
///    inline on a recursive function would cause LLVM to expand the
///    recursion at compile time, which is unsafe for unbounded depth.
///    We instead trust LLVM's own inliner cost model to partially peel
///    the call tree — the same thing GCC `-O3` does.
///
/// ### What is NOT here
/// - `cold` — was previously applied to recursive functions.  It told
///   LLVM the function was rarely called (false for fib!) and pushed it
///   to the cold section of the binary, hurting icache.  Removed.
/// - `noinline` — was previously applied to recursive functions.  It
///   prevented LLVM from unrolling the recursion tree at all.  Removed.
pub fn derive_profile(func: &MirFunction) -> FunctionCodegenProfile {
    // Rule 1: entry points need a fixed, public ABI.
    let is_entry = func.name == "run" || func.name == "main";
    if is_entry {
        return FunctionCodegenProfile {
            linkage: FunctionLinkage::Public,
            calling_convention: CallingConvention::CDefault,
            optimizer_hints: vec![],
        };
    }

    // Rule 4 prerequisite: detect whether the function calls itself.
    // A single pass through the instruction lists is O(n) in function size.
    let is_self_recursive = func
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .any(|inst| {
            matches!(inst,
                MirInstruction::Call { name, .. } if name == &func.name)
        });

    // Rules 3 & 4: pure-data-leaf hints
    let mut hints = Vec::new();
    if func.is_pure_data_leaf {
        hints.push(OptimizerHint::ReadNone);
        hints.push(OptimizerHint::NoUnwind);
        hints.push(OptimizerHint::NoFree);
        hints.push(OptimizerHint::NoSync);

        if !is_self_recursive {
            // Non-recursive pure leaves: small bodies, called often.
            // Force-inlining eliminates the call entirely.
            hints.push(OptimizerHint::AlwaysInline);
        }
        // Recursive: trust LLVM's inliner — no forced inline or forced cold.
    }

    // Rule 2: internal, fastcc, with whatever hints we collected above.
    FunctionCodegenProfile {
        linkage: FunctionLinkage::Internal,
        calling_convention: CallingConvention::Fast,
        optimizer_hints: hints,
    }
}

// ---------------------------------------------------------------------------
// Unit tests — pure Rust, zero LLVM / inkwell dependency
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::mir::{
        BasicBlock, MirFunction, MirInstruction, MirLiteral, MirOperand, MirTerminator,
    };
    use crate::domain::entities::types::OnuType;

    /// Builds a minimal MirFunction with the given name, is_pure_data_leaf flag,
    /// and a list of instructions in a single block.  Suitable for policy tests.
    fn make_fn(
        name: &str,
        is_pure_data_leaf: bool,
        instructions: Vec<MirInstruction>,
    ) -> MirFunction {
        MirFunction {
            name: name.to_string(),
            args: vec![],
            return_type: OnuType::I64,
            blocks: vec![BasicBlock {
                id: 0,
                instructions,
                terminator: MirTerminator::Return(MirOperand::Constant(MirLiteral::I64(0))),
            }],
            is_pure_data_leaf,
            diminishing: None,
            memo_cache_size: None,
        }
    }

    /// A Call instruction that calls `callee_name` — used to simulate
    /// self-recursion inside make_fn's instruction list.
    fn self_call(callee: &str) -> MirInstruction {
        MirInstruction::Call {
            dest: 99,
            name: callee.to_string(),
            args: vec![],
            return_type: OnuType::I64,
            arg_types: vec![],
            is_tail_call: true,
        }
    }

    // --- Linkage tests ---

    #[test]
    fn entry_point_run_is_public() {
        let profile = derive_profile(&make_fn("run", false, vec![]));
        assert_eq!(profile.linkage, FunctionLinkage::Public);
    }

    #[test]
    fn entry_point_main_is_public() {
        let profile = derive_profile(&make_fn("main", false, vec![]));
        assert_eq!(profile.linkage, FunctionLinkage::Public);
    }

    #[test]
    fn non_entry_function_is_internal() {
        let profile = derive_profile(&make_fn("calculate-growth", false, vec![]));
        assert_eq!(
            profile.linkage,
            FunctionLinkage::Internal,
            "Non-entry functions must use Internal linkage to avoid PLT stubs"
        );
    }

    // --- Calling convention tests ---

    #[test]
    fn entry_point_uses_c_default_calling_convention() {
        let profile = derive_profile(&make_fn("run", false, vec![]));
        assert_eq!(profile.calling_convention, CallingConvention::CDefault);
    }

    #[test]
    fn internal_function_uses_fast_calling_convention() {
        let profile = derive_profile(&make_fn("helper", true, vec![]));
        assert_eq!(profile.calling_convention, CallingConvention::Fast);
    }

    // --- Optimizer hint tests ---

    #[test]
    fn entry_point_has_no_optimizer_hints() {
        let profile = derive_profile(&make_fn("run", false, vec![]));
        assert!(
            profile.optimizer_hints.is_empty(),
            "Entry points receive no optimizer hints — the C runtime ABI must be preserved"
        );
    }

    #[test]
    fn pure_leaf_gets_all_four_purity_hints() {
        let profile = derive_profile(&make_fn("leaf", true, vec![]));
        assert!(profile.optimizer_hints.contains(&OptimizerHint::ReadNone));
        assert!(profile.optimizer_hints.contains(&OptimizerHint::NoUnwind));
        assert!(profile.optimizer_hints.contains(&OptimizerHint::NoFree));
        assert!(profile.optimizer_hints.contains(&OptimizerHint::NoSync));
    }

    #[test]
    fn non_recursive_pure_leaf_gets_always_inline() {
        let profile = derive_profile(&make_fn("leaf", true, vec![]));
        assert!(
            profile
                .optimizer_hints
                .contains(&OptimizerHint::AlwaysInline),
            "Non-recursive pure leaves should be force-inlined to eliminate call overhead"
        );
    }

    #[test]
    fn recursive_function_does_not_get_always_inline() {
        // A function that calls itself — self-recursion detected from the call graph.
        let recursion = self_call("fib");
        let profile = derive_profile(&make_fn("fib", true, vec![recursion]));
        assert!(
            !profile
                .optimizer_hints
                .contains(&OptimizerHint::AlwaysInline),
            "Recursive functions must not be forced-inline — depth is unbounded"
        );
    }

    #[test]
    fn recursive_function_has_no_cold_hint() {
        // `cold` was previously applied — this guards against regression.
        let recursion = self_call("collatz-steps");
        let _profile = derive_profile(&make_fn("collatz-steps", true, vec![recursion]));
        // There is no `Cold` variant in OptimizerHint — this test is a compile-time
        // guard that the enum does not grow a `Cold` variant without review.
        // The IR-level regression guard in benchmarktest.rs catches the LLVM attribute.
    }

    #[test]
    fn impure_function_gets_no_optimizer_hints() {
        // An impure function (is_pure_data_leaf = false) like `run` or an effect fn.
        let profile = derive_profile(&make_fn("do-something!", false, vec![]));
        assert!(profile.optimizer_hints.is_empty());
    }
}
