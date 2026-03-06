/// Integer Upgrade Pass: Application Use Case Layer
///
/// # What This Does
/// Automatically promotes the return type of doubly-recursive pure functions
/// from `I64` to `WideInt(bits)` when the maximum literal call-site argument
/// implies that the true mathematical result would overflow a 64-bit integer.
///
/// The canonical example is `fib-naive`: `fib(93)` already exceeds `i64::MAX`,
/// so any call with `n > 92` needs a wider type to produce the correct answer.
///
/// # Algorithm
/// 1. **Detect upgrade candidates** — functions that are:
///    - `is_pure_data_leaf`, `diminishing.is_some()`, single `I64` argument,
///      `I64` return type, and at least two recursive self-calls (doubly recursive).
/// 2. **Find max literal call argument** across all functions in the program.
/// 3. **Compute required bit width** using `ceil(n * log₂(φ) + 4)` rounded to
///    the next multiple of 64 (φ ≈ 1.618, log₂(φ) ≈ 0.6942).
/// 4. **Rewrite the function body** via backward SSA analysis from `Return`
///    terminators:  literal base cases → `WideInt` constants, recursive calls →
///    `WideInt` return type, the final addition → `WideInt` dest type.
/// 5. **Rewrite every caller** using forward SSA propagation from the upgraded
///    call result, updating arithmetic dest types to `WideInt`.  Also repairs the
///    fixed-size buffer and index constants that `AsTextLowerer` inlined for the
///    `as-text` digit-extraction loop, so decimal printing stays correct.
/// 6. **Set `memo_cache_size`** on the upgraded function so `MemoPass` allocates
///    exactly `max_n + 2` cache entries (indices 0 … max_n plus one safety slot)
///    — keeping the arena footprint well under the 1 MB limit even for 128-byte
///    `WideInt(1024)` entries.
///
/// # Why Native LLVM Wide Integers
/// No external BigInt library is required.  LLVM's `iN` type supports
/// arbitrary integer widths; add/sub/mul are lowered by the backend directly,
/// and div/mod on widths > 128 bits are handled by `WideDivLegalizationPass`
/// which emits a software long-division helper.
///
/// # Pattern Used: Pipeline Pass (Pure Function over Value Types)
/// `IntegerUpgradePass::run` consumes a `MirProgram` and returns a transformed
/// one with no shared mutable state.
use std::collections::{HashMap, HashSet};

use crate::domain::entities::mir::{
    BasicBlock, MirBinOp, MirFunction, MirInstruction, MirLiteral, MirOperand, MirProgram,
    MirTerminator,
};
use crate::domain::entities::types::OnuType;

pub struct IntegerUpgradePass;

// Buffer / index constants emitted by AsTextLowerer for I64 input.
// These must be updated when we widen the digit-extraction loop.
const AS_TEXT_OLD_BUF_SIZE: i64 = 32;
const AS_TEXT_OLD_IDX_MAX: i64 = 30;

impl IntegerUpgradePass {
    pub fn run(program: MirProgram) -> MirProgram {
        // Step 1: find functions that need upgrading and the WideInt width to use.
        let upgrades: Vec<(String, u32, usize)> = program
            .functions
            .iter()
            .filter(|f| Self::is_upgrade_candidate(f))
            .filter_map(|f| {
                let max_n = Self::find_max_literal_call_arg(&program, &f.name);
                if max_n > 92 {
                    let bits = Self::required_bits(max_n);
                    Some((f.name.clone(), bits, max_n as usize))
                } else {
                    None
                }
            })
            .collect();

        if upgrades.is_empty() {
            return program;
        }

        let mut functions = program.functions;
        for (fn_name, bits, max_n) in &upgrades {
            let wide_type = OnuType::WideInt(*bits);
            // 2a. Upgrade the target function itself.
            functions = Self::upgrade_function_body(functions, fn_name, *bits, &wide_type, *max_n);
            // 2b. Upgrade every caller (forward propagation + buffer fix).
            functions = Self::upgrade_callers(functions, fn_name, *bits, &wide_type);
        }

        MirProgram { functions }
    }

    // -------------------------------------------------------------------------
    // Candidate detection
    // -------------------------------------------------------------------------

    fn is_upgrade_candidate(func: &MirFunction) -> bool {
        func.is_pure_data_leaf
            && func.diminishing.is_some()
            && func.args.len() == 1
            && func.args[0].typ == OnuType::I64
            && func.return_type == OnuType::I64
            && Self::count_recursive_calls(func) >= 2
    }

    fn count_recursive_calls(func: &MirFunction) -> usize {
        func.blocks
            .iter()
            .flat_map(|b| b.instructions.iter())
            .filter(|inst| {
                matches!(inst, MirInstruction::Call { name, .. } if name == &func.name)
            })
            .count()
    }

    fn find_max_literal_call_arg(program: &MirProgram, fn_name: &str) -> i64 {
        // Build a constant-propagation map: SSA → constant value (for assignments
        // of the form `dest = Constant(I64(n))`).  This handles the common pattern
        // where the Onu compiler first assigns a literal to a variable and then
        // passes that variable as a function argument.
        let mut const_map: HashMap<usize, i64> = HashMap::new();
        for func in &program.functions {
            for block in &func.blocks {
                for inst in &block.instructions {
                    if let MirInstruction::Assign {
                        dest,
                        src: MirOperand::Constant(MirLiteral::I64(v)),
                    } = inst
                    {
                        const_map.insert(*dest, *v);
                    }
                }
            }
        }

        let mut max = 0i64;
        for func in &program.functions {
            for block in &func.blocks {
                for inst in &block.instructions {
                    if let MirInstruction::Call { name, args, .. } = inst {
                        if name == fn_name {
                            for arg in args {
                                let n = match arg {
                                    MirOperand::Constant(MirLiteral::I64(n)) => Some(*n),
                                    MirOperand::Variable(ssa, _) => const_map.get(ssa).copied(),
                                    _ => None,
                                };
                                if let Some(n) = n {
                                    if n > max {
                                        max = n;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        max
    }

    /// Number of bits required to represent fib(max_n) exactly.
    ///
    /// fib(n) ≈ φⁿ / √5, so log₂(fib(n)) ≈ n · log₂(φ) ≈ n · 0.69424.
    /// We add 4 guard bits and round to the next multiple of 64.
    ///
    /// Intermediate arithmetic uses `u64` to prevent `raw + 63` from
    /// overflowing `u32` when `max_n` is very large.  The result is capped
    /// at the largest multiple of 64 that fits in `u32`.
    fn required_bits(max_n: i64) -> u32 {
        const MAX_BITS: u64 = (u32::MAX as u64 / 64) * 64; // largest multiple of 64 in u32
        let raw = (max_n.max(0) as f64 * 0.69424 + 4.0).ceil() as u64;
        let rounded = ((raw + 63) / 64) * 64;
        rounded.min(MAX_BITS) as u32
    }

    // -------------------------------------------------------------------------
    // Upgrade the doubly-recursive function itself
    // -------------------------------------------------------------------------

    fn upgrade_function_body(
        functions: Vec<MirFunction>,
        fn_name: &str,
        bits: u32,
        wide_type: &OnuType,
        max_n: usize,
    ) -> Vec<MirFunction> {
        functions
            .into_iter()
            .map(|mut func| {
                if func.name != fn_name {
                    return func;
                }

                // Change declared return type.
                func.return_type = wide_type.clone();

                // Tell MemoPass how many cache slots to allocate so the 1 MB
                // arena is not exhausted by 128-byte WideInt(1024) entries.
                // cache_size = max_n + 2: one entry for index 0 through max_n
                // plus one extra slot as a safety margin.
                func.memo_cache_size = Some(max_n + 2);

                // Backward SSA analysis: collect SSA vars that flow into returns.
                let result_chain = Self::find_result_chain(&func);

                // Rewrite instructions in the result chain.
                func.blocks = func
                    .blocks
                    .into_iter()
                    .map(|mut block| {
                        // Upgrade Return terminators that carry a literal constant.
                        if let MirTerminator::Return(MirOperand::Constant(MirLiteral::I64(v))) =
                            block.terminator
                        {
                            block.terminator = MirTerminator::Return(MirOperand::Constant(
                                MirLiteral::WideInt(v.to_string(), bits),
                            ));
                        }

                        block.instructions = block
                            .instructions
                            .into_iter()
                            .map(|inst| match inst {
                                // Base-case literals (0, 1) → WideInt constants.
                                MirInstruction::Assign {
                                    dest,
                                    src: MirOperand::Constant(MirLiteral::I64(v)),
                                } if result_chain.contains(&dest) => MirInstruction::Assign {
                                    dest,
                                    src: MirOperand::Constant(MirLiteral::WideInt(
                                        v.to_string(),
                                        bits,
                                    )),
                                },

                                // Arithmetic on the result chain → WideInt.
                                MirInstruction::BinaryOperation {
                                    dest,
                                    op,
                                    lhs,
                                    rhs,
                                    dest_type: OnuType::I64,
                                } if result_chain.contains(&dest)
                                    && !Self::is_comparison(&op) =>
                                {
                                    MirInstruction::BinaryOperation {
                                        dest,
                                        op,
                                        lhs,
                                        rhs,
                                        dest_type: wide_type.clone(),
                                    }
                                }

                                // Recursive calls → WideInt return type.
                                MirInstruction::Call {
                                    dest,
                                    name,
                                    args,
                                    return_type: OnuType::I64,
                                    arg_types,
                                    is_tail_call,
                                } if name == fn_name && result_chain.contains(&dest) => {
                                    MirInstruction::Call {
                                        dest,
                                        name,
                                        args,
                                        return_type: wide_type.clone(),
                                        arg_types,
                                        is_tail_call,
                                    }
                                }

                                inst => inst,
                            })
                            .collect();

                        block
                    })
                    .collect();

                func
            })
            .collect()
    }

    /// Backward BFS from all `Return` terminators to build the set of SSA vars
    /// whose values flow into the function's return.
    fn find_result_chain(func: &MirFunction) -> HashSet<usize> {
        // Map: ssa_var → instruction that defines it.
        let mut def_map: HashMap<usize, &MirInstruction> = HashMap::new();
        for block in &func.blocks {
            for inst in &block.instructions {
                match inst {
                    MirInstruction::Assign { dest, .. }
                    | MirInstruction::BinaryOperation { dest, .. }
                    | MirInstruction::Call { dest, .. } => {
                        def_map.insert(*dest, inst);
                    }
                    _ => {}
                }
            }
        }

        let mut chain: HashSet<usize> = HashSet::new();
        let mut worklist: Vec<usize> = Vec::new();

        for block in &func.blocks {
            if let MirTerminator::Return(MirOperand::Variable(ssa, _)) = block.terminator {
                if chain.insert(ssa) {
                    worklist.push(ssa);
                }
            }
        }

        while let Some(ssa) = worklist.pop() {
            if let Some(inst) = def_map.get(&ssa) {
                match inst {
                    MirInstruction::Assign {
                        src: MirOperand::Variable(x, _),
                        ..
                    } => {
                        if chain.insert(*x) {
                            worklist.push(*x);
                        }
                    }
                    MirInstruction::BinaryOperation { op, lhs, rhs, .. }
                        if !Self::is_comparison(op) =>
                    {
                        if let MirOperand::Variable(x, _) = lhs {
                            if chain.insert(*x) {
                                worklist.push(*x);
                            }
                        }
                        if let MirOperand::Variable(x, _) = rhs {
                            if chain.insert(*x) {
                                worklist.push(*x);
                            }
                        }
                    }
                    // A Call result is a leaf; don't recurse into its arguments.
                    _ => {}
                }
            }
        }

        chain
    }

    // -------------------------------------------------------------------------
    // Upgrade callers (forward propagation + as-text buffer repair)
    // -------------------------------------------------------------------------

    fn upgrade_callers(
        functions: Vec<MirFunction>,
        fn_name: &str,
        bits: u32,
        wide_type: &OnuType,
    ) -> Vec<MirFunction> {
        // Maximum decimal digits for WideInt(bits): ceil(bits * log10(2)) + 1.
        let max_digits = (bits as f64 * 0.30103).ceil() as i64 + 1;
        let new_buf_size = max_digits + 8; // Safety margin.
        let new_idx_max = new_buf_size - 2;

        functions
            .into_iter()
            .map(|func| {
                if func.name == fn_name {
                    return func; // Don't reprocess the target.
                }
                // Check if this function calls fn_name at all.
                let calls_target = func.blocks.iter().any(|b| {
                    b.instructions.iter().any(|inst| {
                        matches!(inst, MirInstruction::Call { name, .. } if name == fn_name)
                    })
                });
                if !calls_target {
                    return func;
                }
                Self::upgrade_caller(func, fn_name, bits, wide_type, new_buf_size, new_idx_max)
            })
            .collect()
    }

    fn upgrade_caller(
        mut func: MirFunction,
        fn_name: &str,
        bits: u32,
        wide_type: &OnuType,
        new_buf_size: i64,
        new_idx_max: i64,
    ) -> MirFunction {
        // --- Pass 1: identify upgraded SSAs and as-text buffer/idx SSAs. ---
        // We do a forward scan collecting:
        //   upgraded_ssas : SSA vars whose value is now WideInt
        //   buf_ssas      : Alloc dest used as as-text buffer (immediately before val_ssa assign)
        //   idx_ssas      : Assign(AS_TEXT_OLD_IDX_MAX) immediately after val_ssa assign

        let mut upgraded_ssas: HashSet<usize> = HashSet::new();
        let mut buf_ssas: HashSet<usize> = HashSet::new();
        let mut idx_ssas: HashSet<usize> = HashSet::new();

        // First, seed upgraded_ssas with direct call results.
        for block in &func.blocks {
            for inst in &block.instructions {
                if let MirInstruction::Call {
                    dest,
                    name,
                    return_type: OnuType::I64,
                    ..
                } = inst
                {
                    if name == fn_name {
                        upgraded_ssas.insert(*dest);
                    }
                }
            }
        }

        // Then find as-text-related buf_ssa / idx_ssa using the consecutive pattern:
        //   [i-1]  Alloc { dest: buf_ssa, size: AS_TEXT_OLD_BUF_SIZE }
        //   [i]    Assign { dest: val_ssa, src: Variable(result_ssa ∈ upgraded) }
        //   [i+1]  Assign { dest: idx_ssa, src: Constant(AS_TEXT_OLD_IDX_MAX) }
        for block in &func.blocks {
            let insts = &block.instructions;
            for i in 0..insts.len() {
                if let MirInstruction::Assign {
                    dest: val_ssa,
                    src: MirOperand::Variable(y, _),
                } = &insts[i]
                {
                    if upgraded_ssas.contains(y) {
                        upgraded_ssas.insert(*val_ssa);

                        // Look backward for Alloc(AS_TEXT_OLD_BUF_SIZE).
                        if i > 0 {
                            if let MirInstruction::Alloc {
                                dest: buf,
                                size_bytes:
                                    MirOperand::Constant(MirLiteral::I64(s)),
                            } = &insts[i - 1]
                            {
                                if *s == AS_TEXT_OLD_BUF_SIZE {
                                    buf_ssas.insert(*buf);
                                }
                            }
                        }

                        // Look forward for Assign(AS_TEXT_OLD_IDX_MAX).
                        if i + 1 < insts.len() {
                            if let MirInstruction::Assign {
                                dest: idx,
                                src: MirOperand::Constant(MirLiteral::I64(v)),
                            } = &insts[i + 1]
                            {
                                if *v == AS_TEXT_OLD_IDX_MAX {
                                    idx_ssas.insert(*idx);
                                }
                            }
                        }
                    }
                }
            }
        }

        // --- Pass 2: forward propagation of WideInt through arithmetic. ---
        // We iterate until stable (fixed-point).
        loop {
            let before = upgraded_ssas.len();
            for block in &func.blocks {
                for inst in &block.instructions {
                    match inst {
                        MirInstruction::Assign {
                            dest,
                            src: MirOperand::Variable(x, _),
                        } if upgraded_ssas.contains(x) => {
                            upgraded_ssas.insert(*dest);
                        }
                        MirInstruction::BinaryOperation { dest, op, lhs, rhs, .. }
                            if !Self::is_comparison(op) =>
                        {
                            let lhs_wide = if let MirOperand::Variable(x, _) = lhs {
                                upgraded_ssas.contains(x)
                            } else {
                                false
                            };
                            let rhs_wide = if let MirOperand::Variable(x, _) = rhs {
                                upgraded_ssas.contains(x)
                            } else {
                                false
                            };
                            if lhs_wide || rhs_wide {
                                upgraded_ssas.insert(*dest);
                            }
                        }
                        _ => {}
                    }
                }
            }
            if upgraded_ssas.len() == before {
                break;
            }
        }

        // --- Pass 3: rewrite instructions. ---
        func.blocks = func
            .blocks
            .into_iter()
            .map(|mut block| {
                block.instructions = block
                    .instructions
                    .into_iter()
                    .map(|inst| match inst {
                        // Upgrade the call return type.
                        MirInstruction::Call {
                            dest,
                            name,
                            args,
                            return_type: OnuType::I64,
                            arg_types,
                            is_tail_call,
                        } if name == fn_name => MirInstruction::Call {
                            dest,
                            name,
                            args,
                            return_type: wide_type.clone(),
                            arg_types,
                            is_tail_call,
                        },

                        // Upgrade arithmetic operations whose dest ended up in
                        // the upgraded set (result chain from the wide call).
                        MirInstruction::BinaryOperation {
                            dest,
                            op,
                            lhs,
                            rhs,
                            dest_type: OnuType::I64,
                        } if upgraded_ssas.contains(&dest) && !Self::is_comparison(&op) => {
                            MirInstruction::BinaryOperation {
                                dest,
                                op,
                                lhs,
                                rhs,
                                dest_type: wide_type.clone(),
                            }
                        }

                        // Fix as-text buffer allocation: grow to fit max_digits.
                        MirInstruction::Alloc {
                            dest,
                            size_bytes: MirOperand::Constant(MirLiteral::I64(s)),
                        } if buf_ssas.contains(&dest) && s == AS_TEXT_OLD_BUF_SIZE => {
                            MirInstruction::Alloc {
                                dest,
                                size_bytes: MirOperand::Constant(MirLiteral::I64(new_buf_size)),
                            }
                        }

                        // Fix initial idx assignment (start writing from near end of buffer).
                        MirInstruction::Assign {
                            dest,
                            src: MirOperand::Constant(MirLiteral::I64(v)),
                        } if idx_ssas.contains(&dest) && v == AS_TEXT_OLD_IDX_MAX => {
                            MirInstruction::Assign {
                                dest,
                                src: MirOperand::Constant(MirLiteral::I64(new_idx_max)),
                            }
                        }

                        // Fix the zero-case idx assignment (AS_TEXT_OLD_IDX_MAX - 1).
                        MirInstruction::Assign {
                            dest,
                            src: MirOperand::Constant(MirLiteral::I64(v)),
                        } if idx_ssas.contains(&dest) && v == AS_TEXT_OLD_IDX_MAX - 1 => {
                            MirInstruction::Assign {
                                dest,
                                src: MirOperand::Constant(MirLiteral::I64(new_idx_max - 1)),
                            }
                        }

                        // Fix the zero-case PointerOffset(buf_ssa, AS_TEXT_OLD_IDX_MAX).
                        MirInstruction::PointerOffset {
                            dest,
                            ptr,
                            offset: MirOperand::Constant(MirLiteral::I64(v)),
                        } if v == AS_TEXT_OLD_IDX_MAX => {
                            // Only fix if the ptr refers to a known buf_ssa.
                            let is_buf_ptr = if let MirOperand::Variable(x, _) = &ptr {
                                buf_ssas.contains(x)
                            } else {
                                false
                            };
                            if is_buf_ptr {
                                MirInstruction::PointerOffset {
                                    dest,
                                    ptr,
                                    offset: MirOperand::Constant(MirLiteral::I64(new_idx_max)),
                                }
                            } else {
                                MirInstruction::PointerOffset {
                                    dest,
                                    ptr,
                                    offset: MirOperand::Constant(MirLiteral::I64(v)),
                                }
                            }
                        }

                        // Fix the result-length subtraction: Sub(AS_TEXT_OLD_IDX_MAX, idx_ssa).
                        MirInstruction::BinaryOperation {
                            dest,
                            op: MirBinOp::Sub,
                            lhs: MirOperand::Constant(MirLiteral::I64(lv)),
                            rhs,
                            dest_type,
                        } if lv == AS_TEXT_OLD_IDX_MAX => {
                            // Only fix when rhs is one of our known idx_ssa vars.
                            let is_idx_rhs = if let MirOperand::Variable(x, _) = &rhs {
                                idx_ssas.contains(x)
                            } else {
                                false
                            };
                            if is_idx_rhs {
                                MirInstruction::BinaryOperation {
                                    dest,
                                    op: MirBinOp::Sub,
                                    lhs: MirOperand::Constant(MirLiteral::I64(new_idx_max)),
                                    rhs,
                                    dest_type,
                                }
                            } else {
                                MirInstruction::BinaryOperation {
                                    dest,
                                    op: MirBinOp::Sub,
                                    lhs: MirOperand::Constant(MirLiteral::I64(lv)),
                                    rhs,
                                    dest_type,
                                }
                            }
                        }

                        inst => inst,
                    })
                    .collect();
                block
            })
            .collect();

        func
    }

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    fn is_comparison(op: &MirBinOp) -> bool {
        matches!(op, MirBinOp::Eq | MirBinOp::Ne | MirBinOp::Gt | MirBinOp::Lt)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::mir::{BasicBlock, MirArgument, MirTerminator};

    fn fib_naive_mir() -> MirFunction {
        // A minimal fib-naive skeleton:
        //   Block 0: check = n==0 → CondBranch(check, base0, recurse)
        //   Block base0: ret = Assign(I64(0)), Return(ret)
        //   Block recurse: left=Call(fib-naive,[n-1]), right=Call(fib-naive,[n-2]),
        //                  res=Add(left,right), Return(res)
        MirFunction {
            name: "fib-naive".to_string(),
            args: vec![MirArgument {
                name: "n".to_string(),
                typ: OnuType::I64,
                ssa_var: 0,
            }],
            return_type: OnuType::I64,
            is_pure_data_leaf: true,
            diminishing: Some("n".to_string()),
            memo_cache_size: None,
            blocks: vec![
                BasicBlock {
                    id: 0,
                    instructions: vec![MirInstruction::BinaryOperation {
                        dest: 10,
                        op: MirBinOp::Eq,
                        lhs: MirOperand::Variable(0, false),
                        rhs: MirOperand::Constant(MirLiteral::I64(0)),
                        dest_type: OnuType::Boolean,
                    }],
                    terminator: MirTerminator::CondBranch {
                        condition: MirOperand::Variable(10, false),
                        then_block: 1,
                        else_block: 2,
                    },
                },
                BasicBlock {
                    id: 1,
                    instructions: vec![MirInstruction::Assign {
                        dest: 11,
                        src: MirOperand::Constant(MirLiteral::I64(0)),
                    }],
                    terminator: MirTerminator::Return(MirOperand::Variable(11, false)),
                },
                BasicBlock {
                    id: 2,
                    instructions: vec![
                        MirInstruction::Call {
                            dest: 12,
                            name: "fib-naive".to_string(),
                            args: vec![MirOperand::Variable(0, false)],
                            return_type: OnuType::I64,
                            arg_types: vec![OnuType::I64],
                            is_tail_call: false,
                        },
                        MirInstruction::Call {
                            dest: 13,
                            name: "fib-naive".to_string(),
                            args: vec![MirOperand::Variable(0, false)],
                            return_type: OnuType::I64,
                            arg_types: vec![OnuType::I64],
                            is_tail_call: false,
                        },
                        MirInstruction::BinaryOperation {
                            dest: 14,
                            op: MirBinOp::Add,
                            lhs: MirOperand::Variable(12, false),
                            rhs: MirOperand::Variable(13, false),
                            dest_type: OnuType::I64,
                        },
                    ],
                    terminator: MirTerminator::Return(MirOperand::Variable(14, false)),
                },
            ],
        }
    }

    fn run_func_with_call(n: i64) -> MirFunction {
        MirFunction {
            name: "run".to_string(),
            args: vec![],
            return_type: OnuType::I64,
            is_pure_data_leaf: false,
            diminishing: None,
            memo_cache_size: None,
            blocks: vec![BasicBlock {
                id: 0,
                instructions: vec![MirInstruction::Call {
                    dest: 20,
                    name: "fib-naive".to_string(),
                    args: vec![MirOperand::Constant(MirLiteral::I64(n))],
                    return_type: OnuType::I64,
                    arg_types: vec![OnuType::I64],
                    is_tail_call: false,
                }],
                terminator: MirTerminator::Return(MirOperand::Variable(20, false)),
            }],
        }
    }

    #[test]
    fn test_no_upgrade_for_small_n() {
        let prog = MirProgram {
            functions: vec![fib_naive_mir(), run_func_with_call(92)],
        };
        let after = IntegerUpgradePass::run(prog);
        // n=92 fits in i64, no upgrade expected.
        assert_eq!(
            after.functions[0].return_type,
            OnuType::I64,
            "Should not upgrade when n<=92"
        );
    }

    #[test]
    fn test_upgrade_for_large_n() {
        let prog = MirProgram {
            functions: vec![fib_naive_mir(), run_func_with_call(1000)],
        };
        let after = IntegerUpgradePass::run(prog);
        let fib_fn = after.functions.iter().find(|f| f.name == "fib-naive").unwrap();
        assert!(
            matches!(fib_fn.return_type, OnuType::WideInt(_)),
            "Should upgrade to WideInt for n=1000, got {:?}",
            fib_fn.return_type
        );
    }

    #[test]
    fn test_caller_call_site_upgraded() {
        let prog = MirProgram {
            functions: vec![fib_naive_mir(), run_func_with_call(1000)],
        };
        let after = IntegerUpgradePass::run(prog);
        let run_fn = after.functions.iter().find(|f| f.name == "run").unwrap();
        let call_inst = run_fn.blocks[0].instructions.iter().find(|i| {
            matches!(i, MirInstruction::Call { name, .. } if name == "fib-naive")
        });
        assert!(call_inst.is_some(), "Call to fib-naive should still exist");
        if let Some(MirInstruction::Call { return_type, .. }) = call_inst {
            assert!(
                matches!(return_type, OnuType::WideInt(_)),
                "Call site return_type should be WideInt, got {:?}",
                return_type
            );
        }
    }

    #[test]
    fn test_memo_cache_size_set() {
        let prog = MirProgram {
            functions: vec![fib_naive_mir(), run_func_with_call(500)],
        };
        let after = IntegerUpgradePass::run(prog);
        let fib_fn = after.functions.iter().find(|f| f.name == "fib-naive").unwrap();
        assert_eq!(fib_fn.memo_cache_size, Some(502), "cache_size = max_n + 2 = 502");
    }

    #[test]
    fn test_required_bits_1000() {
        let bits = IntegerUpgradePass::required_bits(1000);
        // 1000 * 0.69424 + 4 = 698.24, ceil = 699, round to 64 multiple → 704
        assert_eq!(bits, 704, "fib(1000) should need 704 bits");
    }

    #[test]
    fn test_required_bits_no_overflow_large_input() {
        // Ensure that very large max_n values (near i64::MAX) do not cause an
        // arithmetic overflow.  Previously `raw + 63` could overflow u32; now
        // the calculation uses u64 and caps at the largest u32-safe multiple of 64.
        let bits = IntegerUpgradePass::required_bits(i64::MAX);
        // The result must be a multiple of 64 and fit in u32.
        assert_eq!(bits % 64, 0);
        assert!(bits as u64 <= (u32::MAX as u64 / 64) * 64);
    }

    #[test]
    fn test_required_bits_near_u32_max_raw() {
        // max_n just large enough that raw (before rounding) would exceed
        // u32::MAX if widening were not applied.  Verify no panic and that the
        // result is a valid multiple of 64 within u32 range.
        // u32::MAX / 0.69424 ≈ 6_185_051_666 fits in i64.
        let max_n = (u32::MAX as f64 / 0.69424) as i64 + 1;
        let bits = IntegerUpgradePass::required_bits(max_n);
        assert_eq!(bits % 64, 0);
        assert!(bits as u64 <= (u32::MAX as u64 / 64) * 64);
    }
}
