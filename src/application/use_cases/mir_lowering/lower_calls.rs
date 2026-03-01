use crate::domain::entities::hir::HirExpression;
use crate::domain::entities::mir::{MirInstruction, MirOperand};
use crate::domain::entities::types::OnuType;
use crate::application::use_cases::mir_builder::MirBuilder;
use crate::domain::entities::error::OnuError;
use super::super::mir_lowering_service::{MirLoweringService, LoweringContext};
use crate::application::use_cases::stdlib_lowering::StdlibLowering;
use crate::application::ports::environment::EnvironmentPort;
use super::ExprLowerer;

pub struct CallLowerer;

impl ExprLowerer for CallLowerer {
    fn lower<'a, E: EnvironmentPort>(
        &self,
        expr: &HirExpression,
        context: &LoweringContext<'a, E>,
        builder: &mut MirBuilder,
        _is_tail: bool,
    ) -> Result<MirOperand, OnuError> {
        if let HirExpression::Call { name, args } = expr {
            let mut mir_args = Vec::new();
            for arg in args {
                mir_args.push(context.lower_expression(arg, builder, false)?);
            }

            let (return_type, arg_types, arg_is_observation) = if let Some(sig) = context.registry.get_signature(name) {
                (sig.return_type.clone(), sig.input_types.clone(), sig.arg_is_observation.clone())
            } else {
                (OnuType::Nothing, Vec::new(), Vec::new())
            };

            if name == "as-text" && mir_args.len() == 1 {
                let res = StdlibLowering::lower_as_text(&mir_args[0], builder);
                if let MirOperand::Variable(ssa_id, _) = &res {
                    builder.set_ssa_type(*ssa_id, OnuType::Strings);
                }
                
                // Cleanup as-text input if it's a resource (it takes custody)
                if let MirOperand::Variable(ssa_id, _) = &mir_args[0] {
                    if let Some(typ) = builder.resolve_ssa_type(*ssa_id) {
                        if typ.is_resource() {
                            builder.mark_consumed(*ssa_id);
                            builder.schedule_drop(*ssa_id, typ);
                        }
                    }
                }
                return Ok(res);
            }

            if name == "joined-with" && mir_args.len() == 2 {
                let res = StdlibLowering::lower_joined_with(&mir_args[0], &mir_args[1], builder);
                if let MirOperand::Variable(ssa_id, _) = &res {
                    builder.set_ssa_type(*ssa_id, OnuType::Strings);
                }
                
                // Cleanup joined-with inputs
                for arg_op in &mir_args {
                    if let MirOperand::Variable(ssa_id, _) = arg_op {
                        if let Some(typ) = builder.resolve_ssa_type(*ssa_id) {
                            if typ.is_resource() {
                                builder.mark_consumed(*ssa_id);
                                builder.schedule_drop(*ssa_id, typ);
                            }
                        }
                    }
                }
                return Ok(res);
            }

            let dest = builder.new_ssa();
            builder.emit(MirInstruction::Call {
                dest,
                name: name.to_string(),
                args: mir_args.clone(),
                return_type: return_type.clone(),
                arg_types,
            });
            builder.set_ssa_type(dest, return_type);

            // Parent cleanup: mark resource arguments as consumed and schedule drop IF NOT OBSERVATION
            for (i, arg_op) in mir_args.iter().enumerate() {
                if let MirOperand::Variable(ssa_id, _) = arg_op {
                    let is_observation = arg_is_observation.get(i).copied().unwrap_or(false);
                    if !is_observation {
                        if let Some(typ) = builder.resolve_ssa_type(*ssa_id) {
                            if typ.is_resource() {
                                builder.mark_consumed(*ssa_id);
                                builder.schedule_drop(*ssa_id, typ);
                            }
                        }
                    }
                }
            }

            Ok(MirOperand::Variable(dest, false))
        } else {
            Err(OnuError::GrammarViolation {
                message: "Expected Call expression".to_string(),
                span: Default::default(),
            })
        }
    }
}

// --- Legacy Compatibility ---
impl<'a, E: EnvironmentPort> MirLoweringService<'a, E> {
    pub fn lower_call(&self, name: &str, args: &[HirExpression], builder: &mut MirBuilder) -> Result<MirOperand, OnuError> {
        CallLowerer.lower(&HirExpression::Call { name: name.to_string(), args: args.to_vec() }, &self.context, builder, false)
    }
}
