pub mod domain;
pub mod application;
pub mod infrastructure;
pub mod adapters;

use crate::application::use_cases::registry_service::RegistryService;
use crate::application::use_cases::analysis_service::AnalysisService;
use crate::application::use_cases::lowering_service::LoweringService;
use crate::application::use_cases::mir_lowering_service::MirLoweringService;
use crate::application::use_cases::module_service::ModuleService;
use crate::application::ports::compiler_ports::{LexerPort, CodegenPort};
use crate::application::ports::environment::EnvironmentPort;
use crate::application::options::{CompilationOptions, CompilerStage, LogLevel};
use crate::domain::entities::error::OnuError;
use crate::domain::entities::ast::Discourse;
use crate::domain::entities::hir::HirDiscourse;
use crate::domain::entities::core_module::{CoreModule, StandardMathModule};
use crate::infrastructure::extensions::io::OnuIoModule;
use crate::adapters::lexer::OnuLexer;
use crate::adapters::parser::OnuParser;

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
        self.env.log(LogLevel::Info, &format!("Starting compilation for: {}", path));

        let source = self.env.read_file(path)?;
        
        let tokens = self.lex(&source)?;
        if self.options.stop_after == Some(CompilerStage::Lexing) { return Ok(()); }

        self.scan_headers(&tokens)?;
        
        let discourses = self.parse(tokens)?;
        if self.options.stop_after == Some(CompilerStage::Parsing) { return Ok(()); }

        let hir_discourses = self.lower_hir(discourses)?;
        if self.options.stop_after == Some(CompilerStage::Analysis) { return Ok(()); }

        let mir = self.lower_mir(hir_discourses)?;
        if self.options.stop_after == Some(CompilerStage::Mir) { return Ok(()); }

        let ir = self.emit_ir(mir)?;
        if self.options.stop_after == Some(CompilerStage::Codegen) { return Ok(()); }

        let stem = std::path::Path::new(path).file_stem().unwrap().to_str().unwrap();
        let ll_path = format!("{}.ll", stem);
        let prog_path = format!("{}_bin", stem);

        self.env.write_file(&ll_path, &ir)?;
        self.realize(&ll_path, &prog_path)?;

        Ok(())
    }

    pub fn lex(&self, source: &str) -> Result<Vec<crate::application::ports::compiler_ports::Token>, OnuError> {
        self.lexer.lex(source)
    }

    pub fn scan_headers(&mut self, tokens: &[crate::application::ports::compiler_ports::Token]) -> Result<(), OnuError> {
        self.parser.scan_headers(tokens, &mut self.registry)
    }

    pub fn parse(&self, tokens: Vec<crate::application::ports::compiler_ports::Token>) -> Result<Vec<Discourse>, OnuError> {
        self.parser.parse_with_registry(tokens, &self.registry)
    }

    pub fn lower_hir(&self, discourses: Vec<Discourse>) -> Result<Vec<HirDiscourse>, OnuError> {
        let analysis_service = AnalysisService::new(&self.env, &self.registry);
        let mut hir_discourses = Vec::new();
        for discourse in discourses {
            let mut hir = LoweringService::lower_discourse(&discourse, &self.registry);
            analysis_service.analyze_discourse(&mut hir)?;
            if self.options.emit_hir { self.env.log(LogLevel::Debug, &format!("HIR Emit: {:?}", hir)); }
            hir_discourses.push(hir);
        }
        Ok(hir_discourses)
    }

    pub fn lower_mir(&self, hir_discourses: Vec<HirDiscourse>) -> Result<crate::domain::entities::mir::MirProgram, OnuError> {
        let mir_lowering_service = MirLoweringService::new(&self.env, &self.registry);
        mir_lowering_service.lower_program(&hir_discourses)
    }

    pub fn emit_ir(&mut self, mir: crate::domain::entities::mir::MirProgram) -> Result<String, OnuError> {
        self.env.log(LogLevel::Info, "Starting Codegen stage.");
        self.codegen.set_registry(self.registry.clone());
        let ir = self.codegen.generate(&mir)?;
        self.env.log(LogLevel::Debug, &format!("Generated LLVM IR:\n{}", ir));
        Ok(ir)
    }

    fn realize(&self, bitcode_path: &str, output_path: &str) -> Result<(), OnuError> {
        self.env.log(LogLevel::Info, &format!("Realizing binary: {} -> {}", bitcode_path, output_path));
        // Link bitcode natively
        self.env.run_command("clang", &[bitcode_path, "-O3", "-o", output_path, "-Wno-override-module"])?;
        Ok(())
    }
}
