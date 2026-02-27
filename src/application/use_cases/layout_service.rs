/// Ọ̀nụ Layout Service: Application Use Case
///
/// This service transforms a flat token stream into a layout-aware stream
/// by inserting explicit Indent and Dedent tokens based on LineStart info.

use crate::application::ports::compiler_ports::Token;

pub struct LayoutService {
    indent_stack: Vec<usize>,
}

impl LayoutService {
    pub fn new() -> Self {
        Self { indent_stack: vec![0] }
    }

    pub fn process(&mut self, source: &str, tokens: Vec<Token>) -> Vec<Token> {
        let mut result = Vec::new();
        let lines: Vec<&str> = source.lines().collect();
        let mut line_idx = 0;

        for token in tokens {
            if let Token::LineStart(indent) = token {
                // Peek at the actual line to see if it's empty or a comment
                while line_idx < lines.len() {
                    let content = lines[line_idx].trim();
                    if content.is_empty() || content.starts_with("--") {
                        line_idx += 1;
                        continue;
                    }
                    break;
                }

                if line_idx < lines.len() {
                    let current_level = *self.indent_stack.last().unwrap();
                    if indent > current_level {
                        self.indent_stack.push(indent);
                        result.push(Token::Indent);
                    } else if indent < current_level {
                        while self.indent_stack.len() > 1 && indent < *self.indent_stack.last().unwrap() {
                            self.indent_stack.pop();
                            result.push(Token::Dedent);
                        }
                    }
                    line_idx += 1;
                }
                continue;
            }
            result.push(token);
        }

        while self.indent_stack.len() > 1 {
            self.indent_stack.pop();
            result.push(Token::Dedent);
        }

        result
    }
}
