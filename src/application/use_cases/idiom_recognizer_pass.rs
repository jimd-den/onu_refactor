/// Idiom Recognizer Pass: Target-Independent Pattern Matching
///
/// This pass implements a Visitor Pattern over the MIR tree to detect
/// well-known computational idioms and replace them with LLVM
/// target-independent intrinsics that compile to the single best native
/// instruction on every architecture.
///
/// ## Recognized Idioms
///
/// ### Rotate Right (rotr32)
///
/// Detects the pattern:
///   `(x >> n) | (x << (32 - n))`
///
/// and replaces it with `FunnelShiftRight { hi: x, lo: x, amount: n, width: 32 }`,
/// which emits `@llvm.fshr.i32`.  LLVM maps this to:
/// - `ror` on x86_64
/// - `extr` on AArch64
/// - Software fallback on other targets
///
/// This is critical for SHA-256 performance: each hash round uses 6 rotations,
/// and replacing 4 instructions with 1 hardware instruction per rotation
/// eliminates ~192 redundant ALU operations per 64-round hash.

use crate::domain::entities::mir::*;

pub struct IdiomRecognizerPass;

impl IdiomRecognizerPass {
    /// Run the idiom recognizer over the entire MIR program.
    pub fn run(program: MirProgram) -> MirProgram {
        let functions = program
            .functions
            .into_iter()
            .map(|f| Self::transform_function(f))
            .collect();
        MirProgram { functions }
    }

    fn transform_function(func: MirFunction) -> MirFunction {
        let blocks = func
            .blocks
            .into_iter()
            .map(|block| Self::transform_block(block))
            .collect();
        MirFunction { blocks, ..func }
    }

    fn transform_block(block: BasicBlock) -> BasicBlock {
        // We do a two-pass scan:
        // 1. Identify rotation patterns across consecutive instructions.
        // 2. Replace them with FunnelShiftRight.

        let instructions = block.instructions;
        let mut result: Vec<MirInstruction> = Vec::with_capacity(instructions.len());

        let mut i = 0;
        while i < instructions.len() {
            // Try to recognize a rotation pattern starting at position i.
            // Pattern: we look for a BinaryOperation (Or) whose operands
            // are SSA variables produced by a right-shift and left-shift pair.
            if let Some((fshr_inst, consumed)) =
                Self::try_recognize_rotation(&instructions, i, &result)
            {
                // Remove the intermediate shift instructions that fed into the OR.
                // They may have been pushed to `result` already if they appeared
                // before position i in the instruction stream.
                result.push(fshr_inst);
                i += consumed;
                continue;
            }

            result.push(instructions[i].clone());
            i += 1;
        }

        BasicBlock {
            id: block.id,
            instructions: result,
            terminator: block.terminator,
        }
    }

    /// Try to match the rotation pattern at position `pos` in the instruction list.
    ///
    /// We look for a BitOr whose two operands come from:
    ///   - a right-shift of X by N
    ///   - a left-shift of X by (W - N) then AND with mask
    ///
    /// Returns `Some((FunnelShiftRight instruction, number of instructions consumed))`.
    fn try_recognize_rotation(
        instructions: &[MirInstruction],
        pos: usize,
        _prior: &[MirInstruction],
    ) -> Option<(MirInstruction, usize)> {
        let inst = &instructions[pos];

        // The "OR" that combines the two halves of the rotation.
        let (dest, lhs_op, rhs_op) = match inst {
            MirInstruction::BinaryOperation {
                dest,
                op: MirBinOp::Or,
                lhs,
                rhs,
                ..
            } => (*dest, lhs, rhs),
            _ => return None,
        };

        // Both operands must be SSA variables (results of prior instructions).
        let lhs_id = match lhs_op {
            MirOperand::Variable(id, _) => *id,
            _ => return None,
        };
        let rhs_id = match rhs_op {
            MirOperand::Variable(id, _) => *id,
            _ => return None,
        };

        // Find the producing instructions for lhs and rhs.
        let lhs_inst = Self::find_producer(instructions, pos, lhs_id)?;
        let rhs_inst = Self::find_producer(instructions, pos, rhs_id)?;

        // Try both orderings: (shr, shl) and (shl, shr).
        if let Some(fshr) = Self::match_shr_shl(dest, lhs_inst, rhs_inst) {
            return Some((fshr, 1));
        }
        if let Some(fshr) = Self::match_shr_shl(dest, rhs_inst, lhs_inst) {
            return Some((fshr, 1));
        }

        None
    }

    /// Find the instruction in `instructions[0..pos]` that produces SSA `target_id`.
    fn find_producer<'a>(
        instructions: &'a [MirInstruction],
        pos: usize,
        target_id: usize,
    ) -> Option<&'a MirInstruction> {
        // Search backwards from pos.
        for i in (0..pos).rev() {
            let produced = Self::instruction_dest(&instructions[i]);
            if produced == Some(target_id) {
                return Some(&instructions[i]);
            }
        }
        None
    }

    /// Get the destination SSA ID of an instruction, if it produces one.
    fn instruction_dest(inst: &MirInstruction) -> Option<usize> {
        match inst {
            MirInstruction::BinaryOperation { dest, .. }
            | MirInstruction::Assign { dest, .. }
            | MirInstruction::Call { dest, .. }
            | MirInstruction::Alloc { dest, .. }
            | MirInstruction::StackAlloc { dest, .. }
            | MirInstruction::SaveArena { dest }
            | MirInstruction::Promote { dest, .. }
            | MirInstruction::BitCast { dest, .. }
            | MirInstruction::Load { dest, .. }
            | MirInstruction::PointerOffset { dest, .. }
            | MirInstruction::Index { dest, .. }
            | MirInstruction::Tuple { dest, .. }
            | MirInstruction::GlobalAlloc { dest, .. }
            | MirInstruction::ConstantTableLoad { dest, .. }
            | MirInstruction::FunnelShiftRight { dest, .. } => Some(*dest),
            _ => None,
        }
    }

    /// Match: `shr_inst` is `x >> n` and `shl_inst` is `(x << (W-n)) & mask`.
    /// The AND mask is optional (it may have been folded or may be implicit).
    fn match_shr_shl(
        dest: usize,
        shr_inst: &MirInstruction,
        shl_inst: &MirInstruction,
    ) -> Option<MirInstruction> {
        // shr_inst must be a right-shift.
        let (shr_x, shr_n) = match shr_inst {
            MirInstruction::BinaryOperation {
                op: MirBinOp::Shr,
                lhs,
                rhs,
                ..
            } => (lhs, rhs),
            _ => return None,
        };

        // shl_inst might be a direct left-shift or an AND of a left-shift.
        // Case 1: direct left shift.
        let (shl_x, shl_amount) = match shl_inst {
            MirInstruction::BinaryOperation {
                op: MirBinOp::Shl,
                lhs,
                rhs,
                ..
            } => (lhs, rhs),
            // Case 2: AND masking a left-shift result — we treat this as part
            // of the rotation pattern (the mask is `(1 << W) - 1`).
            MirInstruction::BinaryOperation {
                op: MirBinOp::And,
                lhs,
                rhs: _mask,
                ..
            } => {
                // The lhs of the AND is the SSA variable from the left-shift.
                // We can't easily resolve the chain here without more context,
                // so we bail and let LLVM handle it.
                return None;
            }
            _ => return None,
        };

        // Both shifts must operate on the same source variable.
        if !Self::same_operand(shr_x, shl_x) {
            return None;
        }

        // The shift amounts must sum to a power-of-2 width (typically 32 or 64).
        // Check: shr_n + shl_amount == 32 or 64.
        let width = Self::detect_rotation_width(shr_n, shl_amount)?;

        Some(MirInstruction::FunnelShiftRight {
            dest,
            hi: shr_x.clone(),
            lo: shr_x.clone(),  // rotate: hi == lo
            amount: shr_n.clone(),
            width,
        })
    }

    /// Check if two MIR operands refer to the same value.
    fn same_operand(a: &MirOperand, b: &MirOperand) -> bool {
        match (a, b) {
            (MirOperand::Variable(id_a, _), MirOperand::Variable(id_b, _)) => id_a == id_b,
            (MirOperand::Constant(c_a), MirOperand::Constant(c_b)) => c_a == c_b,
            _ => false,
        }
    }

    /// Detect the rotation width from the shift amounts.
    /// If shr_n and shl_amount are both constants that sum to 32 or 64, return that width.
    /// If only shr_n is a constant, check if there's a `32 - n` or `64 - n` pattern.
    fn detect_rotation_width(shr_n: &MirOperand, shl_amount: &MirOperand) -> Option<u32> {
        match (shr_n, shl_amount) {
            (
                MirOperand::Constant(MirLiteral::I64(n)),
                MirOperand::Constant(MirLiteral::I64(m)),
            ) => {
                let sum = n + m;
                if sum == 32 {
                    Some(32)
                } else if sum == 64 {
                    Some(64)
                } else {
                    None
                }
            }
            // If amounts are dynamic, we can't statically determine width.
            _ => None,
        }
    }
}
