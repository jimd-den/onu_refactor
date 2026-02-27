/// Ọ̀nụ Lexer Adapter: Infrastructure/Interface Implementation
///
/// This implements the LexerPort by translating raw source text into
/// a sequence of Tokens that the ParserPort can consume.

use crate::application::ports::compiler_ports::{LexerPort, Token, Literal};
use crate::domain::entities::error::OnuError;
use std::iter::Peekable;
use std::str::Chars;

pub struct OnuLexer;

impl LexerPort for OnuLexer {
    fn lex(&self, source: &str) -> Result<Vec<Token>, OnuError> {
        let mut lexer = LexerInternal::new(source);
        let mut tokens = Vec::new();

        while let Some(token_result) = lexer.next_token() {
            tokens.push(token_result?);
        }

        Ok(tokens)
    }
}

struct LexerInternal<'a> {
    input: Peekable<Chars<'a>>,
    at_line_start: bool,
}

impl<'a> LexerInternal<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input: input.chars().peekable(),
            at_line_start: true,
        }
    }

    fn peek_char(&mut self) -> Option<char> {
        self.input.peek().copied()
    }

    fn skip_whitespace_except_newline(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() && c != '\n' {
                self.input.next();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        self.input.next(); 
        while let Some(c) = self.input.next() {
            if c == '\n' { 
                self.at_line_start = true;
                break; 
            }
        }
    }

    fn next_token(&mut self) -> Option<Result<Token, OnuError>> {
        // Essential: handle at_line_start before skipping normal whitespace
        if self.at_line_start {
            self.at_line_start = false;
            let mut indent = 0;
            while let Some(c) = self.peek_char() {
                if c == ' ' {
                    indent += 1;
                    self.input.next();
                } else if c == '\t' {
                    indent += 4;
                    self.input.next();
                } else if c == '\n' {
                    self.input.next();
                    self.at_line_start = true;
                    indent = 0;
                } else if c == '-' {
                    let mut temp = self.input.clone();
                    temp.next();
                    if let Some('-') = temp.peek() {
                        self.skip_comment(); // sets at_line_start = true
                        return self.next_token();
                    } else { break; }
                } else if c.is_whitespace() {
                    self.input.next();
                } else { break; }
            }
            
            if self.peek_char().is_some() {
                return Some(Ok(Token::LineStart(indent)));
            } else {
                return None;
            }
        }

        self.skip_whitespace_except_newline();
        let first_char = self.peek_char()?;

        if first_char == '\n' {
            self.input.next();
            self.at_line_start = true;
            return self.next_token();
        }

        if first_char == '-' {
            let mut temp = self.input.clone();
            temp.next();
            if let Some('-') = temp.peek() {
                self.skip_comment();
                return self.next_token();
            }
        }

        let token = match first_char {
            '(' => { self.input.next(); Token::Delimiter('(') }
            ')' => { self.input.next(); Token::Delimiter(')') }
            ':' => { self.input.next(); Token::Operator(":".to_string()) }
            '"' => self.lex_string(),
            c if c.is_ascii_digit() => self.lex_number(),
            c if c.is_alphabetic() => self.lex_complex_keyword_or_id(),
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
            "the-module-called", "the-behavior-called", "the-effect-behavior-called",
            "with-intent", "with-concern", "with-diminishing", "no-guaranteed-termination",
            "derives-from", "decreased-by", "partitions-by", "scales-by", "added-to", "utilizes", "as", "takes", "delivers", "called"
        ];

        let mut current_pos = self.input.clone();
        let mut words = Vec::new();
        let mut matched_phrase = None;

        for _ in 0..4 {
            let mut word = String::new();
            while let Some(c) = current_pos.peek() {
                if c.is_alphanumeric() || *c == '-' { word.push(*c); current_pos.next(); } else { break; }
            }
            if word.is_empty() { break; }
            words.push(word);
            let candidate = words.join("-");
            if phrases.contains(&candidate.as_str()) {
                matched_phrase = Some(candidate);
                self.input = current_pos.clone();
            }
            while let Some(c) = current_pos.peek() {
                if c.is_whitespace() && *c != '\n' { current_pos.next(); } else { break; }
            }
        }

        if let Some(p) = matched_phrase { return Token::Identifier(p); }

        let mut s = String::new();
        while let Some(c) = self.peek_char() {
            if c.is_alphanumeric() || c == '-' { s.push(c); self.input.next(); } else { break; }
        }
        
        match s.as_str() {
            "if" => Token::Identifier("if".to_string()),
            "then" => Token::Identifier("then".to_string()),
            "else" => Token::Identifier("else".to_string()),
            "true" => Token::Literal(Literal::Boolean(true)),
            "false" => Token::Literal(Literal::Boolean(false)),
            "nothing" => Token::Keyword("nothing".to_string()),
            _ => Token::Identifier(s),
        }
    }
}
