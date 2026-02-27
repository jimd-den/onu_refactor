/// Compilation Options: Application Layer
///
/// This governs the behavior of the CompilationPipeline,
/// allowing for granular control over debugging and stage execution.

#[derive(Debug, Clone, Default)]
pub struct CompilationOptions {
    pub verbose: bool,
    pub emit_tokens: bool,
    pub emit_hir: bool,
    pub emit_mir: bool,
    pub stop_after: Option<CompilerStage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompilerStage {
    Lexing,
    Parsing,
    Analysis,
    MirLowering,
    Codegen,
    Realization,
}

impl CompilerStage {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "lexing" => Some(CompilerStage::Lexing),
            "parsing" => Some(CompilerStage::Parsing),
            "analysis" => Some(CompilerStage::Analysis),
            "mir" | "mir-lowering" => Some(CompilerStage::MirLowering),
            "codegen" => Some(CompilerStage::Codegen),
            "link" | "realization" => Some(CompilerStage::Realization),
            _ => None,
        }
    }
}
