pub mod adapters;
pub mod application;
pub mod domain;
pub mod infrastructure;

use crate::application::options::{CompilationOptions, CompilerStage, LogLevel};
use crate::application::ports::compiler_ports::{CodegenPort, LexerPort, ParserPort};
use crate::application::ports::environment::EnvironmentPort;
use crate::application::use_cases::analysis_service::AnalysisService;
use crate::application::use_cases::lowering_service::LoweringService;
use crate::application::use_cases::mir_lowering_service::MirLoweringService;
use crate::application::use_cases::module_service::ModuleService;
use crate::application::use_cases::registry_service::RegistryService;
use crate::application::use_cases::safety_pass;
use crate::domain::entities::ast::Discourse;
use crate::domain::entities::core_module::{CoreModule, StandardMathModule};
use crate::domain::entities::error::OnuError;
use crate::domain::entities::hir::HirDiscourse;
use crate::infrastructure::extensions::io::OnuIoModule;

pub struct CompilationPipeline<E: EnvironmentPort, C: CodegenPort> {
    pub env: E,
    pub codegen: C,
    pub options: CompilationOptions,
    pub registry: RegistryService,
    pub lexer: Box<dyn LexerPort>,
    pub parser: Box<dyn ParserPort>,
}

impl<E: EnvironmentPort, C: CodegenPort> CompilationPipeline<E, C> {
    pub fn new(
        env: E,
        codegen: C,
        lexer: Box<dyn LexerPort>,
        parser: Box<dyn ParserPort>,
        options: CompilationOptions,
    ) -> Self {
        let mut registry = RegistryService::new();
        registry.log_level = options.log_level;
        let module_service = ModuleService::new(&env, options.log_level);

        // Register Built-in Modules
        module_service.register_module(&mut registry, &CoreModule);
        module_service.register_module(&mut registry, &StandardMathModule);
        module_service.register_module(&mut registry, &OnuIoModule);

        // Pre-register multi-arg stdlib op signatures so the parser knows
        // their arity before scan_headers / parse_with_registry runs.
        crate::application::use_cases::stdlib::StdlibOpRegistry::register_signatures(&mut registry);

        Self {
            env,
            codegen,
            options: options.clone(),
            registry,
            lexer,
            parser,
        }
    }

    pub fn compile(&mut self, path: &str) -> Result<(), OnuError> {
        self.env.log(
            LogLevel::Info,
            &format!("Starting compilation for: {}", path),
        );

        let source = self.env.read_file(path)?;

        let tokens = self.lex(&source)?;
        if self.options.stop_after == Some(CompilerStage::Lexing) {
            return Ok(());
        }

        self.scan_headers(&tokens)?;

        let discourses = self.parse(tokens)?;
        if self.options.stop_after == Some(CompilerStage::Parsing) {
            return Ok(());
        }

        let hir_discourses = self.lower_hir(discourses)?;
        if self.options.stop_after == Some(CompilerStage::Analysis) {
            return Ok(());
        }

        // Safety pass: enforce S-1/S-2/S-3 grammar rules.
        // Warnings are printed; hard errors abort compilation with a clear message.
        match safety_pass::run(&hir_discourses) {
            Ok(diagnostics) => {
                for d in &diagnostics {
                    eprintln!("[onu warning] {}", d.message);
                }
            }
            Err(e) => {
                // Format and print the full error before returning so the
                // user sees the complete bilingual message in the terminal.
                let msg = match &e {
                    OnuError::GrammarViolation { message, .. } => message.clone(),
                    other => format!("{:?}", other),
                };
                eprintln!("\n{}\n", msg);
                return Err(e);
            }
        }

        let mir = self.lower_mir(hir_discourses)?;
        if self.options.stop_after == Some(CompilerStage::Mir) {
            return Ok(());
        }

        let ir = self.emit_ir(mir)?;
        if self.options.stop_after == Some(CompilerStage::Codegen) {
            return Ok(());
        }

        let stem = std::path::Path::new(path)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap();
        let ll_path = format!("{}.ll", stem);
        let prog_path = format!("{}_bin", stem);

        self.env.write_file(&ll_path, &ir)?;
        self.realize(&ll_path, &prog_path)?;

        Ok(())
    }

    pub fn lex(
        &self,
        source: &str,
    ) -> Result<Vec<crate::application::ports::compiler_ports::Token>, OnuError> {
        self.lexer.lex(source)
    }

    pub fn scan_headers(
        &mut self,
        tokens: &[crate::application::ports::compiler_ports::Token],
    ) -> Result<(), OnuError> {
        self.parser.scan_headers(tokens, &mut self.registry)
    }

    pub fn parse(
        &mut self,
        tokens: Vec<crate::application::ports::compiler_ports::Token>,
    ) -> Result<Vec<Discourse>, OnuError> {
        self.parser.parse_with_registry(tokens, &mut self.registry)
    }

    pub fn lower_hir(&self, discourses: Vec<Discourse>) -> Result<Vec<HirDiscourse>, OnuError> {
        let analysis_service = AnalysisService::new(&self.env, &self.registry);
        let mut hir_discourses = Vec::new();
        for discourse in discourses {
            let mut hir = LoweringService::lower_discourse(&discourse, &self.registry);
            analysis_service.analyze_discourse(&mut hir)?;
            if self.options.emit_hir {
                self.env
                    .log(LogLevel::Debug, &format!("HIR Emit: {:?}", hir));
            }
            hir_discourses.push(hir);
        }
        Ok(hir_discourses)
    }

    pub fn lower_mir(
        &self,
        hir_discourses: Vec<HirDiscourse>,
    ) -> Result<crate::domain::entities::mir::MirProgram, OnuError> {
        use crate::application::use_cases::inline_pass::InlinePass;
        use crate::application::use_cases::integer_upgrade_pass::IntegerUpgradePass;
        use crate::application::use_cases::memo_pass::MemoPass;
        use crate::application::use_cases::tco_pass::TcoPass;

        // Stage 1: Lower HIR → raw MIR (SSA, recursive call structure).
        let mir_lowering_service = MirLoweringService::new(&self.env, &self.registry);
        let mir = mir_lowering_service.lower_program(&hir_discourses)?;

        // Stage 2: Automatically promote doubly-recursive pure functions from
        // I64 to WideInt(bits) when call-site literals imply overflow.
        // Must run before MemoPass so that the wrapper caches WideInt values,
        // and before TcoPass so the doubly-recursive call structure is still
        // visible for candidate detection.
        let mir = IntegerUpgradePass::run(mir);

        // Stage 3: Memoize recursive pure functions annotated with
        // `with diminishing:`. Must run BEFORE TcoPass: TcoPass erases
        // tail-recursive Call instructions into Branch loops, so any
        // memoizable call that is also tail-recursive would be missed.
        let mir = MemoPass::run(mir, &self.registry);

        // Stage 4: Loop-lower self-tail-calls.
        // Recursion → loop so the body becomes finite and inlineable.
        // Acts on .inner functions (produced by MemoPass) as well as
        // non-memoized tail-recursive helpers (e.g. collatz-steps).
        let mir = TcoPass::run(mir);

        // Stage 5: Inline pure loop-shaped callees into their callers.
        // Now that single-recursive functions are loops, InlinePass can fuse them.
        let mir = InlinePass::run(mir);

        // Stage 6: Second TcoPass — catches tail calls exposed by inlining.
        let mir = TcoPass::run(mir);

        // Stage 7: Operation Legalization — replace any WideInt (> 128-bit)
        // division or modulo with a call to a compiler-internal helper
        // (__onu_wide_div_N / __onu_wide_mod_N) so the LLVM backend never sees
        // an sdiv/srem on a type wider than i128 (for which no runtime library
        // helper exists).
        use crate::application::use_cases::wide_div_legalization_pass::WideDivLegalizationPass;
        let mir = WideDivLegalizationPass::run(mir);

        Ok(mir)
    }

    pub fn emit_ir(
        &mut self,
        mir: crate::domain::entities::mir::MirProgram,
    ) -> Result<String, OnuError> {
        self.env.log(LogLevel::Info, "Starting Codegen stage.");
        self.codegen.set_registry(self.registry.clone());
        let ir = self.codegen.generate(&mir)?;
        self.env
            .log(LogLevel::Debug, &format!("Generated LLVM IR:\n{}", ir));
        Ok(ir)
    }

    fn realize(&self, bitcode_path: &str, output_path: &str) -> Result<(), OnuError> {
        self.env.log(
            LogLevel::Info,
            &format!("Realizing binary: {} -> {}", bitcode_path, output_path),
        );
        // Link bitcode natively
        self.env.run_command(
            "clang",
            &[
                bitcode_path,
                "-O3",
                "-o",
                output_path,
                "-Wno-override-module",
            ],
        )?;
        Ok(())
    }
}
