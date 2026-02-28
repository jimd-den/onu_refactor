/// Ọ̀nụ Layout Service: Application Use Case
///
/// This service transforms a flat token stream into a layout-aware stream
/// by inserting explicit Indent and Dedent tokens based on LineStart info.

use crate::application::ports::compiler_ports::Token;
use crate::application::options::LogLevel;
use chrono::Local;

pub struct LayoutService {
    indent_stack: Vec<usize>,
    pub log_level: LogLevel,
}

impl LayoutService {
    pub fn new(log_level: LogLevel) -> Self {
        Self { 
            indent_stack: vec![0],
            log_level,
        }
    }

    fn log(&self, level: LogLevel, message: &str) {
        if level <= self.log_level && level != LogLevel::None {
            let timestamp = Local::now().to_rfc3339();
            eprintln!("[{}] {:?}: [LayoutService] {}", timestamp, level, message);
        }
    }

    pub fn process(&mut self, source: &str, tokens: Vec<Token>) -> Vec<Token> {
        self.log(LogLevel::Info, "Starting layout processing");
        let mut result = Vec::new();
        let lines: Vec<&str> = source.lines().collect();
        let mut line_idx = 0;

        for token in tokens {
            if let Token::LineStart(indent) = token {
                self.log(LogLevel::Trace, &format!("Processing LineStart with indent {}", indent));
                // Peek at the actual line to see if it's empty or a comment
                while line_idx < lines.len() {
                    let content = lines[line_idx].trim();
                    if content.is_empty() || content.starts_with("--") {
                        self.log(LogLevel::Trace, &format!("Skipping empty or comment line: {}", line_idx));
                        line_idx += 1;
                        continue;
                    }
                    break;
                }

                if line_idx < lines.len() {
                    let current_level = *self.indent_stack.last().unwrap();
                    if indent > current_level {
                        self.log(LogLevel::Debug, &format!("Increasing indent: {} -> {}", current_level, indent));
                        self.indent_stack.push(indent);
                        result.push(Token::Indent);
                    } else if indent < current_level {
                        self.log(LogLevel::Debug, &format!("Decreasing indent: {} -> {}", current_level, indent));
                        while self.indent_stack.len() > 1 && indent < *self.indent_stack.last().unwrap() {
                            self.indent_stack.pop();
                            result.push(Token::Dedent);
                        }
                    }
                    result.push(Token::LineStart(indent));
                    line_idx += 1;
                }
                continue;
            }
            self.log(LogLevel::Trace, &format!("Preserving token: {:?}", token));
            result.push(token);
        }

        self.log(LogLevel::Debug, &format!("Cleaning up remaining indent stack (size {})", self.indent_stack.len()));
        while self.indent_stack.len() > 1 {
            self.indent_stack.pop();
            result.push(Token::Dedent);
        }

        self.log(LogLevel::Info, &format!("Layout processing complete: {} tokens", result.len()));
        result
    }
}
