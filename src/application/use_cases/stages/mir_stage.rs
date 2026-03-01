use crate::application::ports::environment::EnvironmentPort;
use crate::application::use_cases::mir_lowering_service::MirLoweringService;
use crate::application::use_cases::registry_service::RegistryService;
use crate::domain::entities::hir::HirDiscourse;
use crate::domain::entities::mir::MirProgram;
use crate::domain::entities::error::OnuError;
use super::PipelineStage;

pub struct MirStage<'a, E: EnvironmentPort> {
    env: &'a E,
    registry: &'a RegistryService,
}

impl<'a, E: EnvironmentPort> MirStage<'a, E> {
    pub fn new(env: &'a E, registry: &'a RegistryService) -> Self {
        Self { env, registry }
    }
}

impl<'a, E: EnvironmentPort> PipelineStage for MirStage<'a, E> {
    type Input = Vec<HirDiscourse>;
    type Output = MirProgram;

    fn execute(&mut self, hir_discourses: Vec<HirDiscourse>) -> Result<MirProgram, OnuError> {
        let mir_lowering_service = MirLoweringService::new(self.env, self.registry);
        mir_lowering_service.lower_program(&hir_discourses)
    }
}
