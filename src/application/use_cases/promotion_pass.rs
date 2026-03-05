use crate::domain::entities::mir::{MirFunction, MirInstruction, MirOperand, MirProgram};
use crate::domain::entities::types::OnuType;
use std::collections::{HashMap, HashSet};

pub struct PromotionPass;

impl PromotionPass {
    pub fn run(program: MirProgram) -> MirProgram {
        // Phase 1: Identify functions that return WideInt (Seeding)
        let wide_returning_fns: HashSet<String> = program
            .functions
            .iter()
            .filter(|f| matches!(f.return_type, OnuType::WideInt(_)))
            .map(|f| f.name.clone())
            .collect();

        if wide_returning_fns.is_empty() {
            return program;
        }

        MirProgram {
            functions: program
                .functions
                .into_iter()
                .map(|f| Self::run_function(f, &wide_returning_fns))
                .collect(),
        }
    }

    fn run_function(mut func: MirFunction, wide_returning_fns: &HashSet<String>) -> MirFunction {
        // Phase 2: Identify all SSA variables that MUST be widened (Propagation)
        // A variable must be widened if it is influenced by a WideInt source (e.g., a Call to a wide-returning fn).
        let mut widened_ssas = HashSet::new();
        let mut types = HashMap::new();

        // Check calls
        for block in &func.blocks {
            for inst in &block.instructions {
                if let MirInstruction::Call { name, dest, .. } = inst {
                    if wide_returning_fns.contains(name) {
                        widened_ssas.insert(*dest);
                        // If it's a seed, we assume the width of the function's own return type
                        // (or propagate it explicitly if we had better tracking).
                        // For now, we'll use the function's own width if it's wide.
                        if let OnuType::WideInt(bits) = func.return_type {
                            types.insert(*dest, OnuType::WideInt(bits));
                        }
                    }
                }
            }
        }

        if widened_ssas.is_empty() {
            return func;
        }

        // Fixed-point propagation
        let mut changed = true;
        while changed {
            changed = false;
            for block in &func.blocks {
                for inst in &block.instructions {
                    match inst {
                        MirInstruction::BinaryOperation { dest, lhs, rhs, .. } => {
                            let lhs_wide = match lhs {
                                MirOperand::Variable(v, _) => widened_ssas.contains(v),
                                _ => false,
                            };
                            let rhs_wide = match rhs {
                                MirOperand::Variable(v, _) => widened_ssas.contains(v),
                                _ => false,
                            };
                            if (lhs_wide || rhs_wide) && !widened_ssas.contains(dest) {
                                widened_ssas.insert(*dest);
                                changed = true;
                                if let OnuType::WideInt(bits) = func.return_type {
                                    types.insert(*dest, OnuType::WideInt(bits));
                                }
                            }
                        }
                        MirInstruction::Assign { dest, src } => {
                            let src_wide = match src {
                                MirOperand::Variable(v, _) => widened_ssas.contains(v),
                                _ => false,
                            };
                            if src_wide && !widened_ssas.contains(dest) {
                                widened_ssas.insert(*dest);
                                changed = true;
                                if let OnuType::WideInt(bits) = func.return_type {
                                    types.insert(*dest, OnuType::WideInt(bits));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Phase 3: Coherence - Update return type and insert Promote instructions
        let target_bits = if let OnuType::WideInt(bits) = func.return_type {
            bits
        } else {
            // If the function isn't supposed to return wide but HAS wide internals,
            // (unlikely given our current seeding), we don't promote the return.
            return func;
        };

        let mut max_ssa = func.args.iter().map(|a| a.ssa_var).max().unwrap_or(0);
        for block in &func.blocks {
            for inst in &block.instructions {
                let dest = match inst {
                    MirInstruction::Assign { dest, .. } => Some(*dest),
                    MirInstruction::BinaryOperation { dest, .. } => Some(*dest),
                    MirInstruction::Call { dest, .. } => Some(*dest),
                    MirInstruction::Tuple { dest, .. } => Some(*dest),
                    MirInstruction::Index { dest, .. } => Some(*dest),
                    MirInstruction::Alloc { dest, .. } => Some(*dest),
                    MirInstruction::PointerOffset { dest, .. } => Some(*dest),
                    _ => None,
                };
                if let Some(d) = dest {
                    if d > max_ssa {
                        max_ssa = d;
                    }
                }
            }
        }

        // Transformation
        for block in &mut func.blocks {
            let mut new_instructions = Vec::new();
            for inst in &block.instructions {
                match inst {
                    MirInstruction::BinaryOperation { dest, op, lhs, rhs } => {
                        let lhs_wide = match lhs {
                            MirOperand::Variable(v, _) => widened_ssas.contains(v),
                            _ => false,
                        };
                        let rhs_wide = match rhs {
                            MirOperand::Variable(v, _) => widened_ssas.contains(v),
                            _ => false,
                        };

                        let mut new_lhs = lhs.clone();
                        let mut new_rhs = rhs.clone();

                        if widened_ssas.contains(dest) {
                            if !lhs_wide {
                                max_ssa += 1;
                                let p_dest = max_ssa;
                                new_instructions.push(MirInstruction::Promote {
                                    dest: p_dest,
                                    src: lhs.clone(),
                                    to_type: OnuType::WideInt(target_bits),
                                });
                                new_lhs = MirOperand::Variable(p_dest, false);
                            }
                            if !rhs_wide {
                                max_ssa += 1;
                                let p_dest = max_ssa;
                                new_instructions.push(MirInstruction::Promote {
                                    dest: p_dest,
                                    src: rhs.clone(),
                                    to_type: OnuType::WideInt(target_bits),
                                });
                                new_rhs = MirOperand::Variable(p_dest, false);
                            }
                        }

                        new_instructions.push(MirInstruction::BinaryOperation {
                            dest: *dest,
                            op: op.clone(),
                            lhs: new_lhs,
                            rhs: new_rhs,
                        });
                    }
                    MirInstruction::Assign { dest, src } => {
                        let src_wide = match src {
                            MirOperand::Variable(v, _) => widened_ssas.contains(v),
                            _ => false,
                        };
                        if widened_ssas.contains(dest) && !src_wide {
                            new_instructions.push(MirInstruction::Promote {
                                dest: *dest,
                                src: src.clone(),
                                to_type: OnuType::WideInt(target_bits),
                            });
                        } else {
                            new_instructions.push(inst.clone());
                        }
                    }
                    MirInstruction::Call {
                        dest,
                        name,
                        args,
                        return_type,
                        arg_types,
                        is_tail_call,
                    } => {
                        if wide_returning_fns.contains(name) {
                            new_instructions.push(MirInstruction::Call {
                                dest: *dest,
                                name: name.clone(),
                                args: args.clone(),
                                return_type: OnuType::WideInt(target_bits),
                                arg_types: arg_types.clone(),
                                is_tail_call: *is_tail_call,
                            });
                        } else {
                            new_instructions.push(inst.clone());
                        }
                    }
                    _ => new_instructions.push(inst.clone()),
                }
            }
            block.instructions = new_instructions;

            // Handle terminator
            if let crate::domain::entities::mir::MirTerminator::Return(MirOperand::Variable(v, _)) =
                &block.terminator
            {
                if !widened_ssas.contains(v) && widened_ssas.contains(v) { // Wait, logic error here
                }
            }
            // If returning a variable that isn't widened, but the function IS wide:
            if let crate::domain::entities::mir::MirTerminator::Return(op) = &block.terminator {
                let op_wide = match op {
                    MirOperand::Variable(v, _) => widened_ssas.contains(v),
                    _ => false,
                };
                if !op_wide {
                    max_ssa += 1;
                    let p_dest = max_ssa;
                    block.instructions.push(MirInstruction::Promote {
                        dest: p_dest,
                        src: op.clone(),
                        to_type: OnuType::WideInt(target_bits),
                    });
                    block.terminator = crate::domain::entities::mir::MirTerminator::Return(
                        MirOperand::Variable(p_dest, false),
                    );
                }
            }
        }

        func
    }
}
