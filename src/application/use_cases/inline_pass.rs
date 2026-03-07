/// Inline Pass: Application Use Case Layer
///
/// # What This Does
/// Expands `is_pure_data_leaf` function bodies directly at their call sites
/// in the MIR, before codegen. When a function A calls a pure function B,
/// B's instructions are copied into A's basic blocks, eliminating the function
/// call overhead and enabling LLVM to apply whole-loop optimizations across
/// the fused body.
///
/// # Why This Matters
/// LLVM's `alwaysinline` attribute is advisory — the inliner uses a cost model
/// that can refuse to inline even marked functions when calling conventions or
/// instruction counts trigger its thresholds. By inlining at the MIR level, we
/// guarantee the expansion regardless of LLVM's heuristics.
///
/// After InlinePass, the TcoPass can then loop-lower any new self-tail-calls
/// that appear in the merged function body, producing a single tight loop.
///
/// # Safety Guarantees
/// - Only pure leaf functions (`is_pure_data_leaf = true`) are expanded.
/// - Functions with `Drop`, `Emit`, `Alloc`, or `Store` are never inlined
///   (the `is_pure_data_leaf` flag is already false for those).
/// - SSA variables are remapped to avoid conflicts with the caller's namespace.
/// - Block IDs are remapped to avoid conflicts with the caller's CFG.
///
/// # Pattern Used: Pipeline Pass (Pure Function over Value Types)
/// `InlinePass::run` is a pure function: it consumes a `MirProgram` and
/// returns a transformed one. No shared state, no side effects.
use crate::domain::entities::mir::{
    BasicBlock, MirFunction, MirInstruction, MirOperand, MirProgram, MirTerminator,
};
use std::collections::HashMap;

pub struct InlinePass;

impl InlinePass {
    /// Entry point: transform an entire program.
    /// Non-inlineable calls pass through unchanged (identity for those sites).
    pub fn run(program: MirProgram) -> MirProgram {
        // Step 1: Seed the inlineable set with directly-marked pure leaf functions.
        // Transitive purity: a function is also inlineable if every non-extern call it
        // makes targets another inlineable function. We propagate until stable (fixed-point).
        //
        // Example: collatz-steps calls stdlib (resolved to BinaryOps, not Calls) and calls
        // itself (self-recursion). If it were called by collatz-range, and collatz-steps
        // is inlineable, then collatz-range can safely inline it.
        let all_fn_names: std::collections::HashSet<String> =
            program.functions.iter().map(|f| f.name.clone()).collect();

        // Collect names of functions in the module that are not safe to inline
        // (have observable side effects even after purity audit).
        let mut inlineable: std::collections::HashSet<String> = program
            .functions
            .iter()
            .filter(|f| {
                if !f.is_pure_data_leaf {
                    return false; // Not pure — hard no.
                }
                // Exclude functions that call themselves.
                // A self-recursive function cannot be expanded at its own call sites
                // because the expansion itself contains more self-calls, leading to
                // infinite expansion at compile time.
                // EXCEPTION: if TcoPass has already loop-lowered the function (converted
                // the self-call into a backward Branch), the body is finite and safe.
                // That case is handled at the per-call-site level in `inline_into`.
                let is_self_recursive = f.blocks.iter().any(|b| {
                    b.instructions.iter().any(
                        |inst| matches!(inst, MirInstruction::Call { name, .. } if name == &f.name),
                    )
                });
                !is_self_recursive
            })
            .map(|f| f.name.clone())
            .collect();

        // Iteratively expand: a function is inlineable if:
        //   1. It is not already known impure (has no Alloc/Emit/Store/Drop).
        //   2. Every Call it makes is either to a function outside the module (extern)
        //      or to an already-inlineable in-module function.
        //
        // This is a fixed-point iteration (worklist). It terminates because each
        // iteration can only ADD entries to `inlineable`, never remove them.
        loop {
            let mut changed = false;
            for func in &program.functions {
                if inlineable.contains(&func.name) {
                    continue; // Already accepted.
                }
                // Hard guard 1: visible side effects — never inline.
                let has_side_effects = func.blocks.iter().any(|b| {
                    b.instructions.iter().any(|inst| {
                        matches!(inst, MirInstruction::Emit(_) | MirInstruction::Store { .. })
                    })
                });
                if has_side_effects {
                    continue;
                }
                // Hard guard 2: self-recursive functions cannot be inlined safely.
                // Expanding a self-recursive call would produce another self-call that
                // also needs expanding — leading to infinite expansion at compile time.
                // Loop-lowered functions (where TcoPass replaced the self-call with a Branch)
                // are safe, and those are handled per-call-site in `inline_into`, not here.
                let is_self_recursive = func.blocks.iter().any(|b| {
                    b.instructions.iter().any(|inst| {
                        matches!(inst, MirInstruction::Call { name, .. } if name == &func.name)
                    })
                });
                if is_self_recursive {
                    continue;
                }
                // Check: every in-module call target must already be inlineable.
                let all_calls_safe = func.blocks.iter().all(|b| {
                    b.instructions.iter().all(|inst| {
                        if let MirInstruction::Call { name, .. } = inst {
                            // In-module calls are only safe if the callee is inlineable.
                            if all_fn_names.contains(name) {
                                return inlineable.contains(name);
                            }
                            // Extern calls (malloc, printf, etc.) are not inlined but are safe.
                            true
                        } else {
                            true
                        }
                    })
                });
                if all_calls_safe {
                    inlineable.insert(func.name.clone());
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        // Build the inline map: only inline in-module pure-or-transitively-pure functions.
        // We exclude self-recursive functions from being inlined into themselves
        // (TcoPass handles self-recursion separately).
        let pure_functions: HashMap<String, MirFunction> = program
            .functions
            .iter()
            .filter(|f| inlineable.contains(&f.name))
            .map(|f| (f.name.clone(), f.clone()))
            .collect();

        eprintln!(
            "[InlinePass] Inlineable functions: {:?}",
            inlineable.iter().collect::<Vec<_>>()
        );

        MirProgram {
            functions: program
                .functions
                .into_iter()
                .map(|f| Self::inline_into(f, &pure_functions))
                .collect(),
        }
    }

    /// Expand all inlineable call sites inside `caller`.
    ///
    /// We process blocks in order. When we find a `Call` instruction whose
    /// target is in `pure_functions`:
    ///   1. Split the block at the call site.
    ///   2. Remap the callee's SSA vars and block IDs above the caller's range.
    ///   3. Bind callee argument SSA vars to the call's arguments via Assign.
    ///   4. Replace callee Return terminators with Branch to the continuation block.
    ///   5. Insert all remapped callee blocks between the split halves.
    fn inline_into(
        mut caller: MirFunction,
        pure_functions: &HashMap<String, MirFunction>,
    ) -> MirFunction {
        // We grow the block list in-place. Process block indices one at a time.
        // Because we may insert new blocks mid-iteration, we use an explicit index.
        let mut block_idx = 0;
        while block_idx < caller.blocks.len() {
            // Find the FIRST inlineable call in this block, if any.
            let call_pos = caller.blocks[block_idx]
                .instructions
                .iter()
                .position(|inst| {
                    if let MirInstruction::Call { name, .. } = inst {
                        if !pure_functions.contains_key(name.as_str()) {
                            return false;
                        }

                        // If it's a call to itself, we can only inline if the callee (itself)
                        // has been loop-lowered. A loop-lowered function will have a Branch
                        // terminator pointing backward (target < current block id) or branching to id 0.
                        // Actually, TcoPass shifts all blocks up and inserts a loop head at id 0.
                        if name == &caller.name {
                            let is_loop_lowered =
                                pure_functions.get(name.as_str()).map_or(false, |f| {
                                    f.blocks.iter().any(|b| {
                                        if let MirTerminator::Branch(target) = b.terminator {
                                            target < b.id
                                        } else {
                                            false
                                        }
                                    })
                                });
                            is_loop_lowered
                        } else {
                            true
                        }
                    } else {
                        false
                    }
                });

            if let Some(call_pos) = call_pos {
                // Extract the call instruction.
                let call_inst = caller.blocks[block_idx].instructions.remove(call_pos);
                let (call_dest, call_name, call_args) =
                    if let MirInstruction::Call {
                        dest, name, args, ..
                    } = call_inst
                    {
                        (dest, name, args)
                    } else {
                        unreachable!()
                    };

                let callee = pure_functions.get(&call_name).unwrap();

                // Compute safe SSA and block ID offsets that clear the caller's namespace.
                let ssa_offset = max_ssa_in_function(&caller) + 1;
                let block_offset = caller.blocks.iter().map(|b| b.id).max().unwrap_or(0) + 1;

                // Continuation block: takes instructions after the call and the original terminator.
                let continuation_id = block_offset + callee.blocks.len();
                let tail_instructions = caller.blocks[block_idx].instructions.split_off(call_pos);
                let original_terminator = std::mem::replace(
                    &mut caller.blocks[block_idx].terminator,
                    MirTerminator::Unreachable,
                );

                let continuation_block = BasicBlock {
                    id: continuation_id,
                    instructions: tail_instructions,
                    terminator: original_terminator,
                };

                // Remap and prepare callee blocks.
                let mut inlined_blocks = remap_callee(
                    callee,
                    &call_args,
                    call_dest,
                    ssa_offset,
                    block_offset,
                    continuation_id,
                );

                // Point the current block end at the callee entry.
                let callee_entry_id = inlined_blocks[0].id;
                caller.blocks[block_idx].terminator = MirTerminator::Branch(callee_entry_id);

                // Insert inlined blocks + continuation after the current block.
                let insert_at = block_idx + 1;
                for (i, block) in inlined_blocks.drain(..).enumerate() {
                    caller.blocks.insert(insert_at + i, block);
                }
                caller
                    .blocks
                    .insert(insert_at + callee.blocks.len(), continuation_block);

                // Do NOT advance block_idx: re-scan the current block for more calls.
                // (The current block's tail instructions moved to continuation, so the
                //  current block is now trivially a Branch — no more calls to find.)
                block_idx += 1;
            } else {
                block_idx += 1;
            }
        }

        caller
    }
}

/// Produce remapped copies of the callee's blocks suitable for insertion into the caller.
///
/// Steps:
/// - Remap all SSA variable IDs: `original + ssa_offset`
/// - Remap all block IDs: `original + block_offset`
/// - Prepend Assign instructions binding callee arg SSA → caller call args.
/// - Replace Return terminators with Assign(call_dest, return_val) + Branch(continuation).
fn remap_callee(
    callee: &MirFunction,
    call_args: &[MirOperand],
    call_dest: usize,
    ssa_offset: usize,
    block_offset: usize,
    continuation_id: usize,
) -> Vec<BasicBlock> {
    let mut blocks = Vec::with_capacity(callee.blocks.len());

    for (block_index, block) in callee.blocks.iter().enumerate() {
        let remapped_id = block.id + block_offset;

        let mut instructions = Vec::new();

        // In the first callee block, bind the callee's argument SSA vars to the
        // call site's actual arguments. This replaces parameter passing.
        //
        // IMPORTANT: `call_args` live in the CALLER's SSA namespace — they must
        // NOT be shifted by ssa_offset.  Only the callee-side destination
        // (arg.ssa_var) gets remapped into the fresh namespace.
        if block_index == 0 {
            for (arg_idx, arg) in callee.args.iter().enumerate() {
                let remapped_arg_ssa = arg.ssa_var + ssa_offset;
                // Clone the caller-side operand verbatim (no remap).
                let src = call_args[arg_idx].clone();
                instructions.push(MirInstruction::Assign {
                    dest: remapped_arg_ssa,
                    src,
                });
            }
        }

        // Remap all instructions, offsetting SSA IDs.
        for inst in &block.instructions {
            instructions.push(remap_instruction(inst, ssa_offset));
        }

        // Remap terminator. Return becomes: Assign(call_dest, return_val) + Branch(continuation).
        let terminator = match &block.terminator {
            MirTerminator::Return(op) => {
                let return_val = remap_operand(op, ssa_offset);
                // Emit the assignment that binds the callee return to the caller's call_dest.
                instructions.push(MirInstruction::Assign {
                    dest: call_dest,
                    src: return_val,
                });
                MirTerminator::Branch(continuation_id)
            }
            MirTerminator::Branch(target) => MirTerminator::Branch(target + block_offset),
            MirTerminator::CondBranch {
                condition,
                then_block,
                else_block,
            } => MirTerminator::CondBranch {
                condition: remap_operand(condition, ssa_offset),
                then_block: then_block + block_offset,
                else_block: else_block + block_offset,
            },
            MirTerminator::Unreachable => MirTerminator::Unreachable,
        };

        blocks.push(BasicBlock {
            id: remapped_id,
            instructions,
            terminator,
        });
    }

    blocks
}

/// Remap a single MIR instruction: shift all destination and source SSA IDs.
fn remap_instruction(inst: &MirInstruction, ssa_offset: usize) -> MirInstruction {
    match inst {
        MirInstruction::Assign { dest, src } => MirInstruction::Assign {
            dest: dest + ssa_offset,
            src: remap_operand(src, ssa_offset),
        },
        MirInstruction::BinaryOperation { dest, op, lhs, rhs, dest_type } => MirInstruction::BinaryOperation {
            dest: dest + ssa_offset,
            op: op.clone(),
            lhs: remap_operand(lhs, ssa_offset),
            rhs: remap_operand(rhs, ssa_offset),
            dest_type: dest_type.clone(),
        },
        MirInstruction::Call {
            dest,
            name,
            args,
            return_type,
            arg_types,
            is_tail_call,
        } => MirInstruction::Call {
            dest: dest + ssa_offset,
            name: name.clone(),
            args: args.iter().map(|a| remap_operand(a, ssa_offset)).collect(),
            return_type: return_type.clone(),
            arg_types: arg_types.clone(),
            is_tail_call: *is_tail_call,
        },
        MirInstruction::Tuple { dest, elements } => MirInstruction::Tuple {
            dest: dest + ssa_offset,
            elements: elements
                .iter()
                .map(|e| remap_operand(e, ssa_offset))
                .collect(),
        },
        MirInstruction::Index {
            dest,
            subject,
            index,
        } => MirInstruction::Index {
            dest: dest + ssa_offset,
            subject: remap_operand(subject, ssa_offset),
            index: *index,
        },
        MirInstruction::Alloc { dest, size_bytes } => MirInstruction::Alloc {
            dest: dest + ssa_offset,
            size_bytes: remap_operand(size_bytes, ssa_offset),
        },
        MirInstruction::GlobalAlloc { dest, size_bytes, name } => MirInstruction::GlobalAlloc {
            dest: dest + ssa_offset,
            size_bytes: *size_bytes,
            name: name.clone(),
        },
        MirInstruction::PointerOffset { dest, ptr, offset } => MirInstruction::PointerOffset {
            dest: dest + ssa_offset,
            ptr: remap_operand(ptr, ssa_offset),
            offset: remap_operand(offset, ssa_offset),
        },
        MirInstruction::Load { dest, ptr, typ } => MirInstruction::Load {
            dest: dest + ssa_offset,
            ptr: remap_operand(ptr, ssa_offset),
            typ: typ.clone(),
        },
        MirInstruction::MemCopy { dest, src, size } => MirInstruction::MemCopy {
            dest: remap_operand(dest, ssa_offset),
            src: remap_operand(src, ssa_offset),
            size: remap_operand(size, ssa_offset),
        },
        MirInstruction::Store { ptr, value } => MirInstruction::Store {
            ptr: remap_operand(ptr, ssa_offset),
            value: remap_operand(value, ssa_offset),
        },
        MirInstruction::TypedStore { ptr, value, typ } => MirInstruction::TypedStore {
            ptr: remap_operand(ptr, ssa_offset),
            value: remap_operand(value, ssa_offset),
            typ: typ.clone(),
        },
        MirInstruction::MemSet { ptr, value, size } => MirInstruction::MemSet {
            ptr: remap_operand(ptr, ssa_offset),
            value: remap_operand(value, ssa_offset),
            size: remap_operand(size, ssa_offset),
        },
        MirInstruction::Emit(op) => MirInstruction::Emit(remap_operand(op, ssa_offset)),
        MirInstruction::Drop {
            ssa_var,
            typ,
            name,
            is_dynamic,
        } => MirInstruction::Drop {
            ssa_var: ssa_var + ssa_offset,
            typ: typ.clone(),
            name: name.clone(),
            is_dynamic: *is_dynamic,
        },
        MirInstruction::Promote { dest, src, to_type } => MirInstruction::Promote {
            dest: dest + ssa_offset,
            src: remap_operand(src, ssa_offset),
            to_type: to_type.clone(),
        },
        MirInstruction::BitCast { dest, src, to_type } => MirInstruction::BitCast {
            dest: dest + ssa_offset,
            src: remap_operand(src, ssa_offset),
            to_type: to_type.clone(),
        },
        MirInstruction::ConstantTableLoad { dest, name, values, index } => MirInstruction::ConstantTableLoad {
            dest: dest + ssa_offset,
            name: name.clone(),
            values: values.clone(),
            index: remap_operand(index, ssa_offset),
        },
    }
}

/// Remap a single MIR operand: offset Variable SSA IDs, leave Constants unchanged.
fn remap_operand(op: &MirOperand, ssa_offset: usize) -> MirOperand {
    match op {
        MirOperand::Variable(id, consuming) => MirOperand::Variable(id + ssa_offset, *consuming),
        MirOperand::Constant(lit) => MirOperand::Constant(lit.clone()),
    }
}

/// Find the maximum SSA variable ID used anywhere in a function.
/// Used to compute a safe offset for remapped callee SSA vars.
fn max_ssa_in_function(func: &MirFunction) -> usize {
    let mut max = func.args.iter().map(|a| a.ssa_var).max().unwrap_or(0);
    for block in &func.blocks {
        for inst in &block.instructions {
            let dest: Option<usize> = match inst {
                MirInstruction::Assign { dest, .. } => Some(*dest),
                MirInstruction::BinaryOperation { dest, .. } => Some(*dest),
                MirInstruction::Call { dest, .. } => Some(*dest),
                MirInstruction::Tuple { dest, .. } => Some(*dest),
                MirInstruction::Index { dest, .. } => Some(*dest),
                MirInstruction::Alloc { dest, .. } => Some(*dest),
                MirInstruction::GlobalAlloc { dest, .. } => Some(*dest),
                MirInstruction::PointerOffset { dest, .. } => Some(*dest),
                MirInstruction::Load { dest, .. } => Some(*dest),
                MirInstruction::Promote { dest, .. } => Some(*dest),
                MirInstruction::BitCast { dest, .. } => Some(*dest),
                MirInstruction::ConstantTableLoad { dest, .. } => Some(*dest),
                _ => None,
            };
            if let Some(d) = dest {
                if d > max {
                    max = d;
                }
            }
        }
    }
    max
}
