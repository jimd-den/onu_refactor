use crate::domain::entities::mir::{MirOperand, MirLiteral};
use crate::application::use_cases::mir_builder::MirBuilder;
use super::StdlibOpLowerer;

pub struct DuplicatedAsLowerer;

impl StdlibOpLowerer for DuplicatedAsLowerer {
    fn name(&self) -> &str { "duplicated-as" }

    fn lower(&self, _args: Vec<MirOperand>, _builder: &mut MirBuilder) -> MirOperand {
        // Dummy implementation since it wasn't originally in stdlib_lowering
        MirOperand::Constant(MirLiteral::Nothing)
    }
}
