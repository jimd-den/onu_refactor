use std::path::Path;
/// # C Comparison Tests
///
/// Red/green TDD harness: compile matching Onu and C programs, run both, and
/// assert their numeric outputs agree.  A failing test is the starting point
/// for a diagnostic hypothesis — not something to skip.
use std::process::Command;

// ---------------------------------------------------------------------------
// Infrastructure
// ---------------------------------------------------------------------------

/// Compile an `.onu` sample through the full pipeline and return its stdout.
fn compile_and_capture_onu(sample_path: &str) -> String {
    let mut options = onu_refactor::application::options::CompilationOptions::default();
    options.log_level = onu_refactor::application::options::LogLevel::Error;

    let env = onu_refactor::infrastructure::os::NativeOsEnvironment::new(options.log_level);
    let codegen = onu_refactor::adapters::codegen::OnuCodegen::new();
    let mut pipeline = onu_refactor::CompilationPipeline::new(env, codegen, options);

    pipeline
        .compile(sample_path)
        .unwrap_or_else(|e| panic!("Onu compilation failed for {sample_path}: {e:?}"));

    let stem = Path::new(sample_path)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap();
    let binary = format!("./{stem}_bin");

    let output = Command::new(&binary)
        .output()
        .unwrap_or_else(|e| panic!("Failed to execute {binary}: {e}"));

    assert!(output.status.success(), "Onu binary exited non-zero");
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Compile the C reference binary if it does not exist, then return its stdout.
/// Uses `-O3 -foptimize-sibling-calls` so TCO is on par with Onu's TCO-pass.
fn compile_and_capture_c(c_source: &str, binary_name: &str) -> String {
    if !Path::new(binary_name).exists() {
        let status = Command::new("gcc")
            .args([
                "-O3",
                "-foptimize-sibling-calls",
                "-o",
                binary_name,
                c_source,
            ])
            .status()
            .expect("gcc must be installed");
        assert!(status.success(), "gcc failed to compile {c_source}");
    }

    let output = Command::new(binary_name)
        .output()
        .unwrap_or_else(|e| panic!("Failed to execute {binary_name}: {e}"));

    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Extract the first `i64` from a multi-line string.
///
/// Strategy (in priority order):
/// 1. After the last `:` on a line  — Onu format: `"label: N"`
/// 2. After the last `=` on a line  — C format:   `"fib(40) = N"`
/// 3. The entire trimmed line        — bare number: `"131434424"` (Onu collatz)
fn first_number(text: &str) -> Option<i64> {
    text.lines().find_map(|line| {
        let trimmed = line.trim();
        if line.contains(':') {
            line.split(':').last()?.trim().parse::<i64>().ok()
        } else if line.contains('=') {
            line.split('=').last()?.trim().parse::<i64>().ok()
        } else {
            trimmed.parse::<i64>().ok()
        }
    })
}

// ---------------------------------------------------------------------------
// Collatz — expected GREEN
// ---------------------------------------------------------------------------

/// Onu's `collatz_benchmark.onu` and the C reference must both report
/// **131 434 424** total steps for inputs 1 to 1 000 000.
#[test]
fn collatz_matches_c_reference() {
    let c_val = first_number(&compile_and_capture_c(
        "cbench_collatz.c",
        "./cbench_collatz",
    ))
    .expect("C output had no number");
    let onu_val = first_number(&compile_and_capture_onu("collatz_benchmark.onu"))
        .expect("Onu output had no number");

    assert_eq!(
        c_val, onu_val,
        "Collatz totals diverge: C={c_val}, Onu={onu_val}"
    );
}

// ---------------------------------------------------------------------------
// Fibonacci(40) — RED until codegen/passes are correct
// ---------------------------------------------------------------------------

/// ## Hypothesis
/// `fibonacci.onu` (calls `calculate-growth(40)`) and a minimal C reference
/// that calls `fib_naive(40)` must both produce **102 334 155**.
///
/// If this test is red, the Onu fibonacci codegen is wrong.  The C value is
/// the ground truth; investigate the LLVM IR / passes next.
#[test]
fn fibonacci_40_matches_c_reference() {
    // Write the minimal C reference inline so there is no separate file to manage.
    // This is the authoritative ground truth for fib(40).
    let c_src = "/tmp/c_fib40.c";
    let c_bin = "/tmp/c_fib40";

    if !Path::new(c_bin).exists() {
        std::fs::write(
            c_src,
            r#"
#include <stdio.h>
static long long fib(long long n) {
    if (n == 0) return 0;
    if (n == 1) return 1;
    return fib(n - 1) + fib(n - 2);
}
int main(void) {
    printf("fib(40) = %lld\n", fib(40));
    return 0;
}
"#,
        )
        .expect("Failed to write C fib source");

        let status = Command::new("gcc")
            .args(["-O2", "-o", c_bin, c_src])
            .status()
            .expect("gcc must be installed");
        assert!(status.success(), "gcc failed to compile fib reference");
    }

    let c_out = Command::new(c_bin)
        .output()
        .expect("Failed to run C fib binary");
    let c_val =
        first_number(&String::from_utf8_lossy(&c_out.stdout)).expect("C fib output had no number");

    let onu_out = compile_and_capture_onu("samples/fibonacci.onu");
    let onu_val = first_number(&onu_out).expect("Onu fibonacci output had no number");

    assert_eq!(
        c_val, onu_val,
        "fib(40) diverges — C ground truth: {c_val}, Onu produced: {onu_val}.\n\
         Full Onu output:\n{onu_out}"
    );
}

// ---------------------------------------------------------------------------
// Runtime benchmark — fib(40): Onu vs cbenchmark.c
// ---------------------------------------------------------------------------

/// ## Runtime Benchmark: fib(40) — Onu vs C
///
/// Compiles both programs once (not counted in timing), then runs each **5
/// times** back-to-back, measuring wall-clock milliseconds per run.
///
/// Outputs an aligned table showing mean time and every individual run, plus
/// the Onu/C ratio.  A ratio > 1 means Onu is slower; ratio < 1 means faster.
///
/// ### Assertion policy
/// - Hard wall: each Onu run must complete in < 10 seconds (guards against hangs).
/// - No ratio threshold is enforced — this is an **observability test** that
///   documents performance, not a correctness gate.  Watch the ratio in CI
///   artifacts to detect regressions.
///
/// Results are written to `test_output/c_comparison_bench.txt`.
#[test]
fn runtime_benchmark_fib40() {
    use std::time::Instant;
    const RUNS: u32 = 5;

    // Compile Onu once — compilation time is intentionally excluded from timing.
    {
        let mut options = onu_refactor::application::options::CompilationOptions::default();
        options.log_level = onu_refactor::application::options::LogLevel::Error;
        let env = onu_refactor::infrastructure::os::NativeOsEnvironment::new(options.log_level);
        let codegen = onu_refactor::adapters::codegen::OnuCodegen::new();
        let mut pipeline = onu_refactor::CompilationPipeline::new(env, codegen, options);
        pipeline
            .compile("samples/fibonacci.onu")
            .expect("Onu compile must succeed");
    }

    // Ensure the C reference binary exists (uses the workspace cbenchmark.c).
    compile_and_capture_c("cbenchmark.c", "./cbenchmark");

    // Time a binary over RUNS executions, returning each run's wall-clock ms.
    let time_binary = |path: &str| -> Vec<u128> {
        (0..RUNS)
            .map(|_| {
                let start = Instant::now();
                let status = Command::new(path)
                    .status()
                    .unwrap_or_else(|e| panic!("Failed to run {path}: {e}"));
                let elapsed = start.elapsed().as_millis();
                assert!(status.success(), "{path} exited non-zero");
                assert!(
                    elapsed < 10_000,
                    "Safety wall: {path} took {elapsed}ms — possible hang"
                );
                elapsed
            })
            .collect()
    };

    let c_times = time_binary("./cbenchmark");
    let onu_times = time_binary("./fibonacci_bin");

    let mean = |v: &[u128]| -> u128 { v.iter().sum::<u128>() / v.len() as u128 };
    let c_mean = mean(&c_times);
    let onu_mean = mean(&onu_times);
    let ratio = onu_mean as f64 / c_mean.max(1) as f64;

    let report = format!(
        "=== Runtime Benchmark: fib(40) — {RUNS} runs each ===\n\
         C   (cbenchmark, gcc -O3):  {:>5} ms/run  runs={:?}\n\
         Onu (fibonacci_bin, LLVM):  {:>5} ms/run  runs={:?}\n\
         Ratio (Onu / C):            {:.2}x\n",
        c_mean, c_times, onu_mean, onu_times, ratio
    );

    println!("\n{report}");
    std::fs::create_dir_all("test_output").unwrap();
    std::fs::write("test_output/c_comparison_bench.txt", &report).unwrap();
}
