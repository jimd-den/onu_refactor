/// Compilation Options: Application Layer
///
/// This module defines the configurable aspects of the compilation pipeline.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilationOptions {
    pub stop_after: Option<CompilerStage>,
    pub log_level: LogLevel,
    pub emit_hir: bool,
    pub emit_mir: bool,
    pub emit_tokens: bool,
    pub optimization_level: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    None = 0,
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerStage {
    Lexing,
    Parsing,
    Analysis,
    Mir,
    Codegen,
    Realization,
}

impl CompilerStage {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "lexing" => Some(CompilerStage::Lexing),
            "parsing" => Some(CompilerStage::Parsing),
            "analysis" => Some(CompilerStage::Analysis),
            "mir" => Some(CompilerStage::Mir),
            "codegen" => Some(CompilerStage::Codegen),
            "realization" => Some(CompilerStage::Realization),
            _ => None,
        }
    }
}

impl Default for CompilationOptions {
    fn default() -> Self {
        Self {
            stop_after: None,
            log_level: LogLevel::Info,
            emit_hir: false,
            emit_mir: false,
            emit_tokens: false,
            optimization_level: 0,
        }
    }
}
