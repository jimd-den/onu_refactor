/// Ọ̀nụ Core Errors: Domain Failure States
///
/// Clean Architecture specifies that errors are domain-specific
/// and should describe "what went wrong" in terms of business logic.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OnuError {
    LexicalViolation { message: String, span: Span },
    GrammarViolation { message: String, span: Span },
    RuntimeViolation { message: String, span: Span },
    BehaviorConflict { name: String, other_name: String },
    MonomorphizationViolation { message: String },
    ResourceViolation { message: String, span: Span },
    RealizationViolation { message: String },
}

impl From<String> for OnuError {
    fn from(message: String) -> Self {
        OnuError::RealizationViolation { message }
    }
}
