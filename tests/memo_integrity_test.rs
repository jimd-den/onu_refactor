use onu_refactor::application::use_cases::memo_pass::MemoPass;
use onu_refactor::application::use_cases::registry_service::RegistryService;
use onu_refactor::domain::entities::mir::{
    BasicBlock, MirArgument, MirFunction, MirInstruction, MirLiteral, MirOperand, MirProgram,
    MirTerminator,
};
use onu_refactor::domain::entities::types::OnuType;

fn make_memoizable_fn(name: &str) -> MirFunction {
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
            instructions: vec![],
            terminator: MirTerminator::Return(MirOperand::Constant(MirLiteral::I64(42))),
        }],
        is_pure_data_leaf: true,
        diminishing: vec!["n".to_string()],
        memo_cache_size: None,
    }
}

#[test]
fn test_primitive_memo_wrapper_integrity() {
    let func = make_memoizable_fn("fib");
    let program = MirProgram {
        functions: vec![func],
    };
    let registry = RegistryService::new();
    let result = MemoPass::run(program, &registry);

    // Should have wrapper and inner
    assert_eq!(result.functions.len(), 2);

    let wrapper = result.functions.iter().find(|f| f.name == "fib").unwrap();
    let _inner = result
        .functions
        .iter()
        .find(|f| f.name == "fib.inner")
        .unwrap();

    // Verify all blocks targeted by terminators exist in the wrapper
    let mut target_ids = std::collections::HashSet::new();
    for block in &wrapper.blocks {
        match &block.terminator {
            MirTerminator::Branch(id) => {
                target_ids.insert(*id);
            }
            MirTerminator::CondBranch {
                then_block,
                else_block,
                ..
            } => {
                target_ids.insert(*then_block);
                target_ids.insert(*else_block);
            }
            _ => {}
        }
    }

    let existing_ids: std::collections::HashSet<_> = wrapper.blocks.iter().map(|b| b.id).collect();
    for target in target_ids {
        assert!(
            existing_ids.contains(&target),
            "Wrapper targets missing block ID {}",
            target
        );
    }

    // Verify the wrapper actually calls the inner function
    let mut found_call = false;
    for block in &wrapper.blocks {
        for inst in &block.instructions {
            if let MirInstruction::Call { name, .. } = inst {
                if name == "fib.inner" {
                    found_call = true;
                }
            }
        }
    }
    assert!(
        found_call,
        "Wrapper does not call the inner function 'fib.inner'"
    );
}

#[test]
fn test_compound_memo_wrapper_integrity() {
    // String is a compound type (struct)
    let mut func = make_memoizable_fn("as_text");
    func.return_type = OnuType::Strings;

    let program = MirProgram {
        functions: vec![func],
    };
    let registry = RegistryService::new();
    let result = MemoPass::run(program, &registry);

    assert_eq!(result.functions.len(), 2);
    let wrapper = result
        .functions
        .iter()
        .find(|f| f.name == "as_text")
        .unwrap();

    // Verify all blocks targeted by terminators exist
    let mut target_ids = std::collections::HashSet::new();
    for block in &wrapper.blocks {
        match &block.terminator {
            MirTerminator::Branch(id) => {
                target_ids.insert(*id);
            }
            MirTerminator::CondBranch {
                then_block,
                else_block,
                ..
            } => {
                target_ids.insert(*then_block);
                target_ids.insert(*else_block);
            }
            _ => {}
        }
    }

    let existing_ids: std::collections::HashSet<_> = wrapper.blocks.iter().map(|b| b.id).collect();
    for target in target_ids {
        assert!(
            existing_ids.contains(&target),
            "Compound wrapper targets missing block ID {}",
            target
        );
    }

    // Verify it calls inner
    let mut found_call = false;
    for block in &wrapper.blocks {
        for inst in &block.instructions {
            if let MirInstruction::Call { name, .. } = inst {
                if name == "as_text.inner" {
                    found_call = true;
                }
            }
        }
    }
    assert!(
        found_call,
        "Compound wrapper does not call the inner function 'as_text.inner'"
    );
}
