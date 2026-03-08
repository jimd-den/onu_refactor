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
    /// Load one i64 element from a compile-time constant array.
    ///
    /// Emits:
    ///   `@name = internal constant [N x i64] [i64 v0, i64 v1, …]`  (once per module)
    ///   `%gep  = getelementptr inbounds [N x i64], [N x i64]* @name, i64 0, i64 <index>`
    ///   `%dest = load i64, i64* %gep`
    ///
    /// The global is read-only, so this is pure LLVM with no arena allocation and full
    /// memory safety.  Used to replace deeply nested if-else constant-table lookups (e.g.
    /// sha256-k) with a single indexed load that LLVM places in L1 cache.
    ConstantTableLoad {
        dest: usize,
        /// Name of the LLVM global (must be unique per table).
        name: String,
        /// The compile-time constant values that populate the table.
        values: Vec<i64>,
        /// Runtime index into the table.
        index: MirOperand,
    },

    // ── Phase 1: Region-Based Memory Management ─────────────────────────

    /// Save the current arena bump pointer onto an implicit stack so that all
    /// allocations made after this point can be reclaimed in O(1) by the
    /// corresponding `RestoreArena`.
    ///
    /// Emits: `%dest = load i8*, i8** @onu_arena_ptr`
    SaveArena {
        dest: usize,
    },

    /// Restore the arena bump pointer to a previously saved value, instantly
    /// freeing all memory allocated since the matching `SaveArena`.
    ///
    /// Emits: `store i8* %saved, i8** @onu_arena_ptr`
    RestoreArena {
        saved: MirOperand,
    },

    /// Stack-promote a fixed-size allocation.  When the compiler can prove
    /// (via escape analysis) that a buffer of `size_bytes` does not escape the
    /// current function, it emits an LLVM `alloca` instead of bumping the
    /// global arena.  LLVM's SROA pass may further promote this to registers.
    ///
    /// Emits: `%dest = alloca [size_bytes x i8]`
    StackAlloc {
        dest: usize,
        size_bytes: usize,
    },

    // ── Phase 3: Target-Independent Idiom Recognition ───────────────────

    /// Funnel-shift-right: `fshr(a, b, amount)` returns the low `width` bits
    /// of `(a concat b) >> amount`.  When `a == b`, this compiles to a single
    /// hardware rotate-right instruction on every major architecture.
    ///
    /// Emits: `%dest = call iN @llvm.fshr.iN(iN %hi, iN %lo, iN %amount)`
    FunnelShiftRight {
        dest: usize,
        hi: MirOperand,
        lo: MirOperand,
        amount: MirOperand,
        /// Bit-width of the rotation (e.g. 32 for rotr32).
        width: u32,
    },

    // ── Phase 4: Buffered I/O ───────────────────────────────────────────

    /// Write bytes to the internal stdout buffer instead of issuing a syscall.
    /// The buffer is flushed when full or when `FlushStdout` is executed.
    ///
    /// This eliminates per-line kernel context switches, matching (or beating)
    /// C's `printf` with its internal `FILE*` buffer.
    BufferedWrite {
        ptr: MirOperand,
        len: MirOperand,
    },

    /// Flush the internal stdout buffer by issuing a single batched syscall.
    /// Emitted at program exit (in the `run`/`main` teardown) to ensure all
    /// buffered output reaches the terminal.
    FlushStdout,
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
    /// Bitwise OR.
    Or,
    /// Bitwise XOR.
    Xor,
    /// Logical (unsigned) right shift – fills high bits with 0.
    Shr,
    /// Left shift.
    Shl,
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
