/// Native OS Infrastructure: Infrastructure/Driver Implementation
///
/// This implements the EnvironmentPort for the physical host OS.
/// It uses standard Rust libraries to interact with the filesystem
/// and external toolchain.

use crate::application::ports::environment::EnvironmentPort;
use crate::application::options::LogLevel;
use crate::domain::entities::error::OnuError;
use std::fs;
use std::process::Command;
use chrono::Local;

pub struct NativeOsEnvironment {
    pub current_log_level: LogLevel,
}

impl NativeOsEnvironment {
    pub fn new(log_level: LogLevel) -> Self {
        Self { current_log_level: log_level }
    }
}

impl EnvironmentPort for NativeOsEnvironment {
    fn read_file(&self, path: &str) -> Result<String, OnuError> {
        self.log(LogLevel::Debug, &format!("Reading file: {}", path));
        fs::read_to_string(path).map_err(|e| OnuError::ResourceViolation {
            message: format!("Failed to read {}: {}", path, e),
            span: crate::domain::entities::error::Span::default(),
        })
    }

    fn write_file(&self, path: &str, content: &str) -> Result<(), OnuError> {
        self.log(LogLevel::Debug, &format!("Writing file: {}", path));
        fs::write(path, content).map_err(|e| OnuError::ResourceViolation {
            message: format!("Failed to write {}: {}", path, e),
            span: crate::domain::entities::error::Span::default(),
        })
    }

    fn write_binary(&self, path: &str, content: &[u8]) -> Result<(), OnuError> {
        self.log(LogLevel::Debug, &format!("Writing binary: {}", path));
        fs::write(path, content).map_err(|e| OnuError::ResourceViolation {
            message: format!("Failed to write binary {}: {}", path, e),
            span: crate::domain::entities::error::Span::default(),
        })
    }

    fn run_command(&self, command: &str, args: &[&str]) -> Result<String, OnuError> {
        self.log(LogLevel::Info, &format!("Executing command: {} {:?}", command, args));
        let output = Command::new(command)
            .args(args)
            .output()
            .map_err(|e| OnuError::ResourceViolation {
                message: format!("Failed to execute {}: {}", command, e),
                span: crate::domain::entities::error::Span::default(),
            })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(OnuError::ResourceViolation {
                message: format!("Command [{}] failed. Error: {}", command, String::from_utf8_lossy(&output.stderr)),
                span: crate::domain::entities::error::Span::default(),
            })
        }
    }

    fn log(&self, level: LogLevel, message: &str) {
        if level <= self.current_log_level && level != LogLevel::None {
            let timestamp = Local::now().to_rfc3339();
            eprintln!("[{}] {:?}: {}", timestamp, level, message);
        }
    }
}
