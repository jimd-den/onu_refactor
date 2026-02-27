/// Ọ̀nụ CLI Parser: Infrastructure Adapter
///
/// This implements a robust command-line argument parser.
/// It translates OS-level arguments into Application-level Options.

use crate::application::options::{CompilationOptions, CompilerStage};
use std::env;

pub struct OnuCliParser;

impl OnuCliParser {
    pub fn parse() -> Result<(String, CompilationOptions), String> {
        let args: Vec<String> = env::args().collect();
        if args.len() < 2 {
            return Err("Usage: onu_refactor <source_file> [--verbose] [--emit-hir] [--emit-mir] [--stop-after <stage>]".to_string());
        }

        let mut source_file = String::new();
        let mut options = CompilationOptions::default();
        let mut i = 1;

        while i < args.len() {
            match args[i].as_str() {
                "--verbose" | "-v" => options.verbose = true,
                "--emit-tokens" => options.emit_tokens = true,
                "--emit-hir" => options.emit_hir = true,
                "--emit-mir" => options.emit_mir = true,
                "--stop-after" => {
                    i += 1;
                    if i < args.len() {
                        options.stop_after = CompilerStage::from_str(&args[i]);
                    } else {
                        return Err("Missing stage after --stop-after".to_string());
                    }
                }
                arg if !arg.starts_with('-') => source_file = arg.to_string(),
                _ => {} // Unknown flag
            }
            i += 1;
        }

        if source_file.is_empty() {
            return Err("No source file specified.".to_string());
        }

        Ok((source_file, options))
    }
}
