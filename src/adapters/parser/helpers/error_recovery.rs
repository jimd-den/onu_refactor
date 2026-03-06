/// Parser Error Recovery: Helper for fault-tolerant parsing.
///
/// This module provides the synchronization strategy used by the
/// fault-tolerant `OnuParser::parse_tolerant()` path.  When the parser
/// encounters an unexpected token it records a `Diagnostic` and calls
/// `synchronize()` to skip tokens until it reaches a safe restart point
/// (the beginning of the next top-level discourse declaration).
///
/// This keeps `ParserInternal` free of recovery-specific logic and
/// preserves the Anti-God-Class constraint.

use crate::application::ports::compiler_ports::Token;
use crate::domain::entities::error::{Diagnostic, Severity, Span};

/// The set of tokens that introduce a new top-level discourse unit.
/// After a syntax error the parser skips tokens until it sees one of
/// these (or EOF), which gives it a known-good restart point.
const DISCOURSE_STARTERS: &[fn(&Token) -> bool] = &[
    |t| matches!(t, Token::TheModuleCalled),
    |t| matches!(t, Token::TheShapeCalled),
    |t| matches!(t, Token::TheBehaviorCalled),
    |t| matches!(t, Token::TheEffectBehaviorCalled),
];

/// Advance `pos` past tokens until we reach a discourse-starter or EOF.
///
/// Returns the new position.
pub fn synchronize(tokens: &[Token], mut pos: usize) -> usize {
    while pos < tokens.len() {
        if is_discourse_starter(&tokens[pos]) {
            return pos;
        }
        pos += 1;
    }
    pos
}

/// Returns `true` if `token` can begin a new top-level discourse unit.
pub fn is_discourse_starter(token: &Token) -> bool {
    DISCOURSE_STARTERS.iter().any(|pred| pred(token))
}

/// Convert an `OnuError` (fail-fast) into a `Diagnostic` (non-fatal) so
/// it can be collected without aborting the parse.
pub fn error_to_diagnostic(err: &crate::domain::entities::error::OnuError) -> Diagnostic {
    use crate::domain::entities::error::OnuError;
    let (message, span) = match err {
        OnuError::GrammarViolation { message, span } => (message.clone(), span.clone()),
        OnuError::ResourceViolation { message, span } => (message.clone(), span.clone()),
        OnuError::AgencyViolation { message, span } => (message.clone(), span.clone()),
        OnuError::OwnershipViolation { message, span } => (message.clone(), span.clone()),
        OnuError::BehaviorConflict { message, span } => (message.clone(), span.clone()),
        OnuError::MonomorphizationError { message } => (message.clone(), Span::default()),
        OnuError::CodeGenError { message } => (message.clone(), Span::default()),
    };
    Diagnostic {
        severity: Severity::Error,
        span,
        message,
        actionable_hint: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synchronize_skips_to_next_discourse_starter() {
        let tokens = vec![
            Token::Identifier("bad".to_string()),
            Token::Identifier("tokens".to_string()),
            Token::TheBehaviorCalled,
            Token::Identifier("foo".to_string()),
        ];
        let new_pos = synchronize(&tokens, 0);
        assert_eq!(new_pos, 2, "Should skip to TheBehaviorCalled at index 2");
    }

    #[test]
    fn test_synchronize_reaches_eof_when_no_starter() {
        let tokens = vec![
            Token::Identifier("x".to_string()),
            Token::Identifier("y".to_string()),
        ];
        let new_pos = synchronize(&tokens, 0);
        assert_eq!(new_pos, tokens.len(), "Should advance to EOF");
    }

    #[test]
    fn test_is_discourse_starter() {
        assert!(is_discourse_starter(&Token::TheModuleCalled));
        assert!(is_discourse_starter(&Token::TheBehaviorCalled));
        assert!(is_discourse_starter(&Token::TheEffectBehaviorCalled));
        assert!(is_discourse_starter(&Token::TheShapeCalled));
        assert!(!is_discourse_starter(&Token::Identifier("x".to_string())));
    }
}
