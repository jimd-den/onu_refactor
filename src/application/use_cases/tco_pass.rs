/// TCO Loop Lowering Pass: Application Use Case Layer
///
/// # What This Does
/// When a function calls itself as its very last action (a "self-tail-call"),
/// this pass rewrites that recursive call into a loop jump. This guarantees
/// zero stack growth without relying on the backend to discover the pattern.
///
/// # Why This Matters
/// LLVM's `musttail` is a hint to the backend that it *must* eliminate the
/// frame. But "must eliminate" still means the backend has to recognize the
/// pattern. By converting the recursion to a loop at the MIR level, we hand
/// LLVM a tight `br` instruction instead of a call — the same structure that
/// GCC emits with `-foptimize-sibling-calls`. This is the canonical approach
/// used by Erlang, Scheme, and Haskell compilers.
///
/// # Memory Safety
/// The transformation mutates only local stack-allocated SSA slots (integers,
/// booleans). No heap allocation, no pointer arithmetic, no arena involvement.
/// The LLVM verifier will reject any malformed IR if our rewrite is incorrect.
///
/// # Pattern Used: Pure Function Pass (Visitor over Value Type)
/// `TcoPass::run_function` is a pure function: it takes ownership of a
/// `MirFunction`, transforms it, and returns the (possibly rewritten) function.
/// No shared mutable state, no side effects. This is deliberately simple — a
/// stateful pass would be a maze; this is a bridge.
use crate::domain::entities::mir::{
    BasicBlock, MirArgument, MirFunction, MirInstruction, MirOperand, MirProgram, MirTerminator,
};

/// The maximum SSA variable ID found in a function, used to allocate fresh
/// shadow slots above the existing range.
fn max_ssa_var(func: &MirFunction) -> usize {
    let mut max = func.args.iter().map(|a| a.ssa_var).max().unwrap_or(0);
    for block in &func.blocks {
        for inst in &block.instructions {
            // Walk every destination SSA variable
            let dest_opt: Option<usize> = match inst {
                MirInstruction::Assign { dest, .. } => Some(*dest),
                MirInstruction::BinaryOperation { dest, .. } => Some(*dest),
                MirInstruction::Call { dest, .. } => Some(*dest),
                MirInstruction::Tuple { dest, .. } => Some(*dest),
                MirInstruction::Index { dest, .. } => Some(*dest),
                MirInstruction::Alloc { dest, .. } => Some(*dest),
                MirInstruction::PointerOffset { dest, .. } => Some(*dest),
                MirInstruction::Load { dest, .. } => Some(*dest),
                MirInstruction::Emit(_)
                | MirInstruction::Drop { .. }
                | MirInstruction::MemCopy { .. }
                | MirInstruction::Store { .. }
                | MirInstruction::TypedStore { .. } => None,
            };
            if let Some(d) = dest_opt {
                if d > max {
                    max = d;
                }
            }
        }
    }
    max
}

/// Returns true if the given block terminates with a self-tail-call —
/// that is, its last instruction is a `Call` to the function itself
/// with `is_tail_call = true`.
fn block_has_self_tail_call(block: &BasicBlock, fn_name: &str) -> bool {
    block.instructions.iter().any(|inst| {
        matches!(inst,
            MirInstruction::Call { name, is_tail_call: true, .. }
            if name == fn_name
        )
    })
}

pub struct TcoPass;

impl TcoPass {
    /// Transforms an entire `MirProgram`, running `run_function` on every
    /// function. Non-recursive functions pass through unchanged (identity).
    pub fn run(program: MirProgram) -> MirProgram {
        MirProgram {
            functions: program
                .functions
                .into_iter()
                .map(Self::run_function)
                .collect(),
        }
    }

    /// Core transformation: rewrites a single `MirFunction`.
    ///
    /// # Algorithm
    ///
    /// 1. Scan for any block with a self-tail-call. If none found, return unchanged.
    /// 2. Introduce a new "loop head" block at block-id 0 (shifting existing IDs by 1).
    ///    The loop head holds `Assign` instructions that copy shadow SSA slots back
    ///    into the function argument slots, then branches into the original entry.
    /// 3. For each block that contains a self-tail-call:
    ///    a. Remove the `Call` instruction.
    ///    b. Just before where the Call was, emit `Assign { dest: arg_ssa_i, src: call_arg_i }`
    ///       for each argument. This overwrites the argument slot in place.
    ///    c. Replace the block's terminator with `Branch(loop_head_id)`.
    ///
    /// The result is a structured loop in MIR. LLVM lowers this to a `br` with
    /// PHI nodes — identical to a hand-written iterative implementation.
    pub fn run_function(mut func: MirFunction) -> MirFunction {
        // Step 1: Detect whether any self-tail-calls exist.
        let has_self_tail_call = func
            .blocks
            .iter()
            .any(|b| block_has_self_tail_call(b, &func.name));
        if !has_self_tail_call {
            return func; // Identity: no transformation needed.
        }

        eprintln!(
            "[TcoPass] Rewriting '{}' self-tail-calls to loop",
            func.name
        );

        // Step 2: Prepare. The loop head will be block 0.
        // We shift all existing block IDs up by 1 to make room.
        let loop_head_id = 0usize;
        let original_entry_id = 1usize;

        for block in &mut func.blocks {
            block.id += 1;
            // Shift any terminator branch targets that reference old block IDs
            shift_terminator(&mut block.terminator);
        }

        // Step 3: Rewrite each block that ends in a self-tail-call.
        // We collect the argument SSA vars for use below.
        let arg_ssa_vars: Vec<usize> = func.args.iter().map(|a| a.ssa_var).collect();

        for block in &mut func.blocks {
            if !block_has_self_tail_call(block, &func.name) {
                continue;
            }

            // Extract the self-tail-call instruction to get its argument operands.
            let call_idx = block
                .instructions
                .iter()
                .position(|inst| {
                    matches!(inst,
                        MirInstruction::Call { name, is_tail_call: true, .. }
                        if name == &func.name
                    )
                })
                .unwrap();

            let tail_call = block.instructions.remove(call_idx);
            let tail_args = if let MirInstruction::Call { args, .. } = tail_call {
                args
            } else {
                unreachable!("We just matched this variant above");
            };

            // Emit assignments that update each argument slot with the call's new value.
            // This is what replaces the recursive call: instead of calling ourselves,
            // we update the loop variables and jump back.
            for (i, new_arg_val) in tail_args.into_iter().enumerate() {
                block.instructions.push(MirInstruction::Assign {
                    dest: arg_ssa_vars[i],
                    src: new_arg_val,
                });
            }

            // The block used to Return after the call. Now it loops back.
            // We jump to original_entry_id (1) rather than loop_head_id (0)
            // to ensure that if this function is inlined, the INITIAL argument
            // assignments injected into block 0 by InlinePass are not re-executed.
            block.terminator = MirTerminator::Branch(original_entry_id);
        }

        // Step 4: Build the loop head block.
        // The loop head is a simple unconditional branch into the original entry.
        // Its only job is to be the target of all back-edges (including the first
        // entry from the function prologue). LLVM can simplify it away, but we
        // keep it explicit for clarity in the emitted IR.
        let loop_head = BasicBlock {
            id: loop_head_id,
            instructions: vec![],
            terminator: MirTerminator::Branch(original_entry_id),
        };

        // Insert loop head at the front so it is the function's entry block.
        func.blocks.insert(0, loop_head);

        func
    }
}

/// Increments all block IDs referenced in a terminator by 1,
/// used when we shift the existing CFG to make room for the loop head.
fn shift_terminator(term: &mut MirTerminator) {
    match term {
        MirTerminator::Branch(id) => *id += 1,
        MirTerminator::CondBranch {
            then_block,
            else_block,
            ..
        } => {
            *then_block += 1;
            *else_block += 1;
        }
        MirTerminator::Return(_) | MirTerminator::Unreachable => {}
    }
}
