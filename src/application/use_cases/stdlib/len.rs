use crate::domain::entities::types::OnuType;
use crate::domain::entities::mir::{MirOperand, MirLiteral};
use crate::application::use_cases::mir_builder::MirBuilder;
use super::StdlibOpLowerer;

pub struct LenLowerer;

impl StdlibOpLowerer for LenLowerer {
    fn name(&self) -> &str { "len" }

    fn lower(&self, args: Vec<MirOperand>, builder: &mut MirBuilder) -> MirOperand {
        if args.len() != 1 { panic!("len requires 1 argument"); }
        let dest = builder.new_ssa();
        builder.set_ssa_type(dest, OnuType::I64);
        builder.build_index(dest, args[0].clone(), 0);
        MirOperand::Variable(dest, false)
    }
}
