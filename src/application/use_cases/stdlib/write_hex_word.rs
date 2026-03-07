/// `write-hex-word` stdlib op.
///
/// Signature: `(string buf, integer word, integer base_offset) -> string`
///
/// Writes the 8 lower-case hex characters representing the 32-bit integer
/// `word` into the mutable arena string `buf` starting at byte `base_offset`.
/// Returns the same `buf` (in-place mutation — no new allocation).
///
/// This replaces the current `word-hex8` path (a separate function call that
/// builds a new arena string) + the `joined-with` tree in `hash-hex` (5
/// `memcpy` calls) with 8 branchless byte stores directly into the final
/// output buffer.  Per hash: 9 arena bumps + 5 memcpy → 1 arena bump (for
/// the buffer itself, done once by the caller) + 64 byte stores.
///
/// Nibble-to-ASCII conversion uses branchless arithmetic:
///   char = nibble + 87 − (nibble_lt_10 × 39)
/// where `nibble_lt_10` is the MirBinOp::Lt result (0 or 1 as i64).
/// LLVM optimizes this to a `select`/`cmov` instruction.
///
/// Memory safety: only writes to `buf`, which the caller must have allocated
/// via `joined-with ""` (arena copy).  No new allocations are made.
use crate::domain::entities::mir::{MirBinOp, MirInstruction, MirLiteral, MirOperand};
use crate::domain::entities::types::OnuType;
use crate::application::use_cases::mir_builder::MirBuilder;
use super::StdlibOpLowerer;

pub struct WriteHexWordLowerer;

impl StdlibOpLowerer for WriteHexWordLowerer {
    fn name(&self) -> &str { "write-hex-word" }

    fn lower(&self, args: Vec<MirOperand>, builder: &mut MirBuilder) -> MirOperand {
        if args.len() != 3 {
            panic!("write-hex-word requires 3 args: (string buf, integer word, integer base_offset)");
        }
        let buf         = args[0].clone();
        let word        = args[1].clone();
        let base_offset = args[2].clone();

        // Extract the raw i8* pointer once from the string struct (field 1).
        let str_ptr = builder.new_ssa();
        builder.set_ssa_type(str_ptr, OnuType::Nothing);
        builder.build_index(str_ptr, buf.clone(), 1);

        // Write nibble i (0 = most-significant nibble of byte 3, i.e. bit 28).
        for i in 0u64..8 {
            let shift_amt = 28_i64 - (i as i64) * 4;

            // 1. Extract nibble: (word >> shift_amt) & 15
            let shifted = builder.new_ssa();
            builder.set_ssa_type(shifted, OnuType::I64);
            builder.emit(MirInstruction::BinaryOperation {
                dest: shifted,
                op: MirBinOp::Shr,
                lhs: word.clone(),
                rhs: MirOperand::Constant(MirLiteral::I64(shift_amt)),
                dest_type: OnuType::I64,
            });
            let nibble = builder.new_ssa();
            builder.set_ssa_type(nibble, OnuType::I64);
            builder.emit(MirInstruction::BinaryOperation {
                dest: nibble,
                op: MirBinOp::And,
                lhs: MirOperand::Variable(shifted, false),
                rhs: MirOperand::Constant(MirLiteral::I64(15)),
                dest_type: OnuType::I64,
            });

            // 2. Branchless nibble→ASCII:  char = nibble + 87 − (lt10 × 39)
            //    If nibble < 10 → char = nibble + 87 − 39 = nibble + 48  ('0'..'9')
            //    If nibble >= 10 → char = nibble + 87 − 0  = nibble + 87  ('a'..'f')
            let lt10 = builder.new_ssa();
            builder.set_ssa_type(lt10, OnuType::Boolean);
            builder.emit(MirInstruction::BinaryOperation {
                dest: lt10,
                op: MirBinOp::Lt,
                lhs: MirOperand::Variable(nibble, false),
                rhs: MirOperand::Constant(MirLiteral::I64(10)),
                dest_type: OnuType::Boolean,
            });
            let adj = builder.new_ssa();
            builder.set_ssa_type(adj, OnuType::I64);
            builder.emit(MirInstruction::BinaryOperation {
                dest: adj,
                op: MirBinOp::Mul,
                lhs: MirOperand::Variable(lt10, false),
                rhs: MirOperand::Constant(MirLiteral::I64(39)),
                dest_type: OnuType::I64,
            });
            let char_base = builder.new_ssa();
            builder.set_ssa_type(char_base, OnuType::I64);
            builder.emit(MirInstruction::BinaryOperation {
                dest: char_base,
                op: MirBinOp::Sub,
                lhs: MirOperand::Constant(MirLiteral::I64(87)),
                rhs: MirOperand::Variable(adj, false),
                dest_type: OnuType::I64,
            });
            let char_code = builder.new_ssa();
            builder.set_ssa_type(char_code, OnuType::I64);
            builder.emit(MirInstruction::BinaryOperation {
                dest: char_code,
                op: MirBinOp::Add,
                lhs: MirOperand::Variable(nibble, false),
                rhs: MirOperand::Variable(char_base, false),
                dest_type: OnuType::I64,
            });

            // 3. Compute byte position: base_offset + i
            let pos = builder.new_ssa();
            builder.set_ssa_type(pos, OnuType::I64);
            builder.emit(MirInstruction::BinaryOperation {
                dest: pos,
                op: MirBinOp::Add,
                lhs: base_offset.clone(),
                rhs: MirOperand::Constant(MirLiteral::I64(i as i64)),
                dest_type: OnuType::I64,
            });

            // 4. Pointer to buf[base_offset + i]
            let target_ptr = builder.new_ssa();
            builder.set_ssa_type(target_ptr, OnuType::Nothing);
            builder.build_pointer_offset(
                target_ptr,
                MirOperand::Variable(str_ptr, false),
                MirOperand::Variable(pos, false),
            );

            // 5. Store char_code (i64 truncated to i8 by StoreStrategy).
            builder.build_store(
                MirOperand::Variable(target_ptr, false),
                MirOperand::Variable(char_code, false),
            );
        }

        // Return the same buf (in-place, no new allocation).
        buf
    }
}
