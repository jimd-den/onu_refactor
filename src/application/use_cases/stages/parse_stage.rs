use crate::application::options::LogLevel;
use crate::application::ports::compiler_ports::{ParserPort, Token};
use crate::adapters::parser::OnuParser;
use crate::application::use_cases::registry_service::RegistryService;
use crate::domain::entities::ast::Discourse;
use crate::domain::entities::error::OnuError;
use super::PipelineStage;

pub struct ParseStage<'a> {
    parser: OnuParser,
    registry: &'a mut RegistryService,
}

impl<'a> ParseStage<'a> {
    pub fn new(registry: &'a mut RegistryService, log_level: LogLevel) -> Self {
        Self {
            parser: OnuParser::new(log_level),
            registry,
        }
    }
}

impl<'a> PipelineStage for ParseStage<'a> {
    type Input = Vec<Token>;
    type Output = Vec<Discourse>;

    fn execute(&mut self, tokens: Vec<Token>) -> Result<Vec<Discourse>, OnuError> {
        self.parser.scan_headers(&tokens, self.registry)?;
        self.parser.parse_with_registry(tokens, self.registry)
    }
}
