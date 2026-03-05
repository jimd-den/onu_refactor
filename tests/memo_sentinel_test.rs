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
}
