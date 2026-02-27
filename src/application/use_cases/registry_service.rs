/// Ọ̀nụ Registry Service: Application Layer Orchestration
///
/// This service coordinates the domain-level SymbolTable and SemanticEngine.
/// It acts as the primary interface for the compiler's compilation stages.

use crate::domain::entities::registry::{SymbolTable, BehaviorSignature};
use crate::domain::rules::dry_enforcement::{SemanticEngine, SemanticHash};
use crate::domain::entities::error::OnuError;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct RegistryService {
    symbols: SymbolTable,
    semantic: SemanticEngine,
    shapes: HashMap<String, Vec<(String, BehaviorSignature)>>,
}

impl RegistryService {
    pub fn new() -> Self {
        Self {
            symbols: SymbolTable::new(),
            semantic: SemanticEngine::new(),
            shapes: HashMap::new(),
        }
    }

    /// Registers a behavior and enforces DRY rules.
    pub fn register_behavior(&mut self, name: String, hash: SemanticHash) -> Result<(), OnuError> {
        self.semantic.register_behavior(name.clone(), hash)?;
        self.symbols.add_name(&name, 0); // Arity should be updated later
        self.symbols.mark_implemented(&name);
        Ok(())
    }

    pub fn add_signature(&mut self, name: &str, signature: BehaviorSignature) {
        self.symbols.add_signature(name, signature);
    }

    pub fn get_signature(&self, name: &str) -> Option<&BehaviorSignature> {
        self.symbols.get_signature(name)
    }

    pub fn add_shape(&mut self, name: &str, behaviors: Vec<(String, BehaviorSignature)>) {
        self.shapes.insert(name.to_string(), behaviors);
    }
}
