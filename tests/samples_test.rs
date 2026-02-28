use onu_refactor::CompilationPipeline;
use onu_refactor::infrastructure::os::NativeOsEnvironment;
use onu_refactor::adapters::codegen::OnuCodegen;
use onu_refactor::application::options::{CompilationOptions, LogLevel};
use std::process::Command;
use std::path::Path;

fn run_sample_test(sample_name: &str) {
    let mut options = CompilationOptions::default();
    options.log_level = LogLevel::Trace; // Enable granular logging for tests
    options.emit_hir = true;
    options.emit_tokens = true;
    
    let env = NativeOsEnvironment::new(options.log_level);
    let codegen = OnuCodegen::new();
    let mut pipeline = CompilationPipeline::new(env, codegen, options);
    
    let sample_path = format!("samples/{}.onu", sample_name);
    assert!(Path::new(&sample_path).exists(), "Sample file not found: {}", sample_path);
    
    // Compile
    pipeline.compile(&sample_path).expect(&format!("Failed to compile {}", sample_name));
    
    // Execute the generated binary
    let prog_path = format!("./{}_bin", sample_name);
    let output = Command::new(&prog_path)
        .output()
        .expect(&format!("Failed to execute {}", sample_name));
    
    assert!(output.status.success(), "Execution of {} failed. Output: {}", sample_name, String::from_utf8_lossy(&output.stderr));
}

macro_rules! sample_test {
    ($name:ident) => {
        #[test]
        fn $name() {
            run_sample_test(stringify!($name));
        }
    };
}

// Ported Samples from original 'onu'
sample_test!(hello_world);
sample_test!(factorial);
sample_test!(fibonacci);
sample_test!(parity);
sample_test!(sample);
sample_test!(collatz);
