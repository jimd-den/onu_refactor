use crate::domain::entities::types::OnuType;
use crate::domain::entities::mir::MirInstruction;
use crate::domain::entities::mir::{MirOperand, MirLiteral};
use crate::application::use_cases::mir_builder::MirBuilder;
use super::StdlibOpLowerer;

pub struct CharAtLowerer;

impl StdlibOpLowerer for CharAtLowerer {
    fn name(&self) -> &str { "char-at" }

    fn lower(&self, args: Vec<MirOperand>, builder: &mut MirBuilder) -> MirOperand {
        if args.len() != 2 { panic!("char-at requires 2 arguments"); }
        let str_op = &args[0];
        let idx_op = &args[1];

        let ptr_ssa = builder.new_ssa();
        builder.set_ssa_type(ptr_ssa, OnuType::Nothing);
        builder.build_index(ptr_ssa, str_op.clone(), 1);

        let char_ptr_ssa = builder.new_ssa();
        builder.set_ssa_type(char_ptr_ssa, OnuType::Nothing);
        builder.build_pointer_offset(char_ptr_ssa, MirOperand::Variable(ptr_ssa, false), idx_op.clone());

        let res_ssa = builder.new_ssa();
        builder.set_ssa_type(res_ssa, OnuType::I64);
        builder.emit(MirInstruction::Index { dest: res_ssa, subject: MirOperand::Variable(char_ptr_ssa, false), index: 0 });
        
        MirOperand::Variable(res_ssa, false)
    }
}
