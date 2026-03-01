use crate::domain::entities::mir::{MirInstruction, MirOperand, MirLiteral, MirBinOp};
use crate::domain::entities::types::OnuType;
use crate::application::use_cases::mir_builder::MirBuilder;
use super::StdlibOpLowerer;

pub struct JoinedWithLowerer;

impl StdlibOpLowerer for JoinedWithLowerer {
    fn name(&self) -> &str { "joined-with" }

    fn lower(&self, args: Vec<MirOperand>, builder: &mut MirBuilder) -> MirOperand {
        if args.len() != 2 {
            panic!("joined-with requires 2 arguments");
        }
        let a = &args[0];
        let b = &args[1];

        let dest = builder.new_ssa();
        builder.set_ssa_type(dest, OnuType::Strings);

        let a_len_ssa = builder.new_ssa();
        builder.set_ssa_type(a_len_ssa, OnuType::I64);
        builder.build_index(a_len_ssa, a.clone(), 0);

        let b_len_ssa = builder.new_ssa();
        builder.set_ssa_type(b_len_ssa, OnuType::I64);
        builder.build_index(b_len_ssa, b.clone(), 0);

        let sum_len_ssa = builder.new_ssa();
        builder.set_ssa_type(sum_len_ssa, OnuType::I64);
        builder.build_binop(sum_len_ssa, MirBinOp::Add, MirOperand::Variable(a_len_ssa, false), MirOperand::Variable(b_len_ssa, false));

        let a_ptr_ssa = builder.new_ssa();
        builder.set_ssa_type(a_ptr_ssa, OnuType::Nothing);
        builder.build_index(a_ptr_ssa, a.clone(), 1);

        let b_ptr_ssa = builder.new_ssa();
        builder.set_ssa_type(b_ptr_ssa, OnuType::Nothing);
        builder.build_index(b_ptr_ssa, b.clone(), 1);

        let alloc_size_ssa = builder.new_ssa();
        builder.set_ssa_type(alloc_size_ssa, OnuType::I64);
        builder.build_binop(alloc_size_ssa, MirBinOp::Add, MirOperand::Variable(sum_len_ssa, false), MirOperand::Constant(MirLiteral::I64(1)));

        let buf_ssa = builder.new_ssa();
        builder.set_ssa_type(buf_ssa, OnuType::Nothing);
        builder.build_alloc(buf_ssa, MirOperand::Variable(alloc_size_ssa, false));

        builder.build_memcpy(
            MirOperand::Variable(buf_ssa, false),
            MirOperand::Variable(a_ptr_ssa, false),
            MirOperand::Variable(a_len_ssa, false)
        );

        let b_dest_ptr_ssa = builder.new_ssa();
        builder.set_ssa_type(b_dest_ptr_ssa, OnuType::Nothing);
        builder.build_pointer_offset(b_dest_ptr_ssa, MirOperand::Variable(buf_ssa, false), MirOperand::Variable(a_len_ssa, false));

        builder.build_memcpy(
            MirOperand::Variable(b_dest_ptr_ssa, false),
            MirOperand::Variable(b_ptr_ssa, false),
            MirOperand::Variable(b_len_ssa, false)
        );

        // Omit null-terminator for pure LLVM strings, as they rely entirely on the length field
        // rather than C-style string functions.
        builder.build_string_tuple(
            dest,
            MirOperand::Variable(sum_len_ssa, false),
            MirOperand::Variable(buf_ssa, false),
            true
        );

        MirOperand::Variable(dest, false)
    }
}
