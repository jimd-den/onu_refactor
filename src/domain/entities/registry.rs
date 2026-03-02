/// Ọ̀nụ Semantic Registry Entities: Data Structures for State.
///
/// SymbolTable handles name resolution, arities, and signatures.
/// These are the core domain objects that represent the "truth"
/// about the program's defined behaviors.

use crate::domain::entities::types::OnuType;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BehaviorSignature {
    pub input_types: Vec<OnuType>,
    pub return_type: OnuType,
    /// Tracks whether each input argument is passed via observation (borrowed).
    pub arg_is_observation: Vec<bool>,
}

pub trait BehaviorRegistryPort {
    fn get_signature(&self, name: &str) -> Option<&BehaviorSignature>;
}

impl BehaviorRegistryPort for SymbolTable {
    fn get_signature(&self, name: &str) -> Option<&BehaviorSignature> {
        self.signatures.get(name)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SymbolTable {
    names: HashSet<String>,
    implemented_names: HashSet<String>,
    arities: HashMap<String, usize>,
    signatures: HashMap<String, BehaviorSignature>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
    }

    pub fn is_implemented(&self, name: &str) -> bool {
        self.implemented_names.contains(name)
    }

    pub fn mark_implemented(&mut self, name: &str) {
        self.implemented_names.insert(name.to_string());
    }

    pub fn add_name(&mut self, name: &str, arity: usize) {
        self.names.insert(name.to_string());
        self.arities.insert(name.to_string(), arity);
    }

    pub fn add_signature(&mut self, name: &str, signature: BehaviorSignature) {
        eprintln!("[DEBUG] Adding signature to SymbolTable: {}", name);
        self.names.insert(name.to_string());
        self.arities.insert(name.to_string(), signature.input_types.len());
        self.signatures.insert(name.to_string(), signature);
    }

    pub fn get_signature(&self, name: &str) -> Option<&BehaviorSignature> {
        let res = self.signatures.get(name);
        if res.is_none() {
            eprintln!("[DEBUG] SymbolTable: signature NOT FOUND for {}, available: {:?}", name, self.signatures.keys());
        }
        res
    }

    pub fn get_arity(&self, name: &str) -> Option<usize> {
        self.arities.get(name).copied()
    }
}

pub trait BuiltInModule {
    fn name(&self) -> &str;
    fn register(&self, table: &mut SymbolTable);
}

pub trait Extension: BuiltInModule {
    fn realization_id(&self) -> &str;
}
