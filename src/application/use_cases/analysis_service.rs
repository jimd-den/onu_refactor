/// Ọ̀nụ Analysis Service: Application Use Case
///
/// This service orchestrates the semantic analysis rules (Ownership, Liveness)
/// to validate the integrity of the proposition.

use crate::domain::entities::hir::HirDiscourse;
use crate::domain::rules::liveness::LivenessRule;
use crate::domain::rules::ownership::OwnershipRule;
use crate::application::use_cases::registry_service::RegistryService;
use crate::domain::entities::error::OnuError;
use crate::application::options::LogLevel;
use chrono::Local;

pub struct AnalysisService<'a> {
    registry: &'a RegistryService,
    liveness_rule: LivenessRule,
    ownership_rule: OwnershipRule<'a>,
}

impl<'a> AnalysisService<'a> {
    pub fn new(registry: &'a RegistryService) -> Self {
        Self {
            registry,
            liveness_rule: LivenessRule,
            ownership_rule: OwnershipRule { registry },
        }
    }

    fn log(&self, level: LogLevel, message: &str) {
        // Since AnalysisService doesn't have env, we log to stderr with timestamp
        if level != LogLevel::None {
            let timestamp = Local::now().to_rfc3339();
            eprintln!("[{}] {:?}: [Analysis] {}", timestamp, level, message);
        }
    }

    pub fn analyze_discourse(&self, discourse: &mut HirDiscourse) -> Result<(), OnuError> {
        self.log(LogLevel::Debug, "Starting discourse analysis");
        match discourse {
            HirDiscourse::Behavior { header, body } => {
                self.log(LogLevel::Trace, &format!("Analyzing behavior: {}", header.name));
                // 1. Perform Liveness Analysis (mutates body)
                self.liveness_rule.analyze(body);

                // 2. Perform Ownership Validation
                self.ownership_rule.validate(header, body)?;

                self.log(LogLevel::Trace, &format!("Validation successful for {}", header.name));
                Ok(())
            }
            _ => {
                self.log(LogLevel::Trace, "Skipping analysis for non-behavior discourse");
                Ok(())
            }
        }
    }
}
