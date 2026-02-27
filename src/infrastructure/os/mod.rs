/// Native OS Infrastructure: Infrastructure/Driver Implementation
///
/// This implements the EnvironmentPort for the physical host OS.
/// It uses standard Rust libraries to interact with the filesystem
/// and external toolchains (like clang/llvm).

use crate::application::ports::environment::EnvironmentPort;
use crate::domain::entities::error::{OnuError, Span};
use std::fs;
use std::process::Command;

pub struct NativeOsEnvironment;

impl EnvironmentPort for NativeOsEnvironment {
    fn read_file(&self, path: &str) -> Result<String, OnuError> {
        fs::read_to_string(path).map_err(|e| OnuError::ResourceViolation {
            message: format!("Failed to read file: {}. Cause: {}", path, e),
            span: Span::default(),
        })
    }

    fn write_file(&self, path: &str, content: &str) -> Result<(), OnuError> {
        fs::write(path, content).map_err(|e| OnuError::ResourceViolation {
            message: format!("Failed to write file: {}. Cause: {}", path, e),
            span: Span::default(),
        })
    }

    fn write_binary(&self, path: &str, content: &[u8]) -> Result<(), OnuError> {
        fs::write(path, content).map_err(|e| OnuError::ResourceViolation {
            message: format!("Failed to write binary file: {}. Cause: {}", path, e),
            span: Span::default(),
        })
    }

    fn run_command(&self, command: &str, args: &[&str]) -> Result<String, OnuError> {
        let output = Command::new(command)
            .args(args)
            .output()
            .map_err(|e| OnuError::ResourceViolation {
                message: format!("Failed to run command: {}. Cause: {}", command, e),
                span: Span::default(),
            })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(OnuError::ResourceViolation {
                message: format!(
                    "Command [{}] failed. Error: {}",
                    command,
                    String::from_utf8_lossy(&output.stderr)
                ),
                span: Span::default(),
            })
        }
    }

    fn log(&self, message: &str) {
        // In a production app, this would use a logging framework (e.g., tracing).
        // For Onu's academic aesthetic, we keep it simple.
        eprintln!("[LOG] {}", message);
    }
}
