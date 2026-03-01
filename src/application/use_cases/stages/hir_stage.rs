use crate::application::options::LogLevel;
use crate::application::ports::environment::EnvironmentPort;
use crate::application::use_cases::analysis_service::AnalysisService;
use crate::application::use_cases::lowering_service::LoweringService;
use crate::application::use_cases::registry_service::RegistryService;
use crate::domain::entities::ast::Discourse;
use crate::domain::entities::hir::HirDiscourse;
use crate::domain::entities::error::OnuError;
use super::PipelineStage;

pub struct HirStage<'a, E: EnvironmentPort> {
    env: &'a E,
    registry: &'a RegistryService,
    emit_hir: bool,
}

impl<'a, E: EnvironmentPort> HirStage<'a, E> {
    pub fn new(env: &'a E, registry: &'a RegistryService, emit_hir: bool) -> Self {
        Self { env, registry, emit_hir }
    }
}

impl<'a, E: EnvironmentPort> PipelineStage for HirStage<'a, E> {
    type Input = Vec<Discourse>;
    type Output = Vec<HirDiscourse>;

    fn execute(&mut self, discourses: Vec<Discourse>) -> Result<Vec<HirDiscourse>, OnuError> {
        let analysis_service = AnalysisService::new(self.env, self.registry);
        let mut hir_discourses = Vec::new();
        for discourse in discourses {
            let mut hir = LoweringService::lower_discourse(&discourse, self.registry);
            analysis_service.analyze_discourse(&mut hir)?;
            if self.emit_hir {
                self.env.log(LogLevel::Debug, &format!("HIR Emit: {:?}", hir));
            }
            hir_discourses.push(hir);
        }
        Ok(hir_discourses)
    }
}
