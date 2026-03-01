use crate::domain::entities::hir::HirExpression;
use crate::domain::entities::mir::{MirInstruction, MirOperand, MirTerminator};
use crate::application::use_cases::mir_builder::MirBuilder;
use crate::domain::entities::error::OnuError;
use super::super::mir_lowering_service::MirLoweringService;
use crate::application::ports::environment::EnvironmentPort;

impl<'a, E: EnvironmentPort> MirLoweringService<'a, E> {
    pub fn lower_if(&self, condition: &HirExpression, then_branch: &HirExpression, else_branch: &HirExpression, builder: &mut MirBuilder, is_tail: bool) -> Result<MirOperand, OnuError> {
        let cond_op = self.lower_expression(condition, builder, false)?;
        let then_start_id = builder.create_block();
        let else_start_id = builder.create_block();

        builder.terminate(MirTerminator::CondBranch {
            condition: cond_op,
            then_block: then_start_id,
            else_block: else_start_id
        });

        // Snapshot consumed variables before branching
        let pre_branch_consumed = builder.get_consumed_vars();

        builder.switch_to_block(then_start_id);
        let then_res = self.lower_expression(then_branch, builder, is_tail)?;
        if is_tail {
            builder.terminate(MirTerminator::Return(then_res.clone()));
        }
        let then_consumed = builder.get_consumed_vars();
        let then_end_id = builder.get_current_block_id();

        // Reset consumed vars to pre-branch state for the else branch
        builder.set_consumed_vars(pre_branch_consumed.clone());

        builder.switch_to_block(else_start_id);
        let else_res = self.lower_expression(else_branch, builder, is_tail)?;
        if is_tail {
            builder.terminate(MirTerminator::Return(else_res.clone()));
        }
        let else_consumed = builder.get_consumed_vars();
        let else_end_id = builder.get_current_block_id();

        // For the merge block, the union of consumed vars should be used
        let mut final_consumed = then_consumed;
        final_consumed.extend(else_consumed);
        builder.set_consumed_vars(final_consumed);

        if is_tail {
            // If it's a tail call, we don't need a merge block because the branches will return
            builder.clear_current_block();
            return Ok(MirOperand::Constant(crate::domain::entities::mir::MirLiteral::Nothing));
        }

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
