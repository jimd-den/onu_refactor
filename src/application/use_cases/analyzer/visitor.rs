/// Analyzer Visitor Trait: Visitor Pattern for HIR traversal.
///
/// This trait defines the visitor interface that `SemanticAnalyzer` (and any
/// future analysis pass) must implement.  By separating the operation from
/// the HIR data structures we satisfy the Anti-God-Class constraint: analysis
/// logic lives here, not in `HirExpression` or `HirDiscourse`.
///
/// The default implementations recursively delegate to child nodes so a
/// concrete visitor only needs to override the nodes it cares about.

use crate::domain::entities::hir::{
    HirDiscourse, HirExpression, HirBehaviorHeader, HirArgument, HirLiteral,
};
use crate::domain::entities::error::Diagnostic;

/// Core visitor trait for HIR traversal.
///
/// Every `visit_*` method has a default no-op implementation so implementors
/// only need to override what they actually analyse.
pub trait AnalyzerVisitor {
    // ------------------------------------------------------------------
    // Top-level entry
    // ------------------------------------------------------------------

    fn visit_program(&mut self, discourses: &[HirDiscourse]) {
        for d in discourses {
            self.visit_discourse(d);
        }
    }

    fn visit_discourse(&mut self, discourse: &HirDiscourse) {
        match discourse {
            HirDiscourse::Module { .. } => self.visit_module(discourse),
            HirDiscourse::Shape { .. } => self.visit_shape(discourse),
            HirDiscourse::Behavior { header, body } => {
                self.visit_behavior_header(header);
                self.visit_expression(body);
            }
        }
    }

    fn visit_module(&mut self, _discourse: &HirDiscourse) {}

    fn visit_shape(&mut self, _discourse: &HirDiscourse) {}

    fn visit_behavior_header(&mut self, _header: &HirBehaviorHeader) {}

    // ------------------------------------------------------------------
    // Expressions
    // ------------------------------------------------------------------

    fn visit_expression(&mut self, expr: &HirExpression) {
        match expr {
            HirExpression::Literal(lit) => self.visit_literal(lit),
            HirExpression::Variable(name, consuming) => {
                self.visit_variable(name, *consuming);
            }
            HirExpression::Call { name, args } => {
                self.visit_call(name, args);
                for arg in args {
                    self.visit_expression(arg);
                }
            }
            HirExpression::BinaryOp { left, right, .. } => {
                self.visit_expression(left);
                self.visit_expression(right);
            }
            HirExpression::Derivation { name, typ, value, body } => {
                self.visit_derivation(name, typ, value, body);
                self.visit_expression(value);
                self.visit_expression(body);
            }
            HirExpression::If { condition, then_branch, else_branch } => {
                self.visit_expression(condition);
                self.visit_expression(then_branch);
                self.visit_expression(else_branch);
            }
            HirExpression::ActsAs { subject, .. } => {
                self.visit_expression(subject);
            }
            HirExpression::Tuple(elements) => {
                for e in elements {
                    self.visit_expression(e);
                }
            }
            HirExpression::Index { subject, .. } => {
                self.visit_expression(subject);
            }
            HirExpression::Block(stmts) => {
                for s in stmts {
                    self.visit_expression(s);
                }
            }
            HirExpression::Emit(inner) | HirExpression::Drop(inner) => {
                self.visit_expression(inner);
            }
        }
    }

    fn visit_literal(&mut self, _lit: &HirLiteral) {}

    fn visit_variable(&mut self, _name: &str, _is_consuming: bool) {}

    fn visit_call(&mut self, _name: &str, _args: &[HirExpression]) {}

    fn visit_derivation(
        &mut self,
        _name: &str,
        _typ: &crate::domain::entities::types::OnuType,
        _value: &HirExpression,
        _body: &HirExpression,
    ) {
    }

    // ------------------------------------------------------------------
    // Diagnostic collection
    // ------------------------------------------------------------------

    /// Return all diagnostics accumulated so far by this visitor.
    fn diagnostics(&self) -> &[Diagnostic];
}
