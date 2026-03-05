use super::{MemoStrategy, max_ssa_in_function};
use crate::domain::entities::mir::{
    BasicBlock, MirArgument, MirBinOp, MirFunction, MirInstruction, MirLiteral, MirOperand,
    MirTerminator,
};
use crate::domain::entities::types::OnuType;

// --- LAYER 1: INFRASTRUCTURE (Managed MIR Builder) ---
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

// --- LAYER 2: DOMAIN LOGIC (Type-Aware Cache Provider) ---
struct CacheProvider<'a> {
    builder: &'a mut MirBuilder,
    cache_ptr_ssa: usize,
    ret_type: OnuType,
}

impl<'a> CacheProvider<'a> {
    /// Determines the stride (byte size) for the specific return type.
    fn get_stride(&self) -> i64 {
        match self.ret_type {
            OnuType::I64 | OnuType::Ptr => 8,
            OnuType::I32 | OnuType::F32 => 4,
            _ => 8, // Defaulting to 8 for safety with compound types
        }
    }

    /// Calculates the byte offset based on the type's stride.
    fn compute_offset(
        &mut self,
        insts: &mut Vec<MirInstruction>,
        logical_idx: MirOperand,
    ) -> usize {
        let offset_ssa = self.builder.alloc_ssa();
        insts.push(MirInstruction::BinaryOperation {
            dest: offset_ssa,
            op: MirBinOp::Mul,
            lhs: logical_idx,
            rhs: MirOperand::Constant(MirLiteral::I64(self.get_stride())),
        });
        offset_ssa
    }

    /// Emits a robust load. If it's a primitive, we can check a sentinel.
    fn emit_load(&mut self, insts: &mut Vec<MirInstruction>, offset: usize) -> usize {
        let ptr_ssa = self.builder.alloc_ssa();
        let val_ssa = self.builder.alloc_ssa();
        insts.push(MirInstruction::PointerOffset {
            dest: ptr_ssa,
            ptr: MirOperand::Variable(self.cache_ptr_ssa, false),
            offset: MirOperand::Variable(offset, false),
        });
        insts.push(MirInstruction::Load {
            dest: val_ssa,
            ptr: MirOperand::Variable(ptr_ssa, false),
            typ: self.ret_type.clone(),
        });
        val_ssa
    }

    /// Emits a robust TypedStore to prevent truncation[cite: 80].
    fn emit_store(&mut self, insts: &mut Vec<MirInstruction>, offset: usize, value_ssa: usize) {
        let ptr_ssa = self.builder.alloc_ssa();
        insts.push(MirInstruction::PointerOffset {
            dest: ptr_ssa,
            ptr: MirOperand::Variable(self.cache_ptr_ssa, false),
            offset: MirOperand::Variable(offset, false),
        });
        insts.push(MirInstruction::TypedStore {
            ptr: MirOperand::Variable(ptr_ssa, false),
            value: MirOperand::Variable(value_ssa, false),
            typ: self.ret_type.clone(),
        });
    }
}

// --- LAYER 3: APPLICATION (The Compound Strategy) ---
pub struct CompoundMemoStrategy;

impl MemoStrategy for CompoundMemoStrategy {
    fn create_wrapper_and_inner(
        &self,
        func: MirFunction,
        cache_size: usize,
    ) -> (MirFunction, MirFunction) {
        let mut builder = MirBuilder::new(&func);
        let orig_name = func.name.clone();
        let ret_type = func.return_type.clone();

        // 1. Generate Wrapper (Handles Allocation and Cleanup)
        let (wrapper, _) = self.build_wrapper(&func, &mut builder, cache_size, &ret_type);

        // 2. Generate Inner (Rewrites recursion to use the provider)
        let mut inner = func.clone();
        inner.name = format!("{}.inner", orig_name);
        let cache_arg_ssa = builder.alloc_ssa();

        inner.args.push(MirArgument {
            name: "cache_ptr".to_string(),
            typ: OnuType::Ptr,
            ssa_var: cache_arg_ssa,
        });

        inner.blocks = self.rewrite_calls(
            inner.blocks,
            &mut builder,
            cache_arg_ssa,
            &orig_name,
            ret_type,
        );

        (wrapper, inner)
    }
}

impl CompoundMemoStrategy {
    fn build_wrapper(
        &self,
        func: &MirFunction,
        builder: &mut MirBuilder,
        size: usize,
        typ: &OnuType,
    ) -> (MirFunction, usize) {
        let cache_ptr = builder.alloc_ssa();

        // Use the provider to get the correct stride for this type
        let provider = CacheProvider {
            builder,
            cache_ptr_ssa: cache_ptr,
            ret_type: typ.clone(),
        };
        let stride = provider.get_stride();

        let total_bytes = (size as i64) * stride;
        let size_ssa = builder.alloc_ssa();
        let loop_idx = builder.alloc_ssa();

        let head_id = builder.alloc_block();
        let body_id = builder.alloc_block();
        let call_id = builder.alloc_block();

        // 1. Entry Block
        let entry_insts = vec![
            MirInstruction::Assign {
                dest: size_ssa,
                src: MirOperand::Constant(MirLiteral::I64(total_bytes)),
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

        // 2. Head Block
        let cond_ssa = builder.alloc_ssa();
        let head_block = BasicBlock {
            id: head_id,
            instructions: vec![MirInstruction::BinaryOperation {
                dest: cond_ssa,
                op: MirBinOp::Lt,
                lhs: MirOperand::Variable(loop_idx, false),
                rhs: MirOperand::Constant(MirLiteral::I64(size as i64)),
            }],
            terminator: MirTerminator::CondBranch {
                condition: MirOperand::Variable(cond_ssa, false),
                then_block: body_id,
                else_block: call_id,
            },
        };

        // 3. Body Block
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
                    rhs: MirOperand::Constant(MirLiteral::I64(stride)),
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

        // 4. Call Block
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

    fn rewrite_calls(
        &self,
        blocks: Vec<BasicBlock>,
        builder: &mut MirBuilder,
        cache_ptr: usize,
        orig_name: &str,
        ret_type: OnuType,
    ) -> Vec<BasicBlock> {
        let mut rewritten = vec![];
        let mut provider = CacheProvider {
            builder,
            cache_ptr_ssa: cache_ptr,
            ret_type,
        };

        for block in blocks {
            let mut insts = vec![];
            let mut curr_id = block.id;

            for inst in block.instructions {
                if let MirInstruction::Call {
                    name, dest, args, ..
                } = &inst
                {
                    if name == orig_name {
                        let hit_id = provider.builder.alloc_block();
                        let miss_id = provider.builder.alloc_block();
                        let cont_id = provider.builder.alloc_block();
                        let cond = provider.builder.alloc_ssa();

                        // Access logic: Scaling  and Loading [cite: 67]
                        let offset = provider.compute_offset(&mut insts, args[0].clone());
                        let val = provider.emit_load(&mut insts, offset);

                        insts.push(MirInstruction::BinaryOperation {
                            dest: cond,
                            op: MirBinOp::Ne,
                            lhs: MirOperand::Variable(val, false),
                            rhs: MirOperand::Constant(MirLiteral::I64(-1)),
                        });

                        rewritten.push(BasicBlock {
                            id: curr_id,
                            instructions: insts.drain(..).collect(),
                            terminator: MirTerminator::CondBranch {
                                condition: MirOperand::Variable(cond, false),
                                then_block: hit_id,
                                else_block: miss_id,
                            },
                        });

                        // Hit: Return cached [cite: 92]
                        rewritten.push(BasicBlock {
                            id: hit_id,
                            instructions: vec![MirInstruction::Assign {
                                dest: *dest,
                                src: MirOperand::Variable(val, false),
                            }],
                            terminator: MirTerminator::Branch(cont_id),
                        });

                        // Miss: Call then Safe Store [cite: 73, 80]
                        let mut miss_insts = vec![inst.clone()];
                        provider.emit_store(&mut miss_insts, offset, *dest);
                        rewritten.push(BasicBlock {
                            id: miss_id,
                            instructions: miss_insts,
                            terminator: MirTerminator::Branch(cont_id),
                        });

                        curr_id = cont_id;
                        continue;
                    }
                }
                insts.push(inst);
            }
            rewritten.push(BasicBlock {
                id: curr_id,
                instructions: insts,
                terminator: block.terminator,
            });
        }
        rewritten
    }
}
