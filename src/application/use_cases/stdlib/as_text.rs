use crate::domain::entities::mir::{MirInstruction, MirOperand, MirLiteral, MirBinOp, MirTerminator};
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
        let input_val = args[0].clone();

        // Constants
        let zero = MirOperand::Constant(MirLiteral::I64(0));
        let ten = MirOperand::Constant(MirLiteral::I64(10));
        let ascii_zero = MirOperand::Constant(MirLiteral::I64(48));

        // 1. Allocate buffer (max 20 digits for i64 + potential sign)
        let buf_size = MirOperand::Constant(MirLiteral::I64(32));
        let buf_ssa = builder.new_ssa();
        builder.set_ssa_type(buf_ssa, OnuType::Nothing);
        builder.build_alloc(buf_ssa, buf_size.clone());

        // 2. Setup variables for itoa
        let val_ssa = builder.new_ssa();
        builder.set_ssa_type(val_ssa, OnuType::I64);
        builder.build_assign(val_ssa, input_val.clone());

        let idx_ssa = builder.new_ssa();
        builder.set_ssa_type(idx_ssa, OnuType::I64);
        builder.build_assign(idx_ssa, MirOperand::Constant(MirLiteral::I64(30))); // Start from 30

        // Loop: while val > 0 (simplified, only positive for now)
        let loop_cond_bb = builder.create_block();
        let loop_body_bb = builder.create_block();
        let loop_end_bb = builder.create_block();

        builder.terminate(MirTerminator::Branch(loop_cond_bb));

        // -- Condition Block --
        builder.switch_to_block(loop_cond_bb);
        let is_gt_zero = builder.new_ssa();
        builder.set_ssa_type(is_gt_zero, OnuType::Boolean);
        builder.build_binop(is_gt_zero, MirBinOp::Gt, MirOperand::Variable(val_ssa, false), zero.clone());
        builder.terminate(MirTerminator::CondBranch {
            condition: MirOperand::Variable(is_gt_zero, false),
            then_block: loop_body_bb,
            else_block: loop_end_bb,
        });

        // -- Body Block --
        builder.switch_to_block(loop_body_bb);
        
        // digit = val % 10
        let quot = builder.new_ssa();
        builder.set_ssa_type(quot, OnuType::I64);
        builder.build_binop(quot, MirBinOp::Div, MirOperand::Variable(val_ssa, false), ten.clone());
        
        let rem_mul = builder.new_ssa();
        builder.set_ssa_type(rem_mul, OnuType::I64);
        builder.build_binop(rem_mul, MirBinOp::Mul, MirOperand::Variable(quot, false), ten.clone());
        
        let digit = builder.new_ssa();
        builder.set_ssa_type(digit, OnuType::I64);
        builder.build_binop(digit, MirBinOp::Sub, MirOperand::Variable(val_ssa, false), MirOperand::Variable(rem_mul, false));

        // ascii = digit + '0'
        let ascii = builder.new_ssa();
        builder.set_ssa_type(ascii, OnuType::I64);
        builder.build_binop(ascii, MirBinOp::Add, MirOperand::Variable(digit, false), ascii_zero.clone());

        // buffer[idx] = ascii
        let target_ptr = builder.new_ssa();
        builder.set_ssa_type(target_ptr, OnuType::Nothing);
        builder.build_pointer_offset(target_ptr, MirOperand::Variable(buf_ssa, false), MirOperand::Variable(idx_ssa, false));
        builder.build_store(MirOperand::Variable(target_ptr, false), MirOperand::Variable(ascii, false));
        
        // Update val = quot
        builder.build_assign(val_ssa, MirOperand::Variable(quot, false));
        
        // Update idx = idx - 1
        let next_idx = builder.new_ssa();
        builder.set_ssa_type(next_idx, OnuType::I64);
        builder.build_binop(next_idx, MirBinOp::Sub, MirOperand::Variable(idx_ssa, false), MirOperand::Constant(MirLiteral::I64(1)));
        builder.build_assign(idx_ssa, MirOperand::Variable(next_idx, false));
        
        builder.terminate(MirTerminator::Branch(loop_cond_bb));

        // -- End Block --
        builder.switch_to_block(loop_end_bb);
        
        // Handle zero case
        let is_zero = builder.new_ssa();
        builder.set_ssa_type(is_zero, OnuType::Boolean);
        builder.build_binop(is_zero, MirBinOp::Eq, input_val.clone(), zero.clone());
        
        let zero_case_bb = builder.create_block();
        let final_bb = builder.create_block();
        
        builder.terminate(MirTerminator::CondBranch {
            condition: MirOperand::Variable(is_zero, false),
            then_block: zero_case_bb,
            else_block: final_bb,
        });
        
        builder.switch_to_block(zero_case_bb);
        let zero_ptr = builder.new_ssa();
        builder.set_ssa_type(zero_ptr, OnuType::Nothing);
        builder.build_pointer_offset(zero_ptr, MirOperand::Variable(buf_ssa, false), MirOperand::Constant(MirLiteral::I64(30)));
        builder.build_store(MirOperand::Variable(zero_ptr, false), ascii_zero.clone());
        
        let final_idx_zero = builder.new_ssa();
        builder.set_ssa_type(final_idx_zero, OnuType::I64);
        builder.build_assign(final_idx_zero, MirOperand::Constant(MirLiteral::I64(29)));
        builder.terminate(MirTerminator::Branch(final_bb));

        builder.switch_to_block(final_bb);
        
        // Result string: len = 30 - idx, ptr = buf + idx + 1
        let result_len = builder.new_ssa();
        builder.set_ssa_type(result_len, OnuType::I64);
        builder.build_binop(result_len, MirBinOp::Sub, MirOperand::Constant(MirLiteral::I64(30)), MirOperand::Variable(idx_ssa, false));
        
        let start_idx = builder.new_ssa();
        builder.set_ssa_type(start_idx, OnuType::I64);
        builder.build_binop(start_idx, MirBinOp::Add, MirOperand::Variable(idx_ssa, false), MirOperand::Constant(MirLiteral::I64(1)));

        let result_ptr = builder.new_ssa();
        builder.set_ssa_type(result_ptr, OnuType::Nothing);
        builder.build_pointer_offset(result_ptr, MirOperand::Variable(buf_ssa, false), MirOperand::Variable(start_idx, false));

        let dest = builder.new_ssa();
        builder.set_ssa_type(dest, OnuType::Strings);
        builder.build_string_tuple(dest, MirOperand::Variable(result_len, false), MirOperand::Variable(result_ptr, false), true);

        MirOperand::Variable(dest, false)
    }
}
