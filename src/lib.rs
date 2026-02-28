pub mod domain;
pub mod application;
pub mod infrastructure;
pub mod adapters;

use crate::application::use_cases::registry_service::RegistryService;
use crate::application::use_cases::analysis_service::AnalysisService;
use crate::application::use_cases::lowering_service::LoweringService;
use crate::application::use_cases::mir_lowering_service::MirLoweringService;
use crate::application::use_cases::layout_service::LayoutService;
use crate::application::use_cases::module_service::ModuleService;
use crate::application::ports::compiler_ports::{LexerPort, ParserPort, CodegenPort};
use crate::application::ports::environment::EnvironmentPort;
use crate::application::options::{CompilationOptions, CompilerStage, LogLevel};
use crate::domain::entities::error::OnuError;
use crate::domain::entities::ast::Discourse;
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
    pub module_service: ModuleService,
}

impl<E: EnvironmentPort, C: CodegenPort> CompilationPipeline<E, C> {
    pub fn new(env: E, codegen: C, options: CompilationOptions) -> Self {
        let mut registry = RegistryService::new();
        registry.log_level = options.log_level;
        let module_service = ModuleService::new(options.log_level);
        
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
            module_service,
        }
    }

    pub fn compile(&mut self, path: &str) -> Result<(), OnuError> {
        self.env.log(LogLevel::Info, &format!("Starting compilation for: {}", path));

        // 1. Lexing
        let source = self.env.read_file(path)?;
        let raw_tokens = self.lexer.lex(&source)?;
        
        // 2. Layout (Linguistic Indentation)
        let mut layout_service = LayoutService::new(self.options.log_level);
        let tokens = layout_service.process(&source, raw_tokens);
        if self.options.stop_after == Some(CompilerStage::Lexing) { return Ok(()); }

        // 3. Parsing (AST)
        let discourses = self.parser.parse(tokens)?;
        
        // 2.5 Register signatures before analysis
        for discourse in &discourses {
            if let Discourse::Behavior { header, .. } = discourse {
                let sig = crate::domain::entities::registry::BehaviorSignature {
                    input_types: header.takes.iter().map(|a| a.type_info.onu_type.clone()).collect(),
                    return_type: header.delivers.0.clone(),
                    arg_is_observation: header.takes.iter().map(|a| a.type_info.is_observation).collect(),
                };
                self.registry.symbols_mut().add_signature(&header.name, sig);
            }
        }
        
        if self.options.stop_after == Some(CompilerStage::Parsing) { return Ok(()); }

        // 3. Analysis & Semantic Validation (HIR)
        let analysis_service = AnalysisService::new(&self.registry);
        let mut hir_discourses = Vec::new();
        for discourse in discourses {
            let mut hir = LoweringService::lower_discourse(&discourse, &self.registry);
            analysis_service.analyze_discourse(&mut hir)?;
            if self.options.emit_hir { self.env.log(LogLevel::Debug, &format!("HIR Emit: {:?}", hir)); }
            hir_discourses.push(hir);
        }
        if self.options.stop_after == Some(CompilerStage::Analysis) { return Ok(()); }

        // 4. MIR Lowering
        let mir_lowering_service = MirLoweringService::new(&self.env);
        let mir = mir_lowering_service.lower_program(&hir_discourses)?;
        if self.options.stop_after == Some(CompilerStage::Mir) { return Ok(()); }

        // 5. Codegen
        self.env.log(LogLevel::Info, "Starting Codegen stage.");
        self.codegen.set_registry(self.registry.clone());
        let ir = self.codegen.generate(&mir)?;
        
        // Log the final LLVM IR
        self.env.log(LogLevel::Debug, &format!("Generated LLVM IR:\n{}", ir));
        
        let stem = std::path::Path::new(path).file_stem().unwrap().to_str().unwrap();
        let ll_path = format!("{}.ll", stem);
        let prog_path = format!("{}_bin", stem);

        self.env.write_file(&ll_path, &ir)?;
        if self.options.stop_after == Some(CompilerStage::Codegen) { return Ok(()); }

        // 6. Realization (Linking)
        self.realize(&ll_path, &prog_path)?;

        Ok(())
    }

    fn realize(&self, bitcode_path: &str, output_path: &str) -> Result<(), OnuError> {
        self.env.log(LogLevel::Info, &format!("Realizing binary: {} -> {}", bitcode_path, output_path));
        // Link bitcode and the C runtime
        self.env.run_command("clang", &[bitcode_path, "runtime.c", "-o", output_path, "-Wno-override-module"])?;
        Ok(())
    }
}
