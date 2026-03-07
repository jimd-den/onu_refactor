#!/bin/bash

OUTPUT_FILE="benchmark_results.txt"

# Helper: compile an Onu benchmark and report success/failure.
compile_onu() {
    local sample="$1"
    local name
    name=$(basename "$sample" .onu)
    echo -n "  Compiling $sample ... "
    if cargo run --release --quiet -- "$sample" > /dev/null 2>&1; then
        echo "OK"
    else
        echo "FAILED (binary will be skipped in results)"
    fi
}

# Helper: run a binary if it exists, otherwise log a compile-failure note.
run_bin() {
    local bin="$1"
    local label="$2"
    echo "" >> "$OUTPUT_FILE"
    echo "--- ${label} ---" >> "$OUTPUT_FILE"
    if [ -x "./${bin}" ]; then
        { time "./${bin}"; } 2>> "$OUTPUT_FILE" >> "$OUTPUT_FILE"
    else
        echo "(skipped — compile failed)" >> "$OUTPUT_FILE"
    fi
}

echo "========================================" > "$OUTPUT_FILE"
echo "        Onu vs C Benchmark Tests        " >> "$OUTPUT_FILE"
echo "========================================" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# 1. Compile C benchmarks
echo "Compiling C benchmarks with Clang (-O3)..."
clang cbench_fib_naive.c -O3 -o c_fib_naive_bin
clang cbench_collatz.c -O3 -o c_collatz_bin
clang cbench_ackermann.c -O3 -o c_ackermann_bin
clang cbench_sha256.c -O3 -o c_sha256_bin

# 2. Compile all Onu benchmarks
echo "Compiling Onu benchmarks..."
compile_onu samples/fib_naive_only.onu
compile_onu samples/fib_bench.onu
compile_onu samples/collatz_bench.onu
compile_onu samples/ackermann_bench.onu
compile_onu samples/sha256.onu

# ── Naive Fibonacci ──────────────────────────────────────────────────────────
echo "========================================" >> "$OUTPUT_FILE"
echo " Naive Fibonacci (fib(40))" >> "$OUTPUT_FILE"
echo "========================================" >> "$OUTPUT_FILE"
run_bin "c_fib_naive_bin"      "C"
run_bin "fib_naive_only_bin"   "Onu (fib_naive_only)"
echo "" >> "$OUTPUT_FILE"

# ── Multi-variant Fibonacci Benchmark ────────────────────────────────────────
echo "========================================" >> "$OUTPUT_FILE"
echo " Fibonacci Benchmark (naive/tco/range)" >> "$OUTPUT_FILE"
echo "========================================" >> "$OUTPUT_FILE"
run_bin "fib_bench_bin"        "Onu (fib_bench)"
echo "" >> "$OUTPUT_FILE"

# ── Collatz ───────────────────────────────────────────────────────────────────
echo "========================================" >> "$OUTPUT_FILE"
echo " Collatz Conjecture Loop (1 to 1,000,000)" >> "$OUTPUT_FILE"
echo "========================================" >> "$OUTPUT_FILE"
run_bin "c_collatz_bin"        "C"
run_bin "collatz_bench_bin"    "Onu"
echo "" >> "$OUTPUT_FILE"

# ── Ackermann ─────────────────────────────────────────────────────────────────
echo "========================================" >> "$OUTPUT_FILE"
echo " Ackermann Function (3, 11)" >> "$OUTPUT_FILE"
echo "========================================" >> "$OUTPUT_FILE"
run_bin "c_ackermann_bin"      "C"
run_bin "ackermann_bench_bin"  "Onu"
echo "" >> "$OUTPUT_FILE"

# ── SHA-256 ───────────────────────────────────────────────────────────────────
echo "========================================" >> "$OUTPUT_FILE"
echo " SHA-256 (1 000 hashes, pure LLVM/native bitwise ops)" >> "$OUTPUT_FILE"
echo "========================================" >> "$OUTPUT_FILE"
# Run 10 back-to-back trials of each so the OS timer resolution is not a
# factor, then report total wall time for 10 000 hashes each.
echo "" >> "$OUTPUT_FILE"
echo "--- C (10 runs × 1 000 hashes = 10 000 total) ---" >> "$OUTPUT_FILE"
if [ -x "./c_sha256_bin" ]; then
    { time for _ in $(seq 1 10); do ./c_sha256_bin > /dev/null; done; } 2>> "$OUTPUT_FILE"
else
    echo "(skipped — compile failed)" >> "$OUTPUT_FILE"
fi
echo "" >> "$OUTPUT_FILE"
echo "--- Onu (10 runs × 1 000 hashes = 10 000 total) ---" >> "$OUTPUT_FILE"
if [ -x "./sha256_bin" ]; then
    { time for _ in $(seq 1 10); do ./sha256_bin > /dev/null; done; } 2>> "$OUTPUT_FILE"
else
    echo "(skipped — compile failed)" >> "$OUTPUT_FILE"
fi
echo "" >> "$OUTPUT_FILE"
echo "--- Correctness check (Onu == C for first 1 000 hashes) ---" >> "$OUTPUT_FILE"
./sha256_bin 2>/dev/null | tail -n +2 > /tmp/onu_sha256.txt 2>/dev/null
./c_sha256_bin 2>/dev/null | tail -n +2 > /tmp/c_sha256.txt 2>/dev/null
if diff -q /tmp/onu_sha256.txt /tmp/c_sha256.txt > /dev/null 2>&1; then
    echo "PASS: all 1 000 digests are identical" >> "$OUTPUT_FILE"
else
    echo "FAIL: digest mismatch detected" >> "$OUTPUT_FILE"
    diff /tmp/onu_sha256.txt /tmp/c_sha256.txt | head -5 >> "$OUTPUT_FILE"
fi
echo "" >> "$OUTPUT_FILE"

echo "========================================" >> "$OUTPUT_FILE"
echo " Benchmarks Complete" >> "$OUTPUT_FILE"
echo "========================================" >> "$OUTPUT_FILE"

echo "Done! Results saved to $OUTPUT_FILE."
