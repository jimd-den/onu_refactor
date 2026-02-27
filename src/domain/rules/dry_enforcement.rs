/// Ọ̀nụ DRY Enforcement: Domain Rule
///
/// This implements the Principle of Non-Repetition.
/// If two behaviors are semantically identical, it constitutes
/// a violation of domain logic.

use crate::domain::entities::error::OnuError;
use std::collections::HashMap;

/// A semantic hash represents the structural uniqueness of an AST node.
pub type SemanticHash = u64;

/// The SemanticEngine handles structural uniqueness and DRY enforcement.
#[derive(Debug, Clone, Default)]
pub struct SemanticEngine {
    entries: HashMap<SemanticHash, String>,
}

impl SemanticEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new behavior hash. Returns an error if the hash already exists.
    pub fn register_behavior(&mut self, name: String, hash: SemanticHash) -> Result<(), OnuError> {
        if let Some(existing_name) = self.entries.get(&hash) {
            if *existing_name != name {
                return Err(OnuError::BehaviorConflict {
                    name,
                    other_name: existing_name.clone(),
                });
            }
        }
        self.entries.insert(hash, name);
        Ok(())
    }
}
