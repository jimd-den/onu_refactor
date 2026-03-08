use onu_refactor::application::options::{CompilationOptions, CompilerStage, LogLevel};
use onu_refactor::infrastructure::os::NativeOsEnvironment;
#[cfg(feature = "llvm")]
use onu_refactor::infrastructure::cli::Repl;
#[cfg(feature = "llvm")]
use onu_refactor::adapters::codegen::OnuCodegen;
use onu_refactor::CompilationPipeline;
use std::env as std_env;

fn main() {
    let args: Vec<String> = std_env::args().collect();

    // REPL mode: `onu --repl`
    #[cfg(feature = "llvm")]
    if args.get(1).map(|s| s.as_str()) == Some("--repl") {
        let mut repl = Repl::new();
        repl.run();
        return;
    }

    if args.len() < 2 {
        eprintln!("Usage: {} <source_file> [--stop-after <stage>] [--verbose]", args[0]);
        eprintln!("       {} --repl", args[0]);
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
    #[cfg(feature = "llvm")]
    let codegen = OnuCodegen::new();
    let lexer = Box::new(onu_refactor::adapters::lexer::OnuLexer::new(options.log_level));
    let parser = Box::new(onu_refactor::adapters::parser::OnuParser::new(options.log_level));
    #[cfg(feature = "llvm")]
    let mut pipeline = CompilationPipeline::new(env, codegen, lexer, parser, options);
    #[cfg(not(feature = "llvm"))]
    {
        let _ = (env, lexer, parser, options);
        eprintln!("This binary was built without LLVM support. Use the wasm feature for browser builds.");
        std::process::exit(1);
    }
    #[cfg(feature = "llvm")]
    match pipeline.compile(source_file) {
        Ok(_) => println!("Discourse Realized Successfully."),
        Err(e) => {
            eprintln!("PIPELINE ERROR: {:?}", e);
            std::process::exit(1);
        }
    }
}
