use onu_refactor::application::use_cases::memo_pass::MemoPass;
use onu_refactor::application::use_cases::registry_service::RegistryService;
use onu_refactor::domain::entities::mir::{
    BasicBlock, MirArgument, MirBinOp, MirFunction, MirInstruction, MirLiteral, MirOperand,
    MirProgram, MirTerminator,
};
use onu_refactor::domain::entities::types::OnuType;

#[test]
fn memo_occupancy_buffer_test() {
    // A function that returns values including potential sentinels.
    let name = "return_values";
    let func = MirFunction {
        name: name.to_string(),
        args: vec![MirArgument {
            name: "n".to_string(),
            typ: OnuType::I64,
            ssa_var: 0,
        }],
        return_type: OnuType::I64,
        blocks: vec![BasicBlock {
            id: 0,
            instructions: vec![MirInstruction::Call {
                dest: 1,
                name: name.to_string(),
                args: vec![MirOperand::Variable(0, false)],
                return_type: OnuType::I64,
                arg_types: vec![OnuType::I64],
                is_tail_call: false,
            }],
            terminator: MirTerminator::Return(MirOperand::Variable(1, false)),
        }],
        is_pure_data_leaf: true,
        diminishing: Some("n".to_string()),
        memo_cache_size: None,
    };

    let program = MirProgram {
        functions: vec![func],
    };
    let registry = RegistryService::new();
    let result = MemoPass::run(program, &registry);

    let inner = result
        .functions
        .iter()
        .find(|f| f.name.ends_with(".inner"))
        .expect("Inner function not found");

    println!("INNER BLOCKS: {:#?}", inner.blocks);

    // Check for the double-buffer occupancy logic:
    // 1. Should have TWO extra Ptr arguments (cache_ptr, occ_ptr)
    assert!(
        inner.args.len() >= 3,
        "Inner function should have at least 3 arguments (n, cache_ptr, occ_ptr)"
    );
    assert_eq!(
        inner.args[inner.args.len() - 2].typ,
        OnuType::Ptr,
        "Missing/Wrong cache_ptr argument"
    );
    assert_eq!(
        inner.args[inner.args.len() - 1].typ,
        OnuType::Ptr,
        "Missing/Wrong occ_ptr argument"
    );

    // 2. Should NOT have a comparison against -1 in the blocks
    let mut found_minus_one_check = false;
    let mut found_occupancy_load = false;
    let mut found_hit_eq_one = false;
    let mut all_inner_calls_not_tail = true;

    for block in &inner.blocks {
        for inst in &block.instructions {
            match inst {
                MirInstruction::Load {
                    typ: OnuType::I8, ..
                } => {
                    found_occupancy_load = true;
                }
                MirInstruction::BinaryOperation {
                    op: MirBinOp::Ne,
                    rhs: MirOperand::Constant(MirLiteral::I64(0)),
                    ..
                } => {
                    found_hit_eq_one = true;
                }
                MirInstruction::BinaryOperation {
                    op: MirBinOp::Ne,
                    rhs: MirOperand::Constant(MirLiteral::I64(-1)),
                    ..
                } => {
                    found_minus_one_check = true;
                }
                MirInstruction::Call {
                    name, is_tail_call, ..
                } if name.ends_with(".inner") => {
                    if *is_tail_call {
                        all_inner_calls_not_tail = false;
                    }
                }
                _ => {}
            }
        }
    }

    assert!(!found_minus_one_check, "Should NOT use -1 sentinel anymore");
    assert!(
        found_occupancy_load,
        "Should load from occupancy buffer (I8)"
    );
    assert!(found_hit_eq_one, "Should compare occupancy flag with Ne 0");
    assert!(
        all_inner_calls_not_tail,
        "All injected .inner calls MUST have is_tail_call = false"
    );
}

#[test]
fn memo_multi_arg_guard_test() {
    // A function with TWO arguments.
    // PrimitiveMemoStrategy should NOT rewrite this because it only supports single-arg indexing.
    let name = "add_recursive";
    let func = MirFunction {
        name: name.to_string(),
        args: vec![
            MirArgument {
                name: "a".to_string(),
                typ: OnuType::I64,
                ssa_var: 0,
            },
            MirArgument {
                name: "b".to_string(),
                typ: OnuType::I64,
                ssa_var: 1,
            },
        ],
        return_type: OnuType::I64,
        blocks: vec![BasicBlock {
            id: 0,
            instructions: vec![MirInstruction::Call {
                dest: 2,
                name: name.to_string(),
                args: vec![
                    MirOperand::Variable(0, false),
                    MirOperand::Variable(1, false),
                ],
                return_type: OnuType::I64,
                arg_types: vec![OnuType::I64, OnuType::I64],
                is_tail_call: false,
            }],
            terminator: MirTerminator::Return(MirOperand::Variable(2, false)),
        }],
        is_pure_data_leaf: true,
        diminishing: Some("a".to_string()),
        memo_cache_size: None,
    };

    let program = MirProgram {
        functions: vec![func],
    };
    let registry = RegistryService::new();

    // NOTE: MemoPass currently filters for args.len() == 1,
    // but we want to ensure the strategy itself is also safe.
    // We'll bypass MemoPass's filter by manually invoking the strategy if needed,
    // or just trust that MemoPass + Strategy guard provide defense-in-depth.

    // Let's test MemoPass first. It should return 1 function (no wrapper/inner)
    // because it filters for single-arg.
    let result = MemoPass::run(program.clone(), &registry);
    assert_eq!(
        result.functions.len(),
        1,
        "MemoPass should have ignored the 2-arg function"
    );

    // Now let's test the strategy directly to ensure ITS guard works.
    use onu_refactor::application::use_cases::memo_strategies::MemoStrategy;
    use onu_refactor::application::use_cases::memo_strategies::primitive_memo_strategy::PrimitiveMemoStrategy;

    let strategy = PrimitiveMemoStrategy;
    let (_wrapper, inner) =
        strategy.create_wrapper_and_inner(program.functions[0].clone(), 100, &registry);

    // The inner function's blocks should NOT be rewritten (stay as 1 block with the original call).
    // If it were rewritten, it would have many more blocks (fetch, hit, miss, store, etc.)
    assert_eq!(
        inner.blocks.len(),
        1,
        "Strategy should NOT have rewritten the blocks for a 2-arg call"
    );

    let inst = &inner.blocks[0].instructions[0];
    if let MirInstruction::Call {
        name: call_name, ..
    } = inst
    {
        assert_eq!(
            call_name, name,
            "The call should still point to the original name, not .inner"
        );
    } else {
        panic!("Expected a Call instruction");
    }
}

#[test]
fn memo_compound_occupancy_test() {
    let name = "get_struct";
    // Returns a Tuple (Compound type)
    let func = MirFunction {
        name: name.to_string(),
        args: vec![MirArgument {
            name: "id".to_string(),
            typ: OnuType::I64,
            ssa_var: 0,
        }],
        return_type: OnuType::Tuple(vec![OnuType::I64, OnuType::I64]),
        blocks: vec![BasicBlock {
            id: 0,
            instructions: vec![MirInstruction::Call {
                dest: 1,
                name: name.to_string(),
                args: vec![MirOperand::Variable(0, false)],
                return_type: OnuType::Tuple(vec![OnuType::I64, OnuType::I64]),
                arg_types: vec![OnuType::I64],
                is_tail_call: false,
            }],
            terminator: MirTerminator::Return(MirOperand::Variable(1, false)),
        }],
        is_pure_data_leaf: true,
        diminishing: Some("id".to_string()),
        memo_cache_size: None,
    };

    let registry = RegistryService::new();
    use onu_refactor::application::use_cases::memo_strategies::MemoStrategy;
    use onu_refactor::application::use_cases::memo_strategies::compound_memo_strategy::CompoundMemoStrategy;

    let strategy = CompoundMemoStrategy;
    let (_wrapper, inner) = strategy.create_wrapper_and_inner(func, 100, &registry);

    let mut found_occupancy_load = false;
    let mut found_hit_cond = false;
    let mut all_calls_not_tail = true;

    for block in &inner.blocks {
        for inst in &block.instructions {
            match inst {
                MirInstruction::Load {
                    typ: OnuType::I8, ..
                } => {
                    found_occupancy_load = true;
                }
                MirInstruction::BinaryOperation {
                    op: MirBinOp::Ne,
                    rhs: MirOperand::Constant(MirLiteral::I64(0)),
                    ..
                } => {
                    found_hit_cond = true;
                }
                MirInstruction::Call {
                    is_tail_call, name, ..
                } if name.ends_with(".inner") => {
                    if *is_tail_call {
                        all_calls_not_tail = false;
                    }
                }
                _ => {}
            }
        }
    }

    assert!(
        found_occupancy_load,
        "Compound strategy should load from occupancy buffer"
    );
    assert!(
        found_hit_cond,
        "Compound strategy should check occupancy flag"
    );
    assert!(
        all_calls_not_tail,
        "Injected calls in compound strategy must not be tail calls"
    );
}
