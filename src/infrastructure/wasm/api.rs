/// Ọ̀nụ WebAssembly Façade — Phase 2 of the Offline Playground
///
/// Exposes two entry points to JavaScript via `wasm-bindgen`:
///
/// * `OnuCompiler.lint(source)` — Lexer + Parser diagnostics as JSON.
/// * `OnuCompiler.compile(source)` — Full pipeline → raw `.wasm` bytes.
use wasm_bindgen::prelude::*;

use crate::adapters::codegen::platform::wasm32::WasmCodegenStrategy;
use crate::adapters::lexer::OnuLexer;
use crate::adapters::parser::OnuParser;
use crate::application::options::{CompilationOptions, LogLevel};
use crate::application::ports::compiler_ports::CodegenPort;
use crate::application::ports::environment::EnvironmentPort;
use crate::application::use_cases::registry_service::RegistryService;
use crate::domain::entities::error::{OnuError, Span};
use crate::domain::entities::mir::MirProgram;
use crate::CompilationPipeline;

// ── Null I/O environment (no filesystem in the browser) ──────────────────────

struct NullEnv;

impl EnvironmentPort for NullEnv {
    fn read_file(&self, _path: &str) -> Result<String, OnuError> {
        Err(OnuError::ResourceViolation {
            message: "no filesystem in WASM".into(),
            span: Span::default(),
        })
    }
    fn write_file(&self, _path: &str, _content: &str) -> Result<(), OnuError> {
        Ok(())
    }
    fn write_binary(&self, _path: &str, _content: &[u8]) -> Result<(), OnuError> {
        Ok(())
    }
    fn run_command(&self, _cmd: &str, _args: &[&str]) -> Result<String, OnuError> {
        Ok(String::new())
    }
    fn log(&self, _level: LogLevel, _msg: &str) {}
}

// ── No-op codegen (the WASM path bypasses LLVM entirely) ─────────────────────

struct NoopCodegen;

impl CodegenPort for NoopCodegen {
    fn generate(&self, _: &MirProgram) -> Result<String, OnuError> {
        Ok(String::new())
    }
    fn set_registry(&mut self, _: RegistryService) {}
}

// ── Public wasm-bindgen API ───────────────────────────────────────────────────

#[wasm_bindgen]
pub struct OnuCompiler;

#[wasm_bindgen]
impl OnuCompiler {
    /// Run lexer + parser and return a JSON array of diagnostic objects.
    ///
    /// Each element: `{ "message": "...", "line": 0, "col": 0 }`
    ///
    /// Returns `"[]"` when the source is error-free.
    pub fn lint(source: &str) -> String {
        let mut pipeline = build_pipeline(LogLevel::None);
        let tokens = match pipeline.lex(source) {
            Ok(t) => t,
            Err(e) => return format!("[{}]", error_to_json(&e)),
        };
        if let Err(e) = pipeline.scan_headers(&tokens) {
            return format!("[{}]", error_to_json(&e));
        }
        match pipeline.parse(tokens) {
            Ok(_) => "[]".to_string(),
            Err(e) => format!("[{}]", error_to_json(&e)),
        }
    }

    /// Compile `source` and return raw `.wasm` bytecode.
    ///
    /// The returned `Uint8Array` can be passed directly to
    /// `WebAssembly.instantiate(bytes, importObject)`.
    pub fn compile(source: &str) -> Result<js_sys::Uint8Array, JsValue> {
        let mut pipeline = build_pipeline(LogLevel::None);

        let tokens = pipeline
            .lex(source)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        pipeline
            .scan_headers(&tokens)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        let ast = pipeline
            .parse(tokens)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        let hir = pipeline
            .lower_hir(ast)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        let mir = pipeline
            .lower_mir(hir)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        let wasm_bytes = WasmCodegenStrategy::emit_program(&mir)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        Ok(js_sys::Uint8Array::from(wasm_bytes.as_slice()))
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn build_pipeline(
    log_level: LogLevel,
) -> CompilationPipeline<NullEnv, NoopCodegen> {
    let options = CompilationOptions {
        log_level,
        ..Default::default()
    };
    CompilationPipeline::new(
        NullEnv,
        NoopCodegen,
        Box::new(OnuLexer::new(log_level)),
        Box::new(OnuParser::new(log_level)),
        options,
    )
}

fn error_to_json(e: &OnuError) -> String {
    let msg = format!("{:?}", e)
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    format!(r#"{{"message":"{}","line":0,"col":0}}"#, msg)
}
