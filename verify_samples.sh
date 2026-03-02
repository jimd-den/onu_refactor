#!/bin/bash

SAMPLES=(
    "ackermann.onu"
    "collatz_bench.onu"
    "collatz.onu"
    "echo_demo.onu"
    "factorial.onu"
    "fibonacci.onu"
    "guess.onu"
    "hanoi.onu"
    "hello_world_int.onu"
    "hello_world.onu"
    "mutation.onu"
    "parity.onu"
    "sample.onu"
    "svo_e2e.onu"
    "test_logic.onu"
    "test_ownership.onu"
    "test_recursion.onu"
)

# Skip samples that require user input or are known to have complex issues
SKIP=(
    "guess.onu"
)

echo "═══════════════════════════════════════════════════════════════════════════"
echo "  Ọ̀nụ Memory Safety & Pure LLVM Verification"
echo "═══════════════════════════════════════════════════════════════════════════"

for sample in "${SAMPLES[@]}"; do
    if [[ " ${SKIP[@]} " =~ " ${sample} " ]]; then
        echo "Skipping $sample (Interactive/Special)"
        continue
    fi

    echo -n "Verifying $sample... "
    
    name=$(basename "$sample" .onu)
    ll_file="${name}.ll"
    bin_file="${name}_bin"

    # 1. Compile to LLVM IR
    cargo run --quiet -- "samples/$sample" -o "$ll_file" 2>/dev/null
    if [ $? -ne 0 ]; then
        echo "FAILED (Compilation)"
        continue
    fi

    # 2. Realize binary with Clang
    clang "$ll_file" -O3 -o "$bin_file" -Wno-override-module 2>/dev/null
    if [ $? -ne 0 ]; then
        echo "FAILED (Realization)"
        continue
    fi

    # 3. Run with Valgrind
    valgrind --leak-check=full --error-exitcode=1 "./$bin_file" > /dev/null 2>&1
    if [ $? -ne 0 ]; then
        echo "FAILED (Valgrind/Runtime)"
        # Show output if it failed
        # valgrind --leak-check=full "./$bin_file"
    else
        echo "PASSED (Perfectly Safe)"
    fi

    # Cleanup
    # rm "$ll_file" "$bin_file"
done
echo "═══════════════════════════════════════════════════════════════════════════"
