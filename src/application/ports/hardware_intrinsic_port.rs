/// Hardware Intrinsic Port: Application Layer Interface
///
/// This trait defines the abstract boundary between the compiler's core
/// lowering logic and hardware-specific acceleration strategies.
///
/// The Domain/Application layers tag functions with `KnownBehavior` metadata.
/// The Infrastructure/Adapter layer provides concrete implementations that
/// emit the appropriate LLVM intrinsics for the target architecture.
///
/// ## Clean Architecture
///
/// The port follows the Strategy Pattern (GoF):
/// - **Domain**: Pure mathematical behavior written in Ọ̀nụ Discourse.
/// - **Application**: Tags functions with `KnownBehavior` via the registry.
/// - **Infrastructure**: `IntrinsicFactory::create(target)` returns the
///   appropriate strategy that the codegen adapter uses transparently.

/// Known computational behaviors that the compiler can accelerate with
/// hardware intrinsics when available.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KnownBehavior {
    /// SHA-256 compression function (64-round Merkle-Damgård).
    Sha256Compress,
    /// SHA-256 message schedule expansion (σ0/σ1 transforms).
    Sha256MessageSchedule,
    /// AES single-round encryption.
    AesEncryptRound,
    /// Generic bit rotation (handled by IdiomRecognizerPass instead).
    RotateRight,
}

/// The abstract port that hardware acceleration strategies implement.
///
/// Each method returns whether the intrinsic was emitted.  If `false`,
/// the codegen falls through to the software implementation.
pub trait HardwareIntrinsicPort: Send + Sync {
    /// Human-readable name of this strategy (for diagnostics).
    fn name(&self) -> &str;

    /// Whether this strategy supports the given behavior on the current target.
    fn supports(&self, behavior: &KnownBehavior) -> bool;

    /// Target triple this strategy is designed for (e.g. "x86_64-unknown-linux-gnu").
    fn target_triple(&self) -> &str;
}

// ── Concrete Strategies ─────────────────────────────────────────────────

/// x86_64 strategy: uses SHA-NI and AES-NI extensions when available.
///
/// Emits intrinsics like `@llvm.x86.sha256rnds2`, `@llvm.x86.sha256msg1`,
/// `@llvm.x86.sha256msg2`, and `@llvm.x86.aesenc`.
pub struct X86_64CryptoStrategy;

impl HardwareIntrinsicPort for X86_64CryptoStrategy {
    fn name(&self) -> &str {
        "x86_64-sha-ni"
    }

    fn supports(&self, behavior: &KnownBehavior) -> bool {
        matches!(
            behavior,
            KnownBehavior::Sha256Compress
                | KnownBehavior::Sha256MessageSchedule
                | KnownBehavior::AesEncryptRound
        )
    }

    fn target_triple(&self) -> &str {
        "x86_64-unknown-linux-gnu"
    }
}

/// AArch64 strategy: uses ARMv8 Crypto Extensions.
///
/// Emits intrinsics like `@llvm.aarch64.crypto.sha256su0`,
/// `@llvm.aarch64.crypto.sha256su1`, `@llvm.aarch64.crypto.sha256h`.
pub struct Aarch64CryptoStrategy;

impl HardwareIntrinsicPort for Aarch64CryptoStrategy {
    fn name(&self) -> &str {
        "aarch64-crypto"
    }

    fn supports(&self, behavior: &KnownBehavior) -> bool {
        matches!(
            behavior,
            KnownBehavior::Sha256Compress | KnownBehavior::Sha256MessageSchedule
        )
    }

    fn target_triple(&self) -> &str {
        "aarch64-unknown-linux-gnu"
    }
}

/// Software fallback: compiles the pure mathematical MIR as-is.
///
/// Used for RISC-V, older Intel chips without SHA-NI, or any target
/// that lacks hardware crypto acceleration.
pub struct SoftwareFallbackStrategy;

impl HardwareIntrinsicPort for SoftwareFallbackStrategy {
    fn name(&self) -> &str {
        "software-fallback"
    }

    fn supports(&self, _behavior: &KnownBehavior) -> bool {
        false
    }

    fn target_triple(&self) -> &str {
        "any"
    }
}

// ── Abstract Factory ────────────────────────────────────────────────────

/// Factory that creates the appropriate hardware intrinsic strategy
/// based on the compilation target triple.
///
/// This is the single injection point — the codegen adapter calls
/// `IntrinsicFactory::create(target)` once during pipeline setup
/// and uses the returned strategy for the entire compilation.
pub struct IntrinsicFactory;

impl IntrinsicFactory {
    /// Create a hardware intrinsic strategy for the given target triple.
    pub fn create(target_triple: &str) -> Box<dyn HardwareIntrinsicPort> {
        if target_triple.starts_with("x86_64") || target_triple.starts_with("x86-64") {
            Box::new(X86_64CryptoStrategy)
        } else if target_triple.starts_with("aarch64") || target_triple.starts_with("arm64") {
            Box::new(Aarch64CryptoStrategy)
        } else {
            Box::new(SoftwareFallbackStrategy)
        }
    }

    /// Create the default strategy for the host architecture.
    pub fn create_for_host() -> Box<dyn HardwareIntrinsicPort> {
        #[cfg(target_arch = "x86_64")]
        { Box::new(X86_64CryptoStrategy) }
        #[cfg(target_arch = "aarch64")]
        { Box::new(Aarch64CryptoStrategy) }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        { Box::new(SoftwareFallbackStrategy) }
    }
}
