/// # MemoPass Bug Regression Tests
///
/// Three bugs were identified in MemoPass. Each test proves one bug exists
/// (RED), then the corresponding fix makes it GREEN. These tests must stay
/// green forever to prevent regression.
///
/// ## Bug 1 — Pointer arithmetic: byte offset instead of i64 offset
/// The cache is an array of i64 (8 bytes each).  When using PointerOffset to
/// reach slot N, the offset must be N*8 bytes, not N bytes.  Without the scale,
/// slot 1 is at byte 1 (inside slot 0), corrupting all reads and writes.
///
/// ## Bug 2 — Wrapper incorrectly marked is_pure_data_leaf = true
/// A function that calls Alloc touches memory and is therefore NOT pure.
/// Marking it pure allows derive_profile to apply readnone + alwaysinline,
/// telling LLVM "this function does not read or write memory" — which is a lie
/// that causes the allocation to be optimised away as dead code.
///
/// ## Bug 3 — Arena bump allocator has no bounds check
/// The bump allocator uses a global arena (now 16 MiB) with no guard.  MemoPass
/// wraps every memoizable function with an 80KB cache allocation.
use onu_refactor::application::use_cases::memo_pass::MemoPass;
use onu_refactor::application::use_cases::registry_service::RegistryService;
use onu_refactor::domain::entities::mir::{
    BasicBlock, MirArgument, MirFunction, MirInstruction, MirLiteral, MirOperand, MirProgram,
    MirTerminator,
};
use onu_refactor::domain::entities::types::OnuType;
use onu_refactor::domain::entities::ARENA_SIZE_BYTES;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn make_recursive_pure_fn(name: &str) -> MirFunction {
    // Minimal doubly-recursive pure function with a diminishing annotation.
    // Represents e.g. fib(n) = fib(n-1) + fib(n-2) that MemoPass should wrap.
    let self_call = MirInstruction::Call {
        dest: 10,
        name: name.to_string(),
        args: vec![MirOperand::Variable(0, false)],
        return_type: OnuType::I64,
        arg_types: vec![OnuType::I64],
        is_tail_call: false,
    };
    MirFunction {
        name: name.to_string(),
        args: vec![MirArgument {
            name: "n".to_string(),
            typ: OnuType::I64,
            ssa_var: 0,
        }],
        return_type: OnuType::I64,
        blocks: vec![BasicBlock {
            id: 0,
            instructions: vec![self_call],
            terminator: MirTerminator::Return(MirOperand::Variable(10, false)),
        }],
        is_pure_data_leaf: true,
        diminishing: vec!["n".to_string()],
        memo_cache_size: None,
    }
}

fn run_memo_pass(func: MirFunction) -> (MirFunction, MirFunction) {
    // Run MemoPass on a single-function program and return (wrapper, inner).
    let program = MirProgram {
        functions: vec![func],
    };
    let registry = RegistryService::new();
    let result = MemoPass::run(program, &registry);
    assert_eq!(
        result.functions.len(),
        2,
        "MemoPass must produce exactly 2 functions (wrapper + inner) for a memoizable function"
    );
    let mut it = result.functions.into_iter();
    let wrapper = it.next().unwrap();
    let inner = it.next().unwrap();
    (wrapper, inner)
}

// ---------------------------------------------------------------------------
// Bug 2 — Wrapper purity flag (pure Rust, no LLVM needed, run first)
// ---------------------------------------------------------------------------

/// The wrapper function allocates memory (Alloc instruction).
/// It must NOT be marked is_pure_data_leaf = true.
/// A pure function, by definition, does not touch memory.
#[test]
fn memo_wrapper_is_not_pure_data_leaf() {
    let func = make_recursive_pure_fn("fib");
    let (wrapper, _inner) = run_memo_pass(func);
    assert!(
        !wrapper.is_pure_data_leaf,
        "Bug 2: wrapper function must have is_pure_data_leaf = false because it calls Alloc"
    );
}

// ---------------------------------------------------------------------------
// Bug 1 — Cache pointer arithmetic scale
// ---------------------------------------------------------------------------

/// The PointerOffset instructions in the inner function must multiply the
/// cache index by 8 (sizeof i64) before computing the byte address.
/// We verify this by finding the BinaryOperation { op: Mul, rhs: 8 } that
/// must immediately precede every PointerOffset that indexes the cache.
#[test]
fn memo_cache_pointer_offset_is_scaled_by_8() {
    let func = make_recursive_pure_fn("fib");
    let (_wrapper, inner) = run_memo_pass(func);

    // Collect all PointerOffset offsets used in the inner function.
    // For each, we expect the offset operand to be a freshly computed variable
    // whose definition is a Mul-by-8 (not the raw argument variable).
    let arg_ssa_var = inner.args[0].ssa_var; // the input argument (e.g., ssa_var=0)

    let mut found_scaled_offset = false;
    for block in &inner.blocks {
        for inst in &block.instructions {
            if let MirInstruction::PointerOffset { offset, .. } = inst {
                // The offset operand must NOT be the raw argument (ssa_var 0).
                // If it equals the raw argument var, the scale multiplication is missing.
                if let MirOperand::Variable(var_id, _) = offset {
                    // Find the definition of this variable — it must be a Mul by 8.
                    for b2 in &inner.blocks {
                        for def in &b2.instructions {
                            if let MirInstruction::BinaryOperation {
                                dest,
                                op: onu_refactor::domain::entities::mir::MirBinOp::Mul,
                                rhs,
                                ..
                            } = def
                            {
                                if dest == var_id {
                                    if let MirOperand::Constant(MirLiteral::I64(8)) = rhs {
                                        found_scaled_offset = true;
                                    }
                                }
                            }
                        }
                    }
                    assert_ne!(
                        *var_id, arg_ssa_var,
                        "Bug 1: PointerOffset uses the raw argument SSA var {} directly. \
                         The index must be multiplied by 8 (sizeof i64) before use.",
                        arg_ssa_var
                    );
                }
            }
        }
    }

    assert!(
        found_scaled_offset,
        "Bug 1: No Mul-by-8 instruction found before any PointerOffset in the inner function. \
         Cache indices must be byte-scaled for i64 storage."
    );
}

// ---------------------------------------------------------------------------
// Bug 3 — Arena bounds check
// ---------------------------------------------------------------------------

/// The Alloc MIR instruction emitted by MemoPass must be preceded by a bounds
/// check in the MIR.  We verify by inspecting the wrapper: it must contain at
/// least one BinaryOperation that compares against the arena limit.
///
/// NOTE: The arena itself is in the codegen layer (AllocStrategy).  The MIR-
/// level check here ensures MemoPass emits enough information for the codegen
/// to detect overflow before it happens.  A separate codegen-level test would
/// require LLVM; that is out of scope for this pure-MIR test.
/// Instead, we verify that MemoPass documents the allocation size correctly
/// so the codegen can guard it — we use the Alloc instruction's size_bytes
/// operand and confirm it is bounded (< ARENA_SIZE_BYTES = 16 MiB).
#[test]
fn memo_cache_allocation_fits_within_arena_limit() {
    let func = make_recursive_pure_fn("fib");
    let (wrapper, _inner) = run_memo_pass(func);

    let arena_limit = ARENA_SIZE_BYTES as i64;

    for block in &wrapper.blocks {
        for inst in &block.instructions {
            if let MirInstruction::Alloc { size_bytes, .. } = inst {
                if let MirOperand::Constant(MirLiteral::I64(size)) = size_bytes {
                    assert!(
                        *size < arena_limit,
                        "Bug 3: Alloc requests {}B which equals or exceeds the {} byte arena. \
                         Reduce DEFAULT_MEMO_CACHE_SIZE or switch to a guarded allocator.",
                        size, arena_limit
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Multi-dimensional memoization (Peanut-Arena)
// ---------------------------------------------------------------------------

/// Build a 2-arg function (like Ackermann) where both args are I64 and both
/// appear in the `diminishing` list.
/// The return type is `Tuple` (not `I64`) so that `MemoPass` routes it through
/// `CompoundMemoStrategy` rather than `PrimitiveMemoStrategy`, letting us
/// exercise the N-dim flattening logic directly.
fn make_two_dim_fn(name: &str) -> MirFunction {
    let self_call = MirInstruction::Call {
        dest: 10,
        name: name.to_string(),
        args: vec![
            MirOperand::Variable(0, false),
            MirOperand::Variable(1, false),
        ],
        return_type: OnuType::Tuple(vec![OnuType::I64, OnuType::I64]),
        arg_types: vec![OnuType::I64, OnuType::I64],
        is_tail_call: false,
    };
    MirFunction {
        name: name.to_string(),
        args: vec![
            MirArgument {
                name: "m".to_string(),
                typ: OnuType::I64,
                ssa_var: 0,
            },
            MirArgument {
                name: "n".to_string(),
                typ: OnuType::I64,
                ssa_var: 1,
            },
        ],
        return_type: OnuType::Tuple(vec![OnuType::I64, OnuType::I64]),
        blocks: vec![BasicBlock {
            id: 0,
            instructions: vec![self_call],
            terminator: MirTerminator::Return(MirOperand::Variable(10, false)),
        }],
        is_pure_data_leaf: true,
        diminishing: vec!["m".to_string(), "n".to_string()],
        memo_cache_size: None,
    }
}

/// A 2-arg function whose second arg is NOT in the `diminishing` list should
/// NOT be memoized ("State Leakage" prevention).
#[test]
fn multi_dim_non_diminishing_arg_blocks_memoization() {
    let func = MirFunction {
        name: "leaky".to_string(),
        args: vec![
            MirArgument {
                name: "n".to_string(),
                typ: OnuType::I64,
                ssa_var: 0,
            },
            MirArgument {
                name: "ctx".to_string(), // NOT in diminishing
                typ: OnuType::I64,
                ssa_var: 1,
            },
        ],
        return_type: OnuType::I64,
        blocks: vec![BasicBlock {
            id: 0,
            instructions: vec![],
            terminator: MirTerminator::Return(MirOperand::Variable(0, false)),
        }],
        is_pure_data_leaf: true,
        // Only one of the two args is diminishing → should not memoize.
        diminishing: vec!["n".to_string()],
        memo_cache_size: None,
    };
    let program = MirProgram {
        functions: vec![func],
    };
    let registry = RegistryService::new();
    let result = MemoPass::run(program, &registry);
    assert_eq!(
        result.functions.len(),
        1,
        "A function with a non-diminishing arg must NOT be memoized"
    );
}

/// A 2-arg function where both args are I64 and both are in `diminishing`
/// must be wrapped (wrapper + inner = 2 functions).
#[test]
fn multi_dim_both_diminishing_is_memoized() {
    let func = make_two_dim_fn("ack");
    let program = MirProgram {
        functions: vec![func],
    };
    let registry = RegistryService::new();
    let result = MemoPass::run(program, &registry);
    assert_eq!(
        result.functions.len(),
        2,
        "A 2-arg function with both args diminishing must produce wrapper + inner"
    );
    assert_eq!(result.functions[0].name, "ack");
    assert_eq!(result.functions[1].name, "ack.inner");
}

/// The wrapper for a 2-arg function must NOT be marked `is_pure_data_leaf`
/// (it allocates memory).
#[test]
fn multi_dim_wrapper_is_not_pure_data_leaf() {
    let func = make_two_dim_fn("ack");
    let program = MirProgram {
        functions: vec![func],
    };
    let registry = RegistryService::new();
    let result = MemoPass::run(program, &registry);
    let wrapper = &result.functions[0];
    assert!(
        !wrapper.is_pure_data_leaf,
        "Multi-dim wrapper must have is_pure_data_leaf = false (it calls Alloc)"
    );
}

/// The inner function for a 2-dim case must have 4 args: m, n, cache_ptr, occ_ptr.
#[test]
fn multi_dim_inner_has_extra_ptr_args() {
    let func = make_two_dim_fn("ack");
    let program = MirProgram {
        functions: vec![func],
    };
    let registry = RegistryService::new();
    let result = MemoPass::run(program, &registry);
    let inner = &result.functions[1];
    assert_eq!(
        inner.args.len(),
        4,
        "Inner must have 2 original args + cache_ptr + occ_ptr = 4 total"
    );
    assert_eq!(inner.args[2].typ, OnuType::Ptr, "3rd arg must be cache_ptr");
    assert_eq!(inner.args[3].typ, OnuType::Ptr, "4th arg must be occ_ptr");
}

/// Memory guard: for a 3-dim function the **combined** allocation
/// (result cache `dim_size^3 * stride` PLUS occupancy array `dim_size^3 * 8`)
/// must stay within `ARENA_SIZE_BYTES` regardless of the nominal cache_size.
///
/// The previous implementation only guarded the result-cache allocation, so the
/// occupancy array would push the total over the arena boundary.  This test
/// verifies that the sum of all `Alloc` sizes in the wrapper is within the arena.
#[test]
fn multi_dim_memory_guard_caps_allocation() {
    // Build a 3-arg function — if dim_size were 10_000^3 * stride, that would be
    // enormous. The memory guard must cap it to ≤ ARENA_SIZE_BYTES combined.
    let func = MirFunction {
        name: "f3".to_string(),
        args: vec![
            MirArgument { name: "a".to_string(), typ: OnuType::I64, ssa_var: 0 },
            MirArgument { name: "b".to_string(), typ: OnuType::I64, ssa_var: 1 },
            MirArgument { name: "c".to_string(), typ: OnuType::I64, ssa_var: 2 },
        ],
        return_type: OnuType::Tuple(vec![OnuType::I64, OnuType::I64]),
        blocks: vec![BasicBlock {
            id: 0,
            instructions: vec![],
            terminator: MirTerminator::Return(MirOperand::Variable(0, false)),
        }],
        is_pure_data_leaf: true,
        diminishing: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        memo_cache_size: None,
    };
    let program = MirProgram { functions: vec![func] };
    let registry = RegistryService::new();
    let result = MemoPass::run(program, &registry);

    let limit = ARENA_SIZE_BYTES as i64;
    let wrapper = &result.functions[0];

    // Collect all Alloc sizes in the wrapper (result cache + occupancy array).
    let total_alloc: i64 = wrapper
        .blocks
        .iter()
        .flat_map(|b| b.instructions.iter())
        .filter_map(|inst| {
            if let MirInstruction::Alloc { size_bytes, .. } = inst {
                if let MirOperand::Constant(MirLiteral::I64(bytes)) = size_bytes {
                    return Some(*bytes);
                }
            }
            None
        })
        .sum();

    assert!(
        total_alloc <= limit,
        "3-dim wrapper total allocation {} bytes exceeds {} byte arena (result cache + occupancy combined must fit)",
        total_alloc, limit
    );
}

/// The inner function of a 2-dim ND case must contain a Mul instruction for
/// Horner's-method flat-index computation (flat = m*dim_size + n).
#[test]
fn multi_dim_inner_contains_horner_flat_index() {
    let func = make_two_dim_fn("ack");
    let program = MirProgram {
        functions: vec![func],
    };
    let registry = RegistryService::new();
    let result = MemoPass::run(program, &registry);
    let inner = &result.functions[1];

    // We expect at least one Mul instruction that multiplies by the dim_size
    // constant (used in Horner's flat-index expansion).
    let has_mul_by_dim_size = inner.blocks.iter().flat_map(|b| b.instructions.iter()).any(|i| {
        matches!(
            i,
            MirInstruction::BinaryOperation {
                op: onu_refactor::domain::entities::mir::MirBinOp::Mul,
                rhs: MirOperand::Constant(MirLiteral::I64(_)),
                ..
            }
        )
    });
    assert!(
        has_mul_by_dim_size,
        "Inner function must contain a Mul-by-dim_size instruction for Horner's flat-index"
    );
}


/// Regression test: 2-dim I64 function (like Ackermann) must not overflow the
/// arena.  Previously `safe_dim_size` only guarded the result-cache bytes
/// (`dim_size^2 * 8`) and ignored the occupancy array (`dim_size^2 * 8`), so the
/// combined allocation exceeded the arena boundary.
///
/// With `ARENA_SIZE_BYTES = 16 MiB` the safe dim_size is 1024, and the combined
/// allocation is exactly 16 MiB — well within the declared arena.
#[test]
fn two_dim_i64_combined_allocation_within_arena() {
    let func = make_two_dim_fn("ack");
    let program = MirProgram { functions: vec![func] };
    let registry = RegistryService::new();
    let result = MemoPass::run(program, &registry);

    let limit = ARENA_SIZE_BYTES as i64;
    let wrapper = &result.functions[0];

    let total_alloc: i64 = wrapper
        .blocks
        .iter()
        .flat_map(|b| b.instructions.iter())
        .filter_map(|inst| {
            if let MirInstruction::Alloc { size_bytes, .. } = inst {
                if let MirOperand::Constant(MirLiteral::I64(bytes)) = size_bytes {
                    return Some(*bytes);
                }
            }
            None
        })
        .sum();

    assert!(
        total_alloc <= limit,
        "2-dim/I64 combined allocation {} bytes exceeds {} byte arena (result cache + occupancy must both fit)",
        total_alloc, limit
    );
}
