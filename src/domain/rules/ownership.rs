/// Ọ̀nụ Ownership Rules: Domain Logic
///
/// This implements the "Legal Custody" rules of the language.
/// It ensures that resources (Strings, Matrices, Arrays) are not
/// used after their custody has been relinquished.

use crate::domain::entities::hir::{HirExpression, HirBehaviorHeader, HirArgument};
use crate::domain::entities::types::OnuType;
use crate::domain::entities::error::OnuError;
use crate::application::use_cases::registry_service::RegistryService;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableStatus {
    Available,
    Consumed,
}

pub struct OwnershipRule<'a> {
    registry: &'a RegistryService,
}

impl<'a> OwnershipRule<'a> {
    pub fn new(registry: &'a RegistryService) -> Self {
        Self { registry }
    }

    pub fn validate(&self, header: &HirBehaviorHeader, body: &HirExpression) -> Result<(), OnuError> {
        let mut env = HashMap::new();
        for arg in &header.args {
            env.insert(arg.name.clone(), (arg.typ.clone(), VariableStatus::Available));
        }
        self.visit_expression(body, &mut env)
    }

    fn visit_expression(&self, expr: &HirExpression, env: &mut HashMap<String, (OnuType, VariableStatus)>) -> Result<(), OnuError> {
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
                for (i, arg) in args.iter().enumerate() {
                    self.visit_expression(arg, env)?;
                    
                    let is_observation = sig.and_then(|s| s.arg_is_observation.get(i)).copied().unwrap_or(false);
                    if !is_observation {
                        if let HirExpression::Variable(vname, _) = arg {
                            if let Some((typ, status)) = env.get_mut(vname) {
                                if matches!(typ, OnuType::Strings | OnuType::Matrix | OnuType::Array(_)) {
                                    *status = VariableStatus::Consumed;
                                }
                            }
                        }
                    }
                }
                Ok(())
            }
            HirExpression::Derivation { name, value, body, typ } => {
                self.visit_expression(value, env)?;
                env.insert(name.clone(), (typ.clone(), VariableStatus::Available));
                self.visit_expression(body, env)?;
                Ok(())
            }
            HirExpression::If { condition, then_branch, else_branch } => {
                self.visit_expression(condition, env)?;
                let mut then_env = env.clone();
                self.visit_expression(then_branch, &mut then_env)?;
                let mut else_env = env.clone();
                self.visit_expression(else_branch, &mut else_env)?;

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
                for e in exprs { self.visit_expression(e, env)?; }
                Ok(())
            }
            _ => Ok(())
        }
    }
}
