/// Ọ̀nụ Refactor: Main Application
///
/// This is the entry point for the Ọ̀nụ compiler refactor.
/// It uses the infrastructure layer to bootstrap the application.

use onu_refactor::infrastructure::os::NativeOsEnvironment;
use onu_refactor::infrastructure::cli::OnuCliParser;
use onu_refactor::adapters::codegen::OnuCodegen;
use onu_refactor::CompilationPipeline;

fn main() {
    // 1. Parse CLI Arguments (Infrastructure)
    let (source_file, options) = match OnuCliParser::parse() {
        Ok(res) => res,
        Err(e) => {
            eprintln!("ERROR: {}", e);
            std::process::exit(1);
        }
    };

    // 2. Setup Environment and Adapters
    let env = NativeOsEnvironment;
    let codegen = OnuCodegen::new();

    // 3. Initialize Pipeline (Application)
    let mut pipeline = CompilationPipeline::new(env, codegen, options);

    // 4. Run Execution
    match pipeline.compile(&source_file) {
        Ok(_) => println!("Discourse Realized Successfully."),
        Err(e) => {
            eprintln!("PIPELINE ERROR: {:?}", e);
            std::process::exit(1);
        }
    }
}
