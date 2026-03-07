/// SHA-256 K-table stdlib op.
///
/// Replaces the deeply-nested Onu if-else chain for sha256-k with a single
/// `getelementptr + load` into a `@sha256_K = internal constant [64 x i64]`
/// global.  The global fits in one or two L1 cache lines after the first
/// access, turning O(log₂ 64) branch-tree lookups into O(1) indexed reads.
///
/// Memory safety: the global is declared `constant`, so LLVM enforces
/// read-only semantics at the IR level — no writes are possible.
use crate::domain::entities::mir::{MirInstruction, MirOperand};
use crate::domain::entities::types::OnuType;
use crate::application::use_cases::mir_builder::MirBuilder;
use super::StdlibOpLowerer;

/// SHA-256 round constants K[0..63] (FIPS 180-4, first 32 bits of the
/// fractional parts of the cube roots of the first 64 prime numbers).
const SHA256_K: [i64; 64] = [
    1116352408, 1899447441, 3049323471, 3921009573,
     961987163, 1508970993, 2453635748, 2870763221,
    3624381080,  310598401,  607225278, 1426881987,
    1925078388, 2162078206, 2614888103, 3248222580,
    3835390401, 4022224774,  264347078,  604807628,
     770255983, 1249150122, 1555081692, 1996064986,
    2554220882, 2821834349, 2952996808, 3210313671,
    3336571891, 3584528711,  113926993,  338241895,
     666307205,  773529912, 1294757372, 1396182291,
    1695183700, 1986661051, 2177026350, 2456956037,
    2730485921, 2820302411, 3259730800, 3345764771,
    3516065817, 3600352804, 4094571909,  275423344,
     430227734,  506948616,  659060556,  883997877,
     958139571, 1322822218, 1537002063, 1747873779,
    1955562222, 2024104815, 2227730452, 2361852424,
    2428436474, 2756734187, 3204031479, 3329325298,
];

pub struct Sha256KTableLowerer;

impl StdlibOpLowerer for Sha256KTableLowerer {
    fn name(&self) -> &str { "sha256-k-table" }

    fn lower(&self, args: Vec<MirOperand>, builder: &mut MirBuilder) -> MirOperand {
        if args.len() != 1 {
            panic!("sha256-k-table requires 1 argument: round index t (0-63)");
        }
        let index = args[0].clone();
        let dest = builder.new_ssa();
        builder.set_ssa_type(dest, OnuType::I64);
        builder.emit(MirInstruction::ConstantTableLoad {
            dest,
            name: "sha256_K".to_string(),
            values: SHA256_K.to_vec(),
            index,
        });
        MirOperand::Variable(dest, true)
    }
}
