#!/bin/bash

OUTPUT_FILE="benchmark_results.txt"

echo "========================================" > "$OUTPUT_FILE"
echo "        Onu vs C Benchmark Tests        " >> "$OUTPUT_FILE"
echo "========================================" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# 1. Compile Benchmarks
echo "Compiling C benchmarks with Clang (-O3)..."
clang cbench_fib_naive.c -O3 -o c_fib_naive_bin
clang cbench_collatz.c -O3 -o c_collatz_bin
clang cbench_ackermann.c -O3 -o c_ackermann_bin

echo "Compiling Onu benchmarks..."
cargo run --quiet -- samples/fib_naive_only.onu > /dev/null 2>&1
cargo run --quiet -- samples/collatz_bench.onu > /dev/null 2>&1
cargo run --quiet -- samples/ackermann_bench.onu > /dev/null 2>&1

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
{ time ./collatz_bench_bin; } 2>> "$OUTPUT_FILE" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"


echo "========================================" >> "$OUTPUT_FILE"
echo " Ackermann Function (3, 11)" >> "$OUTPUT_FILE"
echo "========================================" >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"
echo "--- C ---" >> "$OUTPUT_FILE"
{ time ./c_ackermann_bin; } 2>> "$OUTPUT_FILE" >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"
echo "--- Onu ---" >> "$OUTPUT_FILE"
{ time ./ackermann_bench_bin; } 2>> "$OUTPUT_FILE" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"


echo "========================================" >> "$OUTPUT_FILE"
echo " Benchmarks Complete" >> "$OUTPUT_FILE"
echo "========================================" >> "$OUTPUT_FILE"

echo "Done! Results saved to $OUTPUT_FILE."
