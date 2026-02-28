/// Ọ̀nụ DRY Enforcement: Domain Rule
///
/// This implements the formal "Strict Discourse" rules which 
/// prevent duplicate definitions and semantic ambiguity.

use crate::domain::entities::ast::Discourse;
use crate::domain::entities::error::{OnuError, Span};
use std::collections::HashMap;

pub struct DryEnforcementRule {
    defined_behaviors: HashMap<String, String>, // name -> original name (with hyphens)
}

impl DryEnforcementRule {
    pub fn new() -> Self {
        Self { defined_behaviors: HashMap::new() }
    }

    pub fn validate(&mut self, discourses: &[Discourse]) -> Result<(), OnuError> {
        for discourse in discourses {
            if let Discourse::Behavior { header, .. } = discourse {
                let normalized = header.name.replace('-', "_");
                if let Some(existing_name) = self.defined_behaviors.get(&normalized) {
                    return Err(OnuError::BehaviorConflict {
                        message: format!("Behavior conflict: '{}' normalized to '{}' conflicts with existing behavior '{}'", header.name, normalized, existing_name),
                        span: Span::default(),
                    });
                }
                self.defined_behaviors.insert(normalized, header.name.clone());
            }
        }
        Ok(())
    }
}
