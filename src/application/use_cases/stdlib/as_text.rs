use crate::domain::entities::mir::{MirInstruction, MirOperand, MirLiteral};
use crate::domain::entities::types::OnuType;
use crate::application::use_cases::mir_builder::MirBuilder;
use super::StdlibOpLowerer;

pub struct AsTextLowerer;

impl StdlibOpLowerer for AsTextLowerer {
    fn name(&self) -> &str { "as-text" }

    fn lower(&self, args: Vec<MirOperand>, builder: &mut MirBuilder) -> MirOperand {
        if args.len() != 1 {
            panic!("as-text requires 1 argument");
        }
        let arg = &args[0];

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

        // Schedule metadata drop - central policy will ensure zero-cost if static
        builder.schedule_drop(fmt_str_ssa, OnuType::Strings);

        MirOperand::Variable(dest, false)
    }
}
