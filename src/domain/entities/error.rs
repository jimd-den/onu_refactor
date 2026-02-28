/// Ọ̀nụ Errors: Domain Entities
///
/// This module defines the formal error types of the Ọ̀nụ system.
/// Errors are localized by Span (Line/Column).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

impl Default for Span {
    fn default() -> Self {
        Self { line: 0, column: 0 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OnuError {
    GrammarViolation { message: String, span: Span },
    ResourceViolation { message: String, span: Span },
    AgencyViolation { message: String, span: Span },
    MonomorphizationError { message: String },
    CodeGenError { message: String },
    OwnershipViolation { message: String, span: Span },
    BehaviorConflict { message: String, span: Span },
}

impl From<String> for OnuError {
    fn from(message: String) -> Self {
        OnuError::ResourceViolation { message, span: Span::default() }
    }
}
