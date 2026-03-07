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

# 2. Compile all Onu benchmarks
echo "Compiling Onu benchmarks..."
compile_onu samples/fib_naive_only.onu
compile_onu samples/fib_bench.onu
compile_onu samples/collatz_bench.onu
compile_onu samples/ackermann_bench.onu

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

echo "========================================" >> "$OUTPUT_FILE"
echo " Benchmarks Complete" >> "$OUTPUT_FILE"
echo "========================================" >> "$OUTPUT_FILE"

echo "Done! Results saved to $OUTPUT_FILE."
