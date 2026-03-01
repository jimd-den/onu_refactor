/// Ọ̀nụ Liveness Analysis: Domain Rule
///
/// This implements backward dataflow analysis to determine
/// the last use of a variable. This is critical for
/// implementing linear types and resource management.

use crate::domain::entities::hir::HirExpression;
use std::collections::HashSet;

pub struct LivenessRule;

impl LivenessRule {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze(&self, expr: &mut HirExpression) {
        let mut live_vars = HashSet::new();
        self.visit_backward(expr, &mut live_vars);
    }

    fn visit_backward(&self, expr: &mut HirExpression, live_vars: &mut HashSet<String>) {
        match expr {
            HirExpression::Variable(name, is_consuming) => {
                if !live_vars.contains(name) {
                    *is_consuming = true;
                    live_vars.insert(name.clone());
                } else {
                    *is_consuming = false;
                }
            }
            HirExpression::Call { args, .. } => {
                for arg in args.iter_mut().rev() {
                    self.visit_backward(arg, live_vars);
                }
            }
            HirExpression::Derivation { name, value, body, .. } => {
                self.visit_backward(body, live_vars);
                live_vars.remove(name);
                self.visit_backward(value, live_vars);
            }
            HirExpression::If { condition, then_branch, else_branch } => {
                let mut then_live = live_vars.clone();
                let mut else_live = live_vars.clone();
                
                self.visit_backward(then_branch, &mut then_live);
                self.visit_backward(else_branch, &mut else_live);
                
                live_vars.clear();
                live_vars.extend(then_live);
                live_vars.extend(else_live);
                
                self.visit_backward(condition, live_vars);
            }
            HirExpression::Block(exprs) => {
                for e in exprs.iter_mut().rev() {
                    self.visit_backward(e, live_vars);
                }
            }
            HirExpression::Emit(e) => {
                self.visit_backward(e, live_vars);
            }
            HirExpression::Tuple(elements) => {
                for e in elements.iter_mut().rev() {
                    self.visit_backward(e, live_vars);
                }
            }
            HirExpression::Index { subject, .. } => {
                self.visit_backward(subject, live_vars);
            }
            HirExpression::ActsAs { subject, .. } => {
                self.visit_backward(subject, live_vars);
            }
            HirExpression::BinaryOp { left, right, .. } => {
                self.visit_backward(right, live_vars);
                self.visit_backward(left, live_vars);
            }
            HirExpression::Drop(e) => {
                self.visit_backward(e, live_vars);
            }
            _ => {}
        }
    }
}
