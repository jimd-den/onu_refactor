use crate::application::options::LogLevel;
use crate::application::ports::compiler_ports::{LexerPort, Token};
use crate::adapters::lexer::OnuLexer;
use crate::domain::entities::error::OnuError;
use super::PipelineStage;

pub struct LexStage {
    lexer: OnuLexer,
}

impl LexStage {
    pub fn new(log_level: LogLevel) -> Self {
        Self {
            lexer: OnuLexer::new(log_level),
        }
    }
}

impl PipelineStage for LexStage {
    type Input = String;
    type Output = Vec<Token>;

    fn execute(&mut self, source: String) -> Result<Vec<Token>, OnuError> {
        self.lexer.lex(&source)
    }
}

// For convenience, also implement it for &str
impl<'a> PipelineStage for &'a mut LexStage {
    type Input = &'a str;
    type Output = Vec<Token>;

    fn execute(&mut self, source: &'a str) -> Result<Vec<Token>, OnuError> {
        self.lexer.lex(source)
    }
}
