/// Ọ̀nụ Parser Adapter: Structural Discourse Implementation
///
/// This implements the ParserPort by consuming a sequence of Tokens
/// and building the Domain-level AST (Discourse and Expressions).

use crate::application::ports::compiler_ports::{ParserPort, Token, Literal};
use crate::application::options::LogLevel;
use crate::domain::entities::error::{OnuError, Span};
use crate::domain::entities::ast::{Discourse, Expression, BehaviorHeader, ReturnType, Argument, TypeInfo};
use crate::domain::entities::types::OnuType;
use chrono::Local;

pub struct OnuParser {
    pub log_level: LogLevel,
}

impl OnuParser {
    pub fn new(log_level: LogLevel) -> Self {
        Self { log_level }
    }

    fn log(&self, level: LogLevel, message: &str) {
        if level <= self.log_level && level != LogLevel::None {
            let timestamp = Local::now().to_rfc3339();
            eprintln!("[{}] {:?}: [Parser] {}", timestamp, level, message);
        }
    }
}

impl ParserPort for OnuParser {
    fn parse(&self, tokens: Vec<Token>) -> Result<Vec<Discourse>, OnuError> {
        self.log(LogLevel::Info, "Starting parsing process");
        let mut parser = ParserInternal::new(tokens, self.log_level);
        let mut discourses = Vec::new();

        while !parser.is_at_end() {
            if let Some(d) = parser.parse_discourse()? {
                self.log(LogLevel::Trace, &format!("Parsed discourse unit: {:?}", d));
                discourses.push(d);
            } else {
                break;
            }
        }

        self.log(LogLevel::Info, &format!("Parsing successful: {} discourse units", discourses.len()));
        Ok(discourses)
    }
}

struct ParserInternal {
    tokens: Vec<Token>,
    pos: usize,
    log_level: LogLevel,
}

impl ParserInternal {
    fn new(tokens: Vec<Token>, log_level: LogLevel) -> Self {
        Self { tokens, pos: 0, log_level }
    }

    fn log(&self, level: LogLevel, message: &str) {
        if level <= self.log_level && level != LogLevel::None {
            let timestamp = Local::now().to_rfc3339();
            eprintln!("[{}] {:?}: [ParserInternal] {}", timestamp, level, message);
        }
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

    fn consume(&mut self, expected: Token) -> Result<(), OnuError> {
        self.skip_layout();
        if let Some(t) = self.peek() {
            if *t == expected {
                self.advance();
                return Ok(());
            }
            return Err(OnuError::GrammarViolation { 
                message: format!("Expected {:?}, found {:?}", expected, t), 
                span: Span::default() 
            });
        }
        Err(OnuError::GrammarViolation { message: format!("Expected {:?}, found EOF", expected), span: Span::default() })
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

    fn is_expression_terminator(&self, token: &Token) -> bool {
        matches!(token, Token::Then | Token::Else | Token::Takes | Token::Delivers | Token::As | Token::WithConcern | Token::WithIntent | Token::TheModuleCalled | Token::TheBehaviorCalled | Token::TheEffectBehaviorCalled)
            || matches!(token, Token::Operator(s) if s == ":")
            || matches!(token, Token::Delimiter(')'))
    }

    fn skip_layout(&mut self) {
        while let Some(t) = self.peek() {
            if matches!(t, Token::Indent | Token::Dedent | Token::LineStart(_)) {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn parse_discourse(&mut self) -> Result<Option<Discourse>, OnuError> {
        while let Some(t) = self.peek() {
            match t {
                Token::TheModuleCalled => return Ok(Some(self.parse_module()?)),
                Token::TheBehaviorCalled | Token::TheEffectBehaviorCalled => return Ok(Some(self.parse_behavior()?)),
                _ => { 
                    self.advance(); 
                }
            }
        }
        Ok(None)
    }

    fn parse_module(&mut self) -> Result<Discourse, OnuError> {
        self.log(LogLevel::Debug, "Parsing module");
        self.consume(Token::TheModuleCalled)?;
        self.skip_layout();
        let name = if let Some(Token::Identifier(n)) = self.advance() { n.clone() } else { 
            return Err(OnuError::GrammarViolation { message: "Expected module name".into(), span: Span::default() });
        };
        
        self.skip_layout();
        self.consume(Token::WithConcern)?;
        self.consume(Token::Operator(":".to_string()))?;
        
        // Consume concern words
        let mut concern = String::new();
        while let Some(t) = self.peek() {
            if matches!(t, Token::TheModuleCalled | Token::TheBehaviorCalled | Token::TheEffectBehaviorCalled) { break; }
            if matches!(t, Token::Indent | Token::Dedent | Token::LineStart(_)) { 
                self.advance();
                continue;
            }
            if !concern.is_empty() { concern.push(' '); }
            let t_clone = self.advance().unwrap().clone();
            if let Token::Identifier(s) = t_clone {
                concern.push_str(&s.to_lowercase());
            } else {
                concern.push_str(&format!("{:?}", t_clone).to_lowercase());
            }
        }
        Ok(Discourse::Module { name, concern })
    }

    fn parse_behavior(&mut self) -> Result<Discourse, OnuError> {
        self.skip_layout();
        let is_effect = matches!(self.peek(), Some(Token::TheEffectBehaviorCalled));
        self.log(LogLevel::Debug, &format!("Parsing behavior (is_effect: {})", is_effect));
        self.advance(); // behavior keyword

        self.skip_layout();
        let name = if let Some(Token::Identifier(n)) = self.advance() { n.clone() } else {
            return Err(OnuError::GrammarViolation { message: "Expected behavior name".into(), span: Span::default() });
        };

        let mut takes = Vec::new();
        let mut delivers = ReturnType(OnuType::Nothing);
        let mut intent = String::new();

        while let Some(token) = self.peek() {
            match token {
                Token::WithIntent => {
                    self.advance();
                    self.match_token(Token::Operator(":".to_string()));
                    while let Some(t) = self.peek() {
                        if matches!(t, Token::Takes | Token::Delivers | Token::As | Token::Indent | Token::Dedent) { break; }
                        if !intent.is_empty() { intent.push(' '); }
                        let t_clone = self.advance().unwrap().clone();
                        if let Token::Identifier(s) = t_clone {
                            intent.push_str(&s.to_lowercase());
                        } else {
                            intent.push_str(&format!("{:?}", t_clone).to_lowercase());
                        }
                    }
                }
                Token::Takes => {
                    self.advance();
                    self.match_token(Token::Operator(":".to_string()));
                    takes = self.parse_arguments()?;
                }
                Token::Delivers => {
                    self.advance();
                    self.match_token(Token::Operator(":".to_string()));
                    delivers = self.parse_return_type()?;
                }
                Token::As => break,
                Token::Indent | Token::Dedent | Token::LineStart(_) | Token::NoGuaranteedTermination => { self.advance(); }
                _ => { self.advance(); }
            }
        }
        
        self.consume(Token::As)?;
        self.skip_layout();
        self.match_token(Token::Operator(":".to_string()));

        let body = self.parse_block()?;

        Ok(Discourse::Behavior { header: BehaviorHeader { name, is_effect, intent, takes, delivers, diminishing: None, skip_termination_check: false }, body })
    }

    fn parse_block(&mut self) -> Result<Expression, OnuError> {
        self.log(LogLevel::Trace, "Parsing block (ignoring layout)");
        let mut expressions = Vec::new();

        while !self.is_at_end() {
            if let Some(token) = self.peek() {
                if matches!(token, Token::TheModuleCalled | Token::TheBehaviorCalled | Token::TheEffectBehaviorCalled) { break; }
            }
            
            self.skip_layout();
            if self.is_at_end() { break; }

            let expr = self.parse_expression()?;
            if !matches!(expr, Expression::Nothing) {
                expressions.push(expr);
            }
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
            match token {
                Token::Delivers | Token::As | Token::Dedent => break,
                Token::Identifier(_) | Token::Nothing => {
                    if self.match_token(Token::Nothing) { break; }
                    let type_info = self.parse_type_info()?.unwrap_or(TypeInfo { onu_type: OnuType::I64, display_name: "i64".into(), via_role: None, is_observation: false });
                    self.match_token(Token::Called);
                    self.skip_layout();
                    let name = if let Some(Token::Identifier(n)) = self.advance() { n.clone() } else { "".to_string() };
                    args.push(Argument { name, type_info });
                }
                Token::Indent | Token::LineStart(_) => { self.advance(); }
                _ => { self.advance(); }
            }
        }
        Ok(args)
    }

    fn parse_type_info(&mut self) -> Result<Option<TypeInfo>, OnuError> {
        self.skip_layout();
        if let Some(Token::Identifier(s)) = self.peek() {
            if s == "a" || s == "an" || s == "the" {
                self.advance();
                self.skip_layout();
                let type_name = if let Some(Token::Identifier(tn)) = self.advance() { tn.clone() } else { "i64".into() };
                let onu_type = OnuType::from_name(&type_name).unwrap_or(OnuType::I64);
                return Ok(Some(TypeInfo { onu_type, display_name: type_name, via_role: None, is_observation: false }));
            }
        }
        Ok(None)
    }

    fn parse_return_type(&mut self) -> Result<ReturnType, OnuError> {
        let ti = self.parse_type_info()?.map(|ti| ti.onu_type).unwrap_or(OnuType::Nothing);
        Ok(ReturnType(ti))
    }

    fn parse_expression(&mut self) -> Result<Expression, OnuError> {
        self.skip_layout();
        if self.match_token(Token::Broadcasts) {
            return Ok(Expression::Emit(Box::new(self.parse_expression()?)));
        }
        self.parse_infix(0)
    }

    fn parse_primary(&mut self) -> Result<Expression, OnuError> {
        self.skip_layout();
        if self.is_at_end() { return Ok(Expression::Nothing); }
        let token = self.peek().unwrap().clone();
        
        match &token {
            Token::Literal(Literal::Integer(i)) => { self.advance(); Ok(Expression::I128(*i)) },
            Token::Literal(Literal::FloatBits(f)) => { self.advance(); Ok(Expression::F64(*f)) },
            Token::Literal(Literal::String(s)) => { self.advance(); Ok(Expression::Text(s.clone())) },
            Token::Literal(Literal::Boolean(b)) => { self.advance(); Ok(Expression::Boolean(*b)) },
            Token::If => self.parse_if(),
            Token::Derivation => self.parse_derivation(),
            Token::Identifier(s) => {
                if self.is_expression_terminator(&token) {
                    return Ok(Expression::Nothing);
                }
                let name = s.clone();
                self.advance();
                Ok(Expression::Identifier(name))
            },
            Token::Nothing => { self.advance(); Ok(Expression::Nothing) },
            Token::Delimiter('(') => {
                self.advance();
                let inner = self.parse_expression()?;
                self.consume(Token::Delimiter(')'))?;
                Ok(inner)
            },
            _ => { 
                Ok(Expression::Nothing) 
            }
        }
    }

    fn parse_infix(&mut self, min_precedence: u8) -> Result<Expression, OnuError> {
        let mut lhs = self.parse_primary()?;

        while let Some(token) = self.peek() {
            if self.is_expression_terminator(token) { break; }

            let (op_name, precedence) = match token {
                Token::Matches => ("matches".into(), 2),
                Token::Exceeds => ("exceeds".into(), 2),
                Token::FallsShortOf => ("falls-short-of".into(), 2),
                Token::AddedTo => ("added-to".into(), 3),
                Token::DecreasedBy => ("decreased-by".into(), 3),
                Token::ScalesBy => ("scales-by".into(), 4),
                Token::PartitionsBy => ("partitions-by".into(), 4),
                Token::UnitesWith => ("unites-with".into(), 4),
                Token::JoinsWith => ("joins-with".into(), 4),
                Token::Opposes => ("opposes".into(), 4),
                Token::InitOf => ("init-of".into(), 4),
                Token::TailOf => ("tail-of".into(), 4),
                Token::Utilizes => ("utilizes".into(), 5),
                Token::Identifier(s) => {
                    let p = match s.as_str() {
                        "matches" | "exceeds" | "falls-short-of" => 2,
                        "added-to" | "decreased-by" => 3,
                        "scales-by" | "partitions-by" => 4,
                        "joined-with" | "joins-with" | "unites-with" | "opposes" => 4,
                        "utilizes" => 5,
                        _ => break,
                    };
                    (s.clone(), p)
                }
                _ => break,
            };

            if precedence < min_precedence { break; }
            self.advance();

            if op_name == "utilizes" {
                let target_token = self.advance().cloned();
                if let Some(Token::Identifier(target)) = target_token {
                    let mut args = vec![lhs];
                    loop {
                        let t = match self.peek() {
                            Some(t) => t,
                            None => break,
                        };
                        if matches!(t, Token::LineStart(_)) {
                            break; // Do not consume arguments across lines
                        }
                        self.skip_layout();
                        let t = match self.peek() {
                            Some(t) => t,
                            None => break,
                        };
                        if self.is_expression_terminator(t) || matches!(t, Token::Derivation | Token::Broadcasts | Token::If) {
                            break;
                        }
                        let expr = self.parse_primary()?;
                        if !matches!(expr, Expression::Nothing) {
                            args.push(expr);
                        } else {
                            break;
                        }
                    }
                    lhs = Expression::BehaviorCall { name: target.clone(), args };
                }
            } else {
                let rhs = self.parse_infix(precedence + 1)?;
                lhs = Expression::BehaviorCall { name: op_name, args: vec![lhs, rhs] };
            }
        }

        Ok(lhs)
    }

    fn parse_if(&mut self) -> Result<Expression, OnuError> {
        self.log(LogLevel::Trace, "Parsing if expression");
        self.consume(Token::If)?;
        let condition = self.parse_expression()?;
        
        self.skip_layout();
        self.consume(Token::Then)?;
        let then_branch = self.parse_expression()?;

        self.skip_layout();
        self.consume(Token::Else)?;
        let else_branch = self.parse_expression()?;

        Ok(Expression::If { condition: Box::new(condition), then_branch: Box::new(then_branch), else_branch: Box::new(else_branch) })
    }

    fn parse_derivation(&mut self) -> Result<Expression, OnuError> {
        self.log(LogLevel::Trace, "Parsing derivation");
        self.consume(Token::Derivation)?;
        self.match_token(Token::Operator(":".to_string()));
        
        self.skip_layout();
        let name = if let Some(Token::Identifier(n)) = self.advance() { n.clone() } else { "".into() };
        self.consume(Token::DerivesFrom)?;
        
        let type_info = self.parse_type_info()?;
        let value = self.parse_expression()?;
        
        // In Onu, derivations chain to form blocks, consuming subsequent expressions.
        let mut body_exprs = Vec::new();
        while let Some(t) = self.peek() {
            if self.is_expression_terminator(t) { break; }
            body_exprs.push(self.parse_expression()?);
        }
        
        let body = if body_exprs.is_empty() { Box::new(Expression::Nothing) }
                   else if body_exprs.len() == 1 { Box::new(body_exprs.pop().unwrap()) }
                   else { Box::new(Expression::Block(body_exprs)) };
        
        Ok(Expression::Derivation { name, type_info, value: Box::new(value), body })
    }
}
