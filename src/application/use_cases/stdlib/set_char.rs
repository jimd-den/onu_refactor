use crate::domain::entities::mir::{MirInstruction, MirOperand, MirLiteral, MirBinOp};
use crate::domain::entities::types::OnuType;
use crate::application::use_cases::mir_builder::MirBuilder;
use super::StdlibOpLowerer;

pub struct SetCharLowerer;

impl StdlibOpLowerer for SetCharLowerer {
    fn name(&self) -> &str { "set-char" }

    fn lower(&self, args: Vec<MirOperand>, builder: &mut MirBuilder) -> MirOperand {
        if args.len() != 3 {
            panic!("set-char requires 3 arguments: string, index, char_code");
        }
        let str_tuple = args[0].clone();
        let index = args[1].clone();
        let char_code = args[2].clone();

        // 1. Extract the pointer from the string tuple (index 1)
        let ptr_ssa = builder.new_ssa();
        builder.set_ssa_type(ptr_ssa, OnuType::Nothing);
        builder.build_index(ptr_ssa, str_tuple.clone(), 1);

        // 2. Offset the pointer by the index
        let target_ptr = builder.new_ssa();
        builder.set_ssa_type(target_ptr, OnuType::Nothing);
        builder.build_pointer_offset(target_ptr, MirOperand::Variable(ptr_ssa, false), index);

        // 3. Store the char_code (i64) directly into the target pointer
        // The StoreStrategy automatically truncates the i64 into an i8 for the string pointer
        builder.build_store(MirOperand::Variable(target_ptr, false), char_code);

        // 4. Return the original string tuple
        str_tuple
    }
}
