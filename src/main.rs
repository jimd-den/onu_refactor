use onu_refactor::application::options::{CompilationOptions, CompilerStage, LogLevel};
use onu_refactor::infrastructure::os::NativeOsEnvironment;
use onu_refactor::adapters::codegen::OnuCodegen;
use onu_refactor::CompilationPipeline;
use std::env as std_env;

fn main() {
    let args: Vec<String> = std_env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <source_file> [--stop-after <stage>] [--verbose]", args[0]);
        std::process::exit(1);
    }

    let source_file = &args[1];
    let mut options = CompilationOptions::default();

    let mut i = 2;
    while i < args.len() {
        if args[i] == "--stop-after" && i + 1 < args.len() {
            options.stop_after = CompilerStage::from_str(&args[i+1]);
            i += 1;
        } else if args[i] == "--verbose" {
            options.log_level = LogLevel::Debug;
        }
        i += 1;
    }

    let env = NativeOsEnvironment::new(options.log_level);
    let codegen = OnuCodegen::new();
    let mut pipeline = CompilationPipeline::new(env, codegen, options);

    match pipeline.compile(source_file) {
        Ok(_) => println!("Discourse Realized Successfully."),
        Err(e) => {
            eprintln!("PIPELINE ERROR: {:?}", e);
            std::process::exit(1);
        }
    }
}
