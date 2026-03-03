use crate::domain::entities::mir::{
    BasicBlock, MirBinOp, MirFunction, MirInstruction, MirLiteral, MirOperand, MirProgram,
    MirTerminator,
};
use crate::domain::entities::types::OnuType;

pub struct MemoPass;

impl MemoPass {
    pub fn run(program: MirProgram) -> MirProgram {
        let mut new_functions = vec![];
        for func in program.functions {
            if Self::is_memoizable(&func) {
                let (wrapper, inner) = Self::create_wrapper_and_inner(func);
                new_functions.push(wrapper);
                new_functions.push(inner);
            } else {
                new_functions.push(func);
            }
        }
        MirProgram { functions: new_functions }
    }

    fn is_memoizable(func: &MirFunction) -> bool {
        func.is_pure_data_leaf && func.diminishing.is_some() && func.args.len() == 1 && func.args[0].typ == OnuType::I64
    }

    fn create_wrapper_and_inner(mut func: MirFunction) -> (MirFunction, MirFunction) {
        eprintln!("[MemoPass] Memoizing pure recursive function '{}'", func.name);

        let cache_size = 10000; // Hardcoded small limit for this example.

        let mut inner_func = func.clone();
        inner_func.name = format!("{}.inner", func.name);

        let original_name = func.name.clone();

        let mut next_ssa = max_ssa_in_function(&func) + 1;
        let mut next_block_id = func.blocks.iter().map(|b| b.id).max().unwrap_or(0) + 1;

        // 1. Build Wrapper Function
        let cache_ptr_ssa = next_ssa; next_ssa += 1;
        let size_bytes_ssa = next_ssa; next_ssa += 1;

        let mut entry_instructions = vec![
            MirInstruction::Assign {
                dest: size_bytes_ssa,
                src: MirOperand::Constant(MirLiteral::I64(cache_size as i64 * 8)),
            },
            MirInstruction::Alloc {
                dest: cache_ptr_ssa,
                size_bytes: MirOperand::Variable(size_bytes_ssa, false),
            },
        ];

        let loop_cond_ssa = next_ssa; next_ssa += 1;
        let loop_idx_ssa = next_ssa; next_ssa += 1;
        let loop_idx_next_ssa = next_ssa; next_ssa += 1;
        let loop_ptr_offset_ssa = next_ssa; next_ssa += 1;
        let sentinel_val_ssa = next_ssa; next_ssa += 1;
        let max_idx_ssa = next_ssa; next_ssa += 1;

        let init_loop_head_id = next_block_id; next_block_id += 1;
        let init_loop_body_id = next_block_id; next_block_id += 1;
        let call_inner_id = next_block_id; next_block_id += 1;

        entry_instructions.push(MirInstruction::Assign {
            dest: loop_idx_ssa,
            src: MirOperand::Constant(MirLiteral::I64(0)),
        });
        entry_instructions.push(MirInstruction::Assign {
            dest: max_idx_ssa,
            src: MirOperand::Constant(MirLiteral::I64(cache_size as i64)),
        });
        entry_instructions.push(MirInstruction::Assign {
            dest: sentinel_val_ssa,
            src: MirOperand::Constant(MirLiteral::I64(-1)),
        });

        let wrapper_entry_block = BasicBlock {
            id: 0,
            instructions: entry_instructions,
            terminator: MirTerminator::Branch(init_loop_head_id),
        };

        let init_loop_head = BasicBlock {
            id: init_loop_head_id,
            instructions: vec![
                MirInstruction::BinaryOperation {
                    dest: loop_cond_ssa,
                    op: MirBinOp::Lt,
                    lhs: MirOperand::Variable(loop_idx_ssa, false),
                    rhs: MirOperand::Variable(max_idx_ssa, false),
                }
            ],
            terminator: MirTerminator::CondBranch {
                condition: MirOperand::Variable(loop_cond_ssa, false),
                then_block: init_loop_body_id,
                else_block: call_inner_id,
            },
        };

        let init_loop_body = BasicBlock {
            id: init_loop_body_id,
            instructions: vec![
                MirInstruction::PointerOffset {
                    dest: loop_ptr_offset_ssa,
                    ptr: MirOperand::Variable(cache_ptr_ssa, false),
                    offset: MirOperand::Variable(loop_idx_ssa, false),
                },
                MirInstruction::Store {
                    ptr: MirOperand::Variable(loop_ptr_offset_ssa, false),
                    value: MirOperand::Variable(sentinel_val_ssa, false),
                },
                MirInstruction::BinaryOperation {
                    dest: loop_idx_next_ssa,
                    op: MirBinOp::Add,
                    lhs: MirOperand::Variable(loop_idx_ssa, false),
                    rhs: MirOperand::Constant(MirLiteral::I64(1)),
                },
                MirInstruction::Assign {
                    dest: loop_idx_ssa,
                    src: MirOperand::Variable(loop_idx_next_ssa, false),
                }
            ],
            terminator: MirTerminator::Branch(init_loop_head_id),
        };

        let result_ssa = next_ssa; next_ssa += 1;
        let call_inner_block = BasicBlock {
            id: call_inner_id,
            instructions: vec![
                MirInstruction::Call {
                    dest: result_ssa,
                    name: inner_func.name.clone(),
                    args: vec![
                        MirOperand::Variable(func.args[0].ssa_var, false),
                        MirOperand::Variable(cache_ptr_ssa, false),
                    ],
                    return_type: OnuType::I64,
                    arg_types: vec![OnuType::I64, OnuType::Nothing], // cache is pointer
                    is_tail_call: false,
                }
            ],
            terminator: MirTerminator::Return(MirOperand::Variable(result_ssa, false)),
        };

        let wrapper_func = MirFunction {
            name: original_name.clone(),
            args: func.args.clone(),
            return_type: func.return_type.clone(),
            blocks: vec![wrapper_entry_block, init_loop_head, init_loop_body, call_inner_block],
            is_pure_data_leaf: true,
            diminishing: func.diminishing.clone(),
        };

        // 2. Build Inner Function
        let inner_cache_ptr_ssa = next_ssa; next_ssa += 1;
        inner_func.args.push(crate::domain::entities::mir::MirArgument {
            name: "cache_ptr".to_string(),
            typ: OnuType::Nothing, // representing pointer
            ssa_var: inner_cache_ptr_ssa,
        });

        let mut rewritten_blocks = vec![];
        for block in inner_func.blocks {
            let mut calls_to_rewrite = vec![];

            for (idx, inst) in block.instructions.iter().enumerate() {
                if let MirInstruction::Call { name, .. } = inst {
                    if name == &original_name {
                        calls_to_rewrite.push(idx);
                    }
                }
            }

            if calls_to_rewrite.is_empty() {
                rewritten_blocks.push(block);
                continue;
            }

            let mut current_block_id = block.id;
            let mut current_instructions: Vec<_> = vec![];
            let mut last_idx = 0;

            for call_idx in calls_to_rewrite {
                for inst in block.instructions[last_idx..call_idx].iter() {
                    current_instructions.push(inst.clone());
                }

                let call_inst = block.instructions[call_idx].clone();
                let (dest, args, is_tail_call) = match &call_inst {
                    MirInstruction::Call { dest, args, is_tail_call, .. } => (*dest, args.clone(), *is_tail_call),
                    _ => unreachable!(),
                };

                let arg_op = args[0].clone();

                let bounds_check_upper_ssa = next_ssa; next_ssa += 1;
                let bounds_check_lower_ssa = next_ssa; next_ssa += 1;
                let offset_ssa = next_ssa; next_ssa += 1;
                let cached_val_ssa = next_ssa; next_ssa += 1;
                let hit_cond_ssa = next_ssa; next_ssa += 1;

                let check_block_id = current_block_id;
                let fetch_block_id = next_block_id; next_block_id += 1;
                let miss_block_id = next_block_id; next_block_id += 1;
                let store_block_id = next_block_id; next_block_id += 1;
                let cont_block_id = next_block_id; next_block_id += 1;

                // Bounds Check: arg < cache_size AND arg >= 0
                current_instructions.push(MirInstruction::BinaryOperation {
                    dest: bounds_check_upper_ssa,
                    op: MirBinOp::Lt,
                    lhs: arg_op.clone(),
                    rhs: MirOperand::Constant(MirLiteral::I64(cache_size as i64)),
                });

                // First check upper:
                let lower_check_block_id = next_block_id; next_block_id += 1;

                rewritten_blocks.push(BasicBlock {
                    id: check_block_id,
                    instructions: current_instructions,
                    terminator: MirTerminator::CondBranch {
                        condition: MirOperand::Variable(bounds_check_upper_ssa, false),
                        then_block: lower_check_block_id,
                        else_block: miss_block_id, // out of bounds
                    },
                });

                // Lower bound check (arg >= 0 -> !(arg < 0))
                let lower_check_instructions = vec![
                    MirInstruction::BinaryOperation {
                        dest: bounds_check_lower_ssa,
                        op: MirBinOp::Lt,
                        lhs: arg_op.clone(),
                        rhs: MirOperand::Constant(MirLiteral::I64(0)),
                    },
                ];
                rewritten_blocks.push(BasicBlock {
                    id: lower_check_block_id,
                    instructions: lower_check_instructions,
                    terminator: MirTerminator::CondBranch {
                        condition: MirOperand::Variable(bounds_check_lower_ssa, false),
                        then_block: miss_block_id, // less than 0 -> miss
                        else_block: fetch_block_id, // >= 0 -> fetch
                    }
                });

                let fetch_instructions = vec![
                    MirInstruction::PointerOffset {
                        dest: offset_ssa,
                        ptr: MirOperand::Variable(inner_cache_ptr_ssa, false),
                        offset: arg_op.clone(),
                    },
                    // Since MIR has no `Load` instruction, `Index` instruction supports
                    // memory reads when given a pointer, but its argument must be an integer index.
                    // Wait, `cbench_collatz.c` translation uses `Index`? No, Codegen reads pointer values with a `Load`.
                    // Actually, let's use `Index` correctly. The subject is pointer, index is the integer offset.
                    // Wait, if it is a pointer `offset_ssa`, we just read offset 0!
                    MirInstruction::Index {
                        dest: cached_val_ssa,
                        subject: MirOperand::Variable(offset_ssa, false),
                        index: 0,
                    },
                    MirInstruction::BinaryOperation {
                        dest: hit_cond_ssa,
                        op: MirBinOp::Ne,
                        lhs: MirOperand::Variable(cached_val_ssa, false),
                        rhs: MirOperand::Constant(MirLiteral::I64(-1)),
                    }
                ];

                rewritten_blocks.push(BasicBlock {
                    id: fetch_block_id,
                    instructions: fetch_instructions,
                    terminator: MirTerminator::CondBranch {
                        condition: MirOperand::Variable(hit_cond_ssa, false),
                        then_block: cont_block_id, // Hit
                        else_block: miss_block_id, // Miss
                    },
                });

                let mut new_args = args.clone();
                new_args.push(MirOperand::Variable(inner_cache_ptr_ssa, false));

                let miss_instructions = vec![
                    MirInstruction::Call {
                        dest,
                        name: inner_func.name.clone(),
                        args: new_args,
                        return_type: OnuType::I64,
                        arg_types: vec![OnuType::I64, OnuType::Nothing],
                        is_tail_call,
                    }
                ];
                rewritten_blocks.push(BasicBlock {
                    id: miss_block_id,
                    instructions: miss_instructions,
                    terminator: MirTerminator::Branch(store_block_id),
                });

                let safe_offset_ssa = next_ssa; next_ssa += 1;
                let safe_store_instructions = vec![
                    MirInstruction::PointerOffset {
                        dest: safe_offset_ssa,
                        ptr: MirOperand::Variable(inner_cache_ptr_ssa, false),
                        offset: arg_op.clone(),
                    },
                    MirInstruction::Store {
                        ptr: MirOperand::Variable(safe_offset_ssa, false),
                        value: MirOperand::Variable(dest, false),
                    }
                ];

                let miss_bounds_block_id = next_block_id; next_block_id += 1;
                let mut miss_bounds_args = args.clone();
                miss_bounds_args.push(MirOperand::Variable(inner_cache_ptr_ssa, false));
                rewritten_blocks.push(BasicBlock {
                    id: miss_bounds_block_id,
                    instructions: vec![
                        MirInstruction::Call {
                            dest,
                            name: inner_func.name.clone(),
                            args: miss_bounds_args,
                            return_type: OnuType::I64,
                            arg_types: vec![OnuType::I64, OnuType::Nothing],
                            is_tail_call,
                        }
                    ],
                    terminator: MirTerminator::Branch(cont_block_id), // Skip store
                });

                let len = rewritten_blocks.len();
                rewritten_blocks[len-2].terminator = MirTerminator::CondBranch {
                    condition: MirOperand::Variable(bounds_check_lower_ssa, false),
                    then_block: miss_bounds_block_id, // < 0
                    else_block: fetch_block_id,
                };
                rewritten_blocks[len-3].terminator = MirTerminator::CondBranch {
                    condition: MirOperand::Variable(bounds_check_upper_ssa, false),
                    then_block: lower_check_block_id,
                    else_block: miss_bounds_block_id, // >= size
                };

                rewritten_blocks.push(BasicBlock {
                    id: store_block_id,
                    instructions: safe_store_instructions,
                    terminator: MirTerminator::Branch(cont_block_id),
                });

                let hit_block_id = next_block_id; next_block_id += 1;
                let hit_block = BasicBlock {
                    id: hit_block_id,
                    instructions: vec![
                        MirInstruction::Assign {
                            dest: dest,
                            src: MirOperand::Variable(cached_val_ssa, false),
                        }
                    ],
                    terminator: MirTerminator::Branch(cont_block_id),
                };

                for b in &mut rewritten_blocks {
                    if b.id == fetch_block_id {
                        b.terminator = MirTerminator::CondBranch {
                            condition: MirOperand::Variable(hit_cond_ssa, false),
                            then_block: hit_block_id,
                            else_block: miss_block_id,
                        };
                    }
                }
                rewritten_blocks.push(hit_block);

                current_block_id = cont_block_id;
                current_instructions = vec![];
                last_idx = call_idx + 1;
            }

            for inst in block.instructions[last_idx..].iter() {
                current_instructions.push(inst.clone());
            }

            rewritten_blocks.push(BasicBlock {
                id: current_block_id,
                instructions: current_instructions,
                terminator: block.terminator,
            });
        }

        inner_func.blocks = rewritten_blocks;

        (wrapper_func, inner_func)
    }
}

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
                MirInstruction::PointerOffset { dest, .. } => Some(*dest),
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
