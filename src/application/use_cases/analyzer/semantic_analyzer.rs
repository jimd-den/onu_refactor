/// Semantic Analyzer: LSP-ready analysis pass over the HIR.
///
/// `SemanticAnalyzer` implements the `AnalyzerVisitor` trait to walk the HIR
/// using the Visitor Pattern.  On each `visit_discourse` call it resets its
/// per-behavior tracking state, then:
///
/// 1. Records every variable **defined** via `Derivation` or a behavior
///    argument.
/// 2. Records every variable **used** via `Variable`.
/// 3. After walking the behavior body it emits a `Warning` diagnostic for
///    every variable that was defined but never used.
///
/// This never aborts the pipeline — it only accumulates `Diagnostic`s.

use std::collections::{HashMap, HashSet};

use crate::application::use_cases::analyzer::visitor::AnalyzerVisitor;
use crate::domain::entities::error::{Diagnostic, Span};
use crate::domain::entities::hir::{
    HirBehaviorHeader, HirDiscourse, HirExpression,
};
use crate::domain::entities::types::OnuType;

// ---------------------------------------------------------------------------
// SemanticAnalyzer
// ---------------------------------------------------------------------------

/// A single-pass semantic analysis use case that emits `Warning` diagnostics
/// for variables that are defined but never referenced.
pub struct SemanticAnalyzer {
    /// All diagnostics accumulated across all visited discourses.
    diagnostics: Vec<Diagnostic>,

    // Per-behavior state (reset at the start of each behavior)

    /// Tracks defined variable names → their definition span.
    defined: HashMap<String, Span>,
    /// Tracks all variable names referenced (consumed or observed) in the body.
    used: HashSet<String>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            defined: HashMap::new(),
            used: HashSet::new(),
        }
    }

    /// Run the analysis over a slice of HIR discourses and return all
    /// accumulated `Diagnostic`s.
    pub fn analyze(discourses: &[HirDiscourse]) -> Vec<Diagnostic> {
        let mut analyzer = Self::new();
        analyzer.visit_program(discourses);
        analyzer.diagnostics
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn begin_behavior(&mut self, header: &HirBehaviorHeader) {
        self.defined.clear();
        self.used.clear();

        // Register behavior arguments as defined variables.
        for arg in &header.args {
            // Skip synthetic injected arguments produced by the lowering service.
            if arg.name.starts_with("__") {
                continue;
            }
            self.defined.insert(arg.name.clone(), Span::default());
        }
    }

    fn flush_behavior_diagnostics(&mut self) {
        for (name, span) in &self.defined {
            if !self.used.contains(name.as_str()) {
                self.diagnostics.push(
                    Diagnostic::warning(span.clone(), format!("Variable '{}' is defined but never used", name))
                        .with_hint(format!("Remove the binding '{}' or prefix it with '_' to silence this warning", name)),
                );
            }
        }
    }

}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// AnalyzerVisitor implementation
// ---------------------------------------------------------------------------

impl AnalyzerVisitor for SemanticAnalyzer {
    fn visit_discourse(&mut self, discourse: &HirDiscourse) {
        if let HirDiscourse::Behavior { header, body } = discourse {
            // Reset per-behavior state and register header arguments.
            self.begin_behavior(header);
            // Walk the body through the visitor, which will call
            // `visit_variable` and `visit_derivation` as the tree is traversed.
            self.visit_expression(body);
            // Emit warnings for everything that was defined but never used.
            self.flush_behavior_diagnostics();
        }
        // Modules and Shapes don't introduce local variable bindings.
    }

    fn visit_variable(&mut self, name: &str, _is_consuming: bool) {
        self.used.insert(name.to_string());
    }

    fn visit_derivation(
        &mut self,
        name: &str,
        _typ: &OnuType,
        _value: &HirExpression,
        _body: &HirExpression,
    ) {
        self.defined.insert(name.to_string(), Span::default());
    }

    fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::hir::{HirBehaviorHeader, HirArgument, HirLiteral};
    use crate::domain::entities::types::OnuType;
    use crate::domain::entities::error::Severity;

    fn make_behavior(
        name: &str,
        args: Vec<HirArgument>,
        body: HirExpression,
    ) -> HirDiscourse {
        HirDiscourse::Behavior {
            header: HirBehaviorHeader {
                name: name.to_string(),
                is_effect: false,
                args,
                return_type: OnuType::I64,
                diminishing: None,
                memo_cache_size: None,
            },
            body,
        }
    }

    #[test]
    fn test_no_warning_when_variable_is_used() {
        // derivation x = 5; x  (x is used)
        let body = HirExpression::Derivation {
            name: "x".to_string(),
            typ: OnuType::I64,
            value: Box::new(HirExpression::Literal(HirLiteral::I64(5))),
            body: Box::new(HirExpression::Variable("x".to_string(), true)),
        };
        let discourse = make_behavior("test", vec![], body);
        let diags = SemanticAnalyzer::analyze(&[discourse]);
        assert!(diags.is_empty(), "Expected no warnings but got: {:?}", diags);
    }

    #[test]
    fn test_warning_when_variable_is_unused() {
        // derivation x = 5; nothing  (x is never used)
        let body = HirExpression::Derivation {
            name: "x".to_string(),
            typ: OnuType::I64,
            value: Box::new(HirExpression::Literal(HirLiteral::I64(5))),
            body: Box::new(HirExpression::Literal(HirLiteral::Nothing)),
        };
        let discourse = make_behavior("test", vec![], body);
        let diags = SemanticAnalyzer::analyze(&[discourse]);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
        assert!(diags[0].message.contains("'x'"));
        assert!(diags[0].actionable_hint.is_some());
    }

    #[test]
    fn test_unused_argument_emits_warning() {
        // behavior takes arg `n` but body returns literal without using `n`
        let args = vec![HirArgument {
            name: "n".to_string(),
            typ: OnuType::I64,
            is_observation: false,
        }];
        let body = HirExpression::Literal(HirLiteral::I64(42));
        let discourse = make_behavior("test", args, body);
        let diags = SemanticAnalyzer::analyze(&[discourse]);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
        assert!(diags[0].message.contains("'n'"));
    }

    #[test]
    fn test_used_argument_no_warning() {
        let args = vec![HirArgument {
            name: "n".to_string(),
            typ: OnuType::I64,
            is_observation: false,
        }];
        // body = n  (uses the argument)
        let body = HirExpression::Variable("n".to_string(), true);
        let discourse = make_behavior("test", args, body);
        let diags = SemanticAnalyzer::analyze(&[discourse]);
        assert!(diags.is_empty(), "Expected no warnings");
    }

    #[test]
    fn test_module_discourse_produces_no_warning() {
        let discourse = HirDiscourse::Module {
            name: "Foo".to_string(),
            concern: "testing".to_string(),
        };
        let diags = SemanticAnalyzer::analyze(&[discourse]);
        assert!(diags.is_empty());
    }
}
