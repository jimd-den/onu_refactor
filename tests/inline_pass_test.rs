/// Inline Pass Unit Tests: Application Use Case Layer
///
/// The `InlinePass` expands `is_pure_data_leaf` function bodies directly at
/// their call sites in MIR, before codegen. This fuses inter-function loops
/// into a single LLVM basic block, enabling LLVM to apply full loop optimizations
/// that cannot cross function call boundaries.
///
/// TDD: These tests are written before implementation to define the contract.
use onu_refactor::application::use_cases::inline_pass::InlinePass;
use onu_refactor::domain::entities::mir::{
    BasicBlock, MirArgument, MirBinOp, MirFunction, MirInstruction, MirLiteral, MirOperand,
    MirProgram, MirTerminator,
};
use onu_refactor::domain::entities::types::OnuType;

/// Build a minimal pure callee: `double(n) -> n * 2`
/// This is marked `is_pure_data_leaf = true`.
fn make_pure_callee() -> MirFunction {
    MirFunction {
        name: "double".to_string(),
        args: vec![MirArgument {
            name: "n".to_string(),
            typ: OnuType::I64,
            ssa_var: 0,
        }],
        return_type: OnuType::I64,
        is_pure_data_leaf: true,
        blocks: vec![BasicBlock {
            id: 0,
            instructions: vec![
                // result = n * 2
                MirInstruction::BinaryOperation {
                    dest: 1,
                    op: MirBinOp::Mul,
                    lhs: MirOperand::Variable(0, false),
                    rhs: MirOperand::Constant(MirLiteral::I64(2)),
                },
            ],
            terminator: MirTerminator::Return(MirOperand::Variable(1, false)),
        }],
    }
}

/// Build a caller: `compute(x) -> double(x) + 1`
/// SSA vars: 0 = x (arg), 1..N = temporaries from outer scope (none),
/// 10 = result of call to double, 11 = final add result
fn make_caller() -> MirFunction {
    MirFunction {
        name: "compute".to_string(),
        args: vec![MirArgument {
            name: "x".to_string(),
            typ: OnuType::I64,
            ssa_var: 10,
        }],
        return_type: OnuType::I64,
        is_pure_data_leaf: false,
        blocks: vec![BasicBlock {
            id: 0,
            instructions: vec![
                // doubled = double(x)
                MirInstruction::Call {
                    dest: 20,
                    name: "double".to_string(),
                    args: vec![MirOperand::Variable(10, false)],
                    return_type: OnuType::I64,
                    arg_types: vec![OnuType::I64],
                    is_tail_call: false,
                },
                // result = doubled + 1
                MirInstruction::BinaryOperation {
                    dest: 21,
                    op: MirBinOp::Add,
                    lhs: MirOperand::Variable(20, false),
                    rhs: MirOperand::Constant(MirLiteral::I64(1)),
                },
            ],
            terminator: MirTerminator::Return(MirOperand::Variable(21, false)),
        }],
    }
}

fn make_program() -> MirProgram {
    MirProgram {
        functions: vec![make_pure_callee(), make_caller()],
    }
}

/// RED: Before InlinePass, the caller must contain a Call instruction to "double".
#[test]
fn red_caller_has_call_to_pure_callee_before_pass() {
    let program = make_program();
    let caller = program
        .functions
        .iter()
        .find(|f| f.name == "compute")
        .unwrap();

    let has_call = caller.blocks.iter().any(|b| {
        b.instructions
            .iter()
            .any(|inst| matches!(inst, MirInstruction::Call { name, .. } if name == "double"))
    });

    assert!(
        has_call,
        "Precondition: caller must have a call to 'double' before the pass"
    );
}

/// GREEN: After InlinePass, no Call to "double" should remain in the caller.
/// The callee body is now expanded inline.
#[test]
fn inline_pass_expands_pure_callee_into_caller() {
    let program = make_program();
    let result = InlinePass::run(program);
    let caller = result
        .functions
        .iter()
        .find(|f| f.name == "compute")
        .unwrap();

    let remaining_calls: Vec<_> = caller
        .blocks
        .iter()
        .flat_map(|b| {
            b.instructions.iter().filter(
                |inst| matches!(inst, MirInstruction::Call { name, .. } if name == "double"),
            )
        })
        .collect();

    assert!(
        remaining_calls.is_empty(),
        "InlinePass must eliminate the call to 'double'. Remaining: {:?}",
        remaining_calls
    );
}

/// GREEN: The callee's multiplication instruction must appear in the caller's blocks.
#[test]
fn inline_pass_brings_callee_body_into_caller() {
    let program = make_program();
    let result = InlinePass::run(program);
    let caller = result
        .functions
        .iter()
        .find(|f| f.name == "compute")
        .unwrap();

    let has_mul = caller.blocks.iter().any(|b| {
        b.instructions.iter().any(|inst| {
            matches!(
                inst,
                MirInstruction::BinaryOperation {
                    op: MirBinOp::Mul,
                    ..
                }
            )
        })
    });

    assert!(
        has_mul,
        "After inlining, the caller's blocks must contain the callee's Mul instruction"
    );
}

/// GREEN: The SSA destination of the original Call (ssa 20) must be assigned
/// the callee's return value. This tests that the return value is correctly bound.
#[test]
fn inline_pass_preserves_call_result_ssa() {
    let program = make_program();
    let result = InlinePass::run(program);
    let caller = result
        .functions
        .iter()
        .find(|f| f.name == "compute")
        .unwrap();

    // After inlining, the post-call instruction `result = doubled + 1` uses ssa 20.
    // That ssa must be written somewhere in the caller (via Assign from callee's return).
    let has_dest_20_written = caller.blocks.iter().any(|b| {
        b.instructions
            .iter()
            .any(|inst| matches!(inst, MirInstruction::Assign { dest: 20, .. }))
    });

    assert!(
        has_dest_20_written,
        "InlinePass must emit Assign {{ dest: 20, .. }} to bind the callee return value to the call site's dest SSA"
    );
}

/// GREEN: Calls to external functions (not in the module's pure functions map)
/// must NOT be inlined. This prevents inline of stdlib like malloc, printf.
#[test]
fn inline_pass_is_identity_for_external_calls() {
    let caller = MirFunction {
        name: "uses-extern".to_string(),
        args: vec![],
        return_type: OnuType::I64,
        is_pure_data_leaf: false,
        blocks: vec![BasicBlock {
            id: 0,
            instructions: vec![MirInstruction::Call {
                dest: 5,
                name: "malloc".to_string(), // external — not in module's pure map
                args: vec![MirOperand::Constant(MirLiteral::I64(64))],
                return_type: OnuType::I64,
                arg_types: vec![OnuType::I64],
                is_tail_call: false,
            }],
            terminator: MirTerminator::Return(MirOperand::Variable(5, false)),
        }],
    };

    let program = MirProgram {
        functions: vec![caller],
    };
    let result = InlinePass::run(program);
    let transformed = &result.functions[0];

    let still_has_malloc = transformed.blocks.iter().any(|b| {
        b.instructions
            .iter()
            .any(|inst| matches!(inst, MirInstruction::Call { name, .. } if name == "malloc"))
    });

    assert!(
        still_has_malloc,
        "InlinePass must NOT inline external calls like 'malloc'"
    );
}

/// GREEN: Functions NOT marked `is_pure_data_leaf` must not be inlined.
#[test]
fn inline_pass_is_identity_for_non_pure_callees() {
    let impure_callee = MirFunction {
        name: "side-effect-fn".to_string(),
        args: vec![MirArgument {
            name: "n".to_string(),
            typ: OnuType::I64,
            ssa_var: 0,
        }],
        return_type: OnuType::I64,
        is_pure_data_leaf: false, // NOT pure
        blocks: vec![BasicBlock {
            id: 0,
            instructions: vec![MirInstruction::Emit(MirOperand::Constant(MirLiteral::I64(
                0,
            )))],
            terminator: MirTerminator::Return(MirOperand::Variable(0, false)),
        }],
    };

    let caller = MirFunction {
        name: "caller".to_string(),
        args: vec![MirArgument {
            name: "x".to_string(),
            typ: OnuType::I64,
            ssa_var: 1,
        }],
        return_type: OnuType::I64,
        is_pure_data_leaf: false,
        blocks: vec![BasicBlock {
            id: 0,
            instructions: vec![MirInstruction::Call {
                dest: 10,
                name: "side-effect-fn".to_string(),
                args: vec![MirOperand::Variable(1, false)],
                return_type: OnuType::I64,
                arg_types: vec![OnuType::I64],
                is_tail_call: false,
            }],
            terminator: MirTerminator::Return(MirOperand::Variable(10, false)),
        }],
    };

    let program = MirProgram {
        functions: vec![impure_callee, caller],
    };
    let result = InlinePass::run(program);
    let transformed_caller = result
        .functions
        .iter()
        .find(|f| f.name == "caller")
        .unwrap();

    let call_intact = transformed_caller.blocks.iter().any(|b| {
        b.instructions.iter().any(
            |inst| matches!(inst, MirInstruction::Call { name, .. } if name == "side-effect-fn"),
        )
    });

    assert!(
        call_intact,
        "InlinePass must NOT inline non-pure callee 'side-effect-fn'"
    );
}
