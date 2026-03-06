pub mod adapters;
pub mod application;
pub mod domain;
pub mod infrastructure;

use crate::adapters::lexer::OnuLexer;
use crate::adapters::parser::OnuParser;
use crate::application::options::{CompilationOptions, CompilerStage, LogLevel};
use crate::application::ports::compiler_ports::{CodegenPort, LexerPort};
use crate::application::ports::environment::EnvironmentPort;
use crate::application::use_cases::analysis_service::AnalysisService;
use crate::application::use_cases::lowering_service::LoweringService;
use crate::application::use_cases::mir_lowering_service::MirLoweringService;
use crate::application::use_cases::module_service::ModuleService;
use crate::application::use_cases::registry_service::RegistryService;
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
    pub lexer: OnuLexer,
    pub parser: OnuParser,
}

impl<E: EnvironmentPort, C: CodegenPort> CompilationPipeline<E, C> {
    pub fn new(env: E, codegen: C, options: CompilationOptions) -> Self {
        let mut registry = RegistryService::new();
        registry.log_level = options.log_level;
        let module_service = ModuleService::new(&env, options.log_level);

        // Register Built-in Modules
        module_service.register_module(&mut registry, &CoreModule);
        module_service.register_module(&mut registry, &StandardMathModule);
        module_service.register_module(&mut registry, &OnuIoModule);

        Self {
            env,
            codegen,
            options: options.clone(),
            registry,
            lexer: OnuLexer::new(options.log_level),
            parser: OnuParser::new(options.log_level),
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

        // Stage 2: Loop-lower self-tail-calls.
        // Recursion → loop so the body becomes finite and inlineable.
        let mir = TcoPass::run(mir);

        // Stage 3: Inline pure loop-shaped callees into their callers.
        // Now that single-recursive functions are loops, InlinePass can fuse them.
        let mir = InlinePass::run(mir);

        // Stage 4: Second TcoPass — catches tail calls exposed by inlining.
        let mir = TcoPass::run(mir);

        // Stage 4.5: Automatically promote doubly-recursive pure functions from
        // I64 to WideInt(bits) when call-site literals imply overflow.
        // The correct full-precision answer uses native LLVM wide integers —
        // no external BigInt library required.  MemoPass (Stage 5) then
        // memoizes the widened function for O(n) time.
        let mir = IntegerUpgradePass::run(mir);

        // Stage 5: Memoize doubly-recursive pure functions annotated with
        // `with diminishing:`. Converts O(2^n) call trees to O(n) via a
        // stack-allocated lookup table.
        let mir = MemoPass::run(mir, &self.registry);

        // Stage 6: Operation Legalization — replace any WideInt (> 128-bit)
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
