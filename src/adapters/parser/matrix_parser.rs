/// Matrix Parser: Facade component for parsing matrix literals.
///
/// This module exposes a pure function that parses `[row; row; ...]` syntax
/// from a token slice, returning an `Expression::Matrix` and the number of
/// tokens consumed.  The main parser delegates here when it encounters `[`,
/// keeping the `ParserInternal` struct free of matrix-specific logic.

use crate::application::ports::compiler_ports::{Token, Literal};
use crate::domain::entities::ast::Expression;
use crate::domain::entities::error::{OnuError, Span};

/// Parse a matrix literal from a token slice that begins with `[`.
///
/// Grammar:
/// ```text
/// matrix   := '[' row (';' row)* ']'
/// row      := expr (',' expr)*
/// expr     := integer_literal | float_literal
/// ```
///
/// Returns `(Expression::Matrix { rows, cols, data }, tokens_consumed)`.
///
/// # Errors
/// Returns `OnuError::GrammarViolation` if the token stream is malformed or
/// the matrix is not rectangular.
pub fn parse_matrix(tokens: &[Token]) -> Result<(Expression, usize), OnuError> {
    let mut pos = 0;

    // Expect opening `[`
    match tokens.get(pos) {
        Some(Token::Delimiter('[')) => pos += 1,
        other => {
            return Err(OnuError::GrammarViolation {
                message: format!(
                    "Matrix literal must begin with '[', found {:?}",
                    other
                ),
                span: Span::default(),
            })
        }
    }

    let mut rows: Vec<Vec<Expression>> = Vec::new();
    let mut current_row: Vec<Expression> = Vec::new();

    // State machine: alternates between "expecting a value" and "expecting a
    // separator or closing bracket".  This rejects leading, trailing, and
    // consecutive separators at the point they occur rather than silently
    // producing a mis-shaped matrix or a confusing rectangularity error.
    let mut expect_value = true;

    loop {
        match tokens.get(pos) {
            // Closing bracket: validate state, flush current row, and finish.
            Some(Token::Delimiter(']')) => {
                pos += 1;
                // A trailing separator (e.g. `[1,2,]` or `[1;]`) means we
                // arrived here still expecting a value.
                if expect_value && (!current_row.is_empty() || !rows.is_empty()) {
                    return Err(OnuError::GrammarViolation {
                        message: "Trailing separator in matrix literal: \
                                  each separator must be followed by a value"
                            .to_string(),
                        span: Span::default(),
                    });
                }
                if !current_row.is_empty() {
                    rows.push(current_row);
                }
                break;
            }
            // Row separator `;`: only valid when a value was just parsed.
            Some(Token::Delimiter(';')) => {
                if expect_value {
                    return Err(OnuError::GrammarViolation {
                        message: "Unexpected ';' in matrix literal: \
                                  expected a value before ';'"
                            .to_string(),
                        span: Span::default(),
                    });
                }
                pos += 1;
                rows.push(std::mem::replace(&mut current_row, Vec::new()));
                expect_value = true;
            }
            // Column separator `,`: only valid when a value was just parsed.
            Some(Token::Delimiter(',')) => {
                if expect_value {
                    return Err(OnuError::GrammarViolation {
                        message: "Unexpected ',' in matrix literal: \
                                  expected a value before ','"
                            .to_string(),
                        span: Span::default(),
                    });
                }
                pos += 1;
                expect_value = true;
            }
            // Integer element: only valid when a value is expected.
            Some(Token::Literal(Literal::Integer(n))) => {
                if !expect_value {
                    return Err(OnuError::GrammarViolation {
                        message: "Unexpected integer in matrix literal: \
                                  expected ',' or ';' or ']' after value"
                            .to_string(),
                        span: Span::default(),
                    });
                }
                let n = *n;
                let expr = i64::try_from(n)
                    .map(Expression::I64)
                    .unwrap_or_else(|_| Expression::I128(n));
                current_row.push(expr);
                pos += 1;
                expect_value = false;
            }
            // Float element: only valid when a value is expected.
            Some(Token::Literal(Literal::FloatBits(bits))) => {
                if !expect_value {
                    return Err(OnuError::GrammarViolation {
                        message: "Unexpected float in matrix literal: \
                                  expected ',' or ';' or ']' after value"
                            .to_string(),
                        span: Span::default(),
                    });
                }
                let bits = *bits;
                current_row.push(Expression::F64(bits));
                pos += 1;
                expect_value = false;
            }
            None => {
                return Err(OnuError::GrammarViolation {
                    message: "Unterminated matrix literal: expected ']'".to_string(),
                    span: Span::default(),
                })
            }
            Some(other) => {
                return Err(OnuError::GrammarViolation {
                    message: format!(
                        "Unexpected token in matrix literal: {:?}",
                        other
                    ),
                    span: Span::default(),
                })
            }
        }
    }

    // Validate rectangularity.
    let row_count = rows.len();
    let col_count = rows.first().map(|r| r.len()).unwrap_or(0);
    for (i, row) in rows.iter().enumerate() {
        if row.len() != col_count {
            return Err(OnuError::GrammarViolation {
                message: format!(
                    "Matrix is not rectangular: row 0 has {} columns but row {} has {}",
                    col_count,
                    i,
                    row.len()
                ),
                span: Span::default(),
            });
        }
    }

    let data: Vec<Expression> = rows.into_iter().flatten().collect();
    Ok((Expression::Matrix { rows: row_count, cols: col_count, data }, pos))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(n: i64) -> Token {
        Token::Literal(Literal::Integer(n as i128))
    }

    #[test]
    fn test_parse_2x2_matrix() {
        let tokens = vec![
            Token::Delimiter('['),
            int(1), Token::Delimiter(','), int(2),
            Token::Delimiter(';'),
            int(3), Token::Delimiter(','), int(4),
            Token::Delimiter(']'),
        ];

        let (expr, consumed) = parse_matrix(&tokens).unwrap();
        assert_eq!(consumed, tokens.len());

        if let Expression::Matrix { rows, cols, data } = expr {
            assert_eq!(rows, 2);
            assert_eq!(cols, 2);
            assert_eq!(data, vec![
                Expression::I64(1), Expression::I64(2),
                Expression::I64(3), Expression::I64(4),
            ]);
        } else {
            panic!("Expected Matrix expression");
        }
    }

    #[test]
    fn test_parse_1x3_row_vector() {
        let tokens = vec![
            Token::Delimiter('['),
            int(10), Token::Delimiter(','), int(20), Token::Delimiter(','), int(30),
            Token::Delimiter(']'),
        ];

        let (expr, consumed) = parse_matrix(&tokens).unwrap();
        assert_eq!(consumed, tokens.len());

        if let Expression::Matrix { rows, cols, .. } = expr {
            assert_eq!(rows, 1);
            assert_eq!(cols, 3);
        } else {
            panic!("Expected Matrix expression");
        }
    }

    #[test]
    fn test_non_rectangular_matrix_fails() {
        let tokens = vec![
            Token::Delimiter('['),
            int(1), Token::Delimiter(','), int(2),
            Token::Delimiter(';'),
            int(3),
            Token::Delimiter(']'),
        ];
        assert!(parse_matrix(&tokens).is_err());
    }

    #[test]
    fn test_double_comma_rejected() {
        // [1,,2] must be a deterministic error, not accepted silently.
        let tokens = vec![
            Token::Delimiter('['),
            int(1), Token::Delimiter(','), Token::Delimiter(','), int(2),
            Token::Delimiter(']'),
        ];
        match parse_matrix(&tokens) {
            Err(OnuError::GrammarViolation { message, .. }) => {
                assert!(message.contains("','"), "error message should mention ',': {}", message);
            }
            other => panic!("Expected GrammarViolation, got {:?}", other),
        }
    }

    #[test]
    fn test_comma_then_semicolon_rejected() {
        // [1,;2] — a comma immediately followed by a semicolon is invalid.
        let tokens = vec![
            Token::Delimiter('['),
            int(1), Token::Delimiter(','), Token::Delimiter(';'), int(2),
            Token::Delimiter(']'),
        ];
        assert!(parse_matrix(&tokens).is_err());
    }

    #[test]
    fn test_leading_semicolon_rejected() {
        // [;] — leading semicolon is invalid.
        let tokens = vec![
            Token::Delimiter('['),
            Token::Delimiter(';'),
            Token::Delimiter(']'),
        ];
        assert!(parse_matrix(&tokens).is_err());
    }

    #[test]
    fn test_trailing_comma_rejected() {
        // [1,2,] — trailing comma is invalid.
        let tokens = vec![
            Token::Delimiter('['),
            int(1), Token::Delimiter(','), int(2), Token::Delimiter(','),
            Token::Delimiter(']'),
        ];
        assert!(parse_matrix(&tokens).is_err());
    }

    #[test]
    fn test_trailing_semicolon_rejected() {
        // [1,2;] — trailing semicolon is invalid.
        let tokens = vec![
            Token::Delimiter('['),
            int(1), Token::Delimiter(','), int(2), Token::Delimiter(';'),
            Token::Delimiter(']'),
        ];
        assert!(parse_matrix(&tokens).is_err());
    }

    #[test]
    fn test_empty_matrix_accepted() {
        // [] — empty matrix is valid.
        let tokens = vec![
            Token::Delimiter('['),
            Token::Delimiter(']'),
        ];
        let (expr, consumed) = parse_matrix(&tokens).unwrap();
        assert_eq!(consumed, 2);
        if let Expression::Matrix { rows, cols, data } = expr {
            assert_eq!(rows, 0);
            assert_eq!(cols, 0);
            assert!(data.is_empty());
        } else {
            panic!("Expected Matrix expression");
        }
    }
}
