use crate::domain::entities::hir::HirExpression;
use crate::domain::entities::mir::{MirInstruction, MirOperand, MirTerminator};
use crate::application::use_cases::mir_builder::MirBuilder;
use crate::domain::entities::error::OnuError;
use super::super::mir_lowering_service::{MirLoweringService, LoweringContext};
use crate::application::ports::environment::EnvironmentPort;
use super::ExprLowerer;

pub struct IfLowerer;

impl ExprLowerer for IfLowerer {
    fn lower<'a, E: EnvironmentPort>(
        &self,
        expr: &HirExpression,
        context: &LoweringContext<'a, E>,
        builder: &mut MirBuilder,
        is_tail: bool,
    ) -> Result<MirOperand, OnuError> {
        if let HirExpression::If { condition, then_branch, else_branch } = expr {
            let cond_op = context.lower_expression(condition, builder, false)?;
            
            // STRICT POLICY: Cleanup condition after use
            if let MirOperand::Variable(ssa_id, _) = &cond_op {
                if let Some(typ) = builder.resolve_ssa_type(*ssa_id) {
                    if typ.is_resource() {
                        let is_dyn = builder.resolve_ssa_is_dynamic(*ssa_id);
                        builder.mark_consumed(*ssa_id);
                        if is_dyn {
                            builder.emit(MirInstruction::Drop { ssa_var: *ssa_id, typ, name: format!("if_cond_{}", ssa_id), is_dynamic: is_dyn });
                        }
                    }
                }
            }

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
            let then_res = context.lower_expression(then_branch, builder, is_tail)?;
            if is_tail {
                builder.terminate(MirTerminator::Return(then_res.clone()));
            }
            let then_consumed = builder.get_consumed_vars();
            let then_end_id = builder.get_current_block_id();

            // Reset consumed vars to pre-branch state for the else branch
            builder.set_consumed_vars(pre_branch_consumed.clone());

            builder.switch_to_block(else_start_id);
            let else_res = context.lower_expression(else_branch, builder, is_tail)?;
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
                builder.clear_current_block();
                return Ok(MirOperand::Constant(crate::domain::entities::mir::MirLiteral::Nothing));
            }

            let merge_id = builder.create_block();
            let dest = builder.new_ssa();

            if let Some(id) = then_end_id {
                builder.switch_to_block(id);
                // CUSTODY TRANSFER: If branch result is a resource, mark it consumed as it moves to 'dest'
                if let MirOperand::Variable(ssa_id, _) = &then_res {
                    if builder.resolve_ssa_type(*ssa_id).map(|t| t.is_resource()).unwrap_or(false) {
                        builder.mark_consumed(*ssa_id);
                    }
                }
                builder.emit(MirInstruction::Assign { dest, src: then_res });
                builder.terminate(MirTerminator::Branch(merge_id));
            }
            if let Some(id) = else_end_id {
                builder.switch_to_block(id);
                // CUSTODY TRANSFER: If branch result is a resource, mark it consumed as it moves to 'dest'
                if let MirOperand::Variable(ssa_id, _) = &else_res {
                    if builder.resolve_ssa_type(*ssa_id).map(|t| t.is_resource()).unwrap_or(false) {
                        builder.mark_consumed(*ssa_id);
                    }
                }
                builder.emit(MirInstruction::Assign { dest, src: else_res });
                builder.terminate(MirTerminator::Branch(merge_id));
            }

            builder.switch_to_block(merge_id);
            Ok(MirOperand::Variable(dest, false))
        } else {
            Err(OnuError::GrammarViolation {
                message: "Expected If expression".to_string(),
                span: Default::default(),
            })
        }
    }
}

// --- Legacy Compatibility ---
impl<'a, E: EnvironmentPort> MirLoweringService<'a, E> {
    pub fn lower_if(&self, condition: &HirExpression, then_branch: &HirExpression, else_branch: &HirExpression, builder: &mut MirBuilder, is_tail: bool) -> Result<MirOperand, OnuError> {
        IfLowerer.lower(&HirExpression::If { 
            condition: Box::new(condition.clone()), 
            then_branch: Box::new(then_branch.clone()), 
            else_branch: Box::new(else_branch.clone()) 
        }, &self.context, builder, is_tail)
    }
}
