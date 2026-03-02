/// Ọ̀nụ Lexer Adapter: Infrastructure/Interface Implementation
///
/// This implements the LexerPort by translating raw source text into
/// a sequence of Tokens that the ParserPort can consume.

use crate::application::ports::compiler_ports::{LexerPort, Token, Literal};
use crate::application::options::LogLevel;
use crate::domain::entities::error::OnuError;
use std::iter::Peekable;
use std::str::Chars;
use chrono::Local;

pub struct OnuLexer {
    pub log_level: LogLevel,
}

impl OnuLexer {
    pub fn new(log_level: LogLevel) -> Self {
        Self { log_level }
    }

    fn log(&self, level: LogLevel, message: &str) {
        if level <= self.log_level && level != LogLevel::None {
            let timestamp = Local::now().to_rfc3339();
            eprintln!("[{}] {:?}: [Lexer] {}", timestamp, level, message);
        }
    }
}

impl LexerPort for OnuLexer {
    fn lex(&self, source: &str) -> Result<Vec<Token>, OnuError> {
        self.log(LogLevel::Info, "Starting lexing process");
        let mut lexer = LexerInternal::new(source, self.log_level);
        let mut tokens = Vec::new();

        while let Some(token_result) = lexer.next_token() {
            let token = token_result?;
            self.log(LogLevel::Trace, &format!("Lexed token: {:?}", token));
            tokens.push(token);
        }

        self.log(LogLevel::Info, &format!("Lexing successful: {} tokens", tokens.len()));
        Ok(tokens)
    }
}

struct LexerInternal<'a> {
    input: Peekable<Chars<'a>>,
    log_level: LogLevel,
}

impl<'a> LexerInternal<'a> {
    fn new(input: &'a str, log_level: LogLevel) -> Self {
        Self {
            input: input.chars().peekable(),
            log_level,
        }
    }

    fn log(&self, level: LogLevel, message: &str) {
        if level <= self.log_level && level != LogLevel::None {
            let timestamp = Local::now().to_rfc3339();
            eprintln!("[{}] {:?}: [LexerInternal] {}", timestamp, level, message);
        }
    }

    fn peek_char(&mut self) -> Option<char> {
        self.input.peek().copied()
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            while let Some(c) = self.peek_char() {
                if c.is_whitespace() {
                    self.input.next();
                } else {
                    break;
                }
            }

            if let Some('-') = self.peek_char() {
                let mut temp = self.input.clone();
                temp.next();
                if let Some('-') = temp.peek() {
                    self.skip_comment();
                    continue;
                }
            }
            break;
        }
    }

    fn skip_comment(&mut self) {
        self.log(LogLevel::Trace, "Skipping comment");
        self.input.next(); 
        self.input.next(); // skip both '-'
        while let Some(c) = self.input.next() {
            if c == '\n' { 
                break; 
            }
        }
    }

    fn next_token(&mut self) -> Option<Result<Token, OnuError>> {
        self.skip_whitespace_and_comments();
        let first_char = self.peek_char()?;

        let token = match first_char {
            '(' => { self.input.next(); Token::Delimiter('(') }
            ')' => { self.input.next(); Token::Delimiter(')') }
            ':' => { self.input.next(); Token::Operator(":".to_string()) }
            '"' => self.lex_string(),
            c if c.is_ascii_digit() => self.lex_number(),
            c if c.is_alphanumeric() || c == '-' || c == '_' => self.lex_complex_keyword_or_id(),
            _ => {
                self.input.next();
                return self.next_token();
            }
        };

        Some(Ok(token))
    }

    fn lex_string(&mut self) -> Token {
        self.input.next();
        let mut s = String::new();
        while let Some(c) = self.input.next() {
            if c == '"' { break; }
            s.push(c);
        }
        Token::Literal(Literal::String(s))
    }

    fn lex_number(&mut self) -> Token {
        let mut s = String::new();
        let mut is_float = false;
        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() {
                s.push(c);
                self.input.next();
            } else if c == '.' && !is_float {
                is_float = true;
                s.push(c);
                self.input.next();
            } else { break; }
        }
        if is_float {
            Token::Literal(Literal::FloatBits(s.parse::<f64>().unwrap_or(0.0).to_bits()))
        } else {
            Token::Literal(Literal::Integer(s.parse::<i128>().unwrap_or(0)))
        }
    }

    fn lex_complex_keyword_or_id(&mut self) -> Token {
        let phrases = [
            ("the-module-called", Token::TheModuleCalled),
            ("the-shape-called", Token::TheShapeCalled),
            ("the-behavior-called", Token::TheBehaviorCalled),
            ("the-effect-behavior-called", Token::TheEffectBehaviorCalled),
            ("with-intent", Token::WithIntent),
            ("with-concern", Token::WithConcern),
            ("with-diminishing", Token::WithDiminishing),
            ("no-guaranteed-termination", Token::NoGuaranteedTermination),
            ("derives-from", Token::DerivesFrom),
            ("decreased-by", Token::DecreasedBy),
            ("partitions-by", Token::PartitionsBy),
            ("scales-by", Token::ScalesBy),
            ("added-to", Token::AddedTo),
            ("utilizes", Token::Utilizes),
            ("broadcasts", Token::Broadcasts),
            ("derivation", Token::Derivation),
            ("matches", Token::Matches),
            ("exceeds", Token::Exceeds),
            ("falls-short-of", Token::FallsShortOf),
            ("unites-with", Token::UnitesWith),
            ("joins-with", Token::JoinsWith),
            ("opposes", Token::Opposes),
            ("init-of", Token::InitOf),
            ("tail-of", Token::TailOf),
            ("duplicated-as", Token::DuplicatedAs),
        ];

        let mut current_pos = self.input.clone();
        let mut words = Vec::new();
        let mut matched_token = None;

        for _ in 0..4 {
            let mut word = String::new();
            while let Some(c) = current_pos.peek() {
                if c.is_alphanumeric() || *c == '-' || *c == '_' { word.push(*c); current_pos.next(); } else { break; }
            }
            if word.is_empty() { break; }
            words.push(word);
            let candidate = words.join("-");
            
            for (phrase, token) in &phrases {
                if phrase == &candidate {
                    matched_token = Some(token.clone());
                    self.input = current_pos.clone();
                }
            }

            while let Some(c) = current_pos.peek() {
                if c.is_whitespace() && *c != '\n' { current_pos.next(); } else { break; }
            }
        }

        if let Some(t) = matched_token { return t; }

        let mut s = String::new();
        while let Some(c) = self.peek_char() {
            if c.is_alphanumeric() || c == '-' || c == '_' { s.push(c); self.input.next(); } else { break; }
        }
        
        match s.as_str() {
            "if" => Token::If,
            "then" => Token::Then,
            "else" => Token::Else,
            "takes" => Token::Takes,
            "delivers" => Token::Delivers,
            "called" => Token::Called,
            "as" => Token::As,
            "nothing" => Token::Nothing,
            "true" => Token::Literal(Literal::Boolean(true)),
            "false" => Token::Literal(Literal::Boolean(false)),
            _ => Token::Identifier(s),
        }
    }
}
