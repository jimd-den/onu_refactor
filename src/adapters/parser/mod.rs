/// Ọ̀nụ Parser Adapter: Structural Discourse Implementation
///
/// This implements the ParserPort by consuming a sequence of Tokens
/// and building the Domain-level AST (Discourse and Expressions).

use crate::application::ports::compiler_ports::{ParserPort, Token, Literal};
use crate::application::options::LogLevel;
use crate::domain::entities::error::{OnuError, Span};
use crate::domain::entities::ast::{Discourse, Expression, BehaviorHeader, ReturnType, Argument, TypeInfo, BinOp};
use crate::domain::entities::types::OnuType;
use crate::domain::entities::registry::BehaviorSignature;
use crate::application::use_cases::registry_service::RegistryService;
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

    pub fn scan_headers(&self, tokens: &[Token], registry: &mut RegistryService) -> Result<(), OnuError> {
        self.log(LogLevel::Info, "Starting header scanning");
        let mut parser = ParserInternal::new(tokens.to_vec(), self.log_level);
        
        while !parser.is_at_end() {
            let token = parser.peek();
            if matches!(token, Some(Token::TheBehaviorCalled | Token::TheEffectBehaviorCalled)) {
                let header = parser.parse_behavior_header(registry)?;
                let sig = BehaviorSignature {
                    input_types: header.takes.iter().map(|a| a.type_info.onu_type.clone()).collect(),
                    return_type: header.delivers.0.clone(),
                    arg_is_observation: header.takes.iter().map(|a| a.type_info.is_observation).collect(),
                };
                registry.symbols_mut().add_signature(&header.name, sig);
                
                // Skip to the next discourse start by skipping the body block
                while !parser.is_at_end() {
                    let next = parser.peek();
                    if matches!(next, Some(Token::TheModuleCalled | Token::TheShapeCalled | Token::TheBehaviorCalled | Token::TheEffectBehaviorCalled)) {
                        break;
                    }
                    parser.advance();
                }
            } else if matches!(token, Some(Token::TheShapeCalled)) {
                if let Discourse::Shape { name, fields, behaviors } = parser.parse_shape(registry)? {
                    let field_defs: Vec<(String, OnuType)> = fields.iter().map(|f| (f.name.clone(), f.type_info.onu_type.clone())).collect();
                    let behavior_sigs = behaviors.iter().map(|b| {
                        (b.name.clone(), BehaviorSignature {
                            input_types: b.takes.iter().map(|a| a.type_info.onu_type.clone()).collect(),
                            return_type: b.delivers.0.clone(),
                            arg_is_observation: b.takes.iter().map(|a| a.type_info.is_observation).collect(),
                        })
                    }).collect();

                    // Register field accessors as behaviors
                    for (fname, ftyp) in &field_defs {
                        let sig = BehaviorSignature {
                            input_types: vec![OnuType::Shape(name.clone())],
                            return_type: ftyp.clone(),
                            arg_is_observation: vec![true],
                        };
                        registry.symbols_mut().add_signature(fname, sig);
                    }

                    // Register constructor as behavior
                    let constructor_sig = BehaviorSignature {
                        input_types: field_defs.iter().map(|(_, t)| t.clone()).collect(),
                        return_type: OnuType::Shape(name.clone()),
                        arg_is_observation: vec![false; field_defs.len()],
                    };
                    registry.symbols_mut().add_signature(&name, constructor_sig);

                    registry.add_shape(&name, field_defs, behavior_sigs);
                }
            } else {
                parser.advance();
            }
        }
        Ok(())
    }

    pub fn parse_with_registry(&self, tokens: Vec<Token>, registry: &mut RegistryService) -> Result<Vec<Discourse>, OnuError> {
        self.log(LogLevel::Info, "Starting parsing process with registry");
        let mut parser = ParserInternal::new(tokens, self.log_level);
        let mut discourses = Vec::new();

        while !parser.is_at_end() {
            if let Some(d) = parser.parse_discourse(registry)? {
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

impl ParserPort for OnuParser {
    fn parse(&self, tokens: Vec<Token>) -> Result<Vec<Discourse>, OnuError> {
        let mut registry = RegistryService::new();
        self.parse_with_registry(tokens, &mut registry)
    }
}

struct ParserInternal {
    tokens: Vec<Token>,
    pos: usize,
    log_level: LogLevel,
}

impl ParserInternal {
    fn new(tokens: Vec<Token>, log_level: LogLevel) -> Self {
        Self {
            tokens,
            pos: 0,
            log_level,
        }
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


    fn parse_discourse(&mut self, registry: &mut RegistryService) -> Result<Option<Discourse>, OnuError> {
        while let Some(t) = self.peek() {
            match t {
                Token::TheModuleCalled => return Ok(Some(self.parse_module()?)),
                Token::TheShapeCalled => return Ok(Some(self.parse_shape(registry)?)),
                Token::TheBehaviorCalled | Token::TheEffectBehaviorCalled => return Ok(Some(self.parse_behavior(registry)?)),
                _ => { 
                    self.advance(); 
                }
            }
        }
        Ok(None)
    }

    fn parse_shape(&mut self, registry: &mut RegistryService) -> Result<Discourse, OnuError> {
        self.log(LogLevel::Debug, "Parsing shape");
        self.consume(Token::TheShapeCalled)?;
        let name = if let Some(Token::Identifier(n)) = self.advance() { n.clone() } else { 
            return Err(OnuError::GrammarViolation { message: "Expected shape name".into(), span: Span::default() });
        };
        
        let mut fields = Vec::new();
        let mut behaviors = Vec::new();

        while let Some(t) = self.peek() {
            if matches!(t, Token::TheModuleCalled | Token::TheShapeCalled | Token::TheBehaviorCalled | Token::TheEffectBehaviorCalled) { break; }
            match t {
                Token::Takes => {
                    self.advance();
                    self.match_token(Token::Operator(":".to_string()));
                    fields = self.parse_arguments(registry)?;
                }
                _ => { self.advance(); }
            }
        }
        Ok(Discourse::Shape { name, fields, behaviors })
    }

    fn parse_module(&mut self) -> Result<Discourse, OnuError> {
        self.log(LogLevel::Debug, "Parsing module");
        self.consume(Token::TheModuleCalled)?;
        let name = if let Some(Token::Identifier(n)) = self.advance() { n.clone() } else { 
            return Err(OnuError::GrammarViolation { message: "Expected module name".into(), span: Span::default() });
        };
        
        let mut concern = String::new();
        while let Some(t) = self.peek() {
            if matches!(t, Token::TheModuleCalled | Token::TheShapeCalled | Token::TheBehaviorCalled | Token::TheEffectBehaviorCalled) { break; }
            if matches!(t, Token::WithConcern) {
                self.advance();
                self.match_token(Token::Operator(":".to_string()));
                if let Some(Token::Identifier(c)) = self.advance() {
                    concern = c.clone();
                }
            } else {
                self.advance();
            }
        }
        Ok(Discourse::Module { name, concern })
    }

    fn parse_behavior(&mut self, registry: &mut RegistryService) -> Result<Discourse, OnuError> {
        let header = self.parse_behavior_header(registry)?;

        self.consume(Token::As)?;
        self.match_token(Token::Operator(":".to_string()));

        let body = self.parse_block(registry)?;

        Ok(Discourse::Behavior { header, body })
    }

    fn parse_behavior_header(&mut self, registry: &mut RegistryService) -> Result<BehaviorHeader, OnuError> {
        let is_effect = matches!(self.peek(), Some(Token::TheEffectBehaviorCalled));
        let behavior_keyword = if is_effect { Token::TheEffectBehaviorCalled } else { Token::TheBehaviorCalled };
        self.consume(behavior_keyword)?;

        let name = if let Some(Token::Identifier(n)) = self.advance() { n.clone() } else { 
            return Err(OnuError::GrammarViolation { message: "Expected behavior name".into(), span: Span::default() });
        };

        let mut intent = String::new();
        let mut takes = Vec::new();
        let mut delivers = ReturnType(OnuType::Nothing);
        let mut diminishing = None;
        let mut skip_termination_check = false;

        while let Some(token) = self.peek() {
            match token {
                Token::WithIntent => {
                    self.advance();
                    self.match_token(Token::Operator(":".to_string()));
                    // Consume everything until next keyword
                    let mut parts = Vec::new();
                    while let Some(t) = self.peek() {
                        if matches!(t, Token::Takes | Token::Delivers | Token::WithDiminishing | Token::NoGuaranteedTermination | Token::As) {
                            break;
                        }
                        if let Some(Token::Identifier(s)) = self.advance() {
                            parts.push(s.clone());
                        } else {
                            break;
                        }
                    }
                    intent = parts.join(" ");
                }
                Token::Takes => {
                    self.advance();
                    self.match_token(Token::Operator(":".to_string()));
                    takes = self.parse_arguments(registry)?;
                }
                Token::Delivers => {
                    self.advance();
                    self.match_token(Token::Operator(":".to_string()));
                    delivers = self.parse_return_type(registry)?;
                }
                Token::WithDiminishing => {
                    self.advance();
                    self.match_token(Token::Operator(":".to_string()));
                    if let Some(Token::Identifier(d)) = self.advance() {
                        diminishing = Some(d.clone());
                    }
                }
                Token::NoGuaranteedTermination => {
                    self.advance();
                    skip_termination_check = true;
                }
                Token::As => break,
                _ => { self.advance(); }
            }
        }

        Ok(BehaviorHeader { name, is_effect, intent, takes, delivers, diminishing, skip_termination_check })
    }

    fn parse_block(&mut self, registry: &mut RegistryService) -> Result<Expression, OnuError> {
        let mut exprs = Vec::new();
        while let Some(t) = self.peek() {
            if matches!(t, Token::TheModuleCalled | Token::TheShapeCalled | Token::TheBehaviorCalled | Token::TheEffectBehaviorCalled) {
                break;
            }
            if matches!(t, Token::As) {
                break;
            }
            exprs.push(self.parse_expression(registry)?);
        }
        
        if exprs.is_empty() {
            Ok(Expression::Nothing)
        } else if exprs.len() == 1 {
            Ok(exprs.pop().unwrap())
        } else {
            Ok(Expression::Block(exprs))
        }
    }

    fn parse_expression(&mut self, registry: &mut RegistryService) -> Result<Expression, OnuError> {
        if matches!(self.peek(), Some(Token::If)) {
            return self.parse_if(registry);
        }
        if matches!(self.peek(), Some(Token::Derivation)) {
            return self.parse_derivation(registry);
        }
        self.parse_infix(0, registry)
    }

    fn parse_if(&mut self, registry: &mut RegistryService) -> Result<Expression, OnuError> {
        self.log(LogLevel::Trace, "Parsing if expression");
        self.consume(Token::If)?;
        let condition = self.parse_expression(registry)?;
        
        self.consume(Token::Then)?;
        let then_branch = self.parse_expression(registry)?;

        self.consume(Token::Else)?;
        let else_branch = self.parse_expression(registry)?;

        Ok(Expression::If { condition: Box::new(condition), then_branch: Box::new(then_branch), else_branch: Box::new(else_branch) })
    }

    fn parse_derivation(&mut self, registry: &mut RegistryService) -> Result<Expression, OnuError> {
        self.log(LogLevel::Trace, "Parsing derivation");
        self.consume(Token::Derivation)?;
        self.match_token(Token::Operator(":".to_string()));
        let name = if let Some(Token::Identifier(n)) = self.advance() { n.clone() } else { 
            return Err(OnuError::GrammarViolation { message: "Expected derivation name".into(), span: Span::default() });
        };
        
        self.consume(Token::DerivesFrom)?;
        
        let type_info = self.parse_type_info(registry)?;
        let value = self.parse_expression(registry)?;

        // In Onu, derivations chain to form blocks, consuming subsequent expressions.
        let mut body_exprs = Vec::new();
        while let Some(t) = self.peek() {
            if self.is_expression_terminator(t) {
                break;
            }
            if matches!(t, Token::TheModuleCalled | Token::TheShapeCalled | Token::TheBehaviorCalled | Token::TheEffectBehaviorCalled | Token::As) {
                break;
            }
            if matches!(t, Token::Derivation) {
                body_exprs.push(self.parse_derivation(registry)?);
                break; // A derivation consumes the rest of the scope
            }
            body_exprs.push(self.parse_expression(registry)?);
        }
        
        let body = if body_exprs.is_empty() { Box::new(Expression::Nothing) }
                   else if body_exprs.len() == 1 { Box::new(body_exprs.pop().unwrap()) }
                   else { Box::new(Expression::Block(body_exprs)) };
        
        Ok(Expression::Derivation { name, type_info, value: Box::new(value), body } )
    }

    fn parse_infix(&mut self, min_precedence: u8, registry: &mut RegistryService) -> Result<Expression, OnuError> {
        let mut lhs = self.parse_primary(registry)?;

        while let Some(token) = self.peek() {
            if self.is_expression_terminator(token) { break; }

            let (op, precedence) = match token {
                Token::Operator(s) => {
                    let p = match s.as_str() {
                        "matches" => 2,
                        _ => 1,
                    };
                    (s.clone(), p)
                }
                Token::Matches => ("matches".to_string(), 2),
                Token::Exceeds => ("exceeds".to_string(), 2),
                Token::FallsShortOf => ("falls-short-of".to_string(), 2),
                Token::AddedTo => ("added-to".to_string(), 3),
                Token::DecreasedBy => ("decreased-by".to_string(), 3),
                Token::ScalesBy => ("scales-by".to_string(), 4),
                Token::PartitionsBy => ("partitions-by".to_string(), 4),
                Token::JoinsWith => ("joins-with".to_string(), 4),
                Token::UnitesWith => ("unites-with".to_string(), 4),
                Token::Opposes => ("opposes".to_string(), 4),
                Token::InitOf => ("init-of".to_string(), 4),
                Token::TailOf => ("tail-of".to_string(), 4),
                Token::DuplicatedAs => ("duplicated-as".to_string(), 4),
                Token::Utilizes => ("utilizes".to_string(), 5),
                Token::Identifier(s) => {
                    let p = match s.as_str() {
                        "matches" | "exceeds" | "falls-short-of" => 2,
                        "added-to" | "decreased-by" => 3,
                        "scales-by" | "partitions-by" => 4,
                        "joined-with" | "joins-with" | "unites-with" | "opposes" => 4,
                        "init-of" | "tail-of" | "duplicated-as" => 4,
                        "char-at" | "charat" => 4,
                        "utilizes" => 5,
                        _ => break,
                    };
                    (s.clone(), p)
                }
                _ => break,
            };

            if precedence < min_precedence { break; }

            self.advance(); // consume operator

            if op == "utilizes" {
                let target = self.parse_behavior_name()?;
                
                let mut args = vec![lhs];
                
                // Heuristic: check registry for arity to know how many more arguments to consume
                if let Some(sig) = registry.get_signature(&target) {
                    let additional_args_needed = sig.input_types.len().saturating_sub(1);
                    for _ in 0..additional_args_needed {
                        args.push(self.parse_expression(registry)?);
                    }
                } else {
                    // Default to 1 additional argument if unknown
                    if let Some(next) = self.peek() {
                        if !self.is_expression_terminator(next) {
                            args.push(self.parse_expression(registry)?);
                        }
                    }
                }
                lhs = Expression::BehaviorCall { name: target, args };
            } else if op == "duplicated-as" || op == "init-of" || op == "tail-of" {
                lhs = Expression::BehaviorCall { name: op, args: vec![lhs] };
            } else if op == "matches" || op == "exceeds" || op == "falls-short-of" || 
                      op == "added-to" || op == "decreased-by" || op == "scales-by" || op == "partitions-by" ||
                      op == "joined-with" || op == "char-at" {
                let rhs = self.parse_infix(precedence + 1, registry)?;
                lhs = Expression::BehaviorCall { name: op, args: vec![lhs, rhs] };
            } else {
                let rhs = self.parse_infix(precedence + 1, registry)?;
                lhs = Expression::BinaryOp { left: Box::new(lhs), op: BinOp::Add, right: Box::new(rhs) }; // Placeholder
            }
        }

        Ok(lhs)
    }

    fn parse_behavior_name(&mut self) -> Result<String, OnuError> {
        let token = self.peek().cloned().ok_or_else(|| {
            OnuError::GrammarViolation { message: "Expected behavior name".into(), span: Span::default() }
        })?;

        let name = match token {
            Token::Identifier(s) => s.clone(),
            Token::InitOf => "init-of".to_string(),
            Token::TailOf => "tail-of".to_string(),
            Token::DuplicatedAs => "duplicated-as".to_string(),
            Token::Broadcasts => "broadcasts".to_string(),
            _ => return Err(OnuError::GrammarViolation { 
                message: format!("Expected behavior name, found {:?}", token), 
                span: Span::default() 
            }),
        };
        self.advance();
        Ok(name)
    }

    fn parse_primary(&mut self, registry: &mut RegistryService) -> Result<Expression, OnuError> {
        let token = self.peek().cloned().ok_or_else(|| {
            OnuError::GrammarViolation { message: "Unexpected end of input".into(), span: Span::default() }
        })?;

        match &token {
            Token::Literal(l) => {
                self.advance();
                match l {
                    Literal::Integer(n) => Ok(Expression::I64((*n).try_into().unwrap_or(0))),
                    Literal::FloatBits(n) => Ok(Expression::F32(*n as u32)),
                    Literal::Boolean(b) => Ok(Expression::Boolean(*b)),
                    Literal::String(s) => Ok(Expression::Text(s.clone())),
                }
            },
            Token::Identifier(s) => {
                if self.is_expression_terminator(&token) {
                    return Ok(Expression::Nothing);
                }
                let name = s.clone();
                self.advance();
                
                // If it's a known behavior with zero arity, parse as call
                if let Some(sig) = registry.get_signature(&name) {
                    if sig.input_types.is_empty() {
                        return Ok(Expression::BehaviorCall { name, args: vec![] });
                    }
                }
                
                Ok(Expression::Identifier(name))
            },
            Token::Nothing => { self.advance(); Ok(Expression::Nothing) },
            Token::Delimiter('(') => {
                self.advance();
                let inner = self.parse_expression(registry)?;
                self.consume(Token::Delimiter(')'))?;
                Ok(inner)
            },
            Token::Broadcasts => {
                self.advance();
                let inner = self.parse_expression(registry)?;
                Ok(Expression::Emit(Box::new(inner)))
            }
            _ => Err(OnuError::GrammarViolation { message: format!("Unexpected token in primary: {:?}", token), span: Span::default() }),
        }
    }

    fn parse_type_info(&mut self, registry: &mut RegistryService) -> Result<Option<TypeInfo>, OnuError> {
        if let Some(Token::Identifier(s)) = self.peek() {
            if s == "a" || s == "an" || s == "the" {
                self.advance();
                let onu_type = self.parse_type_name(registry)?;
                let display_name = format!("{:?}", onu_type); 
                
                return Ok(Some(TypeInfo { onu_type, display_name, via_role: None, is_observation: false }));
            }
        }
        Ok(None)
    }

    fn parse_type_name(&mut self, registry: &mut RegistryService) -> Result<OnuType, OnuError> {
        let name = self.parse_behavior_name()?;

        if name == "tuple" {
            if let Some(Token::Identifier(of)) = self.peek() {
                if of == "of" {
                    self.advance();
                    self.consume(Token::Delimiter('('))?;
                    let mut elements = Vec::new();
                    while let Some(t) = self.peek() {
                        if matches!(t, Token::Delimiter(')')) { break; }
                        if !elements.is_empty() {
                            self.match_token(Token::Operator(":".to_string()));
                        }
                        if let Some(ti) = self.parse_type_info(registry)? {
                            elements.push(ti.onu_type);
                        } else {
                            break;
                        }
                    }
                    self.consume(Token::Delimiter(')'))?;
                    return Ok(OnuType::Tuple(elements));
                }
            }
        }

        if let Some(primitive) = OnuType::from_name(&name) {
            return Ok(primitive);
        }

        if registry.is_shape(&name) {
            return Ok(OnuType::Shape(name));
        }

        // Default fallback or error
        Ok(OnuType::Shape(name)) // Assume it's a shape if not primitive
    }

    fn parse_return_type(&mut self, registry: &mut RegistryService) -> Result<ReturnType, OnuError> {
        if self.match_token(Token::Nothing) {
            return Ok(ReturnType(OnuType::Nothing));
        }
        let ti = self.parse_type_info(registry)?.ok_or_else(|| {
            OnuError::GrammarViolation { message: "Strict typing enforced: Missing explicit return type".into(), span: Span::default() }
        })?;
        Ok(ReturnType(ti.onu_type))
    }

    fn parse_arguments(&mut self, registry: &mut RegistryService) -> Result<Vec<Argument>, OnuError> {
        self.log(LogLevel::Trace, "Parsing arguments");
        let mut args = Vec::new();
        while let Some(token) = self.peek() {
            self.log(LogLevel::Trace, &format!("Arguments loop peeking: {:?}", token));
            if matches!(token, Token::TheModuleCalled | Token::TheShapeCalled | Token::TheBehaviorCalled | Token::TheEffectBehaviorCalled | Token::Delivers | Token::As) {
                break;
            }
            match token {
                Token::Identifier(s) if s == "a" || s == "an" || s == "the" => {
                    let mut type_info = self.parse_type_info(registry)?.ok_or_else(|| {
                        OnuError::GrammarViolation { message: "Strict typing enforced: Missing explicit type indicator (e.g. 'a', 'an', 'the') for argument".into(), span: Span::default() }
                    })?;
                    self.match_token(Token::Called);
                    let name = if let Some(Token::Identifier(n)) = self.advance() { n.clone() } else { "".to_string() };
                    
                    // Check for 'via observation'
                    if let Some(Token::Identifier(v)) = self.peek() {
                        if v == "via" {
                            self.advance();
                            if let Some(Token::Identifier(o)) = self.peek() {
                                if o == "observation" {
                                    self.advance();
                                    type_info.is_observation = true;
                                }
                            }
                        }
                    }
                    
                    self.log(LogLevel::Debug, &format!("Parsed argument: {} of type {:?}", name, type_info.onu_type));
                    args.push(Argument { name, type_info });
                }
                _ => { 
                    self.log(LogLevel::Trace, &format!("Advancing past non-argument token: {:?}", token));
                    self.advance(); 
                }
            }
        }
        Ok(args)
    }
}
