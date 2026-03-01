use crate::application::options::LogLevel;
use crate::application::ports::environment::EnvironmentPort;
use crate::domain::entities::error::OnuError;
use super::PipelineStage;

pub struct RealizationStage<'a, E: EnvironmentPort> {
    env: &'a E,
}

impl<'a, E: EnvironmentPort> RealizationStage<'a, E> {
    pub fn new(env: &'a E) -> Self {
        Self { env }
    }
}

impl<'a, E: EnvironmentPort> PipelineStage for RealizationStage<'a, E> {
    type Input = (String, String); // (bitcode_path, output_path)
    type Output = ();

    fn execute(&mut self, (bitcode_path, output_path): (String, String)) -> Result<(), OnuError> {
        self.env.log(LogLevel::Info, &format!("Realizing binary: {} -> {}", bitcode_path, output_path));
        // Link bitcode natively
        self.env.run_command("clang", &[&bitcode_path, "-O3", "-o", &output_path, "-Wno-override-module"])?;
        Ok(())
    }
}
