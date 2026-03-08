use onu_refactor::CompilationPipeline;
use onu_refactor::infrastructure::os::NativeOsEnvironment;
use onu_refactor::adapters::codegen::OnuCodegen;
use onu_refactor::application::options::CompilationOptions;
use std::process::Command;
use std::fs;

#[test]
fn test_deep_recursion_repro() {
    let mut options = CompilationOptions::default();
    options.log_level = onu_refactor::application::options::LogLevel::Error;
    let env = NativeOsEnvironment::new(options.log_level);
    let codegen = OnuCodegen::new();
    let lexer = Box::new(onu_refactor::adapters::lexer::OnuLexer::new(options.log_level));
    let parser = Box::new(onu_refactor::adapters::parser::OnuParser::new(options.log_level));
    let mut pipeline = CompilationPipeline::new(env, codegen, lexer, parser, options);
    
    let sample_path = "samples/deep_recursion.onu";
    pipeline.compile(sample_path).expect("Failed to compile deep_recursion.onu");

    // CHECK FOR TAIL IN IR
    let ir = fs::read_to_string("deep_recursion.ll").expect("Failed to read IR file");
    assert!(ir.contains("tail call"), "Generated IR should contain 'tail call' for TCO");

    let binary_path = "./deep_recursion_bin";
    let output = Command::new(binary_path)
        .output()
        .expect("Failed to execute deep_recursion_bin");

    assert!(output.status.success(), "Deep recursion should succeed with TCO");
}

#[test]
fn test_mutual_recursion_tco() {
    let mut options = CompilationOptions::default();
    options.log_level = onu_refactor::application::options::LogLevel::Error;
    let env = NativeOsEnvironment::new(options.log_level);
    let codegen = OnuCodegen::new();
    let lexer = Box::new(onu_refactor::adapters::lexer::OnuLexer::new(options.log_level));
    let parser = Box::new(onu_refactor::adapters::parser::OnuParser::new(options.log_level));
    let mut pipeline = CompilationPipeline::new(env, codegen, lexer, parser, options);
    
    let sample_path = "samples/mutual_recursion.onu";
    pipeline.compile(sample_path).expect("Failed to compile mutual_recursion.onu");

    let ir = fs::read_to_string("mutual_recursion.ll").expect("Failed to read IR file");
    assert!(ir.contains("tail call"), "IR should contain 'tail call'");

    let binary_path = "./mutual_recursion_bin";
    let output = Command::new(binary_path)
        .output()
        .expect("Failed to execute mutual_recursion_bin");

    assert!(output.status.success(), "Mutual recursion should succeed with TCO");
}

#[test]
fn test_complex_args_tco() {
    let mut options = CompilationOptions::default();
    options.log_level = onu_refactor::application::options::LogLevel::Error;
    let env = NativeOsEnvironment::new(options.log_level);
    let codegen = OnuCodegen::new();
    let lexer = Box::new(onu_refactor::adapters::lexer::OnuLexer::new(options.log_level));
    let parser = Box::new(onu_refactor::adapters::parser::OnuParser::new(options.log_level));
    let mut pipeline = CompilationPipeline::new(env, codegen, lexer, parser, options);
    
    let sample_path = "samples/complex_args_tco.onu";
    pipeline.compile(sample_path).expect("Failed to compile complex_args_tco.onu");

    let ir = fs::read_to_string("complex_args_tco.ll").expect("Failed to read IR file");
    assert!(ir.contains("tail call"), "IR should contain 'tail call' for complex args");

    let binary_path = "./complex_args_tco_bin";
    let output = Command::new(binary_path)
        .output()
        .expect("Failed to execute complex_args_tco_bin");

    assert!(output.status.success(), "Complex args TCO should succeed");
}
