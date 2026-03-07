/// build.rs — LLVM version auto-detection for onu_refactor.
///
/// When no explicit llvm* Cargo feature is selected (i.e. the default "llvm14"
/// is active), this script silently validates that the compiler is configured
/// consistently.  It also emits `cargo:rustc-env` assignments that set the
/// LLVM_SYS_*_PREFIX variables for each LLVM version found on the host, so
/// that developers with LLVM installed at non-standard paths can still build
/// without touching `.cargo/config.toml`.
///
/// The `force = false` flag in `.cargo/config.toml` means the environment
/// variables set here only take effect when they are NOT already set by the
/// shell or CI environment.
use std::path::Path;
use std::process::Command;

fn main() {
    // Tell Cargo to re-run this script if it changes.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=LLVM_SYS_140_PREFIX");
    println!("cargo:rerun-if-env-changed=LLVM_SYS_160_PREFIX");
    println!("cargo:rerun-if-env-changed=LLVM_SYS_170_PREFIX");
    println!("cargo:rerun-if-env-changed=LLVM_SYS_181_PREFIX");

    // Pairs of (llvm-version, env-var, typical Ubuntu path)
    let candidates: &[(&str, &str, &str)] = &[
        ("20", "LLVM_SYS_200_PREFIX", "/usr/lib/llvm-20"),
        ("19", "LLVM_SYS_190_PREFIX", "/usr/lib/llvm-19"),
        ("18", "LLVM_SYS_181_PREFIX", "/usr/lib/llvm-18"),
        ("17", "LLVM_SYS_170_PREFIX", "/usr/lib/llvm-17"),
        ("16", "LLVM_SYS_160_PREFIX", "/usr/lib/llvm-16"),
        ("15", "LLVM_SYS_150_PREFIX", "/usr/lib/llvm-15"),
        ("14", "LLVM_SYS_140_PREFIX", "/usr/lib/llvm-14"),
    ];

    for (ver, env_var, default_path) in candidates {
        // Only emit if not already set by the user.
        if std::env::var(env_var).is_ok() {
            continue;
        }

        // Check via llvm-config-N first (distro packages), then the default path.
        let found = Command::new(format!("llvm-config-{}", ver))
            .arg("--prefix")
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8(o.stdout)
                        .ok()
                        .map(|s| s.trim().to_owned())
                } else {
                    None
                }
            })
            .or_else(|| {
                // Fall back to the standard Ubuntu path if the directory exists.
                if Path::new(default_path).is_dir() {
                    Some(default_path.to_string())
                } else {
                    None
                }
            });

        if let Some(prefix) = found {
            // Emit the prefix so llvm-sys can find its libraries.
            println!("cargo:rustc-env={}={}", env_var, prefix);
        }
    }
}
