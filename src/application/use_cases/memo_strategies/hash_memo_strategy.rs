/// # HashMemoStrategy
///
/// A hash-table–based memoization strategy for multi-dimensional functions
/// where one or more arguments can take arbitrarily large values.
///
/// ## Why it exists
///
/// `CompoundMemoStrategy` allocates a flat N-dimensional array indexed by
/// `(a₀, a₁, …, aₙ₋₁)` with each dimension capped at `dim_size`.  For
/// Ackermann(3, 11) the outer recursive call is `ackermann(m-1, spiral_step)`
/// where `spiral_step = ackermann(m, n-1)` grows exponentially.  With
/// `dim_size = 1024`, 99.99 % of the 147 million recursive calls have
/// `spiral_step ≥ 1024` and fall through to the uncached slow path.  The
/// flat-array design is structurally incapable of helping here.
///
/// `HashMemoStrategy` replaces the flat array with a direct-mapped hash table:
///
/// ```text
/// slot  = (a₀ * PRIME + a₁ * PRIME² + … ) & (table_size − 1)
/// cache_ptr[slot] = return_value      (8 bytes for I64)
/// keys_ptr[slot]  = [key₀, key₁, …, keyₙ₋₁, valid]  ((N+1)×8 bytes)
/// ```
///
/// Every (m, n) pair maps to a slot regardless of its magnitude.  On a hit
/// the stored keys are compared with the current arguments; a mismatch
/// means a collision — the function is recomputed without evicting the
/// existing occupant (open-addressed direct-map, no chaining).
///
/// ## Sizing
///
/// With 16 MiB arena and N=2, stride=8:
///   bytes_per_slot = 8 (value) + (2+1)×8 (keys+valid) = 32
///   table_size     = 16 MiB / 32 = 524,288 entries  (2¹⁹, power of 2)
///
/// For Ackermann(3, 11) there are ≈41 K unique (m, n) pairs → 7.8 % load
/// factor → virtually zero collisions.
///
/// ## Slot layout (keys_ptr)
/// ```
/// byte  0 ..  7  : key₀  (i64)
/// byte  8 .. 15  : key₁  (i64)
/// …
/// byte  N*8      : valid  (i8, stored as i8 but in an 8-byte slot)
/// ```
///
/// The `valid` flag at `keys_ptr + slot*key_stride + N*8` is initialised to
/// 0 by the wrapper's `MemSet` and set to 1 after the first computation.

use super::{MemoStrategy, max_ssa_in_function};
use crate::application::use_cases::registry_service::RegistryService;
use crate::domain::entities::mir::{
    BasicBlock, MirArgument, MirBinOp, MirFunction, MirInstruction, MirLiteral, MirOperand,
    MirTerminator,
};
use crate::domain::entities::types::OnuType;
use crate::domain::entities::ARENA_SIZE_BYTES;

/// Knuth's multiplicative hashing constant (golden-ratio derivation).
/// Provides excellent bit avalanche for small integer keys like Ackermann's
/// (m, n) pairs.
const HASH_PRIME: i64 = 2_654_435_769_i64;

/// Byte width of each key field stored in the keys/valid table.
/// All function arguments are I64 (8 bytes) and the valid flag occupies one
/// padded 8-byte slot for alignment.
const KEY_FIELD_BYTES: i64 = 8;

// ---------------------------------------------------------------------------
// Memory layout helper
// ---------------------------------------------------------------------------

/// Return the largest power-of-2 table size whose combined allocation
/// (value table + key/valid table) fits within `ARENA_SIZE_BYTES`.
///
/// Per slot:
///   - value: `stride` bytes
///   - keys + valid: `(n_dims + 1) * KEY_FIELD_BYTES` bytes
fn safe_table_size(n_dims: usize, stride: usize) -> usize {
    let bytes_per_slot = stride + (n_dims + 1) * KEY_FIELD_BYTES as usize;
    if bytes_per_slot == 0 {
        return 1;
    }
    let max_entries = ARENA_SIZE_BYTES / bytes_per_slot;
    let mut p = 1usize;
    while p * 2 <= max_entries {
        p *= 2;
    }
    p.max(1)
}

// ---------------------------------------------------------------------------
// Private MIR builder (mirrors the one in CompoundMemoStrategy)
// ---------------------------------------------------------------------------

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
        let id = self.next_ssa;
        self.next_ssa += 1;
        id
    }
    fn alloc_block(&mut self) -> usize {
        let id = self.next_block_id;
        self.next_block_id += 1;
        id
    }
}

// ---------------------------------------------------------------------------
// Strategy
// ---------------------------------------------------------------------------

pub struct HashMemoStrategy;

impl MemoStrategy for HashMemoStrategy {
    fn create_wrapper_and_inner(
        &self,
        func: MirFunction,
        _cache_size: usize,
        registry: &RegistryService,
    ) -> (MirFunction, MirFunction) {
        let mut builder = MirBuilder::new(&func);
        let orig_name = func.name.clone();
        let ret_type = func.return_type.clone();
        let n_dims = func.args.len();
        let stride = registry.size_of(&ret_type) as usize;
        let table_size = safe_table_size(n_dims, stride);

        let (wrapper, cache_ptr, keys_ptr) =
            build_wrapper(&func, &mut builder, n_dims, stride, table_size);

        let mut inner = func.clone();
        inner.name = format!("{}.inner", orig_name);
        // The inner function reads from and writes to the hash table, so it
        // must NOT inherit is_pure_data_leaf=true from the original.  A pure
        // (ReadNone) attribute would tell LLVM the function has no memory
        // effects, letting DSE eliminate the cache stores.
        inner.is_pure_data_leaf = false;

        let cache_arg_ssa = builder.alloc_ssa();
        let keys_arg_ssa = builder.alloc_ssa();

        inner.args.push(MirArgument {
            name: "cache_ptr".to_string(),
            typ: OnuType::Ptr,
            ssa_var: cache_arg_ssa,
        });
        inner.args.push(MirArgument {
            name: "keys_ptr".to_string(),
            typ: OnuType::Ptr,
            ssa_var: keys_arg_ssa,
        });

        inner.blocks = rewrite_calls(
            inner.blocks,
            &mut builder,
            cache_arg_ssa,
            keys_arg_ssa,
            &orig_name,
            ret_type,
            n_dims,
            stride,
            table_size,
        );

        (wrapper, inner)
    }
}

// ---------------------------------------------------------------------------
// Wrapper builder
// ---------------------------------------------------------------------------

/// Emit a wrapper that allocates both tables, zeroes the key/valid table,
/// then delegates to `{name}.inner`.
fn build_wrapper(
    func: &MirFunction,
    builder: &mut MirBuilder,
    n_dims: usize,
    stride: usize,
    table_size: usize,
) -> (MirFunction, usize, usize) {
    let cache_ptr = builder.alloc_ssa();
    let keys_ptr = builder.alloc_ssa();
    let cache_size_ssa = builder.alloc_ssa();
    let keys_size_ssa = builder.alloc_ssa();
    let call_id = builder.alloc_block();

    let cache_bytes = (table_size as i64) * (stride as i64);
    let key_stride = ((n_dims + 1) as i64) * KEY_FIELD_BYTES;
    let keys_bytes = (table_size as i64) * key_stride;

    let entry_insts = vec![
        MirInstruction::Assign {
            dest: cache_size_ssa,
            src: MirOperand::Constant(MirLiteral::I64(cache_bytes)),
        },
        MirInstruction::Alloc {
            dest: cache_ptr,
            size_bytes: MirOperand::Variable(cache_size_ssa, false),
        },
        MirInstruction::Assign {
            dest: keys_size_ssa,
            src: MirOperand::Constant(MirLiteral::I64(keys_bytes)),
        },
        MirInstruction::Alloc {
            dest: keys_ptr,
            size_bytes: MirOperand::Variable(keys_size_ssa, false),
        },
        // Zero only the key/valid table; the cache values are written before
        // being read, so they need not be initialised.
        MirInstruction::MemSet {
            ptr: MirOperand::Variable(keys_ptr, false),
            value: MirOperand::Constant(MirLiteral::I64(0)),
            size: MirOperand::Variable(keys_size_ssa, false),
        },
    ];

    let entry_block = BasicBlock {
        id: 0,
        instructions: entry_insts,
        terminator: MirTerminator::Branch(call_id),
    };

    let res_ssa = builder.alloc_ssa();
    let mut call_args: Vec<MirOperand> = func
        .args
        .iter()
        .map(|a| MirOperand::Variable(a.ssa_var, false))
        .collect();
    call_args.push(MirOperand::Variable(cache_ptr, false));
    call_args.push(MirOperand::Variable(keys_ptr, false));

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
        keys_ptr,
    )
}

// ---------------------------------------------------------------------------
// Call-site rewriter
// ---------------------------------------------------------------------------

/// Rewrite every call to `orig_name(args…)` inside `blocks` to the hash-table
/// lookup + `{orig_name}.inner(args…, cache_ptr, keys_ptr)` fallback.
///
/// Generated control flow per call site:
///
/// ```text
/// PREAMBLE  → compute hash → slot → key/cache byte offsets
///           → load valid flag
///           → is_occupied? → KEY_CHECK_0 : MISS_EMPTY
///
/// KEY_CHECK_i  → load stored key[i], compare with args[i]
///             → match? → KEY_CHECK_{i+1} / HIT : MISS_COLLISION
///
/// HIT          → load cached value → assign dest → CONT
/// MISS_EMPTY   → call .inner → STORE
/// MISS_COLLISION → call .inner (no eviction) → CONT
/// STORE        → write value + all keys + valid=1 → CONT
/// CONT         → rest of original block
/// ```
#[allow(clippy::too_many_arguments)]
fn rewrite_calls(
    blocks: Vec<BasicBlock>,
    builder: &mut MirBuilder,
    cache_ptr_ssa: usize,
    keys_ptr_ssa: usize,
    orig_name: &str,
    ret_type: OnuType,
    n_dims: usize,
    stride: usize,
    table_size: usize,
) -> Vec<BasicBlock> {
    // Bitmask for power-of-2 modulo: slot = hash & mask
    let mask = (table_size - 1) as i64;
    // Bytes per slot in the keys/valid table: N keys × 8 + 1 valid × 8
    let key_stride = ((n_dims + 1) as i64) * KEY_FIELD_BYTES;
    let value_stride = stride as i64;

    let mut rewritten: Vec<BasicBlock> = vec![];

    for block in blocks {
        let mut insts: Vec<MirInstruction> = vec![];
        let mut curr_id = block.id;

        for inst in block.instructions {
            match inst {
                MirInstruction::Call {
                    ref name,
                    dest,
                    ref args,
                    ref return_type,
                    ref arg_types,
                    ..
                } if name == orig_name && args.len() == n_dims => {
                    // ── Allocate block IDs ──────────────────────────────────
                    let key_check_ids: Vec<usize> =
                        (0..n_dims).map(|_| builder.alloc_block()).collect();
                    let hit_id = builder.alloc_block();
                    let miss_empty_id = builder.alloc_block();
                    let miss_collision_id = builder.alloc_block();
                    let store_id = builder.alloc_block();
                    let cont_id = builder.alloc_block();

                    // ── PREAMBLE (in curr_id block) ──────────────────────────
                    //
                    // 1. hash = Horner polynomial: (…(args[0] * P + args[1]) * P + …)
                    let hash_ssa = {
                        let first = builder.alloc_ssa();
                        insts.push(MirInstruction::Assign {
                            dest: first,
                            src: args[0].clone(),
                        });
                        let mut acc = first;
                        for arg in &args[1..] {
                            let scaled = builder.alloc_ssa();
                            insts.push(MirInstruction::BinaryOperation {
                                dest: scaled,
                                op: MirBinOp::Mul,
                                lhs: MirOperand::Variable(acc, false),
                                rhs: MirOperand::Constant(MirLiteral::I64(HASH_PRIME)),
                                dest_type: OnuType::I64,
                            });
                            let added = builder.alloc_ssa();
                            insts.push(MirInstruction::BinaryOperation {
                                dest: added,
                                op: MirBinOp::Add,
                                lhs: MirOperand::Variable(scaled, false),
                                rhs: arg.clone(),
                                dest_type: OnuType::I64,
                            });
                            acc = added;
                        }
                        acc
                    };

                    // 2. slot = hash & mask  (power-of-2 modulo)
                    let slot_ssa = builder.alloc_ssa();
                    insts.push(MirInstruction::BinaryOperation {
                        dest: slot_ssa,
                        op: MirBinOp::And,
                        lhs: MirOperand::Variable(hash_ssa, false),
                        rhs: MirOperand::Constant(MirLiteral::I64(mask)),
                        dest_type: OnuType::I64,
                    });

                    // 3. key_byte = slot * key_stride
                    let key_byte_ssa = builder.alloc_ssa();
                    insts.push(MirInstruction::BinaryOperation {
                        dest: key_byte_ssa,
                        op: MirBinOp::Mul,
                        lhs: MirOperand::Variable(slot_ssa, false),
                        rhs: MirOperand::Constant(MirLiteral::I64(key_stride)),
                        dest_type: OnuType::I64,
                    });

                    // 4. cache_byte = slot * value_stride
                    let cache_byte_ssa = builder.alloc_ssa();
                    insts.push(MirInstruction::BinaryOperation {
                        dest: cache_byte_ssa,
                        op: MirBinOp::Mul,
                        lhs: MirOperand::Variable(slot_ssa, false),
                        rhs: MirOperand::Constant(MirLiteral::I64(value_stride)),
                        dest_type: OnuType::I64,
                    });

                    // 5. valid_byte = key_byte + n_dims*8
                    //    (valid flag sits after all key fields in the slot)
                    let valid_byte_ssa = builder.alloc_ssa();
                    insts.push(MirInstruction::BinaryOperation {
                        dest: valid_byte_ssa,
                        op: MirBinOp::Add,
                        lhs: MirOperand::Variable(key_byte_ssa, false),
                        rhs: MirOperand::Constant(MirLiteral::I64((n_dims as i64) * KEY_FIELD_BYTES)),
                        dest_type: OnuType::I64,
                    });

                    // 6. valid_ptr = keys_ptr + valid_byte
                    let valid_ptr_ssa = builder.alloc_ssa();
                    insts.push(MirInstruction::PointerOffset {
                        dest: valid_ptr_ssa,
                        ptr: MirOperand::Variable(keys_ptr_ssa, false),
                        offset: MirOperand::Variable(valid_byte_ssa, false),
                    });

                    // 7. valid_flag = load i8 from valid_ptr
                    let valid_flag_ssa = builder.alloc_ssa();
                    insts.push(MirInstruction::Load {
                        dest: valid_flag_ssa,
                        ptr: MirOperand::Variable(valid_ptr_ssa, false),
                        typ: OnuType::I8,
                    });

                    // 8. is_occupied = (valid_flag != 0)
                    let is_occupied_ssa = builder.alloc_ssa();
                    insts.push(MirInstruction::BinaryOperation {
                        dest: is_occupied_ssa,
                        op: MirBinOp::Ne,
                        lhs: MirOperand::Variable(valid_flag_ssa, false),
                        rhs: MirOperand::Constant(MirLiteral::I64(0)),
                        dest_type: OnuType::Boolean,
                    });

                    // Close current (preamble) block
                    rewritten.push(BasicBlock {
                        id: curr_id,
                        instructions: insts.drain(..).collect(),
                        terminator: MirTerminator::CondBranch {
                            condition: MirOperand::Variable(is_occupied_ssa, false),
                            then_block: key_check_ids[0],
                            else_block: miss_empty_id,
                        },
                    });

                    // ── KEY_CHECK_i blocks ───────────────────────────────────
                    for (i, &check_id) in key_check_ids.iter().enumerate() {
                        let key_field_byte = (i as i64) * KEY_FIELD_BYTES;
                        let key_off_ssa = builder.alloc_ssa();
                        let key_ptr_ssa = builder.alloc_ssa();
                        let stored_key_ssa = builder.alloc_ssa();
                        let key_match_ssa = builder.alloc_ssa();
                        let next_block =
                            if i + 1 < n_dims { key_check_ids[i + 1] } else { hit_id };

                        rewritten.push(BasicBlock {
                            id: check_id,
                            instructions: vec![
                                // key_off = key_byte + i*8
                                MirInstruction::BinaryOperation {
                                    dest: key_off_ssa,
                                    op: MirBinOp::Add,
                                    lhs: MirOperand::Variable(key_byte_ssa, false),
                                    rhs: MirOperand::Constant(MirLiteral::I64(key_field_byte)),
                                    dest_type: OnuType::I64,
                                },
                                // key_ptr = keys_ptr + key_off
                                MirInstruction::PointerOffset {
                                    dest: key_ptr_ssa,
                                    ptr: MirOperand::Variable(keys_ptr_ssa, false),
                                    offset: MirOperand::Variable(key_off_ssa, false),
                                },
                                // stored_key = load i64 from key_ptr
                                MirInstruction::Load {
                                    dest: stored_key_ssa,
                                    ptr: MirOperand::Variable(key_ptr_ssa, false),
                                    typ: OnuType::I64,
                                },
                                // key_match = (stored_key == args[i])
                                MirInstruction::BinaryOperation {
                                    dest: key_match_ssa,
                                    op: MirBinOp::Eq,
                                    lhs: MirOperand::Variable(stored_key_ssa, false),
                                    rhs: args[i].clone(),
                                    dest_type: OnuType::Boolean,
                                },
                            ],
                            terminator: MirTerminator::CondBranch {
                                condition: MirOperand::Variable(key_match_ssa, false),
                                then_block: next_block,
                                else_block: miss_collision_id,
                            },
                        });
                    }

                    // ── HIT block ────────────────────────────────────────────
                    let cache_hit_ptr_ssa = builder.alloc_ssa();
                    let cached_val_ssa = builder.alloc_ssa();
                    rewritten.push(BasicBlock {
                        id: hit_id,
                        instructions: vec![
                            MirInstruction::PointerOffset {
                                dest: cache_hit_ptr_ssa,
                                ptr: MirOperand::Variable(cache_ptr_ssa, false),
                                offset: MirOperand::Variable(cache_byte_ssa, false),
                            },
                            MirInstruction::Load {
                                dest: cached_val_ssa,
                                ptr: MirOperand::Variable(cache_hit_ptr_ssa, false),
                                typ: ret_type.clone(),
                            },
                            MirInstruction::Assign {
                                dest,
                                src: MirOperand::Variable(cached_val_ssa, false),
                            },
                        ],
                        terminator: MirTerminator::Branch(cont_id),
                    });

                    // ── Shared inner-call args for miss paths ────────────────
                    let mut new_arg_types = arg_types.clone();
                    new_arg_types.push(OnuType::Ptr);
                    new_arg_types.push(OnuType::Ptr);
                    let mut new_args = args.clone();
                    new_args.push(MirOperand::Variable(cache_ptr_ssa, false));
                    new_args.push(MirOperand::Variable(keys_ptr_ssa, false));

                    // ── MISS_EMPTY block (slot was free → compute + store) ───
                    rewritten.push(BasicBlock {
                        id: miss_empty_id,
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

                    // ── MISS_COLLISION block (different key owns this slot) ──
                    // Just recompute; do NOT evict the existing entry.
                    rewritten.push(BasicBlock {
                        id: miss_collision_id,
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

                    // ── STORE block ──────────────────────────────────────────
                    let mut store_insts: Vec<MirInstruction> = vec![];

                    // Write the return value
                    let cache_store_ptr_ssa = builder.alloc_ssa();
                    store_insts.push(MirInstruction::PointerOffset {
                        dest: cache_store_ptr_ssa,
                        ptr: MirOperand::Variable(cache_ptr_ssa, false),
                        offset: MirOperand::Variable(cache_byte_ssa, false),
                    });
                    store_insts.push(MirInstruction::TypedStore {
                        ptr: MirOperand::Variable(cache_store_ptr_ssa, false),
                        value: MirOperand::Variable(dest, false),
                        typ: ret_type.clone(),
                    });

                    // Write each key dimension
                    for (i, key_arg) in args.iter().enumerate() {
                        let koff_ssa = builder.alloc_ssa();
                        let kptr_ssa = builder.alloc_ssa();
                        store_insts.push(MirInstruction::BinaryOperation {
                            dest: koff_ssa,
                            op: MirBinOp::Add,
                            lhs: MirOperand::Variable(key_byte_ssa, false),
                            rhs: MirOperand::Constant(MirLiteral::I64((i as i64) * KEY_FIELD_BYTES)),
                            dest_type: OnuType::I64,
                        });
                        store_insts.push(MirInstruction::PointerOffset {
                            dest: kptr_ssa,
                            ptr: MirOperand::Variable(keys_ptr_ssa, false),
                            offset: MirOperand::Variable(koff_ssa, false),
                        });
                        store_insts.push(MirInstruction::TypedStore {
                            ptr: MirOperand::Variable(kptr_ssa, false),
                            value: key_arg.clone(),
                            typ: OnuType::I64,
                        });
                    }

                    // Write valid = 1  (reuses valid_ptr_ssa from preamble)
                    store_insts.push(MirInstruction::TypedStore {
                        ptr: MirOperand::Variable(valid_ptr_ssa, false),
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
        // Emit the continuation block with any remaining instructions
        rewritten.push(BasicBlock {
            id: curr_id,
            instructions: insts,
            terminator: block.terminator,
        });
    }
    rewritten
}
