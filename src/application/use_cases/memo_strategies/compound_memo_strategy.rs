use crate::domain::entities::mir::{
    BasicBlock, MirBinOp, MirFunction, MirInstruction, MirLiteral, MirOperand, MirTerminator, MirArgument
};
use crate::domain::entities::types::OnuType;
use super::{MemoStrategy, max_ssa_in_function};

pub struct CompoundMemoStrategy;

impl MemoStrategy for CompoundMemoStrategy {
    fn create_wrapper_and_inner(&self, mut func: MirFunction, cache_size: usize) -> (MirFunction, MirFunction) {
        eprintln!(
            "[CompoundMemoStrategy] Memoizing recursive function '{}' with return type {:?}",
            func.name,
            func.return_type
        );

        let mut inner_func = func.clone();
        inner_func.name = format!("{}.inner", func.name);

        let original_name = func.name.clone();

        let mut next_ssa = max_ssa_in_function(&func) + 1;
        let mut next_block_id = func.blocks.iter().map(|b| b.id).max().unwrap_or(0) + 1;

        let return_type = func.return_type.clone();

        // 1. Build Wrapper Function
        let cache_ptr_ssa = next_ssa;
        next_ssa += 1;
        let size_bytes_ssa = next_ssa;
        next_ssa += 1;

        // Note: We need a dynamic byte stride for compound types,
        // For simplicity, we assign a large enough stride per element or
        // assume pointer indirection
        let stride_bytes = match &return_type {
            OnuType::Strings | OnuType::Tuple(_) | OnuType::Array(_) | OnuType::HashMap(_, _) | OnuType::Tree(_) | OnuType::Matrix => 24, // Pointer + Metadata (e.g. len)
            _ => 8,
        };

        let total_cache_bytes = cache_size * stride_bytes;

        let mut entry_instructions = vec![
            MirInstruction::Assign {
                dest: size_bytes_ssa,
                src: MirOperand::Constant(MirLiteral::I64(total_cache_bytes as i64)),
            },
            MirInstruction::Alloc {
                dest: cache_ptr_ssa,
                size_bytes: MirOperand::Variable(size_bytes_ssa, false),
            },
        ];

        let loop_cond_ssa = next_ssa;
        next_ssa += 1;
        let loop_idx_ssa = next_ssa;
        next_ssa += 1;
        let loop_idx_next_ssa = next_ssa;
        next_ssa += 1;
        let loop_ptr_offset_ssa = next_ssa;
        next_ssa += 1;
        let sentinel_val_ssa = next_ssa;
        next_ssa += 1;
        let max_idx_ssa = next_ssa;
        next_ssa += 1;

        let init_loop_head_id = next_block_id;
        next_block_id += 1;
        let init_loop_body_id = next_block_id;
        next_block_id += 1;
        let call_inner_id = next_block_id;
        next_block_id += 1;

        entry_instructions.push(MirInstruction::Assign {
            dest: loop_idx_ssa,
            src: MirOperand::Constant(MirLiteral::I64(0)),
        });
        entry_instructions.push(MirInstruction::Assign {
            dest: max_idx_ssa,
            src: MirOperand::Constant(MirLiteral::I64(cache_size as i64)),
        });
        // We use -1 or Null as sentinel
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
            instructions: vec![MirInstruction::BinaryOperation {
                dest: loop_cond_ssa,
                op: MirBinOp::Lt,
                lhs: MirOperand::Variable(loop_idx_ssa, false),
                rhs: MirOperand::Variable(max_idx_ssa, false),
            }],
            terminator: MirTerminator::CondBranch {
                condition: MirOperand::Variable(loop_cond_ssa, false),
                then_block: init_loop_body_id,
                else_block: call_inner_id,
            },
        };

        // For compound types, we just init the first 8 bytes of the slot to -1 to denote "empty"
        let mut scaled_offset_ssa = next_ssa;
        next_ssa += 1;
        let init_loop_body = BasicBlock {
            id: init_loop_body_id,
            instructions: vec![
                MirInstruction::BinaryOperation {
                    dest: scaled_offset_ssa,
                    op: MirBinOp::Mul,
                    lhs: MirOperand::Variable(loop_idx_ssa, false),
                    rhs: MirOperand::Constant(MirLiteral::I64(stride_bytes as i64)),
                },
                MirInstruction::PointerOffset {
                    dest: loop_ptr_offset_ssa,
                    ptr: MirOperand::Variable(cache_ptr_ssa, false),
                    offset: MirOperand::Variable(scaled_offset_ssa, false),
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
                },
            ],
            terminator: MirTerminator::Branch(init_loop_head_id),
        };

        let result_ssa = next_ssa;
        next_ssa += 1;

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
                    return_type: return_type.clone(),
                    arg_types: vec![OnuType::I64, OnuType::Ptr], // cache is i8* pointer
                    is_tail_call: false,
                },
                MirInstruction::Drop {
                    ssa_var: cache_ptr_ssa,
                    typ: OnuType::Ptr,
                    name: "memo_cache".to_string(),
                    is_dynamic: true,
                },
            ],
            terminator: MirTerminator::Return(MirOperand::Variable(result_ssa, false)),
        };

        let wrapper_func = MirFunction {
            name: original_name.clone(),
            args: func.args.clone(),
            return_type: return_type.clone(),
            blocks: vec![
                wrapper_entry_block,
                init_loop_head,
                init_loop_body,
                call_inner_block,
            ],
            is_pure_data_leaf: false,
            diminishing: func.diminishing.clone(),
        };

        // 2. Build Inner Function
        let inner_cache_ptr_ssa = next_ssa;
        next_ssa += 1;
        inner_func
            .args
            .push(MirArgument {
                name: "cache_ptr".to_string(),
                typ: OnuType::Ptr,
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
                    MirInstruction::Call {
                        dest,
                        args,
                        is_tail_call,
                        ..
                    } => (*dest, args.clone(), *is_tail_call),
                    _ => unreachable!(),
                };

                let arg_op = args[0].clone();

                let bounds_check_upper_ssa = next_ssa;
                next_ssa += 1;
                let bounds_check_lower_ssa = next_ssa;
                next_ssa += 1;

                let byte_offset_ssa = next_ssa;
                next_ssa += 1;
                let offset_ssa = next_ssa;
                next_ssa += 1;
                let sentinel_check_ssa = next_ssa;
                next_ssa += 1;
                let hit_cond_ssa = next_ssa;
                next_ssa += 1;
                let cached_val_ssa = next_ssa;
                next_ssa += 1;

                let check_block_id = current_block_id;
                let fetch_block_id = next_block_id;
                next_block_id += 1;
                let hit_block_id = next_block_id;
                next_block_id += 1;
                let miss_block_id = next_block_id;
                next_block_id += 1;
                let store_block_id = next_block_id;
                next_block_id += 1;
                let cont_block_id = next_block_id;
                next_block_id += 1;

                current_instructions.push(MirInstruction::BinaryOperation {
                    dest: bounds_check_upper_ssa,
                    op: MirBinOp::Lt,
                    lhs: arg_op.clone(),
                    rhs: MirOperand::Constant(MirLiteral::I64(cache_size as i64)),
                });

                let lower_check_block_id = next_block_id;
                next_block_id += 1;

                rewritten_blocks.push(BasicBlock {
                    id: check_block_id,
                    instructions: current_instructions,
                    terminator: MirTerminator::CondBranch {
                        condition: MirOperand::Variable(bounds_check_upper_ssa, false),
                        then_block: lower_check_block_id,
                        else_block: miss_block_id,
                    },
                });

                let lower_check_instructions = vec![
                    MirInstruction::BinaryOperation {
                        dest: bounds_check_lower_ssa,
                        op: MirBinOp::Lt,
                        lhs: arg_op.clone(),
                        rhs: MirOperand::Constant(MirLiteral::I64(0)),
                    },
                    MirInstruction::BinaryOperation {
                        dest: byte_offset_ssa,
                        op: MirBinOp::Mul,
                        lhs: arg_op.clone(),
                        rhs: MirOperand::Constant(MirLiteral::I64(stride_bytes as i64)),
                    },
                ];
                rewritten_blocks.push(BasicBlock {
                    id: lower_check_block_id,
                    instructions: lower_check_instructions,
                    terminator: MirTerminator::CondBranch {
                        condition: MirOperand::Variable(bounds_check_lower_ssa, false),
                        then_block: miss_block_id,
                        else_block: fetch_block_id,
                    },
                });

                let fetch_instructions = vec![
                    MirInstruction::PointerOffset {
                        dest: offset_ssa,
                        ptr: MirOperand::Variable(inner_cache_ptr_ssa, false),
                        offset: MirOperand::Variable(byte_offset_ssa, false),
                    },
                    // We check if the first 8 bytes have our -1 sentinel
                    MirInstruction::Load {
                        dest: sentinel_check_ssa,
                        ptr: MirOperand::Variable(offset_ssa, false),
                        typ: OnuType::I64,
                    },
                    MirInstruction::BinaryOperation {
                        dest: hit_cond_ssa,
                        op: MirBinOp::Ne,
                        lhs: MirOperand::Variable(sentinel_check_ssa, false),
                        rhs: MirOperand::Constant(MirLiteral::I64(-1)),
                    },
                ];

                rewritten_blocks.push(BasicBlock {
                    id: fetch_block_id,
                    instructions: fetch_instructions,
                    terminator: MirTerminator::CondBranch {
                        condition: MirOperand::Variable(hit_cond_ssa, false),
                        then_block: hit_block_id,
                        else_block: miss_block_id,
                    },
                });

                // Cache HIT: Load full compound type
                let hit_instructions = vec![
                    MirInstruction::Load {
                        dest: cached_val_ssa,
                        ptr: MirOperand::Variable(offset_ssa, false),
                        typ: return_type.clone(),
                    },
                    MirInstruction::Assign {
                        dest: dest,
                        src: MirOperand::Variable(cached_val_ssa, false),
                    }
                ];

                rewritten_blocks.push(BasicBlock {
                    id: hit_block_id,
                    instructions: hit_instructions,
                    terminator: MirTerminator::Branch(cont_block_id),
                });

                let mut new_args = args.clone();
                new_args.push(MirOperand::Variable(inner_cache_ptr_ssa, false));

                let miss_instructions = vec![MirInstruction::Call {
                    dest,
                    name: inner_func.name.clone(),
                    args: new_args,
                    return_type: return_type.clone(),
                    arg_types: vec![OnuType::I64, OnuType::Ptr],
                    is_tail_call,
                }];
                rewritten_blocks.push(BasicBlock {
                    id: miss_block_id,
                    instructions: miss_instructions,
                    terminator: MirTerminator::Branch(store_block_id),
                });

                let safe_offset_ssa = next_ssa;
                next_ssa += 1;
                let safe_store_instructions = vec![
                    MirInstruction::PointerOffset {
                        dest: safe_offset_ssa,
                        ptr: MirOperand::Variable(inner_cache_ptr_ssa, false),
                        offset: MirOperand::Variable(byte_offset_ssa, false),
                    },
                    MirInstruction::TypedStore {
                        ptr: MirOperand::Variable(safe_offset_ssa, false),
                        value: MirOperand::Variable(dest, false),
                        typ: return_type.clone(),
                    },
                ];

                let miss_bounds_block_id = next_block_id;
                next_block_id += 1;
                let mut miss_bounds_args = args.clone();
                miss_bounds_args.push(MirOperand::Variable(inner_cache_ptr_ssa, false));
                rewritten_blocks.push(BasicBlock {
                    id: miss_bounds_block_id,
                    instructions: vec![MirInstruction::Call {
                        dest,
                        name: inner_func.name.clone(),
                        args: miss_bounds_args,
                        return_type: return_type.clone(),
                        arg_types: vec![OnuType::I64, OnuType::Ptr],
                        is_tail_call,
                    }],
                    terminator: MirTerminator::Branch(cont_block_id),
                });

                for b in &mut rewritten_blocks {
                    if b.id == lower_check_block_id {
                        b.terminator = MirTerminator::CondBranch {
                            condition: MirOperand::Variable(bounds_check_lower_ssa, false),
                            then_block: miss_bounds_block_id,
                            else_block: fetch_block_id,
                        };
                    }
                    if b.id == check_block_id {
                        b.terminator = MirTerminator::CondBranch {
                            condition: MirOperand::Variable(bounds_check_upper_ssa, false),
                            then_block: lower_check_block_id,
                            else_block: miss_bounds_block_id,
                        };
                    }
                }

                rewritten_blocks.push(BasicBlock {
                    id: store_block_id,
                    instructions: safe_store_instructions,
                    terminator: MirTerminator::Branch(cont_block_id),
                });

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
