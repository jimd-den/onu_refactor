/// Ọ̀nụ Core Types: The Domain Logic Layer
///
/// This module defines the formal type system of Ọ̀nụ.
/// Following Clean Architecture, types are first-class domain entities.

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum OnuType {
    // --- Integers ---
    I8, I16, I32, I64, I128,
    U8, U16, U32, U64, U128,
    
    // --- Floats ---
    F32, F64,
    
    // --- Boolean ---
    Boolean,

    // --- Other Primitives ---
    Strings,
    Matrix,
    Nothing,       // The void type
    
    // --- Structural ---
    Tuple(Vec<OnuType>), // Fixed-size collection
    Array(Box<OnuType>), // Variable-size collection
    HashMap(Box<OnuType>, Box<OnuType>), // Key-Value pair collection
    Tree(Box<OnuType>), // Ordered collection
    
    // --- Abstract ---
    Shape(String), // Reference to a Shape (Interface)
}

impl OnuType {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "integer" | "i64" => Some(OnuType::I64),
            "float" | "f64" => Some(OnuType::F64),
            "boolean" => Some(OnuType::Boolean),
            "string" => Some(OnuType::Strings),
            "nothing" => Some(OnuType::Nothing),
            _ => None,
        }
    }

    /// Returns true if this type is passed by reference/custody.
    pub fn is_resource(&self) -> bool {
        matches!(self, OnuType::Strings | OnuType::Matrix | OnuType::Array(_) | OnuType::HashMap(_, _) | OnuType::Tree(_))
    }
}
