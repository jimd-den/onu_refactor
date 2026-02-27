/// Ọ̀nụ IO Extension: Infrastructure Layer
///
/// This implements the Ọ̀nụ-IO built-in module, providing
/// terminal and CLI argument capabilities.

use crate::application::ports::compiler_ports::ExtensionPort;
use crate::domain::entities::registry::{BuiltInModule, SymbolTable, BehaviorSignature};
use crate::domain::entities::types::OnuType;

pub struct OnuIoModule;

impl BuiltInModule for OnuIoModule {
    fn name(&self) -> &str { "Ọ̀nụ-IO" }
    fn register(&self, table: &mut SymbolTable) {
        let io_verbs = vec![
            ("broadcasts", BehaviorSignature { 
                input_types: vec![OnuType::Strings], 
                return_type: OnuType::Nothing, 
                arg_is_observation: vec![true] 
            }),
            ("receives-argument", BehaviorSignature { 
                input_types: vec![OnuType::I64], 
                return_type: OnuType::Strings, 
                arg_is_observation: vec![false] 
            }),
            ("argument-count", BehaviorSignature { 
                input_types: vec![], 
                return_type: OnuType::I64, 
                arg_is_observation: vec![] 
            }),
            ("receives-line", BehaviorSignature { 
                input_types: vec![], 
                return_type: OnuType::Strings, 
                arg_is_observation: vec![] 
            }),
        ];
        
        for (name, sig) in io_verbs {
            table.add_signature(name, sig);
            table.mark_implemented(name);
        }
    }
}

impl ExtensionPort for OnuIoModule {
    fn realization_id(&self) -> &str { "io" }
}
