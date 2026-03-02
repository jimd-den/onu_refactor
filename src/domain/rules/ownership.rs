/// Ọ̀nụ Ownership Rules: Domain Logic
///
/// This implements the "Legal Custody" rules of the language.
/// It ensures that resources (Strings, Matrices, Arrays) are not
/// used after their custody has been relinquished.

use crate::domain::entities::hir::{HirExpression, HirBehaviorHeader};
use crate::domain::entities::types::OnuType;
use crate::domain::entities::error::OnuError;
use crate::domain::entities::registry::BehaviorRegistryPort;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableStatus {
    Available,
    Consumed,
}

pub struct OwnershipRule<'a> {
    pub registry: &'a dyn BehaviorRegistryPort,
}

impl<'a> OwnershipRule<'a> {
    pub fn new(registry: &'a dyn BehaviorRegistryPort) -> Self {
        Self { registry }
    }

    pub fn validate(&self, header: &HirBehaviorHeader, body: &mut HirExpression) -> Result<(), OnuError> {
        let mut env = HashMap::new();
        for arg in &header.args {
            env.insert(arg.name.clone(), (arg.typ.clone(), VariableStatus::Available));
        }
        self.visit_and_mutate_expression(body, &mut env)?;

        // Scope ends: Any remaining resources in the environment must be dropped explicitly.
        let mut drops_to_insert = Vec::new();
        for (name, (typ, status)) in env.iter() {
            if *status == VariableStatus::Available && typ.is_resource() {
                drops_to_insert.push(name.clone());
            }
        }

        if !drops_to_insert.is_empty() {
            let old_body = std::mem::replace(body, HirExpression::Literal(crate::domain::entities::hir::HirLiteral::Nothing));
            let mut block_exprs = vec![old_body];
            for name in drops_to_insert {
                block_exprs.push(HirExpression::Drop(Box::new(HirExpression::Variable(name, true))));
            }
            *body = HirExpression::Block(block_exprs);
        }

        Ok(())
    }

    fn visit_and_mutate_expression(&self, expr: &mut HirExpression, env: &mut HashMap<String, (OnuType, VariableStatus)>) -> Result<(), OnuError> {
        match expr {
            HirExpression::Variable(name, _) => {
                if let Some((_, status)) = env.get(name) {
                    if *status == VariableStatus::Consumed {
                        return Err(OnuError::ResourceViolation {
                            message: format!("Legal Custody Violation: '{}' has already been relinquished.", name),
                            span: Default::default(),
                        });
                    }
                }
                Ok(())
            }
            HirExpression::Call { name, args } => {
                let sig = self.registry.get_signature(name);
                for (i, arg) in args.iter_mut().enumerate() {
                    self.visit_and_mutate_expression(arg, env)?;
                    
                    let is_observation = sig.and_then(|s| s.arg_is_observation.get(i)).copied().unwrap_or(false);
                    if !is_observation {
                        if let HirExpression::Variable(vname, _) = arg {
                            if let Some((typ, status)) = env.get_mut(vname) {
                                if typ.is_resource() {
                                    *status = VariableStatus::Consumed;
                                }
                            }
                        }
                    }
                }
                Ok(())
            }
            HirExpression::Derivation { name, value, body, typ } => {
                self.visit_and_mutate_expression(value, env)?;
                env.insert(name.clone(), (typ.clone(), VariableStatus::Available));
                self.visit_and_mutate_expression(body, env)?;

                // End of derivation scope: check if the derived resource is still available
                if let Some((t, status)) = env.remove(name) {
                    if status == VariableStatus::Available && t.is_resource() {
                        // We must append a Drop block to the body of this derivation
                        let old_body = std::mem::replace(body.as_mut(), HirExpression::Literal(crate::domain::entities::hir::HirLiteral::Nothing));
                        *body = Box::new(HirExpression::Block(vec![
                            old_body,
                            HirExpression::Drop(Box::new(HirExpression::Variable(name.clone(), true)))
                        ]));
                    }
                }
                Ok(())
            }
            HirExpression::If { condition, then_branch, else_branch } => {
                self.visit_and_mutate_expression(condition, env)?;
                let mut then_env = env.clone();
                self.visit_and_mutate_expression(then_branch, &mut then_env)?;
                let mut else_env = env.clone();
                self.visit_and_mutate_expression(else_branch, &mut else_env)?;

                // Reconcile environments: if consumed in either branch, it's consumed.
                for (name, (_, status)) in env.iter_mut() {
                    let then_status = then_env.get(name).map(|(_, s)| s).unwrap_or(&VariableStatus::Available);
                    let else_status = else_env.get(name).map(|(_, s)| s).unwrap_or(&VariableStatus::Available);
                    if *then_status == VariableStatus::Consumed || *else_status == VariableStatus::Consumed {
                        *status = VariableStatus::Consumed;
                    }
                }
                Ok(())
            }
            HirExpression::Block(exprs) => {
                for e in exprs.iter_mut() { self.visit_and_mutate_expression(e, env)?; }
                Ok(())
            }
            HirExpression::Emit(e) => {
                self.visit_and_mutate_expression(e, env)?;
                // Emit takes custody of the resource!
                if let HirExpression::Variable(vname, _) = e.as_ref() {
                    if let Some((typ, status)) = env.get_mut(vname) {
                        if typ.is_resource() {
                            *status = VariableStatus::Consumed;
                        }
                    }
                }
                Ok(())
            }
            HirExpression::Drop(e) => {
                self.visit_and_mutate_expression(e, env)?;
                if let HirExpression::Variable(vname, _) = e.as_ref() {
                    if let Some((typ, status)) = env.get_mut(vname) {
                        if typ.is_resource() {
                            *status = VariableStatus::Consumed;
                        }
                    }
                }
                Ok(())
            }
            HirExpression::BinaryOp { left, right, .. } => {
                self.visit_and_mutate_expression(left, env)?;
                self.visit_and_mutate_expression(right, env)?;
                Ok(())
            }
            HirExpression::Tuple(elements) => {
                for e in elements { self.visit_and_mutate_expression(e, env)?; }
                Ok(())
            }
            HirExpression::Index { subject, .. } => {
                self.visit_and_mutate_expression(subject, env)?;
                Ok(())
            }
            _ => Ok(())
        }
    }
}
