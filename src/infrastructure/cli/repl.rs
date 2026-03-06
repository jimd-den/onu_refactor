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
        println!("Ọ̀nụ REPL — type a complete Ọ̀nụ program, or 'quit' to exit.");
        println!("Each program is JIT-compiled and executed; execution time is reported.");
        println!();

        let stdin = io::stdin();
        let mut stdout = io::stdout();

        loop {
            self.state = ReplState::Idle;
            print!("onu> ");
            stdout.flush().ok();

            let mut source = String::new();
            match stdin.lock().read_line(&mut source) {
                Ok(0) => break,          // EOF
                Err(_) => break,
                Ok(_) => {}
            }

            let trimmed = source.trim();
            if trimmed == "quit" || trimmed == "exit" {
                break;
            }
            if trimmed.is_empty() {
                continue;
            }

            self.state = ReplState::Evaluating;
            let start = Instant::now();

            match self.compile_and_jit(trimmed) {
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

        self.state = ReplState::Idle;
        println!("Farewell.");
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
        let mir = TcoPass::run(mir);
        let mir = InlinePass::run(mir);
        let mir = TcoPass::run(mir);
        let mir = IntegerUpgradePass::run(mir);
        let mir = MemoPass::run(mir, &registry);
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
}
