use onu_refactor::CompilationPipeline;
use onu_refactor::infrastructure::os::NativeOsEnvironment;
use onu_refactor::adapters::codegen::OnuCodegen;
use onu_refactor::application::options::{CompilationOptions, LogLevel};

#[test]
fn repro_factorial_compilation() {
    let mut options = CompilationOptions::default();
    options.log_level = LogLevel::Trace;
    
    let env = NativeOsEnvironment::new(options.log_level);
    let codegen = OnuCodegen::new();
    let lexer = Box::new(onu_refactor::adapters::lexer::OnuLexer::new(options.log_level));
    let parser = Box::new(onu_refactor::adapters::parser::OnuParser::new(options.log_level));
    let mut pipeline = CompilationPipeline::new(env, codegen, lexer, parser, options);
    
    let sample_path = "samples/factorial.onu";
    
    // This is expected to fail with the current bug
    let result = pipeline.compile(sample_path);
    assert!(result.is_ok(), "Compilation of factorial.onu failed: {:?}", result.err());
}
