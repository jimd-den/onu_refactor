/// Standard Library MIR Lowering: Application Layer Helper
///
/// This module encapsulates the logic for lowering specific built-in standard library
/// functions directly into raw zero-cost MIR memory operations, bypassing the need
/// for a heavy C-based runtime library.

use crate::domain::entities::mir::{MirInstruction, MirOperand, MirLiteral, MirBinOp};
use crate::domain::entities::types::OnuType;
use crate::application::use_cases::mir_builder::MirBuilder;

pub struct StdlibLowering;

impl StdlibLowering {
    pub fn lower_as_text(arg: &MirOperand, builder: &mut MirBuilder) -> MirOperand {
        let dest = builder.new_ssa();
        builder.set_ssa_type(dest, OnuType::Strings);

        let alloc_size_ssa = builder.new_ssa();
        builder.set_ssa_type(alloc_size_ssa, OnuType::I64);
        builder.build_assign(alloc_size_ssa, MirOperand::Constant(MirLiteral::I64(32)));

        let buf_ssa = builder.new_ssa();
        builder.set_ssa_type(buf_ssa, OnuType::Nothing);
        builder.build_alloc(buf_ssa, MirOperand::Variable(alloc_size_ssa, false));

        let fmt_str_ssa = builder.new_ssa();
        builder.set_ssa_type(fmt_str_ssa, OnuType::Strings);
        builder.build_assign(fmt_str_ssa, MirOperand::Constant(MirLiteral::Text("%lld".to_string())));

        let fmt_str_ptr_ssa = builder.new_ssa();
        builder.set_ssa_type(fmt_str_ptr_ssa, OnuType::Nothing);
        builder.build_index(fmt_str_ptr_ssa, MirOperand::Variable(fmt_str_ssa, false), 1);

        let sprintf_ret_ssa = builder.new_ssa();
        builder.set_ssa_type(sprintf_ret_ssa, OnuType::I32);
        builder.emit(MirInstruction::Call {
            dest: sprintf_ret_ssa,
            name: "sprintf".to_string(),
            args: vec![
                MirOperand::Variable(buf_ssa, false),
                MirOperand::Variable(fmt_str_ptr_ssa, false),
                arg.clone()
            ],
            return_type: OnuType::I32,
            arg_types: vec![OnuType::Nothing, OnuType::Nothing, OnuType::I64],
        });

        let cast_len_ssa = builder.new_ssa();
        builder.set_ssa_type(cast_len_ssa, OnuType::I64);
        builder.emit(MirInstruction::Call {
            dest: cast_len_ssa,
            name: "strlen".to_string(),
            args: vec![MirOperand::Variable(buf_ssa, false)],
            return_type: OnuType::I64,
            arg_types: vec![OnuType::Nothing],
        });

        builder.build_string_tuple(
            dest,
            MirOperand::Variable(cast_len_ssa, false),
            MirOperand::Variable(buf_ssa, false),
            true
        );

        // Cleanup temporary constant string metadata to prevent leaks
        builder.emit(MirInstruction::Drop { ssa_var: fmt_str_ssa, typ: OnuType::Strings, name: "fmt_str_metadata".to_string() });

        MirOperand::Variable(dest, false)
    }

    pub fn lower_joined_with(a: &MirOperand, b: &MirOperand, builder: &mut MirBuilder) -> MirOperand {
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

        let null_dest_ptr_ssa = builder.new_ssa();
        builder.set_ssa_type(null_dest_ptr_ssa, OnuType::Nothing);
        builder.build_pointer_offset(null_dest_ptr_ssa, MirOperand::Variable(buf_ssa, false), MirOperand::Variable(sum_len_ssa, false));

        let null_char_ptr_ssa = builder.new_ssa();
        builder.set_ssa_type(null_char_ptr_ssa, OnuType::Strings);
        builder.build_assign(null_char_ptr_ssa, MirOperand::Constant(MirLiteral::Text("".to_string())));

        let null_char_str_ptr_ssa = builder.new_ssa();
        builder.set_ssa_type(null_char_str_ptr_ssa, OnuType::Nothing);
        builder.build_index(null_char_str_ptr_ssa, MirOperand::Variable(null_char_ptr_ssa, false), 1);

        builder.build_memcpy(
            MirOperand::Variable(null_dest_ptr_ssa, false),
            MirOperand::Variable(null_char_str_ptr_ssa, false),
            MirOperand::Constant(MirLiteral::I64(1))
        );

        builder.build_string_tuple(
            dest,
            MirOperand::Variable(sum_len_ssa, false),
            MirOperand::Variable(buf_ssa, false),
            true
        );

        // Cleanup temporary constant string metadata
        builder.emit(MirInstruction::Drop { ssa_var: null_char_ptr_ssa, typ: OnuType::Strings, name: "null_char_metadata".to_string() });

        MirOperand::Variable(dest, false)
    }
}
