/// Ọ̀nụ Analysis Service: Application Use Case
///
/// This service orchestrates the semantic analysis rules (Ownership, Liveness)
/// to validate the integrity of the proposition.

use crate::domain::entities::hir::{HirDiscourse, HirExpression};
use crate::domain::entities::error::OnuError;
use crate::domain::rules::ownership::OwnershipRule;
use crate::domain::rules::liveness::LivenessRule;
use crate::application::use_cases::registry_service::RegistryService;

pub struct AnalysisService<'a> {
    ownership_rule: OwnershipRule<'a>,
    liveness_rule: LivenessRule,
}

impl<'a> AnalysisService<'a> {
    pub fn new(registry: &'a RegistryService) -> Self {
        Self {
            ownership_rule: OwnershipRule::new(registry),
            liveness_rule: LivenessRule::new(),
        }
    }

    pub fn analyze_discourse(&self, discourse: &mut HirDiscourse) -> Result<(), OnuError> {
        match discourse {
            HirDiscourse::Behavior { header, body } => {
                // 1. Perform Liveness Analysis (mutates body)
                self.liveness_rule.analyze(body);

                // 2. Perform Ownership Validation
                self.ownership_rule.validate(header, body)?;

                Ok(())
            }
            _ => Ok(()),
        }
    }
}
