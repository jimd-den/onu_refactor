/// SVO Parser: Facade component for Subject-Verb-Object I/O syntax.
///
/// This module handles the English-flavoured inline I/O statements supported
/// by Ọ̀nụ, keeping the main `ParserInternal` struct free of SVO-specific
/// branching.
///
/// Supported grammar (parsed as pure functions over token slices):
/// ```text
/// svo_write := 'write' simple_expr 'to' destination
/// svo_read  := 'read'  name        'from' source
///
/// simple_expr  := identifier | integer_literal | float_literal | string_literal
/// destination  := 'console' | identifier
/// source       := 'console' | identifier
/// ```
///
/// `write <expr> to console`  →  `Expression::Emit(expr)`
/// `read  <name> from console` →  `Expression::BehaviorCall { name: "receives-line", args: [] }`
///   (the caller is expected to bind the result to `<name>` via a Derivation)

use crate::application::ports::compiler_ports::{Token, Literal};
use crate::domain::entities::ast::Expression;
use crate::domain::entities::error::{OnuError, Span};

/// Attempt to parse a `write <expr> to <dest>` statement.
///
/// `tokens` must start immediately **after** the `write` keyword has been
/// consumed by the caller.
///
/// Returns `(Expression::Emit(inner), tokens_consumed)` where
/// `tokens_consumed` counts the tokens taken from `tokens` (not including the
/// already-consumed `write` keyword).
pub fn parse_write(tokens: &[Token]) -> Result<(Expression, usize), OnuError> {
    let mut pos = 0;

    // Parse the simple subject expression.
    let (subject, subject_consumed) = parse_simple_expr(tokens)?;
    pos += subject_consumed;

    // Expect `to`.
    match tokens.get(pos) {
        Some(Token::To) => pos += 1,
        other => {
            return Err(OnuError::GrammarViolation {
                message: format!(
                    "SVO 'write': expected 'to' after expression, found {:?}",
                    other
                ),
                span: Span::default(),
            })
        }
    }

    // Consume the destination (e.g. `console` or any identifier) — ignored at
    // the AST level; all destinations currently lower to stdout.
    match tokens.get(pos) {
        Some(Token::Identifier(_)) => pos += 1,
        other => {
            return Err(OnuError::GrammarViolation {
                message: format!(
                    "SVO 'write': expected destination identifier after 'to', found {:?}",
                    other
                ),
                span: Span::default(),
            })
        }
    }

    Ok((Expression::Emit(Box::new(subject)), pos))
}

/// Attempt to parse a `read <name> from <src>` statement.
///
/// `tokens` must start immediately **after** the `read` keyword has been
/// consumed by the caller.
///
/// Returns `(Expression::BehaviorCall { name: "receives-line", args: [] }, tokens_consumed)`.
/// The caller typically wraps this in a `Derivation` to bind the result to
/// the requested `<name>` variable.
pub fn parse_read(tokens: &[Token]) -> Result<(Expression, usize), OnuError> {
    let mut pos = 0;

    // Consume the bound variable name (informational at AST level).
    match tokens.get(pos) {
        Some(Token::Identifier(_)) => pos += 1,
        other => {
            return Err(OnuError::GrammarViolation {
                message: format!(
                    "SVO 'read': expected variable name, found {:?}",
                    other
                ),
                span: Span::default(),
            })
        }
    }

    // Expect `from`.
    match tokens.get(pos) {
        Some(Token::From) => pos += 1,
        other => {
            return Err(OnuError::GrammarViolation {
                message: format!(
                    "SVO 'read': expected 'from' after variable name, found {:?}",
                    other
                ),
                span: Span::default(),
            })
        }
    }

    // Consume the source identifier (e.g. `console`).
    match tokens.get(pos) {
        Some(Token::Identifier(_)) => pos += 1,
        other => {
            return Err(OnuError::GrammarViolation {
                message: format!(
                    "SVO 'read': expected source identifier after 'from', found {:?}",
                    other
                ),
                span: Span::default(),
            })
        }
    }

    // Map to the built-in `receives-line` call which reads a line from stdin.
    Ok((
        Expression::BehaviorCall {
            name: "receives-line".to_string(),
            args: vec![],
        },
        pos,
    ))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Parse a single "simple" expression: an identifier, or a numeric/string/
/// boolean literal.  Returns `(Expression, tokens_consumed)`.
fn parse_simple_expr(tokens: &[Token]) -> Result<(Expression, usize), OnuError> {
    match tokens.first() {
        Some(Token::Identifier(name)) => {
            Ok((Expression::Identifier(name.clone()), 1))
        }
        Some(Token::Literal(Literal::Integer(n))) => {
            let expr = i64::try_from(*n)
                .map(Expression::I64)
                .unwrap_or_else(|_| Expression::I128(*n));
            Ok((expr, 1))
        }
        Some(Token::Literal(Literal::FloatBits(bits))) => {
            Ok((Expression::F64(*bits), 1))
        }
        Some(Token::Literal(Literal::String(s))) => {
            Ok((Expression::Text(s.clone()), 1))
        }
        Some(Token::Literal(Literal::Boolean(b))) => {
            Ok((Expression::Boolean(*b), 1))
        }
        Some(Token::Nothing) => Ok((Expression::Nothing, 1)),
        other => Err(OnuError::GrammarViolation {
            message: format!(
                "SVO: expected a simple expression (identifier or literal), found {:?}",
                other
            ),
            span: Span::default(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_identifier_to_console() {
        let tokens = vec![
            Token::Identifier("result".to_string()),
            Token::To,
            Token::Identifier("console".to_string()),
        ];
        let (expr, consumed) = parse_write(&tokens).unwrap();
        assert_eq!(consumed, 3);
        assert!(matches!(expr, Expression::Emit(_)));
    }

    #[test]
    fn test_write_literal_to_console() {
        let tokens = vec![
            Token::Literal(Literal::Integer(42)),
            Token::To,
            Token::Identifier("console".to_string()),
        ];
        let (expr, consumed) = parse_write(&tokens).unwrap();
        assert_eq!(consumed, 3);
        if let Expression::Emit(inner) = expr {
            assert_eq!(*inner, Expression::I64(42));
        } else {
            panic!("Expected Emit expression");
        }
    }

    #[test]
    fn test_write_missing_to_fails() {
        let tokens = vec![
            Token::Identifier("x".to_string()),
            Token::Identifier("console".to_string()),
        ];
        assert!(parse_write(&tokens).is_err());
    }

    #[test]
    fn test_read_from_console() {
        let tokens = vec![
            Token::Identifier("line".to_string()),
            Token::From,
            Token::Identifier("console".to_string()),
        ];
        let (expr, consumed) = parse_read(&tokens).unwrap();
        assert_eq!(consumed, 3);
        if let Expression::BehaviorCall { name, args } = expr {
            assert_eq!(name, "receives-line");
            assert!(args.is_empty());
        } else {
            panic!("Expected BehaviorCall");
        }
    }
}
