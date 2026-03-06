use super::{MemoStrategy, max_ssa_in_function};
use crate::domain::entities::mir::{
    BasicBlock, MirArgument, MirBinOp, MirFunction, MirInstruction, MirLiteral, MirOperand,
    MirTerminator,
};
use crate::domain::entities::types::OnuType;

/// Maximum cache memory per function (1 MiB).  When the product of per-dimension
/// bounds would exceed this limit the dimension size is capped so total allocation
/// stays within the arena.
const CACHE_MEMORY_LIMIT: usize = 1_048_576;

// --- LAYER 1: INFRASTRUCTURE ---
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

// --- LAYER 2: DOMAIN LOGIC (Security & Layout) ---
struct CacheProvider<'a> {
    builder: &'a mut MirBuilder,
    cache_ptr_ssa: usize,
    occ_ptr_ssa: usize,
    ret_type: OnuType,
    /// Number of cache entries along each dimension (capped by memory guard).
    dim_size: usize,
    /// Number of dimensions (= original function arg count).
    n_dims: usize,
    registry: &'a crate::application::use_cases::registry_service::RegistryService,
}

impl<'a> CacheProvider<'a> {
    fn get_stride(&self) -> i64 {
        self.registry.size_of(&self.ret_type) as i64
    }

    /// Total number of cache entries: `dim_size ^ n_dims`.
    /// Uses saturating arithmetic since `safe_dim_size` already guarantees this fits.
    fn total_entries(&self) -> i64 {
        (self.dim_size as i64).saturating_pow(self.n_dims as u32)
    }

    /// Compute the byte offset into the cache for a given logical (flat) index SSA.
    fn compute_byte_offset(
        &mut self,
        insts: &mut Vec<MirInstruction>,
        flat_idx_ssa: usize,
    ) -> usize {
        let offset_ssa = self.builder.alloc_ssa();
        insts.push(MirInstruction::BinaryOperation {
            dest: offset_ssa,
            op: MirBinOp::Mul,
            lhs: MirOperand::Variable(flat_idx_ssa, false),
            rhs: MirOperand::Constant(MirLiteral::I64(self.get_stride())),
            dest_type: OnuType::I64,
        });
        offset_ssa
    }

    /// Emit Horner's-method flat-index computation: `(...((a0*S + a1)*S + a2)...+ a_{N-1})`.
    /// For N=1 this is just an Assign of `args[0]`.
    fn compute_flat_index(
        &mut self,
        insts: &mut Vec<MirInstruction>,
        args: &[MirOperand],
    ) -> usize {
        let first_ssa = self.builder.alloc_ssa();
        insts.push(MirInstruction::Assign {
            dest: first_ssa,
            src: args[0].clone(),
        });
        if args.len() == 1 {
            return first_ssa;
        }
        let dim_size_lit = MirOperand::Constant(MirLiteral::I64(self.dim_size as i64));
        let mut acc = first_ssa;
        for arg in &args[1..] {
            let scaled = self.builder.alloc_ssa();
            insts.push(MirInstruction::BinaryOperation {
                dest: scaled,
                op: MirBinOp::Mul,
                lhs: MirOperand::Variable(acc, false),
                rhs: dim_size_lit.clone(),
                dest_type: OnuType::I64,
            });
            let summed = self.builder.alloc_ssa();
            insts.push(MirInstruction::BinaryOperation {
                dest: summed,
                op: MirBinOp::Add,
                lhs: MirOperand::Variable(scaled, false),
                rhs: arg.clone(),
                dest_type: OnuType::I64,
            });
            acc = summed;
        }
        acc
    }
}

// --- LAYER 3: THE FULL STRATEGY ---
pub struct CompoundMemoStrategy;

impl MemoStrategy for CompoundMemoStrategy {
    fn create_wrapper_and_inner(
        &self,
        func: MirFunction,
        cache_size: usize,
        registry: &crate::application::use_cases::registry_service::RegistryService,
    ) -> (MirFunction, MirFunction) {
        let mut builder = MirBuilder::new(&func);
        let orig_name = func.name.clone();
        let ret_type = func.return_type.clone();
        let n_dims = func.args.len();

        let (wrapper, _, _) =
            self.build_wrapper(&func, &mut builder, cache_size, &ret_type, registry);

        let mut inner = func.clone();
        inner.name = format!("{}.inner", orig_name);
        let cache_arg_ssa = builder.alloc_ssa();
        let occ_arg_ssa = builder.alloc_ssa();

        inner.args.push(MirArgument {
            name: "cache_ptr".to_string(),
            typ: OnuType::Ptr,
            ssa_var: cache_arg_ssa,
        });

        inner.args.push(MirArgument {
            name: "occ_ptr".to_string(),
            typ: OnuType::Ptr,
            ssa_var: occ_arg_ssa,
        });

        // Compute the same safe dim_size used by build_wrapper.
        let stride = registry.size_of(&ret_type) as usize;
        let dim_size = Self::safe_dim_size(n_dims, stride, cache_size);

        inner.blocks = self.rewrite_calls(
            inner.blocks,
            &mut builder,
            cache_arg_ssa,
            occ_arg_ssa,
            &orig_name,
            ret_type,
            dim_size,
            n_dims,
            registry,
        );

        (wrapper, inner)
    }
}

impl CompoundMemoStrategy {
    /// Compute the largest per-dimension cache size such that the total allocation
    /// (`dim_size ^ n_dims * stride`) stays within `CACHE_MEMORY_LIMIT`.
    /// This is the "memory guard" that prevents arena overflow for large ND inputs.
    fn safe_dim_size(n_dims: usize, stride: usize, nominal: usize) -> usize {
        let stride = stride.max(1);
        let limit_entries = CACHE_MEMORY_LIMIT / stride;
        // Use floating-point for an initial approximation, then validate
        // and adjust downward to correct for any rounding error.
        let mut max_dim = (limit_entries as f64).powf(1.0 / n_dims as f64) as usize;
        max_dim = max_dim.max(1);
        // Ensure dim_size^n_dims doesn't overflow usize and stays within the limit.
        while max_dim > 1 {
            let product = (max_dim as u128).pow(n_dims as u32);
            if product <= limit_entries as u128 {
                break;
            }
            max_dim -= 1;
        }
        max_dim.min(nominal).max(1)
    }

    fn build_wrapper(
        &self,
        func: &MirFunction,
        builder: &mut MirBuilder,
        size: usize,
        typ: &OnuType,
        registry: &crate::application::use_cases::registry_service::RegistryService,
    ) -> (MirFunction, usize, usize) {
        let cache_ptr = builder.alloc_ssa();
        let occ_ptr = builder.alloc_ssa();
        let cache_size_ssa = builder.alloc_ssa();
        let occ_size_ssa = builder.alloc_ssa();

        let call_id = builder.alloc_block();

        let n_dims = func.args.len();
        let stride = registry.size_of(typ) as usize;
        // Memory guard: cap per-dimension size so total stays within the 1 MiB limit.
        let dim_size = Self::safe_dim_size(n_dims, stride, size);
        // safe_dim_size guarantees dim_size^n_dims * stride <= CACHE_MEMORY_LIMIT,
        // so these products fit in i64 without overflow.
        let total_entries = (dim_size as i64).saturating_pow(n_dims as u32);
        let total_bytes = total_entries.saturating_mul(stride as i64);
        let occ_bytes = total_entries;

        // 1. Entry Block
        let entry_insts = vec![
            MirInstruction::Assign {
                dest: cache_size_ssa,
                src: MirOperand::Constant(MirLiteral::I64(total_bytes)),
            },
            MirInstruction::Alloc {
                dest: cache_ptr,
                size_bytes: MirOperand::Variable(cache_size_ssa, false),
            },
            MirInstruction::Assign {
                dest: occ_size_ssa,
                src: MirOperand::Constant(MirLiteral::I64(occ_bytes)),
            },
            MirInstruction::Alloc {
                dest: occ_ptr,
                size_bytes: MirOperand::Variable(occ_size_ssa, false),
            },
            MirInstruction::MemSet {
                ptr: MirOperand::Variable(occ_ptr, false),
                value: MirOperand::Constant(MirLiteral::I64(0)),
                size: MirOperand::Variable(occ_size_ssa, false),
            },
        ];

        let entry_block = BasicBlock {
            id: 0,
            instructions: entry_insts,
            terminator: MirTerminator::Branch(call_id),
        };

        // 2. Call Block
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
                    .chain(std::iter::once(OnuType::Ptr))
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
                blocks: vec![entry_block, call_block],
                is_pure_data_leaf: false,
                ..func.clone()
            },
            cache_ptr,
            occ_ptr,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn rewrite_calls(
        &self,
        blocks: Vec<BasicBlock>,
        builder: &mut MirBuilder,
        _cache_ptr: usize,
        _occ_ptr: usize,
        orig_name: &str,
        ret_type: OnuType,
        dim_size: usize,
        n_dims: usize,
        registry: &crate::application::use_cases::registry_service::RegistryService,
    ) -> Vec<BasicBlock> {
        let mut rewritten = vec![];
        let mut provider = CacheProvider {
            builder,
            cache_ptr_ssa: _cache_ptr,
            occ_ptr_ssa: _occ_ptr,
            ret_type,
            dim_size,
            n_dims,
            registry,
        };

        for block in blocks {
            let mut insts = vec![];
            let mut curr_id = block.id;

            for inst in block.instructions {
                match inst {
                    MirInstruction::Call {
                        ref name,
                        dest,
                        ref args,
                        is_tail_call: _,
                        ref return_type,
                        ref arg_types,
                    } if name == orig_name && args.len() == n_dims => {
                        let fetch_id = provider.builder.alloc_block();
                        let miss_in_bounds_id = provider.builder.alloc_block();
                        let miss_out_of_bounds_id = provider.builder.alloc_block();
                        let hit_id = provider.builder.alloc_block();
                        let store_id = provider.builder.alloc_block();
                        let cont_id = provider.builder.alloc_block();

                        // --- Compute flat index (Horner's) in current block ---
                        let flat_ssa =
                            provider.compute_flat_index(&mut insts, args);

                        // --- Build per-dimension bound-check chain ---
                        let upper_check_0 = provider.builder.alloc_block();
                        let extra_check_blocks: Vec<(usize, usize)> = (1..n_dims)
                            .map(|_| {
                                (
                                    provider.builder.alloc_block(),
                                    provider.builder.alloc_block(),
                                )
                            })
                            .collect();

                        // --- curr_id: lower bound check for dim 0 ---
                        let l_check_0 = provider.builder.alloc_ssa();
                        insts.push(MirInstruction::BinaryOperation {
                            dest: l_check_0,
                            op: MirBinOp::Lt,
                            lhs: args[0].clone(),
                            rhs: MirOperand::Constant(MirLiteral::I64(0)),
                            dest_type: OnuType::Boolean,
                        });
                        rewritten.push(BasicBlock {
                            id: curr_id,
                            instructions: insts.drain(..).collect(),
                            terminator: MirTerminator::CondBranch {
                                condition: MirOperand::Variable(l_check_0, false),
                                then_block: miss_out_of_bounds_id,
                                else_block: upper_check_0,
                            },
                        });

                        // --- upper_check_0: upper bound check for dim 0 ---
                        let next_after_0 = if extra_check_blocks.is_empty() {
                            fetch_id
                        } else {
                            extra_check_blocks[0].0
                        };
                        let u_check_0 = provider.builder.alloc_ssa();
                        rewritten.push(BasicBlock {
                            id: upper_check_0,
                            instructions: vec![MirInstruction::BinaryOperation {
                                dest: u_check_0,
                                op: MirBinOp::Lt,
                                lhs: args[0].clone(),
                                rhs: MirOperand::Constant(MirLiteral::I64(dim_size as i64)),
                                dest_type: OnuType::Boolean,
                            }],
                            terminator: MirTerminator::CondBranch {
                                condition: MirOperand::Variable(u_check_0, false),
                                then_block: next_after_0,
                                else_block: miss_out_of_bounds_id,
                            },
                        });

                        // --- Remaining dimension checks (dims 1..N-1) ---
                        for (i, (lower_id, upper_id)) in extra_check_blocks.iter().enumerate() {
                            let actual_dim = i + 1;
                            let next_pass = if i + 1 < extra_check_blocks.len() {
                                extra_check_blocks[i + 1].0
                            } else {
                                fetch_id
                            };

                            let l_check = provider.builder.alloc_ssa();
                            rewritten.push(BasicBlock {
                                id: *lower_id,
                                instructions: vec![MirInstruction::BinaryOperation {
                                    dest: l_check,
                                    op: MirBinOp::Lt,
                                    lhs: args[actual_dim].clone(),
                                    rhs: MirOperand::Constant(MirLiteral::I64(0)),
                                    dest_type: OnuType::Boolean,
                                }],
                                terminator: MirTerminator::CondBranch {
                                    condition: MirOperand::Variable(l_check, false),
                                    then_block: miss_out_of_bounds_id,
                                    else_block: *upper_id,
                                },
                            });

                            let u_check = provider.builder.alloc_ssa();
                            rewritten.push(BasicBlock {
                                id: *upper_id,
                                instructions: vec![MirInstruction::BinaryOperation {
                                    dest: u_check,
                                    op: MirBinOp::Lt,
                                    lhs: args[actual_dim].clone(),
                                    rhs: MirOperand::Constant(MirLiteral::I64(dim_size as i64)),
                                    dest_type: OnuType::Boolean,
                                }],
                                terminator: MirTerminator::CondBranch {
                                    condition: MirOperand::Variable(u_check, false),
                                    then_block: next_pass,
                                    else_block: miss_out_of_bounds_id,
                                },
                            });
                        }

                        // --- 3. FETCH BLOCK ---
                        let mut fetch_insts = vec![];
                        let occ_ptr_slot = provider.builder.alloc_ssa();
                        let occ_flag_ssa = provider.builder.alloc_ssa();

                        fetch_insts.push(MirInstruction::PointerOffset {
                            dest: occ_ptr_slot,
                            ptr: MirOperand::Variable(provider.occ_ptr_ssa, false),
                            offset: MirOperand::Variable(flat_ssa, false),
                        });
                        fetch_insts.push(MirInstruction::Load {
                            dest: occ_flag_ssa,
                            ptr: MirOperand::Variable(occ_ptr_slot, false),
                            typ: OnuType::I8,
                        });

                        let hit_cond = provider.builder.alloc_ssa();
                        fetch_insts.push(MirInstruction::BinaryOperation {
                            dest: hit_cond,
                            op: MirBinOp::Ne,
                            lhs: MirOperand::Variable(occ_flag_ssa, false),
                            rhs: MirOperand::Constant(MirLiteral::I64(0)),
                            dest_type: OnuType::Boolean,
                        });

                        let byte_offset = provider.compute_byte_offset(&mut fetch_insts, flat_ssa);
                        let ptr_ssa = provider.builder.alloc_ssa();
                        let val_ssa = provider.builder.alloc_ssa();
                        fetch_insts.push(MirInstruction::PointerOffset {
                            dest: ptr_ssa,
                            ptr: MirOperand::Variable(provider.cache_ptr_ssa, false),
                            offset: MirOperand::Variable(byte_offset, false),
                        });
                        fetch_insts.push(MirInstruction::Load {
                            dest: val_ssa,
                            ptr: MirOperand::Variable(ptr_ssa, false),
                            typ: provider.ret_type.clone(),
                        });

                        rewritten.push(BasicBlock {
                            id: fetch_id,
                            instructions: fetch_insts,
                            terminator: MirTerminator::CondBranch {
                                condition: MirOperand::Variable(hit_cond, false),
                                then_block: hit_id,
                                else_block: miss_in_bounds_id,
                            },
                        });

                        // --- 4. HIT BLOCK ---
                        rewritten.push(BasicBlock {
                            id: hit_id,
                            instructions: vec![MirInstruction::Assign {
                                dest,
                                src: MirOperand::Variable(val_ssa, false),
                            }],
                            terminator: MirTerminator::Branch(cont_id),
                        });

                        // --- 5. MISS BLOCKS (In-Bounds vs Out-of-Bounds) ---
                        let mut new_arg_types = arg_types.clone();
                        new_arg_types.push(OnuType::Ptr);
                        new_arg_types.push(OnuType::Ptr);
                        let mut new_args = args.clone();
                        new_args.push(MirOperand::Variable(provider.cache_ptr_ssa, false));
                        new_args.push(MirOperand::Variable(provider.occ_ptr_ssa, false));

                        // Path A: In-Bounds. Call and then store the result.
                        rewritten.push(BasicBlock {
                            id: miss_in_bounds_id,
                            instructions: vec![MirInstruction::Call {
                                name: format!("{}.inner", orig_name),
                                dest,
                                args: new_args.clone(),
                                is_tail_call: false,
                                return_type: return_type.clone(),
                                arg_types: new_arg_types.clone(),
                            }],
                            terminator: MirTerminator::Branch(store_id),
                        });

                        // Path B: Out-of-Bounds. Call and then skip the store.
                        rewritten.push(BasicBlock {
                            id: miss_out_of_bounds_id,
                            instructions: vec![MirInstruction::Call {
                                name: format!("{}.inner", orig_name),
                                dest,
                                args: new_args,
                                is_tail_call: false,
                                return_type: return_type.clone(),
                                arg_types: new_arg_types,
                            }],
                            terminator: MirTerminator::Branch(cont_id),
                        });

                        // --- 6. STORE BLOCK ---
                        let mut store_insts = vec![];
                        let store_byte_offset =
                            provider.compute_byte_offset(&mut store_insts, flat_ssa);
                        let store_ptr = provider.builder.alloc_ssa();
                        store_insts.push(MirInstruction::PointerOffset {
                            dest: store_ptr,
                            ptr: MirOperand::Variable(provider.cache_ptr_ssa, false),
                            offset: MirOperand::Variable(store_byte_offset, false),
                        });
                        store_insts.push(MirInstruction::TypedStore {
                            ptr: MirOperand::Variable(store_ptr, false),
                            value: MirOperand::Variable(dest, false),
                            typ: provider.ret_type.clone(),
                        });

                        let occ_store_ptr = provider.builder.alloc_ssa();
                        store_insts.push(MirInstruction::PointerOffset {
                            dest: occ_store_ptr,
                            ptr: MirOperand::Variable(provider.occ_ptr_ssa, false),
                            offset: MirOperand::Variable(flat_ssa, false),
                        });
                        store_insts.push(MirInstruction::TypedStore {
                            ptr: MirOperand::Variable(occ_store_ptr, false),
                            value: MirOperand::Constant(MirLiteral::I64(1)),
                            typ: OnuType::I8,
                        });

                        rewritten.push(BasicBlock {
                            id: store_id,
                            instructions: store_insts,
                            terminator: MirTerminator::Branch(cont_id),
                        });

                        curr_id = cont_id;
                    }
                    inst => insts.push(inst),
                }
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
