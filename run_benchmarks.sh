#!/bin/bash

OUTPUT_FILE="benchmark_results.txt"

echo "========================================" > "$OUTPUT_FILE"
echo "        Onu vs C Benchmark Tests        " >> "$OUTPUT_FILE"
echo "========================================" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# 1. Compile C Benchmarks
echo "Compiling C benchmarks with Clang (-O3)..."
clang cbench_fib_naive.c -O3 -o c_fib_naive_bin
clang cbench_collatz.c -O3 -o c_collatz_bin

# 2. Compile Onu Benchmarks
echo "Compiling Onu benchmarks..."
# The --run flag realizes the bin to the same directory
cargo run --quiet -- samples/fib_naive_only.onu -o fib_naive_only.ll 2>/dev/null >/dev/null

cargo run --quiet -- collatz_benchmark.onu -o collatz_benchmark.ll 2>/dev/null >/dev/null

echo "========================================" >> "$OUTPUT_FILE"
echo " Naive Fibonacci (fib(40))" >> "$OUTPUT_FILE"
echo "========================================" >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"
echo "--- C ---" >> "$OUTPUT_FILE"
{ time ./c_fib_naive_bin; } 2>> "$OUTPUT_FILE" >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"
echo "--- Onu ---" >> "$OUTPUT_FILE"
{ time ./fib_naive_only_bin; } 2>> "$OUTPUT_FILE" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"


echo "========================================" >> "$OUTPUT_FILE"
echo " Collatz Conjecture Loop (1 to 1,000,000)" >> "$OUTPUT_FILE"
echo "========================================" >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"
echo "--- C ---" >> "$OUTPUT_FILE"
{ time ./c_collatz_bin; } 2>> "$OUTPUT_FILE" >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"
echo "--- Onu ---" >> "$OUTPUT_FILE"
{ time ./collatz_benchmark_bin; } 2>> "$OUTPUT_FILE" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

echo "========================================" >> "$OUTPUT_FILE"
echo " Benchmarks Complete" >> "$OUTPUT_FILE"
echo "========================================" >> "$OUTPUT_FILE"

echo "Done! Results saved to $OUTPUT_FILE."
