/// Ọ̀nụ MIR: Domain Entities
///
/// This module defines the Mid-level Intermediate Representation.
/// MIR is a flat, SSA-based representation suitable for optimizations
/// and machine code generation.

use crate::domain::entities::types::OnuType;

#[derive(Debug, Clone, PartialEq)]
pub struct MirProgram {
    pub functions: Vec<MirFunction>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MirFunction {
    pub name: String,
    pub args: Vec<MirArgument>,
    pub return_type: OnuType,
    pub blocks: Vec<BasicBlock>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MirArgument {
    pub name: String,
    pub typ: OnuType,
    pub ssa_var: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BasicBlock {
    pub id: usize,
    pub instructions: Vec<MirInstruction>,
    pub terminator: MirTerminator,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MirInstruction {
    Assign { dest: usize, src: MirOperand },
    BinaryOperation { dest: usize, op: MirBinOp, lhs: MirOperand, rhs: MirOperand },
    Call { 
        dest: usize, 
        name: String, 
        args: Vec<MirOperand>,
        return_type: OnuType,
        arg_types: Vec<OnuType>,
    },
    Tuple { dest: usize, elements: Vec<MirOperand> },
    Index { dest: usize, subject: MirOperand, index: usize },
    Emit(MirOperand),
    Drop { ssa_var: usize, typ: OnuType, name: String },
    Alloc { dest: usize, size_bytes: MirOperand },
    MemCopy { dest: MirOperand, src: MirOperand, size: MirOperand },
    PointerOffset { dest: usize, ptr: MirOperand, offset: MirOperand },
}

#[derive(Debug, Clone, PartialEq)]
pub enum MirBinOp {
    Add, Sub, Mul, Div, Eq, Ne, Gt, Lt,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MirOperand {
    Constant(MirLiteral),
    Variable(usize, bool), // (ssa_var, is_consuming)
}

#[derive(Debug, Clone, PartialEq)]
pub enum MirLiteral {
    I64(i64),
    F64(u64),
    Boolean(bool),
    Text(String),
    Nothing,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MirTerminator {
    Return(MirOperand),
    Branch(usize), // block id
    CondBranch { condition: MirOperand, then_block: usize, else_block: usize },
    Unreachable,
}
