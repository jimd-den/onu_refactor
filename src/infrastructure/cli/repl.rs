/// Ọ̀nụ REPL: Interactive JIT Read-Eval-Print Loop
///
/// This module implements a benchmarked interactive REPL using the State
/// Pattern to manage evaluation lifecycle.  The REPL compiles each entry
/// through the full pipeline, then executes the resulting LLVM IR via
/// Inkwell's JIT `ExecutionEngine`, printing both the output and the wall
/// time elapsed.
///
/// State transitions:
/// ```text
/// Idle ──(input received)──► Evaluating ──(done / error)──► Idle
/// ```
///
/// Architecture note: All REPL state is local to this module.  The core
/// compiler domain (`lib.rs`, MIR, HIR, etc.) is unaware of REPL semantics.

use std::io::{self, BufRead, Write};
use std::time::Instant;

use inkwell::context::Context;
use inkwell::memory_buffer::MemoryBuffer;
use inkwell::OptimizationLevel;

use crate::adapters::codegen::OnuCodegen;
use crate::adapters::lexer::OnuLexer;
use crate::adapters::parser::OnuParser;
use crate::application::options::{CompilationOptions, LogLevel};
use crate::application::ports::compiler_ports::LexerPort;
use crate::application::use_cases::analysis_service::AnalysisService;
use crate::application::use_cases::lowering_service::LoweringService;
use crate::application::use_cases::mir_lowering_service::MirLoweringService;
use crate::application::use_cases::module_service::ModuleService;
use crate::application::use_cases::registry_service::RegistryService;
use crate::domain::entities::core_module::{CoreModule, StandardMathModule};
use crate::domain::entities::error::OnuError;
use crate::infrastructure::extensions::io::OnuIoModule;
use crate::infrastructure::os::NativeOsEnvironment;

// ---------------------------------------------------------------------------
// State Machine
// ---------------------------------------------------------------------------

/// The lifecycle states of the REPL evaluator.
#[derive(Debug, PartialEq)]
pub enum ReplState {
    /// Waiting for user input.
    Idle,
    /// Actively compiling and executing a snippet.
    Evaluating,
}

// ---------------------------------------------------------------------------
// REPL struct
// ---------------------------------------------------------------------------

/// Interactive JIT REPL for the Ọ̀nụ language.
pub struct Repl {
    state: ReplState,
    lexer: OnuLexer,
    parser: OnuParser,
}

impl Repl {
    /// Construct a new REPL in the `Idle` state.
    pub fn new() -> Self {
        Self {
            state: ReplState::Idle,
            lexer: OnuLexer::new(LogLevel::Error),
            parser: OnuParser::new(LogLevel::Error),
        }
    }

    /// Return the current REPL state (useful for testing).
    pub fn state(&self) -> &ReplState {
        &self.state
    }

    /// Run the interactive loop, reading from `stdin` until EOF or `quit`.
    pub fn run(&mut self) {
        println!("Ọ̀nụ REPL — enter a complete Ọ̀nụ program, then submit with a blank line.");
        println!("Type ':run' on its own line to submit without a trailing blank line.");
        println!("Type 'quit' or 'exit' to leave.");
        println!();

        let stdin = io::stdin();
        let mut stdout = io::stdout();

        loop {
            self.state = ReplState::Idle;

            match Self::read_program(&stdin, &mut stdout) {
                // EOF or quit/exit
                None => break,
                // Blank submission (user just pressed Enter with no content)
                Some(src) if src.trim().is_empty() => continue,
                Some(src) => {
                    self.state = ReplState::Evaluating;
                    let start = Instant::now();

                    match self.compile_and_jit(src.trim()) {
                        Ok(result_msg) => {
                            let elapsed = start.elapsed();
                            println!("{}", result_msg);
                            println!("[JIT benchmark: {}µs]", elapsed.as_micros());
                        }
                        Err(e) => {
                            let elapsed = start.elapsed();
                            println!("Error: {:?}", e);
                            println!("[failed after {}µs]", elapsed.as_micros());
                        }
                    }
                }
            }
        }

        self.state = ReplState::Idle;
        println!("Farewell.");
    }

    /// Accumulate lines from `stdin` into a complete program.
    ///
    /// The first prompt is `onu> `; continuation lines show `  .. `.
    /// Input is submitted (returned) when:
    /// - The user enters a **blank line** after at least one non-blank line, or
    /// - The user enters `:run` on its own line.
    ///
    /// Returns `None` on EOF or when `quit`/`exit` is typed as the very first
    /// line (so the caller can break the REPL loop).
    fn read_program(stdin: &io::Stdin, stdout: &mut io::Stdout) -> Option<String> {
        let mut accumulated = String::new();

        loop {
            // Choose the prompt: primary on first line, continuation otherwise.
            if accumulated.is_empty() {
                print!("onu> ");
            } else {
                print!("  .. ");
            }
            stdout.flush().ok();

            let mut line = String::new();
            match stdin.lock().read_line(&mut line) {
                Ok(0) => {
                    // EOF: return whatever has been accumulated (may be empty).
                    return if accumulated.is_empty() { None } else { Some(accumulated) };
                }
                Err(_) => return None,
                Ok(_) => {}
            }

            // Trim all surrounding whitespace once, handling \r\n, \n, and
            // any other platform-specific newline sequences uniformly.
            let trimmed = line.trim();

            // Check quit/exit only when nothing has been entered yet.
            if accumulated.is_empty() && (trimmed == "quit" || trimmed == "exit") {
                return None;
            }

            // `:run` submits immediately regardless of trailing blank.
            if trimmed == ":run" {
                return Some(accumulated);
            }

            // A blank line after content submits; a blank line at the very
            // start is silently skipped (avoids submitting an empty program).
            if trimmed.is_empty() {
                if !accumulated.is_empty() {
                    return Some(accumulated);
                }
                // Still at the start — ignore leading blank line.
                continue;
            }

            // Accumulate the line.
            if !accumulated.is_empty() {
                accumulated.push('\n');
            }
            accumulated.push_str(trimmed);
        }
    }

    // -----------------------------------------------------------------------
    // Compilation & JIT execution
    // -----------------------------------------------------------------------

    /// Compile `source` through the full pipeline, JIT-execute the resulting
    /// LLVM IR via Inkwell's `ExecutionEngine`, and return a display string.
    fn compile_and_jit(&self, source: &str) -> Result<String, OnuError> {
        let ir = self.compile_to_ir(source)?;
        self.jit_execute(&ir)
    }

    /// Run source through lex → parse → HIR → MIR → LLVM IR.
    fn compile_to_ir(&self, source: &str) -> Result<String, OnuError> {
        let env = NativeOsEnvironment::new(LogLevel::Error);
        let options = CompilationOptions::default();

        let mut registry = RegistryService::new();
        let module_service = ModuleService::new(&env, options.log_level);
        module_service.register_module(&mut registry, &CoreModule);
        module_service.register_module(&mut registry, &StandardMathModule);
        module_service.register_module(&mut registry, &OnuIoModule);

        // Lex
        let tokens = self.lexer.lex(source)?;

        // Scan headers (registers behavior signatures for forward references)
        self.parser.scan_headers(&tokens, &mut registry)?;

        // Parse
        let discourses = self.parser.parse_with_registry(tokens, &mut registry)?;

        // HIR
        let analysis_service = AnalysisService::new(&env, &registry);
        let mut hir_discourses = Vec::new();
        for discourse in discourses {
            let mut hir = LoweringService::lower_discourse(&discourse, &registry);
            analysis_service.analyze_discourse(&mut hir)?;
            hir_discourses.push(hir);
        }

        // MIR (with all optimization passes)
        use crate::application::use_cases::inline_pass::InlinePass;
        use crate::application::use_cases::integer_upgrade_pass::IntegerUpgradePass;
        use crate::application::use_cases::memo_pass::MemoPass;
        use crate::application::use_cases::tco_pass::TcoPass;
        use crate::application::use_cases::wide_div_legalization_pass::WideDivLegalizationPass;
        use crate::application::ports::compiler_ports::CodegenPort;

        let mir_service = MirLoweringService::new(&env, &registry);
        let mir = mir_service.lower_program(&hir_discourses)?;
        let mir = IntegerUpgradePass::run(mir);
        let mir = MemoPass::run(mir, &registry);
        let mir = TcoPass::run(mir);
        let mir = InlinePass::run(mir);
        // Second TcoPass: catches tail calls exposed by inlining.
        let mir = TcoPass::run(mir);
        let mir = WideDivLegalizationPass::run(mir);

        // Codegen
        let mut codegen = OnuCodegen::new();
        codegen.set_registry(registry);
        let ir = codegen.generate(&mir)?;

        Ok(ir)
    }

    /// Parse the LLVM IR string into an Inkwell module, create a JIT
    /// `ExecutionEngine`, and call the `main` function.
    ///
    /// Returns a display string with the exit code / result.
    fn jit_execute(&self, ir: &str) -> Result<String, OnuError> {
        let context = Context::create();

        // Parse the IR text into an in-memory module.
        let buf = MemoryBuffer::create_from_memory_range(ir.as_bytes(), "repl_snippet");
        let module = context
            .create_module_from_ir(buf)
            .map_err(|e| OnuError::GrammarViolation {
                message: format!("LLVM IR parse error: {}", e),
                span: Default::default(),
            })?;

        // Create a JIT execution engine.
        let ee = module
            .create_jit_execution_engine(OptimizationLevel::Default)
            .map_err(|e| OnuError::GrammarViolation {
                message: format!("JIT engine creation failed: {}", e),
                span: Default::default(),
            })?;

        // Locate and call `main`.  Ọ̀nụ programs always emit a C-ABI `main`.
        let result = unsafe {
            match ee.get_function::<unsafe extern "C" fn() -> i64>("main") {
                Ok(f) => f.call(),
                Err(_) => {
                    // Try a void main for effect-only (nothing-returning) programs.
                    match ee.get_function::<unsafe extern "C" fn()>("main") {
                        Ok(f) => {
                            f.call();
                            0
                        }
                        Err(_) => {
                            return Err(OnuError::GrammarViolation {
                                message: "JIT: 'main' function not found or has an unsupported \
                                         signature (expected `fn() -> i64` or `fn()`)"
                                    .to_string(),
                                span: Default::default(),
                            });
                        }
                    }
                }
            }
        };

        Ok(format!("=> {}", result))
    }
}

impl Default for Repl {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repl_starts_idle() {
        let repl = Repl::new();
        assert_eq!(*repl.state(), ReplState::Idle);
    }

    #[test]
    fn test_repl_state_transitions() {
        // Verify that the state enum variants exist and are comparable.
        let idle = ReplState::Idle;
        let evaluating = ReplState::Evaluating;
        assert_ne!(idle, evaluating);
    }

    #[test]
    fn test_compile_to_ir_simple_program() {
        let repl = Repl::new();
        let source = r#"
the-module-called Repl with-concern: testing

the-behavior-called main
    with-intent: return a constant
    takes: nothing
    delivers: an integer
    as:
        42
"#;
        let result = repl.compile_to_ir(source);
        assert!(result.is_ok(), "Compile failed: {:?}", result.err());
        let ir = result.unwrap();
        assert!(ir.contains("define"), "IR should contain function definitions");
    }

    // -----------------------------------------------------------------------
    // Multi-line input helpers
    // -----------------------------------------------------------------------

    /// Helper: simulate the multi-line accumulator logic on a sequence of
    /// pre-supplied text lines without touching stdin.
    ///
    /// Mirrors the logic in `Repl::read_program`.
    fn simulate_accumulate(lines: &[&str]) -> Option<String> {
        let mut accumulated = String::new();

        for line in lines {
            let trimmed = line.trim();

            if accumulated.is_empty() && (trimmed == "quit" || trimmed == "exit") {
                return None;
            }

            if trimmed == ":run" {
                return Some(accumulated);
            }

            if trimmed.is_empty() {
                if !accumulated.is_empty() {
                    return Some(accumulated);
                }
                continue;
            }

            if !accumulated.is_empty() {
                accumulated.push('\n');
            }
            accumulated.push_str(trimmed);
        }

        if accumulated.is_empty() { None } else { Some(accumulated) }
    }

    #[test]
    fn test_multiline_blank_line_submits() {
        let lines = &[
            "the-module-called Test with-concern: test",
            "the-behavior-called main",
            "    takes: nothing",
            "",  // blank line → submit
        ];
        let result = simulate_accumulate(lines);
        assert!(result.is_some());
        let src = result.unwrap();
        assert!(src.contains("the-module-called Test"));
        assert!(src.contains("the-behavior-called main"));
    }

    #[test]
    fn test_multiline_run_command_submits() {
        let lines = &[
            "the-module-called Test with-concern: test",
            ":run",  // explicit submit
        ];
        let result = simulate_accumulate(lines);
        assert!(result.is_some());
        let src = result.unwrap();
        assert!(src.contains("the-module-called Test"));
        // `:run` itself should not appear in the accumulated source
        assert!(!src.contains(":run"));
    }

    #[test]
    fn test_multiline_quit_returns_none() {
        let lines = &["quit"];
        assert!(simulate_accumulate(lines).is_none());
    }

    #[test]
    fn test_multiline_exit_returns_none() {
        let lines = &["exit"];
        assert!(simulate_accumulate(lines).is_none());
    }

    #[test]
    fn test_multiline_leading_blank_lines_ignored() {
        let lines = &["", "", "first-line", ""];
        let result = simulate_accumulate(lines);
        assert!(result.is_some());
        assert_eq!(result.unwrap().trim(), "first-line");
    }
}
