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
    let mut pipeline = CompilationPipeline::new(env, codegen, options);
    
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
    let mut pipeline = CompilationPipeline::new(env, codegen, options);
    
    let sample_path = "samples/mutual_recursion.onu";
    pipeline.compile(sample_path).expect("Failed to compile mutual_recursion.onu");

    let ir = fs::read_to_string("mutual_recursion.ll").expect("Failed to read IR file");
    // Depending on names, hyphens might be preserved or replaced. 
    // Usually Onu hyphenated names are preserved in MIR/LLVM unless specified.
    assert!(ir.contains("tail call fastcc i64 @even"), "IR should contain 'tail call' for even");
    assert!(ir.contains("tail call fastcc i64 @odd"), "IR should contain 'tail call' for odd");

    let binary_path = "./mutual_recursion_bin";
    let output = Command::new(binary_path)
        .output()
        .expect("Failed to execute mutual_recursion_bin");

    assert!(output.status.success(), "Mutual recursion should succeed with TCO");
}
