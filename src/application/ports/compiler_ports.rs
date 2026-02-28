/// Compiler Ports: Application Layer Interfaces
///
/// These traits define the required behavior for the compilation pipeline stages.
/// Concrete adapters (e.g., Lexer, Parser) must implement these interfaces.

use crate::domain::entities::error::OnuError;
use crate::domain::entities::ast::Discourse;
use crate::domain::entities::mir::MirProgram;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token {
    Identifier(String),
    Literal(Literal),
    Operator(String),
    Delimiter(char),
    Indent,
    Dedent,
    LineStart(usize), // Indentation level

    // --- Keywords ---
    TheModuleCalled,
    TheBehaviorCalled,
    TheEffectBehaviorCalled,
    WithConcern,
    WithIntent,
    WithDiminishing,
    NoGuaranteedTermination,
    DerivesFrom,
    DecreasedBy,
    PartitionsBy,
    ScalesBy,
    AddedTo,
    Utilizes,
    As,
    Takes,
    Delivers,
    Called,
    If,
    Then,
    Else,
    Derivation,
    Broadcasts,
    Nothing,
    Matches,
    Exceeds,
    FallsShortOf,
    UnitesWith,
    JoinsWith,
    Opposes,
    InitOf,
    TailOf,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Literal {
    Integer(i128),
    FloatBits(u64),
    String(String),
    Boolean(bool),
}

pub trait LexerPort {
    fn lex(&self, source: &str) -> Result<Vec<Token>, OnuError>;
}

pub trait ParserPort {
    fn parse(&self, tokens: Vec<Token>) -> Result<Vec<Discourse>, OnuError>;
}

pub trait CodegenPort {
    fn generate(&self, program: &MirProgram) -> Result<String, OnuError>;
    fn set_registry(&mut self, registry: crate::application::use_cases::registry_service::RegistryService);
}

pub trait ExtensionPort: crate::domain::entities::registry::BuiltInModule {
    fn realization_id(&self) -> &str;
}
