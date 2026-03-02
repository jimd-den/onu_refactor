use crate::domain::entities::mir::{MirOperand, MirLiteral};
use crate::domain::entities::types::OnuType;
use crate::application::use_cases::mir_builder::MirBuilder;
use super::StdlibOpLowerer;

pub struct CharFromCodeLowerer;

impl StdlibOpLowerer for CharFromCodeLowerer {
    fn name(&self) -> &str { "char-from-code" }

    fn lower(&self, args: Vec<MirOperand>, builder: &mut MirBuilder) -> MirOperand {
        if args.len() != 1 { panic!("char-from-code requires 1 argument"); }
        let code_op = &args[0];

        let buf_ssa = builder.new_ssa();
        builder.set_ssa_type(buf_ssa, OnuType::Nothing);
        builder.build_alloc(buf_ssa, MirOperand::Constant(MirLiteral::I64(1)));

        builder.build_store(MirOperand::Variable(buf_ssa, false), code_op.clone());

        let dest = builder.new_ssa();
        builder.set_ssa_type(dest, OnuType::Strings);
        builder.build_string_tuple(dest, MirOperand::Constant(MirLiteral::I64(1)), MirOperand::Variable(buf_ssa, false), true);

        MirOperand::Variable(dest, false)
    }
}
