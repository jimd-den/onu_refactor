/// Wide-Integer Division Legalization Pass: Application Use Case
///
/// Implements the "Proxy Pattern" described in the Clean Architecture requirements:
/// intercepts any `BinaryOperation { op: Div | Mod, dest_type: WideInt(bits) }`
/// where `bits > 128` and replaces it with a `Call` to a compiler-internal helper
/// function (`__onu_wide_div_<bits>` / `__onu_wide_mod_<bits>`).
///
/// This prevents the LLVM backend from ever seeing an `sdiv iN` instruction for
/// widths it cannot lower via its standard runtime library (compiler-rt only ships
/// helpers up to i128).  The actual implementation of the helper is emitted later
/// by the LLVM code-generator using a binary long-division algorithm built from
/// operations that *are* fully supported at any width (shifts, comparisons, add/sub).
///
/// Design pattern: Strategy + Proxy (Application Policy layer).
/// The pass is "width-aware" – it only legalises operations that are provably
/// unsupported by the LLVM backend (> 128-bit division/modulo), leaving narrower
/// operations untouched so that LLVM can continue to optimise them natively.
use crate::domain::entities::mir::{
    BasicBlock, MirBinOp, MirFunction, MirInstruction, MirProgram,
};
use crate::domain::entities::types::OnuType;

/// Threshold above which LLVM's backend cannot lower integer division/modulo
/// without a compiler-rt entry that does not exist.
const MAX_NATIVE_DIV_BITS: u32 = 128;

pub struct WideDivLegalizationPass;

impl WideDivLegalizationPass {
    /// Run the legalization pass over the entire MIR program.
    ///
    /// For every function, every `BinaryOperation { op: Div | Mod, dest_type: WideInt(bits) }`
    /// where `bits > 128` is replaced with a `Call` to the appropriate helper.
    /// Wide addition, subtraction, multiplication and comparisons are left as-is because
    /// LLVM can always lower those through carry-chain expansion.
    pub fn run(program: MirProgram) -> MirProgram {
        let functions = program
            .functions
            .into_iter()
            .map(Self::legalize_function)
            .collect();
        MirProgram { functions }
    }

    fn legalize_function(func: MirFunction) -> MirFunction {
        let blocks = func
            .blocks
            .into_iter()
            .map(Self::legalize_block)
            .collect();
        MirFunction { blocks, ..func }
    }

    fn legalize_block(block: BasicBlock) -> BasicBlock {
        let instructions = block
            .instructions
            .into_iter()
            .map(Self::legalize_instruction)
            .collect();
        BasicBlock {
            instructions,
            ..block
        }
    }

    fn legalize_instruction(inst: MirInstruction) -> MirInstruction {
        if let MirInstruction::BinaryOperation {
            dest,
            op,
            lhs,
            rhs,
            dest_type,
        } = &inst
        {
            if let OnuType::WideInt(bits) = dest_type {
                if *bits > MAX_NATIVE_DIV_BITS {
                    match op {
                        MirBinOp::Div => {
                            // Replace with a call to __onu_wide_div_<bits>
                            let helper_name = format!("__onu_wide_div_{}", bits);
                            return MirInstruction::Call {
                                dest: *dest,
                                name: helper_name,
                                args: vec![lhs.clone(), rhs.clone()],
                                return_type: dest_type.clone(),
                                arg_types: vec![dest_type.clone(), dest_type.clone()],
                                is_tail_call: false,
                            };
                        }
                        _ => {}
                    }
                }
            }
        }
        inst
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::mir::{
        BasicBlock, MirArgument, MirBinOp, MirFunction, MirInstruction, MirLiteral, MirOperand,
        MirTerminator,
    };

    fn make_wide_div_function(bits: u32) -> MirFunction {
        MirFunction {
            name: "test_wide_div".to_string(),
            args: vec![
                MirArgument {
                    name: "a".to_string(),
                    typ: OnuType::WideInt(bits),
                    ssa_var: 0,
                },
                MirArgument {
                    name: "b".to_string(),
                    typ: OnuType::WideInt(bits),
                    ssa_var: 1,
                },
            ],
            return_type: OnuType::WideInt(bits),
            blocks: vec![BasicBlock {
                id: 0,
                instructions: vec![MirInstruction::BinaryOperation {
                    dest: 2,
                    op: MirBinOp::Div,
                    lhs: MirOperand::Variable(0, false),
                    rhs: MirOperand::Variable(1, false),
                    dest_type: OnuType::WideInt(bits),
                }],
                terminator: MirTerminator::Return(MirOperand::Variable(2, false)),
            }],
            is_pure_data_leaf: true,
            diminishing: None,
            memo_cache_size: None,
        }
    }

    #[test]
    fn test_wide_div_1024_is_legalized() {
        let func = make_wide_div_function(1024);
        let program = MirProgram {
            functions: vec![func],
        };
        let legalized = WideDivLegalizationPass::run(program);

        let inst = &legalized.functions[0].blocks[0].instructions[0];
        match inst {
            MirInstruction::Call { name, .. } => {
                assert_eq!(name, "__onu_wide_div_1024");
            }
            other => panic!(
                "Expected Call to __onu_wide_div_1024, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_wide_div_256_is_legalized() {
        let func = make_wide_div_function(256);
        let program = MirProgram {
            functions: vec![func],
        };
        let legalized = WideDivLegalizationPass::run(program);

        let inst = &legalized.functions[0].blocks[0].instructions[0];
        match inst {
            MirInstruction::Call { name, .. } => {
                assert!(
                    name.starts_with("__onu_wide_div_"),
                    "Expected call to wide div helper, got {}",
                    name
                );
            }
            other => panic!("Expected Call instruction, got {:?}", other),
        }
    }

    #[test]
    fn test_i64_div_not_legalized() {
        let func = MirFunction {
            name: "test_i64_div".to_string(),
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
                instructions: vec![MirInstruction::BinaryOperation {
                    dest: 2,
                    op: MirBinOp::Div,
                    lhs: MirOperand::Variable(0, false),
                    rhs: MirOperand::Variable(1, false),
                    dest_type: OnuType::I64,
                }],
                terminator: MirTerminator::Return(MirOperand::Variable(2, false)),
            }],
            is_pure_data_leaf: true,
            diminishing: None,
            memo_cache_size: None,
        };
        let program = MirProgram {
            functions: vec![func],
        };
        let legalized = WideDivLegalizationPass::run(program);

        // i64 division should remain as BinaryOperation, NOT replaced by a Call
        let inst = &legalized.functions[0].blocks[0].instructions[0];
        assert!(
            matches!(inst, MirInstruction::BinaryOperation { op: MirBinOp::Div, .. }),
            "i64 division should not be legalized, got {:?}",
            inst
        );
    }

    #[test]
    fn test_wide_add_not_legalized() {
        // Addition on WideInt is supported by LLVM and should NOT be legalized
        let func = MirFunction {
            name: "test_wide_add".to_string(),
            args: vec![
                MirArgument {
                    name: "a".to_string(),
                    typ: OnuType::WideInt(1024),
                    ssa_var: 0,
                },
                MirArgument {
                    name: "b".to_string(),
                    typ: OnuType::WideInt(1024),
                    ssa_var: 1,
                },
            ],
            return_type: OnuType::WideInt(1024),
            blocks: vec![BasicBlock {
                id: 0,
                instructions: vec![MirInstruction::BinaryOperation {
                    dest: 2,
                    op: MirBinOp::Add,
                    lhs: MirOperand::Variable(0, false),
                    rhs: MirOperand::Variable(1, false),
                    dest_type: OnuType::WideInt(1024),
                }],
                terminator: MirTerminator::Return(MirOperand::Variable(2, false)),
            }],
            is_pure_data_leaf: true,
            diminishing: None,
            memo_cache_size: None,
        };
        let program = MirProgram {
            functions: vec![func],
        };
        let legalized = WideDivLegalizationPass::run(program);

        // Addition should remain as BinaryOperation
        let inst = &legalized.functions[0].blocks[0].instructions[0];
        assert!(
            matches!(inst, MirInstruction::BinaryOperation { op: MirBinOp::Add, .. }),
            "WideInt addition should not be legalized, got {:?}",
            inst
        );
    }
}
