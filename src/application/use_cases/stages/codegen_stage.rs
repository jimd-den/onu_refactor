use crate::application::options::LogLevel;
use crate::application::ports::compiler_ports::CodegenPort;
use crate::application::ports::environment::EnvironmentPort;
use crate::application::use_cases::registry_service::RegistryService;
use crate::domain::entities::mir::MirProgram;
use crate::domain::entities::error::OnuError;
use super::PipelineStage;

pub struct CodegenStage<'a, E: EnvironmentPort, C: CodegenPort> {
    env: &'a E,
    codegen: &'a mut C,
    registry: RegistryService,
}

impl<'a, E: EnvironmentPort, C: CodegenPort> CodegenStage<'a, E, C> {
    pub fn new(env: &'a E, codegen: &'a mut C, registry: RegistryService) -> Self {
        Self { env, codegen, registry }
    }
}

impl<'a, E: EnvironmentPort, C: CodegenPort> PipelineStage for CodegenStage<'a, E, C> {
    type Input = MirProgram;
    type Output = String;

    fn execute(&mut self, mir: MirProgram) -> Result<String, OnuError> {
        self.env.log(LogLevel::Info, "Starting Codegen stage.");
        self.codegen.set_registry(self.registry.clone());
        let ir = self.codegen.generate(&mir)?;
        self.env.log(LogLevel::Debug, &format!("Generated LLVM IR:\n{}", ir));
        Ok(ir)
    }
}
