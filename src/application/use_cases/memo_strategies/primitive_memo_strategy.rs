use super::{MemoStrategy, max_ssa_in_function};
use crate::domain::entities::mir::{
    BasicBlock, MirArgument, MirBinOp, MirFunction, MirInstruction, MirLiteral, MirOperand,
    MirTerminator,
};
use crate::domain::entities::types::OnuType;

// --- LAYER 1: INFRASTRUCTURE (The Builder) ---
struct MirBuilder {
    next_ssa: usize,
    next_block_id: usize,
}

impl MirBuilder {
    fn new(func: &MirFunction) -> Self {
        Self {
            next_ssa: max_ssa_in_function(func) + 1,
            next_block_id: func.blocks.iter().map(|b| b.id).max().unwrap_or(0) + 1,
        }
    }

    fn alloc_ssa(&mut self) -> usize {
        let ssa = self.next_ssa;
        self.next_ssa += 1;
        ssa
    }

    fn alloc_block(&mut self) -> usize {
        let id = self.next_block_id;
        self.next_block_id += 1;
        id
    }
}

// --- LAYER 2: DOMAIN LOGIC (Cache Accessor) ---
struct CacheAccessor<'a> {
    builder: &'a mut MirBuilder,
    cache_ptr_ssa: usize,
}

impl<'a> CacheAccessor<'a> {
    fn compute_byte_offset(
        &mut self,
        insts: &mut Vec<MirInstruction>,
        logical_idx: MirOperand,
    ) -> usize {
        let byte_offset_ssa = self.builder.alloc_ssa();
        insts.push(MirInstruction::BinaryOperation {
            dest: byte_offset_ssa,
            op: MirBinOp::Mul,
            lhs: logical_idx,
            rhs: MirOperand::Constant(MirLiteral::I64(8)),
        });
        byte_offset_ssa
    }

    fn emit_load(&mut self, insts: &mut Vec<MirInstruction>, byte_offset: usize) -> usize {
        let ptr_ssa = self.builder.alloc_ssa();
        let val_ssa = self.builder.alloc_ssa();
        insts.push(MirInstruction::PointerOffset {
            dest: ptr_ssa,
            ptr: MirOperand::Variable(self.cache_ptr_ssa, false),
            offset: MirOperand::Variable(byte_offset, false),
        });
        insts.push(MirInstruction::Load {
            dest: val_ssa,
            ptr: MirOperand::Variable(ptr_ssa, false),
            typ: OnuType::I64,
        });
        val_ssa
    }

    fn emit_safe_store(
        &mut self,
        insts: &mut Vec<MirInstruction>,
        byte_offset: usize,
        value_ssa: usize,
    ) {
        let ptr_ssa = self.builder.alloc_ssa();
        insts.push(MirInstruction::PointerOffset {
            dest: ptr_ssa,
            ptr: MirOperand::Variable(self.cache_ptr_ssa, false),
            offset: MirOperand::Variable(byte_offset, false),
        });
        insts.push(MirInstruction::TypedStore {
            ptr: MirOperand::Variable(ptr_ssa, false),
            value: MirOperand::Variable(value_ssa, false),
            typ: OnuType::I64,
        });
    }
}

// --- LAYER 3: APPLICATION (The Strategy) ---
pub struct PrimitiveMemoStrategy;

impl MemoStrategy for PrimitiveMemoStrategy {
    fn create_wrapper_and_inner(
        &self,
        func: MirFunction,
        cache_size: usize,
    ) -> (MirFunction, MirFunction) {
        let mut builder = MirBuilder::new(&func);
        let original_name = func.name.clone();

        // 1. Build Wrapper
        let (wrapper_func, _) = self.build_wrapper(&func, &mut builder, cache_size);

        // 2. Build Inner
        let mut inner_func = func.clone();
        inner_func.name = format!("{}.inner", original_name);
        let inner_cache_ptr_ssa = builder.alloc_ssa();

        inner_func.args.push(MirArgument {
            name: "cache_ptr".to_string(),
            typ: OnuType::Ptr,
            ssa_var: inner_cache_ptr_ssa,
        });

        inner_func.blocks = self.rewrite_inner_calls(
            inner_func.blocks,
            &mut builder,
            inner_cache_ptr_ssa,
            &original_name,
            cache_size,
        );

        (wrapper_func, inner_func)
    }
}

impl PrimitiveMemoStrategy {
    fn build_wrapper(
        &self,
        func: &MirFunction,
        builder: &mut MirBuilder,
        cache_size: usize,
    ) -> (MirFunction, usize) {
        let cache_ptr = builder.alloc_ssa();
        let size_ssa = builder.alloc_ssa();
        let loop_idx = builder.alloc_ssa();

        let head_id = builder.alloc_block();
        let body_id = builder.alloc_block();
        let call_id = builder.alloc_block();

        // 1. Entry Block (ID 0)
        let entry_insts = vec![
            MirInstruction::Assign {
                dest: size_ssa,
                src: MirOperand::Constant(MirLiteral::I64(cache_size as i64 * 8)),
            },
            MirInstruction::Alloc {
                dest: cache_ptr,
                size_bytes: MirOperand::Variable(size_ssa, false),
            },
            MirInstruction::Assign {
                dest: loop_idx,
                src: MirOperand::Constant(MirLiteral::I64(0)),
            },
        ];

        let entry_block = BasicBlock {
            id: 0,
            instructions: entry_insts,
            terminator: MirTerminator::Branch(head_id),
        };

        // 2. Head Block (While loop condition)
        let cond_ssa = builder.alloc_ssa();
        let head_block = BasicBlock {
            id: head_id,
            instructions: vec![MirInstruction::BinaryOperation {
                dest: cond_ssa,
                op: MirBinOp::Lt,
                lhs: MirOperand::Variable(loop_idx, false),
                rhs: MirOperand::Constant(MirLiteral::I64(cache_size as i64)),
            }],
            terminator: MirTerminator::CondBranch {
                condition: MirOperand::Variable(cond_ssa, false),
                then_block: body_id,
                else_block: call_id,
            },
        };

        // 3. Body Block (Store sentinel and increment index)
        let byte_offset_ssa = builder.alloc_ssa();
        let ptr_ssa = builder.alloc_ssa();
        let next_idx_ssa = builder.alloc_ssa();
        let body_block = BasicBlock {
            id: body_id,
            instructions: vec![
                MirInstruction::BinaryOperation {
                    dest: byte_offset_ssa,
                    op: MirBinOp::Mul,
                    lhs: MirOperand::Variable(loop_idx, false),
                    rhs: MirOperand::Constant(MirLiteral::I64(8)),
                },
                MirInstruction::PointerOffset {
                    dest: ptr_ssa,
                    ptr: MirOperand::Variable(cache_ptr, false),
                    offset: MirOperand::Variable(byte_offset_ssa, false),
                },
                MirInstruction::TypedStore {
                    ptr: MirOperand::Variable(ptr_ssa, false),
                    value: MirOperand::Constant(MirLiteral::I64(-1)),
                    typ: OnuType::I64,
                },
                MirInstruction::BinaryOperation {
                    dest: next_idx_ssa,
                    op: MirBinOp::Add,
                    lhs: MirOperand::Variable(loop_idx, false),
                    rhs: MirOperand::Constant(MirLiteral::I64(1)),
                },
                MirInstruction::Assign {
                    dest: loop_idx,
                    src: MirOperand::Variable(next_idx_ssa, false),
                },
            ],
            terminator: MirTerminator::Branch(head_id),
        };

        // 4. Call Block (Call inner and return)
        let res_ssa = builder.alloc_ssa();
        let mut call_args: Vec<MirOperand> = func
            .args
            .iter()
            .map(|arg| MirOperand::Variable(arg.ssa_var, false))
            .collect();
        call_args.push(MirOperand::Variable(cache_ptr, false));

        let call_block = BasicBlock {
            id: call_id,
            instructions: vec![MirInstruction::Call {
                name: format!("{}.inner", func.name),
                dest: res_ssa,
                args: call_args,
                return_type: func.return_type.clone(),
                arg_types: func
                    .args
                    .iter()
                    .map(|a| a.typ.clone())
                    .chain(std::iter::once(OnuType::Ptr))
                    .collect(),
                is_tail_call: false,
            }],
            terminator: MirTerminator::Return(MirOperand::Variable(res_ssa, false)),
        };

        (
            MirFunction {
                name: func.name.clone(),
                args: func.args.clone(),
                blocks: vec![entry_block, head_block, body_block, call_block],
                is_pure_data_leaf: false,
                ..func.clone()
            },
            cache_ptr,
        )
    }

    fn rewrite_inner_calls(
        &self,
        blocks: Vec<BasicBlock>,
        builder: &mut MirBuilder,
        cache_ptr: usize,
        orig_name: &str,
        cache_size: usize,
    ) -> Vec<BasicBlock> {
        let mut rewritten = vec![];
        let mut accessor = CacheAccessor {
            builder,
            cache_ptr_ssa: cache_ptr,
        };

        for block in blocks {
            let mut current_insts = vec![];
            let mut current_block_id = block.id;

            for inst in block.instructions {
                match inst {
                    MirInstruction::Call {
                        ref name,
                        dest,
                        ref args,
                        is_tail_call,
                        ref return_type,
                        ref arg_types,
                    } if name == orig_name => {
                        let upper_check_id = accessor.builder.alloc_block();
                        let fetch_id = accessor.builder.alloc_block();
                        let miss_id = accessor.builder.alloc_block();
                        let hit_id = accessor.builder.alloc_block();
                        let store_id = accessor.builder.alloc_block();
                        let cont_id = accessor.builder.alloc_block();

                        // 1. Lower Bound Check (arg >= 0)
                        let l_check = accessor.builder.alloc_ssa();
                        current_insts.push(MirInstruction::BinaryOperation {
                            dest: l_check,
                            op: MirBinOp::Lt,
                            lhs: args[0].clone(),
                            rhs: MirOperand::Constant(MirLiteral::I64(0)),
                        });

                        rewritten.push(BasicBlock {
                            id: current_block_id,
                            instructions: current_insts.drain(..).collect(),
                            terminator: MirTerminator::CondBranch {
                                condition: MirOperand::Variable(l_check, false),
                                then_block: miss_id,
                                else_block: upper_check_id,
                            },
                        });

                        // 2. Upper Bound Check (arg < cache_size)
                        let u_check = accessor.builder.alloc_ssa();
                        rewritten.push(BasicBlock {
                            id: upper_check_id,
                            instructions: vec![MirInstruction::BinaryOperation {
                                dest: u_check,
                                op: MirBinOp::Lt,
                                lhs: args[0].clone(),
                                rhs: MirOperand::Constant(MirLiteral::I64(cache_size as i64)),
                            }],
                            terminator: MirTerminator::CondBranch {
                                condition: MirOperand::Variable(u_check, false),
                                then_block: fetch_id,
                                else_block: miss_id,
                            },
                        });

                        // 3. FETCH BLOCK
                        let mut fetch_insts = vec![];
                        let offset =
                            accessor.compute_byte_offset(&mut fetch_insts, args[0].clone());
                        let val = accessor.emit_load(&mut fetch_insts, offset);

                        let hit_cond = accessor.builder.alloc_ssa();
                        fetch_insts.push(MirInstruction::BinaryOperation {
                            dest: hit_cond,
                            op: MirBinOp::Ne,
                            lhs: MirOperand::Variable(val, false),
                            rhs: MirOperand::Constant(MirLiteral::I64(-1)),
                        });

                        rewritten.push(BasicBlock {
                            id: fetch_id,
                            instructions: fetch_insts,
                            terminator: MirTerminator::CondBranch {
                                condition: MirOperand::Variable(hit_cond, false),
                                then_block: hit_id,
                                else_block: miss_id,
                            },
                        });

                        // 4. HIT BLOCK
                        rewritten.push(BasicBlock {
                            id: hit_id,
                            instructions: vec![MirInstruction::Assign {
                                dest,
                                src: MirOperand::Variable(val, false),
                            }],
                            terminator: MirTerminator::Branch(cont_id),
                        });

                        // 5. MISS BLOCK - Fix: Update arg_types to include Ptr for the cache pointer
                        let mut new_arg_types = arg_types.clone();
                        new_arg_types.push(OnuType::Ptr);

                        let mut new_args = args.clone();
                        new_args.push(MirOperand::Variable(cache_ptr, false));

                        rewritten.push(BasicBlock {
                            id: miss_id,
                            instructions: vec![MirInstruction::Call {
                                name: format!("{}.inner", orig_name),
                                dest,
                                args: new_args,
                                is_tail_call,
                                return_type: return_type.clone(),
                                arg_types: new_arg_types,
                            }],
                            terminator: MirTerminator::Branch(store_id),
                        });

                        // 6. STORE BLOCK
                        let mut store_insts = vec![];
                        let store_offset =
                            accessor.compute_byte_offset(&mut store_insts, args[0].clone());
                        accessor.emit_safe_store(&mut store_insts, store_offset, dest);

                        rewritten.push(BasicBlock {
                            id: store_id,
                            instructions: store_insts,
                            terminator: MirTerminator::Branch(cont_id),
                        });

                        current_block_id = cont_id;
                    }
                    inst => current_insts.push(inst),
                }
            }
            rewritten.push(BasicBlock {
                id: current_block_id,
                instructions: current_insts,
                terminator: block.terminator,
            });
        }
        rewritten
    }
}
