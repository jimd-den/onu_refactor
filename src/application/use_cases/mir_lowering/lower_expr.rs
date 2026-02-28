use crate::domain::entities::hir::HirLiteral;
use crate::domain::entities::mir::{MirOperand, MirLiteral};
use crate::domain::entities::types::OnuType;
use crate::application::use_cases::mir_builder::MirBuilder;
use crate::domain::entities::error::OnuError;
use super::super::mir_lowering_service::MirLoweringService;
use crate::application::ports::environment::EnvironmentPort;

impl<'a, E: EnvironmentPort> MirLoweringService<'a, E> {
    pub fn lower_literal(&self, lit: &HirLiteral) -> Result<MirOperand, OnuError> {
        let mir_lit = match lit {
            HirLiteral::I64(n) => MirLiteral::I64(*n),
            HirLiteral::F64(n) => MirLiteral::F64(*n),
            HirLiteral::Boolean(b) => MirLiteral::Boolean(*b),
            HirLiteral::Text(s) => MirLiteral::Text(s.clone()),
            HirLiteral::Nothing => MirLiteral::Nothing,
        };
        Ok(MirOperand::Constant(mir_lit))
    }

    pub fn lower_variable(&self, name: &str, is_consuming: bool, builder: &mut MirBuilder) -> Result<MirOperand, OnuError> {
        let ssa_var = builder.resolve_variable(name)
            .ok_or_else(|| OnuError::GrammarViolation {
                message: format!("Unresolved variable: {}", name),
                span: crate::domain::entities::error::Span::default()
            })?;
        let typ = builder.resolve_variable_type(name).unwrap_or(OnuType::Nothing);
        if is_consuming {
            builder.schedule_drop(ssa_var, typ.clone());
            builder.mark_consumed(ssa_var);
        }
        Ok(MirOperand::Variable(ssa_var, is_consuming))
    }
}
