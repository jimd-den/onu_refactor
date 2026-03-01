use crate::domain::entities::hir::HirExpression;
use crate::domain::entities::mir::{MirInstruction, MirOperand, MirLiteral};
use crate::domain::entities::types::OnuType;
use crate::application::use_cases::mir_builder::MirBuilder;
use crate::domain::entities::error::OnuError;
use super::super::mir_lowering_service::MirLoweringService;
use crate::application::ports::environment::EnvironmentPort;

impl<'a, E: EnvironmentPort> MirLoweringService<'a, E> {
    pub fn lower_block(&self, exprs: &[HirExpression], builder: &mut MirBuilder, is_tail: bool) -> Result<MirOperand, OnuError> {
        let mut last_op = MirOperand::Constant(MirLiteral::Nothing);
        let len = exprs.len();
        for (i, expr) in exprs.iter().enumerate() {
            let is_last = i == len - 1;
            last_op = self.lower_expression(expr, builder, is_tail && is_last)?;
            
            if builder.get_current_block_id() == Some(9999) { break; }
        }
        Ok(last_op)
    }

    pub fn lower_derivation(&self, name: &str, typ: &OnuType, value: &HirExpression, body: &HirExpression, builder: &mut MirBuilder, is_tail: bool) -> Result<MirOperand, OnuError> {
        let val_op = self.lower_expression(value, builder, false)?;
        
        // If the value is a resource variable, mark it as consumed to transfer ownership
        if let MirOperand::Variable(ssa_id, _) = val_op {
            if typ.is_resource() {
                builder.mark_consumed(ssa_id);
            }
        }

        let ssa_var = builder.new_ssa();
        builder.emit(MirInstruction::Assign { dest: ssa_var, src: val_op });
        builder.enter_scope();
        builder.define_variable(name, ssa_var, typ.clone());

        let res = self.lower_expression(body, builder, is_tail)?;

        // If the result is a resource variable, mark it as consumed to transfer ownership to the caller
        if let MirOperand::Variable(res_id, _) = &res {
            builder.mark_consumed(*res_id);
        }

        builder.exit_scope();
        Ok(res)
    }
}
