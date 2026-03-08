/// Ọ̀nụ Errors: Domain Entities
///
/// This module defines the formal error types of the Ọ̀nụ system.
/// Errors are localized by Span (Line/Column).
///
/// In addition to the fail-fast `OnuError` used by the core pipeline, this
/// module exposes a rich `Diagnostic` entity suitable for LSP consumers.  A
/// `Diagnostic` records a `Severity`, a `Span` with start *and* end positions,
/// a human-readable `message`, and an optional machine-readable `hint` that
/// an IDE can use to offer a code action.

// ---------------------------------------------------------------------------
// Span
// ---------------------------------------------------------------------------

/// Source location, either a point (`end == start`) or a range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
    /// Inclusive end line (defaults to `line`).
    pub end_line: usize,
    /// Inclusive end column (defaults to `column`).
    pub end_column: usize,
}

impl Default for Span {
    fn default() -> Self {
        Self { line: 0, column: 0, end_line: 0, end_column: 0 }
    }
}

impl Span {
    /// Construct a point span (zero-width) at `(line, column)`.
    pub fn point(line: usize, column: usize) -> Self {
        Self { line, column, end_line: line, end_column: column }
    }

    /// Construct a range span.
    pub fn range(line: usize, column: usize, end_line: usize, end_column: usize) -> Self {
        Self { line, column, end_line, end_column }
    }
}

// ---------------------------------------------------------------------------
// Diagnostic — LSP-ready rich diagnostic
// ---------------------------------------------------------------------------

/// Diagnostic severity level, ordered from most to least critical.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
    Hint,
}

/// A rich, non-fatal diagnostic suitable for LSP consumers and IDE tooling.
///
/// Unlike `OnuError`, a `Diagnostic` does not abort the pipeline; it is
/// collected into a `Vec<Diagnostic>` and reported after parsing / analysis
/// is complete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub span: Span,
    pub message: String,
    /// Optional IDE hint / quick-fix suggestion.
    pub actionable_hint: Option<String>,
}

impl Diagnostic {
    pub fn error(span: Span, message: impl Into<String>) -> Self {
        Self { severity: Severity::Error, span, message: message.into(), actionable_hint: None }
    }

    pub fn warning(span: Span, message: impl Into<String>) -> Self {
        Self { severity: Severity::Warning, span, message: message.into(), actionable_hint: None }
    }

    pub fn hint(span: Span, message: impl Into<String>) -> Self {
        Self { severity: Severity::Hint, span, message: message.into(), actionable_hint: None }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.actionable_hint = Some(hint.into());
        self
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
