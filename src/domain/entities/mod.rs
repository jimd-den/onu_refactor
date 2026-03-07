pub mod types; pub mod error; pub mod registry; pub mod ast; pub mod hir; pub mod mir; pub mod core_module;

/// Size of the global bump-allocator arena in bytes.
///
/// This constant is the single source of truth shared by:
/// * `application::use_cases::memo_strategies::compound_memo_strategy` —
///   which uses it as `CACHE_MEMORY_LIMIT` to cap the per-function cache size,
///   and
/// * `adapters::codegen::OnuCodegen` — which allocates the physical
///   `[ARENA_SIZE_BYTES x i8]` LLVM global.
///
/// Both values **must** stay in sync: the MIR-level allocation guard and the
/// LLVM arena declaration together prevent out-of-bounds memory access.
///
/// 16 MiB gives a 2-dim memoization window of 1024 × 1024 entries
/// (versus the 256 × 256 limit at 1 MiB), covering Ackermann(3, 11)'s
/// most-frequent recursive sub-problems.
pub const ARENA_SIZE_BYTES: usize = 16 * 1_048_576;
