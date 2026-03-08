/// Analyzer Use Cases: Visitor-based HIR analysis passes.
///
/// - `visitor`: the `AnalyzerVisitor` trait (Visitor Pattern interface).
/// - `semantic_analyzer`: concrete `SemanticAnalyzer` use case (unused-variable
///   warnings).

pub mod visitor;
pub mod semantic_analyzer;

pub use semantic_analyzer::SemanticAnalyzer;
