use crate::domain::entities::mir::{MirOperand, MirLiteral};
use crate::application::use_cases::mir_builder::MirBuilder;
use super::StdlibOpLowerer;

pub struct LenLowerer;

impl StdlibOpLowerer for LenLowerer {
    fn name(&self) -> &str { "len" }

    fn lower(&self, _args: Vec<MirOperand>, _builder: &mut MirBuilder) -> MirOperand {
        // Dummy implementation since it wasn't originally in stdlib_lowering
        MirOperand::Constant(MirLiteral::Nothing)
    }
}
