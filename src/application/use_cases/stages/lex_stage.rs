use crate::application::ports::compiler_ports::{LexerPort, Token};
use crate::domain::entities::error::OnuError;
use super::PipelineStage;

pub struct LexStage {
    lexer: Box<dyn LexerPort>,
}

impl LexStage {
    pub fn new(lexer: Box<dyn LexerPort>) -> Self {
        Self { lexer }
    }
}

impl PipelineStage for LexStage {
    type Input = String;
    type Output = Vec<Token>;

    fn execute(&mut self, source: String) -> Result<Vec<Token>, OnuError> {
        self.lexer.lex(&source)
    }
}
