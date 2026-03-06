use crate::application::options::LogLevel;
/// Ọ̀nụ Registry Service: Application Layer Orchestration
///
/// This service coordinates the domain-level SymbolTable and SemanticEngine.
/// It acts as the primary interface for the compiler's compilation stages.
use crate::domain::entities::registry::{BehaviorRegistryPort, BehaviorSignature, SymbolTable};
use crate::domain::entities::types::OnuType;
use chrono::Local;
use std::collections::HashMap;

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
        self.log(
            LogLevel::Trace,
            &format!("Looking up signature for: {}", name),
        );
        self.symbols.get_signature(name)
    }

    pub fn add_shape(
        &mut self,
        name: &str,
        fields: Vec<(String, OnuType)>,
        behaviors: Vec<(String, BehaviorSignature)>,
    ) {
        self.log(LogLevel::Debug, &format!("Adding shape: {}", name));
        self.shapes
            .insert(name.to_string(), ShapeDefinition { fields, behaviors });
    }

    pub fn is_shape(&self, name: &str) -> bool {
        let found = self.shapes.contains_key(name);
        eprintln!("[DEBUG] Checking if {} is a shape: {}", name, found);
        found
    }

    pub fn get_shape(&self, name: &str) -> Option<&ShapeDefinition> {
        let res = self.shapes.get(name);
        eprintln!(
            "[DEBUG] Looking up shape definition for {}: {}",
            name,
            res.is_some()
        );
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

    /// Returns the size in bytes of a given OnuType.
    /// Follows C-style packing/alignment for Tuples and Shapes.
    pub fn size_of(&self, typ: &OnuType) -> usize {
        match typ {
            OnuType::I8 | OnuType::U8 | OnuType::Boolean => 1,
            OnuType::I16 | OnuType::U16 => 2,
            OnuType::I32 | OnuType::U32 | OnuType::F32 => 4,
            OnuType::I64 | OnuType::U64 | OnuType::F64 | OnuType::Ptr => 8,
            OnuType::I128 | OnuType::U128 => 16,
            OnuType::WideInt(bits) => (*bits as usize + 7) / 8,
            OnuType::Strings => {
                // Strings is { i64 len, i8* ptr, i1 is_dynamic }
                // We calculate this dynamically to ensure alignment is handled correctly.
                let fields = vec![OnuType::I64, OnuType::Ptr, OnuType::Boolean];
                self.size_of(&OnuType::Tuple(fields))
            }
            OnuType::Matrix => 8, // Pointer to heap structure
            OnuType::Nothing => 0,
            OnuType::Tuple(elements) => {
                let mut offset = 0;
                for elem in elements {
                    let size = self.size_of(elem);
                    let align = self.align_of(elem);
                    offset = (offset + align - 1) & !(align - 1); // Align
                    offset += size;
                }
                let total_align = self.align_of(typ);
                (offset + total_align - 1) & !(total_align - 1) // Final padding
            }
            OnuType::Shape(name) => {
                if let Some(def) = self.get_shape(name) {
                    let mut offset = 0;
                    for (_, ftype) in &def.fields {
                        let size = self.size_of(ftype);
                        let align = self.align_of(ftype);
                        offset = (offset + align - 1) & !(align - 1);
                        offset += size;
                    }
                    let total_align = self.align_of(typ);
                    (offset + total_align - 1) & !(total_align - 1)
                } else {
                    8 // Default to pointer/i64 for unknown shapes
                }
            }
            OnuType::Array(_) | OnuType::HashMap(_, _) | OnuType::Tree(_) => 8, // Reference types
        }
    }

    /// Returns the data alignment requirement of a given OnuType.
    pub fn align_of(&self, typ: &OnuType) -> usize {
        match typ {
            OnuType::I8 | OnuType::U8 | OnuType::Boolean => 1,
            OnuType::I16 | OnuType::U16 => 2,
            OnuType::I32 | OnuType::U32 | OnuType::F32 => 4,
            OnuType::I64 | OnuType::U64 | OnuType::F64 | OnuType::Ptr => 8,
            OnuType::I128 | OnuType::U128 => 16,
            OnuType::WideInt(bits) => (*bits as usize + 7) / 8,
            OnuType::Strings => 8, // Max alignment of { i64, i8*, i1 } is 8
            OnuType::Matrix => 8,
            OnuType::Nothing => 1,
            OnuType::Tuple(elements) => {
                elements.iter().map(|e| self.align_of(e)).max().unwrap_or(1)
            }
            OnuType::Shape(name) => {
                if let Some(def) = self.get_shape(name) {
                    def.fields
                        .iter()
                        .map(|(_, t)| self.align_of(t))
                        .max()
                        .unwrap_or(1)
                } else {
                    8
                }
            }
            OnuType::Array(_) | OnuType::HashMap(_, _) | OnuType::Tree(_) => 8,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_size_and_alignment() {
        let registry = RegistryService::new();
        let string_type = OnuType::Strings;

        // Strings is { i64, i8*, i1 }
        // Alignment should be 8
        assert_eq!(registry.align_of(&string_type), 8);

        // Size should be 24 (8 + 8 + 1, padded to 8-byte boundary)
        assert_eq!(registry.size_of(&string_type), 24);
    }

    #[test]
    fn test_tuple_alignment() {
        let registry = RegistryService::new();
        let tuple = OnuType::Tuple(vec![OnuType::I8, OnuType::I64, OnuType::I8]);

        // { i8, (7 bytes padding), i64, i8, (7 bytes padding) } => 24 bytes
        assert_eq!(registry.align_of(&tuple), 8);
        assert_eq!(registry.size_of(&tuple), 24);
    }
}
