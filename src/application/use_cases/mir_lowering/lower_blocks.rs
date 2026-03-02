use crate::domain::entities::hir::HirExpression;
use crate::domain::entities::mir::{MirInstruction, MirOperand, MirLiteral};
use crate::domain::entities::types::OnuType;
use crate::application::use_cases::mir_builder::MirBuilder;
use crate::domain::entities::error::OnuError;
use super::super::mir_lowering_service::{MirLoweringService, LoweringContext};
use crate::application::ports::environment::EnvironmentPort;
use super::ExprLowerer;

pub struct BlockLowerer;
pub struct DerivationLowerer;

impl ExprLowerer for BlockLowerer {
    fn lower<'a, E: EnvironmentPort>(
        &self,
        expr: &HirExpression,
        context: &LoweringContext<'a, E>,
        builder: &mut MirBuilder,
        is_tail: bool,
    ) -> Result<MirOperand, OnuError> {
        if let HirExpression::Block(exprs) = expr {
            let mut last_op = MirOperand::Constant(MirLiteral::Nothing);
            let len = exprs.len();
            for (i, e) in exprs.iter().enumerate() {
                let is_last = i == len - 1;
                
                last_op = context.lower_expression(e, builder, is_tail && is_last)?;
                
                // If it's not the last expression in the block, we must drop the result 
                // because it's an intermediate that won't be used by anyone.
                if !is_last {
                    if let MirOperand::Variable(ssa_id, _) = &last_op {
                        if let Some(typ) = builder.resolve_ssa_type(*ssa_id) {
                            if typ.is_resource() && !builder.is_consumed(*ssa_id) {
                                let is_dyn = builder.resolve_ssa_is_dynamic(*ssa_id);
                                builder.mark_consumed(*ssa_id);
                                if is_dyn {
                                    builder.emit(MirInstruction::Drop { ssa_var: *ssa_id, typ, name: format!("block_inter_{}", ssa_id), is_dynamic: is_dyn });
                                }
                            }
                        }
                    }
                }
                
                if builder.get_current_block_id().is_none() { break; }
            }
            Ok(last_op)
        } else {
            Err(OnuError::GrammarViolation {
                message: "Expected Block expression".to_string(),
                span: Default::default(),
            })
        }
    }
}

impl ExprLowerer for DerivationLowerer {
    fn lower<'a, E: EnvironmentPort>(
        &self,
        expr: &HirExpression,
        context: &LoweringContext<'a, E>,
        builder: &mut MirBuilder,
        is_tail: bool,
    ) -> Result<MirOperand, OnuError> {
        if let HirExpression::Derivation { name, typ, value, body } = expr {
            let val_op = context.lower_expression(value, builder, false)?;
            
            let mut is_val_dyn = false;

            // STRICT POLICY: Transfer custody to the named variable.
            // Mark the source as consumed so it isn't dropped by pending_drops,
            // but DO NOT emit a Drop instruction here because the pointer
            // is now owned by the new named variable.
            if let MirOperand::Variable(ssa_id, _) = &val_op {
                if let Some(vt) = builder.resolve_ssa_type(*ssa_id) {
                    if vt.is_resource() && !builder.is_consumed(*ssa_id) {
                        builder.mark_consumed(*ssa_id);
                        is_val_dyn = builder.resolve_ssa_is_dynamic(*ssa_id);
                    }
                }
            }

            let ssa_var = builder.new_ssa();
            builder.emit(MirInstruction::Assign { dest: ssa_var, src: val_op });
            builder.set_ssa_type(ssa_var, typ.clone());
            if is_val_dyn {
                builder.set_ssa_is_dynamic(ssa_var, true);
            }
            
            eprintln!("[DEBUG] Defining variable: {} (SSA: {})", name, ssa_var);
            builder.enter_scope();
            builder.define_variable(name, ssa_var, typ.clone());

            let res = context.lower_expression(body, builder, is_tail)?;

            eprintln!("[DEBUG] Exiting scope for: {}", name);
            builder.exit_scope();
            Ok(res)
        } else {
            Err(OnuError::GrammarViolation {
                message: "Expected Derivation expression".to_string(),
                span: Default::default(),
            })
        }
    }
}

// --- Legacy Compatibility ---
impl<'a, E: EnvironmentPort> MirLoweringService<'a, E> {
    pub fn lower_block(&self, exprs: &[HirExpression], builder: &mut MirBuilder, is_tail: bool) -> Result<MirOperand, OnuError> {
        BlockLowerer.lower(&HirExpression::Block(exprs.to_vec()), &self.context, builder, is_tail)
    }

    pub fn lower_derivation(&self, name: &str, typ: &OnuType, value: &HirExpression, body: &HirExpression, builder: &mut MirBuilder, is_tail: bool) -> Result<MirOperand, OnuError> {
        DerivationLowerer.lower(&HirExpression::Derivation { 
            name: name.to_string(), 
            typ: typ.clone(), 
            value: Box::new(value.clone()), 
            body: Box::new(body.clone()) 
        }, &self.context, builder, is_tail)
    }
}
