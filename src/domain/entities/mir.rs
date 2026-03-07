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
    pub is_pure_data_leaf: bool,
    pub diminishing: Vec<String>,
    /// Override the memoization cache entry count.  When `None` the MemoPass
    /// uses its internal default (10 000).  IntegerUpgradePass sets this to
    /// `max_call_arg + 1` so that the per-entry WideInt allocation stays well
    /// within the 1 MB arena.
    pub memo_cache_size: Option<usize>,
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
    Assign {
        dest: usize,
        src: MirOperand,
    },
    BinaryOperation {
        dest: usize,
        op: MirBinOp,
        lhs: MirOperand,
        rhs: MirOperand,
        dest_type: OnuType,
    },
    Call {
        dest: usize,
        name: String,
        args: Vec<MirOperand>,
        return_type: OnuType,
        arg_types: Vec<OnuType>,
        is_tail_call: bool,
    },
    Tuple {
        dest: usize,
        elements: Vec<MirOperand>,
    },
    Index {
        dest: usize,
        subject: MirOperand,
        index: usize,
    },
    Emit(MirOperand),
    Drop {
        ssa_var: usize,
        typ: OnuType,
        name: String,
        is_dynamic: bool,
    },
    Alloc {
        dest: usize,
        size_bytes: MirOperand,
    },
    /// Declares (or references) a named LLVM global zeroed byte-array of `size_bytes` bytes
    /// and yields a pointer to its first element in SSA `dest`.
    ///
    /// Unlike `Alloc` (which bumps the per-call arena), `GlobalAlloc` is backed by an
    /// LLVM module-level global — it is allocated exactly once (zero-initialised by the
    /// OS/loader) and persists for the lifetime of the program.  This is the correct
    /// backing store for memo caches that must survive across many calls to the wrapper.
    GlobalAlloc {
        dest: usize,
        size_bytes: usize,
        name: String,
    },
    MemCopy {
        dest: MirOperand,
        src: MirOperand,
        size: MirOperand,
    },
    PointerOffset {
        dest: usize,
        ptr: MirOperand,
        offset: MirOperand,
    },
    /// Load a value of `typ` from a raw pointer (e.g. an i8* produced by PointerOffset).
    /// The codegen casts the pointer to `typ`* before loading.
    /// This is how the memoization cache reads i64 values back from the byte arena.
    Load {
        dest: usize,
        ptr: MirOperand,
        typ: OnuType,
    },
    Store {
        ptr: MirOperand,
        value: MirOperand,
    },
    /// Typed store to a raw pointer (symmetric counterpart to Load).
    /// Casts the i8* pointer from PointerOffset to `typ`* before writing.
    /// Prevents StoreStrategy from truncating an i64 to i8 when stored via i8* pointer.
    TypedStore {
        ptr: MirOperand,
        value: MirOperand,
        typ: OnuType,
    },
    MemSet {
        ptr: MirOperand,
        value: MirOperand,
        size: MirOperand,
    },
    Promote {
        dest: usize,
        src: MirOperand,
        to_type: OnuType,
    },
    /// Reinterpret the bit-pattern of `src` as `to_type` (equivalent to LLVM `bitcast`).
    /// Used by the wide-int legalization layer to transition between a "Mathematical Integer"
    /// (e.g. WideInt(1024)) and a lower-level representation such as a byte array,
    /// satisfying the Clean Architecture boundary between the domain model and the
    /// memory-detail (limb) layer.
    BitCast {
        dest: usize,
        src: MirOperand,
        to_type: OnuType,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum MirBinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Gt,
    Lt,
    /// Bitwise AND.  Used by HashMemoStrategy to reduce a hash value to a
    /// power-of-2 table slot with a single instruction (`and rX, mask`).
    And,
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
    WideInt(String, u32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum MirTerminator {
    Return(MirOperand),
    Branch(usize), // block id
    CondBranch {
        condition: MirOperand,
        then_block: usize,
        else_block: usize,
    },
    Unreachable,
}
