/// Ọ̀nụ Parser Adapter: Structural Discourse Implementation
///
/// This implements the ParserPort by consuming a sequence of Tokens
/// and building the Domain-level AST (Discourse and Expressions).

use crate::application::ports::compiler_ports::{ParserPort, Token, Literal};
use crate::domain::entities::error::{OnuError, Span};
use crate::domain::entities::ast::{Discourse, Expression, BehaviorHeader, ReturnType, Argument, TypeInfo};
use crate::domain::entities::types::OnuType;

pub struct OnuParser;

impl ParserPort for OnuParser {
    fn parse(&self, tokens: Vec<Token>) -> Result<Vec<Discourse>, OnuError> {
        let mut parser = ParserInternal::new(tokens);
        let mut discourses = Vec::new();

        while !parser.is_at_end() {
            if let Some(d) = parser.parse_discourse()? {
                discourses.push(d);
            } else {
                break;
            }
        }

        Ok(discourses)
    }
}

struct ParserInternal {
    tokens: Vec<Token>,
    pos: usize,
}

impl ParserInternal {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&Token> {
        if !self.is_at_end() {
            self.pos += 1;
            self.tokens.get(self.pos - 1)
        } else {
            None
        }
    }

    fn match_id(&mut self, id: &str) -> bool {
        if let Some(Token::Identifier(s)) = self.peek() {
            if s == id {
                self.advance();
                return true;
            }
        }
        false
    }

    fn match_token(&mut self, token: Token) -> bool {
        if let Some(t) = self.peek() {
            if *t == token {
                self.advance();
                return true;
            }
        }
        false
    }

    fn parse_discourse(&mut self) -> Result<Option<Discourse>, OnuError> {
        while let Some(t) = self.peek() {
            match t {
                Token::Identifier(s) if s == "the-module-called" => return Ok(Some(self.parse_module()?)),
                Token::Identifier(s) if s == "the-behavior-called" || s == "the-effect-behavior-called" => return Ok(Some(self.parse_behavior(s == "the-effect-behavior-called")?)),
                _ => { self.advance(); }
            }
        }
        Ok(None)
    }

    fn parse_module(&mut self) -> Result<Discourse, OnuError> {
        self.advance(); 
        if let Some(Token::Identifier(name)) = self.advance() {
            let name = name.clone();
            Ok(Discourse::Module { name, concern: "General".to_string() })
        } else {
            Err(OnuError::GrammarViolation { message: "Expected module name".to_string(), span: Span::default() })
        }
    }

    fn parse_behavior(&mut self, is_effect: bool) -> Result<Discourse, OnuError> {
        self.advance(); 
        let name = if let Some(Token::Identifier(n)) = self.advance() {
            n.clone()
        } else {
            return Err(OnuError::GrammarViolation { message: "Expected behavior name".to_string(), span: Span::default() });
        };

        let mut takes = Vec::new();
        let mut delivers = ReturnType(OnuType::Nothing);

        while let Some(token) = self.peek() {
            let token = token.clone();
            match token {
                Token::Identifier(s) if s == "takes" => {
                    self.advance();
                    self.match_id(":");
                    takes = self.parse_arguments()?;
                }
                Token::Identifier(s) if s == "delivers" => {
                    self.advance();
                    self.match_id(":");
                    delivers = self.parse_return_type()?;
                }
                Token::Identifier(s) if s == "as" => break,
                _ => { self.advance(); }
            }
        }
        
        self.match_id("as");
        self.match_id(":");

        let body = self.parse_block()?;

        Ok(Discourse::Behavior { header: BehaviorHeader { name, is_effect, intent: "Standard".to_string(), takes, delivers, diminishing: None, skip_termination_check: false }, body })
    }

    fn parse_block(&mut self) -> Result<Expression, OnuError> {
        let indented = self.match_token(Token::Indent);
        let mut expressions = Vec::new();

        while !self.is_at_end() {
            if let Some(token) = self.peek() {
                if matches!(token, Token::Dedent) {
                    self.advance();
                    break;
                }
                if let Token::Identifier(s) = token {
                    if s == "the-module-called" || s == "the-behavior-called" || s == "the-effect-behavior-called" { break; }
                }
            }
            expressions.push(self.parse_expression()?);
            if !indented { break; }
        }

        if expressions.len() == 1 {
            Ok(expressions.pop().unwrap())
        } else if expressions.is_empty() {
            Ok(Expression::Nothing)
        } else {
            Ok(Expression::Block(expressions))
        }
    }

    fn parse_arguments(&mut self) -> Result<Vec<Argument>, OnuError> {
        let mut args = Vec::new();
        while let Some(token) = self.peek() {
            let token = token.clone();
            match token {
                Token::Identifier(s) if s == "delivers" || s == "as" => break,
                Token::Identifier(s) if s == "an" || s == "a" => {
                    self.advance();
                    let typ_name = if let Some(Token::Identifier(t)) = self.advance() { t.clone() } else { "".to_string() };
                    self.match_id("called");
                    let name = if let Some(Token::Identifier(n)) = self.advance() { n.clone() } else { "".to_string() };
                    args.push(Argument { name, type_info: TypeInfo { onu_type: OnuType::from_name(&typ_name).unwrap_or(OnuType::I64), display_name: typ_name, via_role: None, is_observation: false } });
                }
                _ => { self.advance(); }
            }
        }
        Ok(args)
    }

    fn parse_return_type(&mut self) -> Result<ReturnType, OnuError> {
        if let Some(Token::Identifier(s)) = self.advance() {
            let s = s.clone();
            Ok(ReturnType(OnuType::from_name(&s).unwrap_or(OnuType::Nothing)))
        } else {
            Ok(ReturnType(OnuType::Nothing))
        }
    }

    fn parse_expression(&mut self) -> Result<Expression, OnuError> {
        self.parse_infix(0)
    }

    fn parse_primary(&mut self) -> Result<Expression, OnuError> {
        if self.is_at_end() { return Ok(Expression::Nothing); }
        let token = self.peek().unwrap().clone();

        match token {
            Token::Literal(Literal::Integer(i)) => { self.advance(); Ok(Expression::I128(i)) },
            Token::Literal(Literal::FloatBits(f)) => { self.advance(); Ok(Expression::F64(f)) },
            Token::Literal(Literal::String(s)) => { self.advance(); Ok(Expression::Text(s)) },
            Token::Literal(Literal::Boolean(b)) => { self.advance(); Ok(Expression::Boolean(b)) },
            Token::Identifier(s) if s == "if" => { self.advance(); self.parse_if() },
            Token::Identifier(s) if s == "derivation" => { self.advance(); self.parse_derivation() },
            Token::Identifier(s) => {
                let name = s.clone();
                self.advance();
                Ok(Expression::Identifier(name))
            },
            Token::Keyword(s) if s == "nothing" => { self.advance(); Ok(Expression::Nothing) },
            Token::Delimiter('(') => {
                self.advance();
                let inner = self.parse_expression()?;
                self.match_id(")");
                Ok(inner)
            },
            Token::Indent => { 
                self.advance(); 
                let res = self.parse_block()?; 
                self.match_token(Token::Dedent); 
                Ok(res) 
            }
            _ => { self.advance(); Ok(Expression::Nothing) }
        }
    }

    fn parse_infix(&mut self, min_precedence: u8) -> Result<Expression, OnuError> {
        let mut lhs = self.parse_primary()?;

        while let Some(Token::Identifier(op)) = self.peek() {
            let op = op.clone();
            let precedence = match op.as_str() {
                "broadcasts" => 1,
                "matches" | "exceeds" | "falls-short-of" => 2,
                "added-to" | "decreased-by" => 3,
                "scales-by" | "partitions-by" => 4,
                "utilizes" => 5,
                _ => break,
            };

            if precedence < min_precedence { break; }
            self.advance();

            if op == "utilizes" {
                if let Some(Token::Identifier(target)) = self.advance() {
                    let target = target.clone();
                    let mut args = vec![lhs];
                    // Peek if next token is NOT an operator or delimiter to consume as extra arg
                    if let Some(t) = self.peek() {
                        if !matches!(t, Token::Delimiter(')') | Token::Dedent | Token::Identifier(_)) {
                             args.push(self.parse_expression()?);
                        } else if let Token::Identifier(next_op) = t {
                             // If it's a known op, don't consume it as an arg
                             if !["matches", "exceeds", "falls-short-of", "added-to", "decreased-by", "scales-by", "partitions-by", "utilizes", "broadcasts"].contains(&next_op.as_str()) {
                                 args.push(self.parse_expression()?);
                             }
                        }
                    }
                    lhs = Expression::BehaviorCall { name: target, args };
                } else {
                    return Err(OnuError::GrammarViolation { message: "Expected behavior name after utilizes".to_string(), span: Span::default() });
                }
            } else if op == "broadcasts" {
                let rhs = self.parse_infix(precedence)?;
                lhs = Expression::Emit(Box::new(rhs));
            } else {
                let rhs = self.parse_infix(precedence + 1)?;
                lhs = Expression::BehaviorCall { name: op, args: vec![lhs, rhs] };
            }
        }

        Ok(lhs)
    }

    fn parse_if(&mut self) -> Result<Expression, OnuError> {
        let condition = self.parse_expression()?;
        self.match_id("then");
        let then_branch = self.parse_expression()?;
        self.match_id("else");
        let else_branch = self.parse_expression()?;
        Ok(Expression::If { condition: Box::new(condition), then_branch: Box::new(then_branch), else_branch: Box::new(else_branch) })
    }

    fn parse_derivation(&mut self) -> Result<Expression, OnuError> {
        if let Some(Token::Identifier(n)) = self.advance() {
            let name = n.clone();
            self.match_id(":");
            self.match_id("derives-from");
            let value = self.parse_expression()?;
            // Look for body: either indented block or next expression
            let body = self.parse_expression()?;
            return Ok(Expression::Derivation { name, type_info: None, value: Box::new(value), body: Box::new(body) });
        }
        Ok(Expression::Nothing)
    }
}
