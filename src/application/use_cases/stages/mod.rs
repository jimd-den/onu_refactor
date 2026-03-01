use crate::domain::entities::error::OnuError;

pub mod lex_stage;
pub mod parse_stage;
pub mod hir_stage;
pub mod mir_stage;
pub mod codegen_stage;
pub mod realization_stage;

pub trait PipelineStage {
    type Input;
    type Output;
    fn execute(&mut self, input: Self::Input) -> Result<Self::Output, OnuError>;
}
