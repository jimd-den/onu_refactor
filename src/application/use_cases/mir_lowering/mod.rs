pub mod lower_expr;
pub mod lower_blocks;
pub mod lower_calls;
pub mod lower_control_flow;

use crate::domain::entities::hir::HirExpression;
use crate::domain::entities::mir::MirOperand;
use crate::application::use_cases::mir_builder::MirBuilder;
use crate::application::use_cases::mir_lowering_service::LoweringContext;
use crate::domain::entities::error::OnuError;
use crate::application::ports::environment::EnvironmentPort;

pub trait ExprLowerer {
    fn lower<'a, E: EnvironmentPort>(
        &self,
        expr: &HirExpression,
        context: &LoweringContext<'a, E>,
        builder: &mut MirBuilder,
        is_tail: bool,
    ) -> Result<MirOperand, OnuError>;
}
