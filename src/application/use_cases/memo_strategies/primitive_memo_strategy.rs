use super::{MemoStrategy, max_ssa_in_function};
use crate::application::use_cases::registry_service::RegistryService;
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

/// --- LAYER 2: DOMAIN LOGIC (Cache Accessor) ---
/// This component manages the low-level MIR generation for cache interactions.
/// It is decoupled from the high-level strategy to ensure that memory layout
/// logic remains consistent and testable.
struct CacheAccessor<'a> {
    builder: &'a mut MirBuilder,
    cache_ptr_ssa: usize,
    occ_ptr_ssa: usize,
    ret_type: OnuType,
    registry: &'a RegistryService,
}

impl<'a> CacheAccessor<'a> {
    /// Computes the physical byte offset for a given logical cache index.
    /// We use dynamic stride calculation via the RegistryService to ensure
    /// that we always allocate and access the correct amount of memory,
    /// preventing sub-word corruption.
    fn compute_byte_offset(
        &mut self,
        insts: &mut Vec<MirInstruction>,
        logical_idx: MirOperand,
    ) -> usize {
        let stride = self.registry.size_of(&self.ret_type) as i64;
        let byte_offset_ssa = self.builder.alloc_ssa();
        insts.push(MirInstruction::BinaryOperation {
            dest: byte_offset_ssa,
            op: MirBinOp::Mul,
            lhs: logical_idx,
            rhs: MirOperand::Constant(MirLiteral::I64(stride)),
            dest_type: OnuType::I64,
        });
        byte_offset_ssa
    }

    /// Emits a Load instruction from the cache.
    /// The load is typed according to the function's return type to ensure
    /// that the correct number of bytes are fetched.
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
            typ: self.ret_type.clone(),
        });
        val_ssa
    }

    /// Emits a Store instruction to the cache.
    /// Fixes the previous scope error by explicitly using the provided type.
    /// This ensures memory safety by matching the store width to the data width.
    fn emit_safe_store(
        &mut self,
        insts: &mut Vec<MirInstruction>,
        byte_offset: usize,
        value_ssa: usize,
        typ: OnuType,
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
            typ,
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
        registry: &RegistryService,
    ) -> (MirFunction, MirFunction) {
        let mut builder = MirBuilder::new(&func);
        let original_name = func.name.clone();
        let ret_type = func.return_type.clone();

        // 1. Build Wrapper
        let (wrapper_func, _) =
            self.build_wrapper(&func, &mut builder, cache_size, &ret_type, registry);

        // 2. Build Inner
        let mut inner_func = func.clone();
        inner_func.name = format!("{}.inner", original_name);
        let inner_cache_ptr_ssa = builder.alloc_ssa();
        let inner_occ_ptr_ssa = builder.alloc_ssa();

        inner_func.args.push(MirArgument {
            name: "cache_ptr".to_string(),
            typ: OnuType::Ptr,
            ssa_var: inner_cache_ptr_ssa,
        });

        inner_func.args.push(MirArgument {
            name: "occ_ptr".to_string(),
            typ: OnuType::Ptr,
            ssa_var: inner_occ_ptr_ssa,
        });

        inner_func.blocks = self.rewrite_inner_calls(
            inner_func.blocks,
            &mut builder,
            inner_cache_ptr_ssa,
            inner_occ_ptr_ssa,
            &original_name,
            cache_size,
            ret_type,
            registry,
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
        ret_type: &OnuType,
        registry: &RegistryService,
    ) -> (MirFunction, usize) {
        // 1. Safe Size Calculation (Checked Math)
        let stride = registry.size_of(ret_type) as i64;
        let total_bytes = (cache_size as i64)
            .checked_mul(stride)
            .expect("Memo cache size calculation overflowed i64") as usize;

        // 2. Global backing store — allocated exactly once, zero-initialised by
        //    the OS/loader.  This avoids draining the per-call arena bump-pointer
        //    when the wrapper is invoked from an outer loop.
        let cache_ptr = builder.alloc_ssa();
        let occ_ptr   = builder.alloc_ssa();

        let call_id = builder.alloc_block();

        // 3. Entry Block: obtain pointers to the global arrays.
        //    No Alloc/MemSet needed — GlobalAlloc yields a pointer to a
        //    module-level zeroed global, so the occupancy flags start at 0
        //    and the block is idempotent across multiple calls.
        let entry_insts = vec![
            MirInstruction::GlobalAlloc {
                dest: cache_ptr,
                size_bytes: total_bytes,
                name: format!("{}_cache_val", func.name),
            },
            MirInstruction::GlobalAlloc {
                dest: occ_ptr,
                size_bytes: cache_size,
                name: format!("{}_cache_occ", func.name),
            },
        ];

        let entry_block = BasicBlock {
            id: 0,
            instructions: entry_insts,
            terminator: MirTerminator::Branch(call_id),
        };

        // 4. Call Block (Call inner and return)
        let res_ssa = builder.alloc_ssa();
        let mut call_args: Vec<MirOperand> = func
            .args
            .iter()
            .map(|arg| MirOperand::Variable(arg.ssa_var, false))
            .collect();
        call_args.push(MirOperand::Variable(cache_ptr, false));
        call_args.push(MirOperand::Variable(occ_ptr, false));

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
                    .chain(vec![OnuType::Ptr, OnuType::Ptr].into_iter())
                    .collect(),
                is_tail_call: false,
            }],
            terminator: MirTerminator::Return(MirOperand::Variable(res_ssa, false)),
        };

        (
            MirFunction {
                name: func.name.clone(),
                args: func.args.clone(),
                blocks: vec![entry_block, call_block],
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
        occ_ptr: usize,
        orig_name: &str,
        cache_size: usize,
        ret_type: OnuType,
        registry: &RegistryService,
    ) -> Vec<BasicBlock> {
        let mut rewritten = vec![];
        let mut accessor = CacheAccessor {
            builder,
            cache_ptr_ssa: cache_ptr,
            occ_ptr_ssa: occ_ptr,
            ret_type: ret_type.clone(),
            registry,
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
                        ref return_type,
                        ref arg_types,
                        ..
                    } if name == orig_name && args.len() == 1 => {
                        let upper_check_id = accessor.builder.alloc_block();
                        let fetch_id = accessor.builder.alloc_block();
                        let miss_id = accessor.builder.alloc_block();
                        let raw_call_id = accessor.builder.alloc_block(); // NEW: Call without store
                        let hit_id = accessor.builder.alloc_block();
                        let store_id = accessor.builder.alloc_block();
                        let cont_id = accessor.builder.alloc_block();

                        // --- DOMINATING CALCULATIONS ---
                        // We compute offsets in the current block so they are available
                        // to both fetch_id and store_id blocks.
                        let occ_offset = accessor.builder.alloc_ssa();
                        current_insts.push(MirInstruction::BinaryOperation {
                            dest: occ_offset,
                            op: MirBinOp::Mul,
                            lhs: args[0].clone(),
                            rhs: MirOperand::Constant(MirLiteral::I64(1)),
                            dest_type: OnuType::I64,
                        });

                        let val_offset =
                            accessor.compute_byte_offset(&mut current_insts, args[0].clone());

                        // 1. Lower Bound Check (arg >= 0)
                        let l_check = accessor.builder.alloc_ssa();
                        current_insts.push(MirInstruction::BinaryOperation {
                            dest: l_check,
                            op: MirBinOp::Lt,
                            lhs: args[0].clone(),
                            rhs: MirOperand::Constant(MirLiteral::I64(0)),
                            dest_type: OnuType::Boolean,
                        });

                        rewritten.push(BasicBlock {
                            id: current_block_id,
                            instructions: current_insts.drain(..).collect(),
                            terminator: MirTerminator::CondBranch {
                                condition: MirOperand::Variable(l_check, false),
                                then_block: raw_call_id, // Skip cache if negative
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
                                dest_type: OnuType::Boolean,
                            }],
                            terminator: MirTerminator::CondBranch {
                                condition: MirOperand::Variable(u_check, false),
                                then_block: fetch_id,
                                else_block: raw_call_id, // Skip cache if out of bounds
                            },
                        });

                        // 3. FETCH BLOCK
                        let mut fetch_insts = vec![];
                        let fetch_occ_ptr = accessor.builder.alloc_ssa();
                        let occ_flag = accessor.builder.alloc_ssa();
                        fetch_insts.push(MirInstruction::PointerOffset {
                            dest: fetch_occ_ptr,
                            ptr: MirOperand::Variable(accessor.occ_ptr_ssa, false),
                            offset: MirOperand::Variable(occ_offset, false),
                        });
                        fetch_insts.push(MirInstruction::Load {
                            dest: occ_flag,
                            ptr: MirOperand::Variable(fetch_occ_ptr, false),
                            typ: OnuType::I8,
                        });

                        let hit_cond = accessor.builder.alloc_ssa();
                        fetch_insts.push(MirInstruction::BinaryOperation {
                            dest: hit_cond,
                            op: MirBinOp::Ne,
                            lhs: MirOperand::Variable(occ_flag, false),
                            rhs: MirOperand::Constant(MirLiteral::I64(0)),
                            dest_type: OnuType::Boolean,
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
                        let mut hit_insts = vec![];
                        let val = accessor.emit_load(&mut hit_insts, val_offset);
                        hit_insts.push(MirInstruction::Assign {
                            dest,
                            src: MirOperand::Variable(val, false),
                        });
                        rewritten.push(BasicBlock {
                            id: hit_id,
                            instructions: hit_insts,
                            terminator: MirTerminator::Branch(cont_id),
                        });

                        // 5. RAW CALL BLOCK (For out-of-bounds keys)
                        let mut raw_args = args.clone();
                        raw_args.push(MirOperand::Variable(cache_ptr, false));
                        raw_args.push(MirOperand::Variable(occ_ptr, false));
                        let mut raw_arg_types = arg_types.clone();
                        raw_arg_types.push(OnuType::Ptr);
                        raw_arg_types.push(OnuType::Ptr);

                        rewritten.push(BasicBlock {
                            id: raw_call_id,
                            instructions: vec![MirInstruction::Call {
                                name: format!("{}.inner", orig_name),
                                dest,
                                args: raw_args,
                                is_tail_call: false, // Must be false as we are not in tail position
                                return_type: return_type.clone(),
                                arg_types: raw_arg_types,
                            }],
                            terminator: MirTerminator::Branch(cont_id),
                        });

                        // 6. MISS BLOCK
                        let mut miss_args = args.clone();
                        miss_args.push(MirOperand::Variable(cache_ptr, false));
                        miss_args.push(MirOperand::Variable(occ_ptr, false));
                        let mut miss_arg_types = arg_types.clone();
                        miss_arg_types.push(OnuType::Ptr);
                        miss_arg_types.push(OnuType::Ptr);

                        rewritten.push(BasicBlock {
                            id: miss_id,
                            instructions: vec![MirInstruction::Call {
                                name: format!("{}.inner", orig_name),
                                dest,
                                args: miss_args,
                                is_tail_call: false, // Must be false as we follow with store
                                return_type: return_type.clone(),
                                arg_types: miss_arg_types,
                            }],
                            terminator: MirTerminator::Branch(store_id),
                        });

                        // 7. STORE BLOCK
                        let mut store_insts = vec![];
                        accessor.emit_safe_store(
                            &mut store_insts,
                            val_offset,
                            dest,
                            accessor.ret_type.clone(),
                        );
                        let store_occ_ptr = accessor.builder.alloc_ssa();
                        store_insts.push(MirInstruction::PointerOffset {
                            dest: store_occ_ptr,
                            ptr: MirOperand::Variable(accessor.occ_ptr_ssa, false),
                            offset: MirOperand::Variable(occ_offset, false),
                        });
                        store_insts.push(MirInstruction::TypedStore {
                            ptr: MirOperand::Variable(store_occ_ptr, false),
                            value: MirOperand::Constant(MirLiteral::I64(1)),
                            typ: OnuType::I8,
                        });

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
