/// Ọ̀nụ Core Modules: Domain Layer
///
/// This module implements the standard library of Ọ̀nụ as built-in modules.
/// These provide the fundamental operations for strings, numbers, and collections.

use crate::domain::entities::registry::{BuiltInModule, SymbolTable, BehaviorSignature};
use crate::domain::entities::types::OnuType;

pub struct CoreModule;

impl BuiltInModule for CoreModule {
    fn name(&self) -> &str { "Core" }
    fn register(&self, table: &mut SymbolTable) {
        let core_builtins = vec![
            ("joined-with", BehaviorSignature { input_types: vec![OnuType::Strings, OnuType::Strings], return_type: OnuType::Strings, arg_is_observation: vec![true, true] }),
            ("len", BehaviorSignature { input_types: vec![OnuType::Strings], return_type: OnuType::I64, arg_is_observation: vec![true] }),
            ("char-at", BehaviorSignature { input_types: vec![OnuType::Strings, OnuType::I64], return_type: OnuType::I64, arg_is_observation: vec![true, false] }),
            ("as-text", BehaviorSignature { input_types: vec![OnuType::I64], return_type: OnuType::Strings, arg_is_observation: vec![false] }),
            ("set-char", BehaviorSignature { input_types: vec![OnuType::Strings, OnuType::I64, OnuType::I64], return_type: OnuType::Strings, arg_is_observation: vec![false, false, false] }),
            ("inplace-set-char", BehaviorSignature { input_types: vec![OnuType::Strings, OnuType::I64, OnuType::I64], return_type: OnuType::Strings, arg_is_observation: vec![false, false, false] }),
            ("tail-of", BehaviorSignature { input_types: vec![OnuType::Strings], return_type: OnuType::Strings, arg_is_observation: vec![false] }),
            ("init-of", BehaviorSignature { input_types: vec![OnuType::Strings], return_type: OnuType::Strings, arg_is_observation: vec![false] }),
            ("char-from-code", BehaviorSignature { input_types: vec![OnuType::I64], return_type: OnuType::Strings, arg_is_observation: vec![false] }),
            ("duplicated-as", BehaviorSignature { input_types: vec![OnuType::Strings], return_type: OnuType::Strings, arg_is_observation: vec![true] }),
            ("clears", BehaviorSignature { input_types: vec![OnuType::Nothing], return_type: OnuType::Nothing, arg_is_observation: vec![false] }),
            ("creates-map", BehaviorSignature { input_types: vec![], return_type: OnuType::HashMap(Box::new(OnuType::Nothing), Box::new(OnuType::Nothing)), arg_is_observation: vec![] }),
            ("creates-tree", BehaviorSignature { input_types: vec![], return_type: OnuType::Tree(Box::new(OnuType::Nothing)), arg_is_observation: vec![] }),
            ("as-integer", BehaviorSignature { input_types: vec![OnuType::Strings], return_type: OnuType::I64, arg_is_observation: vec![true] }),
            ("receives-entropy", BehaviorSignature { input_types: vec![], return_type: OnuType::I64, arg_is_observation: vec![] }),
        ];
        for (name, sig) in core_builtins {
            table.add_signature(name, sig);
            table.mark_implemented(name);
        }
    }
}

pub struct StandardMathModule;

impl BuiltInModule for StandardMathModule {
    fn name(&self) -> &str { "StandardMath" }
    fn register(&self, table: &mut SymbolTable) {
        let math_signatures = vec![
            ("added-to", BehaviorSignature { input_types: vec![OnuType::I64, OnuType::I64], return_type: OnuType::I64, arg_is_observation: vec![false, false] }),
            ("decreased-by", BehaviorSignature { input_types: vec![OnuType::I64, OnuType::I64], return_type: OnuType::I64, arg_is_observation: vec![false, false] }),
            ("scales-by", BehaviorSignature { input_types: vec![OnuType::I64, OnuType::I64], return_type: OnuType::I64, arg_is_observation: vec![false, false] }),
            ("partitions-by", BehaviorSignature { input_types: vec![OnuType::I64, OnuType::I64], return_type: OnuType::I64, arg_is_observation: vec![false, false] }),
            ("matches", BehaviorSignature { input_types: vec![OnuType::I64, OnuType::I64], return_type: OnuType::I64, arg_is_observation: vec![false, false] }),
            ("exceeds", BehaviorSignature { input_types: vec![OnuType::I64, OnuType::I64], return_type: OnuType::I64, arg_is_observation: vec![false, false] }),
            ("falls-short-of", BehaviorSignature { input_types: vec![OnuType::I64, OnuType::I64], return_type: OnuType::I64, arg_is_observation: vec![false, false] }),
        ];
        for (name, sig) in math_signatures {
            table.add_signature(name, sig);
            table.mark_implemented(name);
        }
    }
}
