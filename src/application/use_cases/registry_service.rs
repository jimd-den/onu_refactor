/// Ọ̀nụ Registry Service: Application Layer Orchestration
///
/// This service coordinates the domain-level SymbolTable and SemanticEngine.
/// It acts as the primary interface for the compiler's compilation stages.

use crate::domain::entities::registry::{SymbolTable, BehaviorSignature, BehaviorRegistryPort};
use crate::domain::entities::types::OnuType;
use crate::application::options::LogLevel;
use std::collections::HashMap;
use chrono::Local;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShapeDefinition {
    pub fields: Vec<(String, OnuType)>,
    pub behaviors: Vec<(String, BehaviorSignature)>,
}

pub struct RegistryService {
    symbols: SymbolTable,
    shapes: HashMap<String, ShapeDefinition>,
    pub log_level: LogLevel,
}

impl BehaviorRegistryPort for RegistryService {
    fn get_signature(&self, name: &str) -> Option<&BehaviorSignature> {
        self.symbols.get_signature(name)
    }
}

impl RegistryService {
    pub fn new() -> Self {
        Self {
            symbols: SymbolTable::new(),
            shapes: HashMap::new(),
            log_level: LogLevel::Info,
        }
    }

    fn log(&self, level: LogLevel, message: &str) {
        if level <= self.log_level && level != LogLevel::None {
            let timestamp = Local::now().to_rfc3339();
            eprintln!("[{}] {:?}: [Registry] {}", timestamp, level, message);
        }
    }

    pub fn get_signature(&self, name: &str) -> Option<&BehaviorSignature> {
        self.log(LogLevel::Trace, &format!("Looking up signature for: {}", name));
        self.symbols.get_signature(name)
    }

    pub fn add_shape(&mut self, name: &str, fields: Vec<(String, OnuType)>, behaviors: Vec<(String, BehaviorSignature)>) {
        self.log(LogLevel::Debug, &format!("Adding shape: {}", name));
        self.shapes.insert(name.to_string(), ShapeDefinition { fields, behaviors });
    }

    pub fn is_shape(&self, name: &str) -> bool {
        let found = self.shapes.contains_key(name);
        eprintln!("[DEBUG] Checking if {} is a shape: {}", name, found);
        found
    }

    pub fn get_shape(&self, name: &str) -> Option<&ShapeDefinition> {
        let res = self.shapes.get(name);
        eprintln!("[DEBUG] Looking up shape definition for {}: {}", name, res.is_some());
        res
    }

    pub fn find_field(&self, name: &str) -> Option<(&String, usize)> {
        for (sname, sdef) in &self.shapes {
            if let Some(idx) = sdef.fields.iter().position(|(fname, _)| fname == name) {
                return Some((sname, idx));
            }
        }
        None
    }

    pub fn symbols_mut(&mut self) -> &mut SymbolTable {
        &mut self.symbols
    }

    pub fn mark_implemented(&mut self, name: &str) {
        self.log(LogLevel::Trace, &format!("Marking implemented: {}", name));
        self.symbols.mark_implemented(name);
    }
}

impl Clone for RegistryService {
    fn clone(&self) -> Self {
        Self {
            symbols: self.symbols.clone(),
            shapes: self.shapes.clone(),
            log_level: self.log_level,
        }
    }
}
