/// CLI Parser Infrastructure: Infrastructure Implementation
///
/// This module implements the command-line interface for the Ọ̀nụ compiler.
/// It translates CLI arguments into CompilationOptions.

use crate::application::options::{CompilationOptions, CompilerStage, LogLevel};
use crate::domain::entities::error::{OnuError, Span};

pub struct CliParser;

impl CliParser {
    pub fn parse_args(args: &[String]) -> Result<(String, CompilationOptions), OnuError> {
        if args.len() < 2 {
            return Err(OnuError::GrammarViolation { 
                message: "Usage: onu <source_file> [options]".to_string(), 
                span: Span::default() 
            });
        }

        let source_file = args[1].clone();
        let mut options = CompilationOptions::default();

        let mut i = 2;
        while i < args.len() {
            match args[i].as_str() {
                "--verbose" | "-v" => options.log_level = LogLevel::Debug,
                "--stop-after" => {
                    if i + 1 < args.len() {
                        options.stop_after = CompilerStage::from_str(&args[i+1]);
                        i += 1;
                    }
                }
                "--emit-hir" => options.emit_hir = true,
                "--emit-mir" => options.emit_mir = true,
                "--emit-tokens" => options.emit_tokens = true,
                _ => {}
            }
            i += 1;
        }

        Ok((source_file, options))
    }
}
