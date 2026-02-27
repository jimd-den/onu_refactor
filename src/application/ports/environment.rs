/// Environment Port: Application Layer Interface
///
/// This trait defines the abstract interface for OS interaction.
/// It enables the core application logic to remain decoupled from the
/// physical filesystem and OS-specific tools.

use crate::domain::entities::error::OnuError;

pub trait EnvironmentPort {
    fn read_file(&self, path: &str) -> Result<String, OnuError>;
    fn write_file(&self, path: &str, content: &str) -> Result<(), OnuError>;
    fn write_binary(&self, path: &str, content: &[u8]) -> Result<(), OnuError>;
    fn run_command(&self, command: &str, args: &[&str]) -> Result<String, OnuError>;
    fn log(&self, message: &str);
}
