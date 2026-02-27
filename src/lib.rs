/// Ọ̀nụ Refactor: Library Core
///
/// This is the entry point for the library, providing a unified
/// interface for the compiler's functionality.

pub mod domain;
pub mod application;
pub mod adapters;
pub mod infrastructure;

use crate::application::use_cases::registry_service::RegistryService;
use crate::application::use_cases::analysis_service::AnalysisService;
use crate::application::use_cases::lowering_service::LoweringService;
use crate::application::use_cases::mir_lowering_service::MirLoweringService;
use crate::application::use_cases::layout_service::LayoutService;
use crate::application::use_cases::module_service::ModuleService;
use crate::application::ports::compiler_ports::{LexerPort, ParserPort, CodegenPort};
use crate::application::ports::environment::EnvironmentPort;
use crate::application::options::{CompilationOptions, CompilerStage};
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
        let module_service = ModuleService::new();
        
        // Register Built-in Modules
        module_service.register_module(&mut registry, &CoreModule);
        module_service.register_module(&mut registry, &StandardMathModule);
        module_service.register_module(&mut registry, &OnuIoModule);

        Self {
            env,
            codegen,
            options,
            registry,
            lexer: OnuLexer,
            parser: OnuParser,
            module_service,
        }
    }

    /// Orchestrates the compilation of a source file.
    pub fn compile(&mut self, path: &str) -> Result<(), OnuError> {
        self.env.log(&format!("Starting compilation for: {}", path));

        // 1. Lexing
        let source = self.env.read_file(path)?;
        let raw_tokens = self.lexer.lex(&source)?;
        
        // 1.5 Layout Analysis (New Stage)
        let mut layout_service = LayoutService::new();
        let tokens = layout_service.process(&source, raw_tokens);
        
        if self.options.emit_tokens {
            for t in &tokens { self.env.log(&format!("TOKEN: {:?}", t)); }
        }
        
        if self.options.verbose { self.env.log(&format!("Lexing/Layout successful: {} tokens", tokens.len())); }
        if self.options.stop_after == Some(CompilerStage::Lexing) { return Ok(()); }

        // 2. Parsing (AST Generation)
        let discourses = self.parser.parse(tokens)?;
        if self.options.verbose { self.env.log(&format!("Parsing successful: {} discourse units", discourses.len())); }
        
        // 2.5 Register signatures before analysis
        for discourse in &discourses {
            if let Discourse::Behavior { header, .. } = discourse {
                let sig = crate::domain::entities::registry::BehaviorSignature {
                    input_types: header.takes.iter().map(|a| a.type_info.onu_type.clone()).collect(),
                    return_type: header.delivers.0.clone(),
                    arg_is_observation: header.takes.iter().map(|a| a.type_info.is_observation).collect(),
                };
                self.registry.add_signature(&header.name, sig);
            }
        }

        if self.options.stop_after == Some(CompilerStage::Parsing) { return Ok(()); }

        // 3. Analysis & Semantic Validation (HIR)
        let analysis_service = AnalysisService::new(&self.registry);
        let mut hir_discourses = Vec::new();
        for discourse in discourses {
            let mut hir = LoweringService::lower_discourse(&discourse);
            analysis_service.analyze_discourse(&mut hir)?;
            if self.options.emit_hir { self.env.log(&format!("HIR Emit: {:?}", hir)); }
            hir_discourses.push(hir);
        }
        if self.options.stop_after == Some(CompilerStage::Analysis) { return Ok(()); }

        // 4. MIR Lowering
        let mir_service = MirLoweringService::new();
        let mir = mir_service.lower_program(&hir_discourses);
        if self.options.emit_mir { self.env.log(&format!("MIR Emit: {:?}", mir)); }
        if self.options.stop_after == Some(CompilerStage::MirLowering) { return Ok(()); }

        // 5. Codegen
        if self.options.verbose { self.env.log("Starting Codegen stage."); }
        // Pass the registry to codegen so strategies can look up arity/signatures
        self.codegen.set_registry(self.registry.clone());
        let bitcode = self.codegen.generate(&mir)?;
        self.env.write_binary("output.bc", &bitcode)?;
        if self.options.stop_after == Some(CompilerStage::Codegen) { return Ok(()); }

        // 6. Realization (Linking)
        self.realize("output.bc", "onu_prog")?;

        Ok(())
    }

    fn realize(&self, bitcode_path: &str, output_path: &str) -> Result<(), OnuError> {
        self.env.log(&format!("Realizing binary: {} -> {}", bitcode_path, output_path));
        // Link bitcode and the C runtime
        self.env.run_command("clang", &[bitcode_path, "runtime.c", "-o", output_path])?;
        Ok(())
    }
}
