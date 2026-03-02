use crate::domain::entities::mir::{MirInstruction, MirOperand, MirLiteral, MirBinOp};
use crate::domain::entities::types::OnuType;
use crate::application::use_cases::mir_builder::MirBuilder;
use super::StdlibOpLowerer;

pub struct InitOfLowerer;

impl StdlibOpLowerer for InitOfLowerer {
    fn name(&self) -> &str { "init-of" }

    fn lower(&self, args: Vec<MirOperand>, builder: &mut MirBuilder) -> MirOperand {
        if args.len() != 1 { panic!("init-of requires 1 argument"); }
        let str_op = &args[0];

        let len_ssa = builder.new_ssa();
        builder.set_ssa_type(len_ssa, OnuType::I64);
        builder.build_index(len_ssa, str_op.clone(), 0);

        let new_len_ssa = builder.new_ssa();
        builder.set_ssa_type(new_len_ssa, OnuType::I64);
        builder.build_binop(new_len_ssa, MirBinOp::Sub, MirOperand::Variable(len_ssa, false), MirOperand::Constant(MirLiteral::I64(1)));

        let ptr_ssa = builder.new_ssa();
        builder.set_ssa_type(ptr_ssa, OnuType::Nothing);
        builder.build_index(ptr_ssa, str_op.clone(), 1);

        let is_dyn_ssa = builder.new_ssa();
        builder.set_ssa_type(is_dyn_ssa, OnuType::Boolean);
        builder.build_index(is_dyn_ssa, str_op.clone(), 2);

        let dest = builder.new_ssa();
        builder.set_ssa_type(dest, OnuType::Strings);
        builder.emit(MirInstruction::Tuple {
            dest,
            elements: vec![
                MirOperand::Variable(new_len_ssa, false),
                MirOperand::Variable(ptr_ssa, false),
                MirOperand::Variable(is_dyn_ssa, false),
            ],
        });

        MirOperand::Variable(dest, false)
    }
}
