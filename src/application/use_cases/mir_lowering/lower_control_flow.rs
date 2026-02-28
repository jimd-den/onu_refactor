use crate::domain::entities::hir::HirExpression;
use crate::domain::entities::mir::{MirInstruction, MirOperand, MirTerminator};
use crate::application::use_cases::mir_builder::MirBuilder;
use crate::domain::entities::error::OnuError;
use super::super::mir_lowering_service::MirLoweringService;
use crate::application::ports::environment::EnvironmentPort;

impl<'a, E: EnvironmentPort> MirLoweringService<'a, E> {
    pub fn lower_if(&self, condition: &HirExpression, then_branch: &HirExpression, else_branch: &HirExpression, builder: &mut MirBuilder, _is_tail: bool) -> Result<MirOperand, OnuError> {
        let cond_op = self.lower_expression(condition, builder, false)?;
        let then_start_id = builder.create_block();
        let else_start_id = builder.create_block();

        builder.terminate(MirTerminator::CondBranch {
            condition: cond_op,
            then_block: then_start_id,
            else_block: else_start_id
        });

        builder.switch_to_block(then_start_id);
        let then_res = self.lower_expression(then_branch, builder, false)?;
        let then_end_id = builder.get_current_block_id();

        builder.switch_to_block(else_start_id);
        let else_res = self.lower_expression(else_branch, builder, false)?;
        let else_end_id = builder.get_current_block_id();

        let merge_id = builder.create_block();
        let dest = builder.new_ssa();

        if let Some(id) = then_end_id {
            builder.switch_to_block(id);
            builder.emit(MirInstruction::Assign { dest, src: then_res });
            builder.terminate(MirTerminator::Branch(merge_id));
        }
        if let Some(id) = else_end_id {
            builder.switch_to_block(id);
            builder.emit(MirInstruction::Assign { dest, src: else_res });
            builder.terminate(MirTerminator::Branch(merge_id));
        }

        builder.switch_to_block(merge_id);
        Ok(MirOperand::Variable(dest, false))
    }
}
