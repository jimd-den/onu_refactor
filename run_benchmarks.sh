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
echo " SHA-256 (1 000 hashes, pure LLVM, no libc)" >> "$OUTPUT_FILE"
echo " Optimizations applied:" >> "$OUTPUT_FILE"
echo "   • K-table: internal constant [64 x i64] + GEP+load (was 64-BB branch tree)" >> "$OUTPUT_FILE"
echo "   • Hex output: single 64-byte arena alloc + 64 inline byte stores (was 8 calls + 5 memcpy)" >> "$OUTPUT_FILE"
echo "========================================" >> "$OUTPUT_FILE"

# ── Collect user-time for C and Onu over 10 × 1000 hashes each ──────────────
echo "" >> "$OUTPUT_FILE"
C_USER=0
ONU_USER=0

echo "--- C (10 × 1 000 hashes = 10 000 total) ---" >> "$OUTPUT_FILE"
if [ -x "./c_sha256_bin" ]; then
    C_TIME=$( { time for _ in $(seq 1 10); do ./c_sha256_bin > /dev/null; done; } 2>&1 )
    echo "$C_TIME" >> "$OUTPUT_FILE"
    # Parse "Xm Y.ZZZs" robustly: strip 'm'/'s', then minutes*60 + seconds
    C_USER=$(echo "$C_TIME" | awk '/user/ {
        t = $2; gsub(/m/, " ", t); gsub(/s/, "", t); split(t, a, " "); print a[1]*60 + a[2]
    }')
    C_USER=${C_USER:-0}
else
    echo "(skipped — compile failed)" >> "$OUTPUT_FILE"
fi

echo "" >> "$OUTPUT_FILE"
echo "--- Onu (10 × 1 000 hashes = 10 000 total) ---" >> "$OUTPUT_FILE"
if [ -x "./sha256_bin" ]; then
    ONU_TIME=$( { time for _ in $(seq 1 10); do ./sha256_bin > /dev/null; done; } 2>&1 )
    echo "$ONU_TIME" >> "$OUTPUT_FILE"
    ONU_USER=$(echo "$ONU_TIME" | awk '/user/ {
        t = $2; gsub(/m/, " ", t); gsub(/s/, "", t); split(t, a, " "); print a[1]*60 + a[2]
    }')
    ONU_USER=${ONU_USER:-0}
else
    echo "(skipped — compile failed)" >> "$OUTPUT_FILE"
fi

# ── Side-by-side ratio ────────────────────────────────────────────────────────
echo "" >> "$OUTPUT_FILE"
echo "--- C vs Onu comparison ---" >> "$OUTPUT_FILE"
if awk "BEGIN { exit !($C_USER > 0 && $ONU_USER > 0) }"; then
    RATIO=$(awk "BEGIN { printf \"%.2f\", $ONU_USER / $C_USER }")
    echo "  C   user time : ${C_USER}s" >> "$OUTPUT_FILE"
    echo "  Onu user time : ${ONU_USER}s" >> "$OUTPUT_FILE"
    echo "  Onu / C ratio : ${RATIO}×  (pure LLVM, no libc; arena allocator; x86_64 syscalls)" >> "$OUTPUT_FILE"
fi

# ── Correctness gate ──────────────────────────────────────────────────────────
echo "" >> "$OUTPUT_FILE"
echo "--- Correctness check (Onu == C for all 1 000 hashes) ---" >> "$OUTPUT_FILE"
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
