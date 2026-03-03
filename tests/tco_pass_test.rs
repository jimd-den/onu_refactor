use onu_refactor::application::use_cases::tco_pass::TcoPass;
/// TCO Pass Unit Tests: Application Use Case Layer
///
/// These tests validate the self-tail-call loop lowering transformation.
/// The `TcoPass` takes a `MirFunction` whose tail-recursive calls target
/// themselves and rewrites them into a loop using MirTerminator::Branch,
/// eliminating stack growth at the MIR level before LLVM sees the IR.
///
/// TDD Approach: These tests were written BEFORE the implementation.
/// They define the contract that `TcoPass` must fulfill.
use onu_refactor::domain::entities::mir::{
    BasicBlock, MirArgument, MirBinOp, MirFunction, MirInstruction, MirLiteral, MirOperand,
    MirTerminator,
};
use onu_refactor::domain::entities::types::OnuType;

/// Helper: build the MIR for a function that models:
///   collatz-steps(n, count):
///     if n == 1: return count
///     else: return collatz-steps(n / 2, count + 1)   ← self tail call
///
/// Block 0: compute (n == 1), then CondBranch → block 1 or block 2
/// Block 1: Return(count)
/// Block 2: Call collatz-steps(n/2, count+1) marked is_tail_call=true, then Return(result)
fn make_self_tail_call_function() -> MirFunction {
    // SSA vars: 0 = n (arg), 1 = count (arg), 2 = cond, 3 = n_half, 4 = count_next, 5 = result
    MirFunction {
        name: "collatz-steps".to_string(),
        args: vec![
            MirArgument {
                name: "n".to_string(),
                typ: OnuType::I64,
                ssa_var: 0,
            },
            MirArgument {
                name: "count".to_string(),
                typ: OnuType::I64,
                ssa_var: 1,
            },
        ],
        return_type: OnuType::I64,
        is_pure_data_leaf: true,
        diminishing: None,
        blocks: vec![
            // Block 0: condition check
            BasicBlock {
                id: 0,
                instructions: vec![
                    // cond = (n == 1)
                    MirInstruction::BinaryOperation {
                        dest: 2,
                        op: MirBinOp::Eq,
                        lhs: MirOperand::Variable(0, false),
                        rhs: MirOperand::Constant(MirLiteral::I64(1)),
                    },
                ],
                terminator: MirTerminator::CondBranch {
                    condition: MirOperand::Variable(2, false),
                    then_block: 1,
                    else_block: 2,
                },
            },
            // Block 1: base case — return count
            BasicBlock {
                id: 1,
                instructions: vec![],
                terminator: MirTerminator::Return(MirOperand::Variable(1, false)),
            },
            // Block 2: recursive case — self tail call
            BasicBlock {
                id: 2,
                instructions: vec![
                    // n_half = n / 2
                    MirInstruction::BinaryOperation {
                        dest: 3,
                        op: MirBinOp::Div,
                        lhs: MirOperand::Variable(0, false),
                        rhs: MirOperand::Constant(MirLiteral::I64(2)),
                    },
                    // count_next = count + 1
                    MirInstruction::BinaryOperation {
                        dest: 4,
                        op: MirBinOp::Add,
                        lhs: MirOperand::Variable(1, false),
                        rhs: MirOperand::Constant(MirLiteral::I64(1)),
                    },
                    // result = collatz-steps(n_half, count_next)  ← SELF TAIL CALL
                    MirInstruction::Call {
                        dest: 5,
                        name: "collatz-steps".to_string(),
                        args: vec![
                            MirOperand::Variable(3, false),
                            MirOperand::Variable(4, false),
                        ],
                        return_type: OnuType::I64,
                        arg_types: vec![OnuType::I64, OnuType::I64],
                        is_tail_call: true,
                    },
                ],
                terminator: MirTerminator::Return(MirOperand::Variable(5, false)),
            },
        ],
    }
}

/// RED: Before TcoPass, the function must contain a self-tail-call.
/// This is the precondition that proves the test is meaningful.
#[test]
fn red_function_has_self_tail_call_before_pass() {
    let func = make_self_tail_call_function();

    let has_self_tail_call = func.blocks.iter().any(|block| {
        block.instructions.iter().any(|inst| {
            matches!(inst,
                MirInstruction::Call { name, is_tail_call: true, .. }
                if name == &func.name
            )
        })
    });

    assert!(
        has_self_tail_call,
        "Precondition failed: the fixture must have a self-tail-call before the pass runs"
    );
}

/// GREEN CONTRACT: After TcoPass, no self-tail-call instruction must remain.
/// The pass must have rewritten the recursive call into a loop jump.
#[test]
fn tco_pass_eliminates_self_tail_call() {
    let func = make_self_tail_call_function();
    let transformed = TcoPass::run_function(func);

    let remaining_self_tail_calls: Vec<_> = transformed
        .blocks
        .iter()
        .flat_map(|block| {
            block.instructions.iter().filter(|inst| {
                matches!(inst,
                    MirInstruction::Call { name, is_tail_call: true, .. }
                    if name == &transformed.name
                )
            })
        })
        .collect();

    assert!(
        remaining_self_tail_calls.is_empty(),
        "TcoPass must eliminate all self-tail-calls. Found: {:?}",
        remaining_self_tail_calls
    );
}

/// GREEN CONTRACT: The transformed function must contain at least one
/// MirTerminator::Branch pointing to a loop-head block (the reuse of
/// argument shadow slots via a back-edge).
#[test]
fn tco_pass_introduces_loop_branch() {
    let func = make_self_tail_call_function();
    let transformed = TcoPass::run_function(func);

    let has_loop_back_branch = transformed
        .blocks
        .iter()
        .any(|block| matches!(block.terminator, MirTerminator::Branch(_)));

    assert!(
        has_loop_back_branch,
        "TcoPass must introduce a MirTerminator::Branch to form the loop back-edge"
    );
}

/// GREEN CONTRACT: The transformed function must assign new argument values
/// before the loop back-edge, so the next iteration sees the updated arguments.
/// This is verified by finding Assign instructions in the rewritten block.
#[test]
fn tco_pass_emits_argument_assignments_before_loop_back() {
    let func = make_self_tail_call_function();
    let arg_shadow_ssa_vars: Vec<usize> = func.args.iter().map(|a| a.ssa_var).collect();
    let transformed = TcoPass::run_function(func);

    // Find the block that had the self-tail-call (block 2) or its replacement.
    // It must now contain Assign instructions for the argument slots.
    // Find the block that branches BACK to the loop head (id 0).
    // The loop head itself also has Branch(1), so we specifically look for
    // a block that branches to id 0 AND has non-empty instructions — that is
    // the rewritten recursive block, not the empty loop head dispatcher.
    let loop_back_block = transformed.blocks.iter().find(|block| {
        matches!(block.terminator, MirTerminator::Branch(0)) && !block.instructions.is_empty()
    });

    assert!(
        loop_back_block.is_some(),
        "No non-empty block branching back to the loop head (id=0) was found"
    );

    let loop_back_block = loop_back_block.unwrap();
    let assign_dests: Vec<usize> = loop_back_block
        .instructions
        .iter()
        .filter_map(|inst| {
            if let MirInstruction::Assign { dest, .. } = inst {
                Some(*dest)
            } else {
                None
            }
        })
        .collect();

    // Each argument must have an assignment before the back-edge
    // (These will be written into the mutable shadow slots)
    assert!(
        !assign_dests.is_empty(),
        "The loop-back block must contain Assign instructions for argument updates. Got: {:?}",
        loop_back_block.instructions
    );
}

/// GREEN CONTRACT: A function with NO self-tail-call must pass through unchanged.
/// The pass is an identity transform for non-recursive functions.
#[test]
fn tco_pass_is_identity_for_non_recursive_functions() {
    let func = MirFunction {
        name: "simple-add".to_string(),
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
        is_pure_data_leaf: true,
        diminishing: None,
        blocks: vec![BasicBlock {
            id: 0,
            instructions: vec![MirInstruction::BinaryOperation {
                dest: 2,
                op: MirBinOp::Add,
                lhs: MirOperand::Variable(0, false),
                rhs: MirOperand::Variable(1, false),
            }],
            terminator: MirTerminator::Return(MirOperand::Variable(2, false)),
        }],
    };

    let original_block_count = func.blocks.len();
    let transformed = TcoPass::run_function(func);

    assert_eq!(
        transformed.blocks.len(),
        original_block_count,
        "TcoPass must not modify functions without self-tail-calls"
    );
}
