/// Integration tests for the WASM Direct Emitter.
///
/// These tests run without LLVM — they only require `cargo test`.
/// They verify that `WasmCodegenStrategy::emit_program` produces a valid
/// WebAssembly binary (magic + version) from a MIR program.

use onu_refactor::adapters::codegen::platform::wasm32::WasmCodegenStrategy;
use onu_refactor::application::options::{CompilationOptions, LogLevel};
use onu_refactor::application::ports::compiler_ports::CodegenPort;
use onu_refactor::application::use_cases::registry_service::RegistryService;
use onu_refactor::domain::entities::error::OnuError;
use onu_refactor::domain::entities::mir::MirProgram;
use onu_refactor::infrastructure::os::NativeOsEnvironment;
use onu_refactor::CompilationPipeline;

struct MockCodegen;
impl CodegenPort for MockCodegen {
    fn generate(&self, _: &MirProgram) -> Result<String, OnuError> {
        Ok(String::new())
    }
    fn set_registry(&mut self, _: RegistryService) {}
}

/// Lower the given Onu source all the way to MIR and return it.
fn compile_to_mir(source: &str) -> Result<MirProgram, OnuError> {
    let options = CompilationOptions::default();
    let env = NativeOsEnvironment::new(LogLevel::None);
    let lexer = Box::new(onu_refactor::adapters::lexer::OnuLexer::new(LogLevel::None));
    let parser = Box::new(onu_refactor::adapters::parser::OnuParser::new(LogLevel::None));
    let mut pipeline = CompilationPipeline::new(env, MockCodegen, lexer, parser, options);

    let tokens = pipeline.lex(source)?;
    pipeline.scan_headers(&tokens)?;
    let ast = pipeline.parse(tokens)?;
    let hir = pipeline.lower_hir(ast)?;
    pipeline.lower_mir(hir)
}

// ── WASM magic number ─────────────────────────────────────────────────────────

const WASM_MAGIC: &[u8] = b"\0asm";
const WASM_VERSION: &[u8] = &[1, 0, 0, 0];

fn assert_valid_wasm_header(bytes: &[u8]) {
    assert!(
        bytes.len() >= 8,
        "WASM output too short: {} bytes",
        bytes.len()
    );
    assert_eq!(
        &bytes[0..4],
        WASM_MAGIC,
        "Missing WASM magic header"
    );
    assert_eq!(
        &bytes[4..8],
        WASM_VERSION,
        "Unexpected WASM version"
    );
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn test_wasm_emit_hello_world() {
    let source = r#"
the module called HelloTest
    with concern: basic greeting

the effect behavior called run
    with intent: greet the world
    takes: nothing
    delivers: nothing
    as:
        broadcasts "Hello, World!"
"#;

    let mir = compile_to_mir(source).expect("MIR lowering should succeed");
    let wasm = WasmCodegenStrategy::emit_program(&mir)
        .expect("WASM emission should succeed");

    assert_valid_wasm_header(&wasm);
    // The binary should contain a function export named "run"
    let content = String::from_utf8_lossy(&wasm);
    assert!(
        content.contains("run"),
        "WASM binary should export 'run'"
    );
}

#[test]
fn test_wasm_emit_integer_arithmetic() {
    let source = r#"
the module called ArithTest
    with concern: basic arithmetic

the behavior called double
    with intent: double a value
    takes:
        an integer called n
    delivers: an integer
    as:
        n added-to n

the effect behavior called run
    with intent: compute double of 21
    takes: nothing
    delivers: an integer
    as:
        derivation: result derives-from an integer 21 utilizes double
        result
"#;

    let mir = compile_to_mir(source).expect("MIR lowering should succeed");
    let wasm = WasmCodegenStrategy::emit_program(&mir)
        .expect("WASM emission should succeed");

    assert_valid_wasm_header(&wasm);
}

#[test]
fn test_wasm_emit_recursive_function() {
    let source = r#"
the module called FibTest
    with concern: fibonacci

the behavior called fib
    with intent: compute fibonacci number
    takes:
        an integer called n
    delivers: an integer
    with diminishing: n
    as:
        if n matches 0
            then 0
            else if n matches 1
                then 1
                else
                    derivation: a derives-from (n decreased-by 1) utilizes fib
                    derivation: b derives-from (n decreased-by 2) utilizes fib
                    a added-to b

the effect behavior called run
    with intent: compute fib 10
    takes: nothing
    delivers: an integer
    as:
        derivation: result derives-from an integer 10 utilizes fib
        result
"#;

    let mir = compile_to_mir(source).expect("MIR lowering should succeed");
    let wasm = WasmCodegenStrategy::emit_program(&mir)
        .expect("WASM emission should succeed");

    assert_valid_wasm_header(&wasm);
}

#[test]
fn test_wasm_emit_produces_nonempty_module() {
    let source = r#"
the module called MinTest
    with concern: minimal

the effect behavior called run
    with intent: return zero
    takes: nothing
    delivers: an integer
    as:
        0
"#;

    let mir = compile_to_mir(source).expect("MIR lowering should succeed");
    let wasm = WasmCodegenStrategy::emit_program(&mir)
        .expect("WASM emission should succeed");

    assert_valid_wasm_header(&wasm);
    // A module with at least a type, function, and code section should be > 32 bytes
    assert!(wasm.len() > 32, "WASM module seems too small: {} bytes", wasm.len());
}
