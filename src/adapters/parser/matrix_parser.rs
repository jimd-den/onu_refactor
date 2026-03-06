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

    loop {
        match tokens.get(pos) {
            // Closing bracket: flush current row and finish.
            Some(Token::Delimiter(']')) => {
                pos += 1;
                if !current_row.is_empty() {
                    rows.push(current_row);
                }
                break;
            }
            // Row separator: flush current row and start a new one.
            Some(Token::Delimiter(';')) => {
                pos += 1;
                rows.push(current_row);
                current_row = Vec::new();
            }
            // Column separator: just advance.
            Some(Token::Delimiter(',')) => {
                pos += 1;
            }
            // Integer element.
            Some(Token::Literal(Literal::Integer(n))) => {
                let n = *n;
                let expr = i64::try_from(n)
                    .map(Expression::I64)
                    .unwrap_or_else(|_| Expression::I128(n));
                current_row.push(expr);
                pos += 1;
            }
            // Float element.
            Some(Token::Literal(Literal::FloatBits(bits))) => {
                let bits = *bits;
                current_row.push(Expression::F64(bits));
                pos += 1;
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
}
