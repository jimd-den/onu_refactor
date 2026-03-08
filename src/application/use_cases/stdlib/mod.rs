use crate::domain::entities::mir::MirOperand;
use crate::domain::entities::types::OnuType;
use crate::domain::entities::registry::BehaviorSignature;
use crate::application::use_cases::mir_builder::MirBuilder;
use crate::application::use_cases::registry_service::RegistryService;
use std::collections::HashMap;

pub mod joined_with;
pub mod as_text;
pub mod duplicated_as;
pub mod set_char;
pub mod char_at;
pub mod len;
pub mod char_from_code;
pub mod init_of;
pub mod sha256_k;
pub mod write_hex_word;

pub trait StdlibOpLowerer {
    fn name(&self) -> &str;
    fn lower(
        &self,
        args: Vec<MirOperand>,
        builder: &mut MirBuilder,
    ) -> MirOperand;
}

pub struct StdlibOpRegistry {
    ops: HashMap<String, Box<dyn StdlibOpLowerer>>,
}

impl StdlibOpRegistry {
    pub fn new() -> Self {
        let mut ops: HashMap<String, Box<dyn StdlibOpLowerer>> = HashMap::new();
        ops.insert("joined-with".into(), Box::new(joined_with::JoinedWithLowerer));
        ops.insert("as-text".into(), Box::new(as_text::AsTextLowerer));
        ops.insert("duplicated-as".into(), Box::new(duplicated_as::DuplicatedAsLowerer));
        ops.insert("set-char".into(), Box::new(set_char::SetCharLowerer));
        ops.insert("char-at".into(), Box::new(char_at::CharAtLowerer));
        ops.insert("len".into(), Box::new(len::LenLowerer));
        ops.insert("char-from-code".into(), Box::new(char_from_code::CharFromCodeLowerer));
        ops.insert("init-of".into(), Box::new(init_of::InitOfLowerer));
        ops.insert("sha256-k-table".into(), Box::new(sha256_k::Sha256KTableLowerer));
        ops.insert("write-hex-word".into(), Box::new(write_hex_word::WriteHexWordLowerer));
        Self { ops }
    }

    pub fn get(&self, name: &str) -> Option<&dyn StdlibOpLowerer> {
        self.ops.get(name).map(|b| b.as_ref())
    }

    /// Pre-register the arity signatures of multi-arg stdlib ops into the
    /// registry so the parser knows how many arguments to consume.
    /// Must be called before `scan_headers` / `parse_with_registry`.
    pub fn register_signatures(registry: &mut RegistryService) {
        let sym = registry.symbols_mut();
        // write-hex-word: (string buf, integer word, integer base_offset) -> string
        sym.add_signature("write-hex-word", BehaviorSignature {
            input_types: vec![OnuType::Strings, OnuType::I64, OnuType::I64],
            return_type: OnuType::Strings,
            arg_is_observation: vec![true, false, false],
        });
        // sha256-k-table: (integer t) -> integer
        sym.add_signature("sha256-k-table", BehaviorSignature {
            input_types: vec![OnuType::I64],
            return_type: OnuType::I64,
            arg_is_observation: vec![false],
        });
    }
}
