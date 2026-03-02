use crate::domain::entities::mir::MirOperand;
use crate::application::use_cases::mir_builder::MirBuilder;
use std::collections::HashMap;

pub mod joined_with;
pub mod as_text;
pub mod duplicated_as;
pub mod set_char;
pub mod char_at;
pub mod len;
pub mod char_from_code;
pub mod init_of;

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
        Self { ops }
    }

    pub fn get(&self, name: &str) -> Option<&dyn StdlibOpLowerer> {
        self.ops.get(name).map(|b| b.as_ref())
    }
}
