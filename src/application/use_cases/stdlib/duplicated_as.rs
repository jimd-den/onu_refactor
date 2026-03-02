use crate::domain::entities::mir::{MirInstruction, MirOperand, MirLiteral};
use crate::domain::entities::types::OnuType;
use crate::application::use_cases::mir_builder::MirBuilder;
use super::StdlibOpLowerer;

pub struct DuplicatedAsLowerer;

impl StdlibOpLowerer for DuplicatedAsLowerer {
    fn name(&self) -> &str { "duplicated-as" }

    fn lower(&self, args: Vec<MirOperand>, builder: &mut MirBuilder) -> MirOperand {
        if args.len() != 1 { panic!("duplicated-as requires 1 argument"); }
        let str_op = &args[0];

        // 1. Get original length and pointer
        let len_ssa = builder.new_ssa();
        builder.set_ssa_type(len_ssa, OnuType::I64);
        builder.build_index(len_ssa, str_op.clone(), 0);

        let ptr_ssa = builder.new_ssa();
        builder.set_ssa_type(ptr_ssa, OnuType::Nothing);
        builder.build_index(ptr_ssa, str_op.clone(), 1);

        // 2. Allocate new buffer
        let new_ptr_ssa = builder.new_ssa();
        builder.set_ssa_type(new_ptr_ssa, OnuType::Nothing);
        builder.build_alloc(new_ptr_ssa, MirOperand::Variable(len_ssa, false));

        // 3. Memcpy
        builder.emit(MirInstruction::MemCopy {
            dest: MirOperand::Variable(new_ptr_ssa, false),
            src: MirOperand::Variable(ptr_ssa, false),
            size: MirOperand::Variable(len_ssa, false),
        });

        // 4. Return new string tuple (dynamic = true)
        let dest = builder.new_ssa();
        builder.set_ssa_type(dest, OnuType::Strings);
        builder.build_string_tuple(
            dest,
            MirOperand::Variable(len_ssa, false),
            MirOperand::Variable(new_ptr_ssa, false),
            true
        );

        MirOperand::Variable(dest, false)
    }
}
