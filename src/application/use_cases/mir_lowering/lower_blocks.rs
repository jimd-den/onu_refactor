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
                
                // Cleanup handled centrally by each lower_expression call
                last_op = context.lower_expression(e, builder, is_tail && is_last)?;
                
                if builder.get_current_block_id().is_none() { break; }
            }
            
            // CUSTODY TRANSFER: Mark the final result as consumed so the parent owns it
            if let MirOperand::Variable(ssa_id, _) = &last_op {
                if builder.resolve_ssa_type(*ssa_id).map(|t| t.is_resource()).unwrap_or(false) {
                    builder.mark_consumed(*ssa_id);
                }
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
            
            // CUSTODY TRANSFER: Mark original value as consumed (transfer to derivation variable)
            if let MirOperand::Variable(ssa_id, _) = &val_op {
                if builder.resolve_ssa_type(*ssa_id).map(|t| t.is_resource()).unwrap_or(false) {
                    builder.mark_consumed(*ssa_id);
                }
            }

            let ssa_var = builder.new_ssa();
            builder.emit(MirInstruction::Assign { dest: ssa_var, src: val_op });
            builder.set_ssa_type(ssa_var, typ.clone());
            
            builder.enter_scope();
            builder.define_variable(name, ssa_var, typ.clone());

            let res = context.lower_expression(body, builder, is_tail)?;

            // CUSTODY TRANSFER: If result is a resource variable and it's being returned, mark it consumed
            // to transfer ownership to the parent.
            if let MirOperand::Variable(res_id, _) = &res {
                if builder.resolve_ssa_type(*res_id).map(|t| t.is_resource()).unwrap_or(false) {
                    builder.mark_consumed(*res_id);
                }
            }

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
