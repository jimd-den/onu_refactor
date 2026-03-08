#!/bin/bash
#
# run_valgrind.sh — Run Valgrind memory-safety checks on all working Onu samples.
# Skips samples that are intentionally broken, require user input, time out, or
# fail to compile/link due to known missing runtime intrinsics.
#
# Results are written to valgrind_results.txt.

set -euo pipefail

OUTPUT_FILE="valgrind_results.txt"
PASS=0
FAIL=0
SKIP=0

# Samples that are known NOT to work and must be excluded:
#   guess            – interactive (requires stdin)
#   illegal_shared   – intentional compile-time error demo
#   bf               – interpreter that runs indefinitely (timeout)
#   complex_args_tco – exits with code 100 at runtime
#   mutual_recursion – exits with code 1 at runtime
#   echo_demo        – missing runtime intrinsics (argument-count / receives-argument)
#   mutation         – compile failure
#   test_logic       – compile failure
SKIP_LIST=(
    "guess"
    "illegal_shared"
    "bf"
    "complex_args_tco"
    "mutual_recursion"
    "echo_demo"
    "mutation"
    "test_logic"
)

{
echo "════════════════════════════════════════════════════════════════════"
echo "  Ọ̀nụ Valgrind Memory-Safety Report"
echo "════════════════════════════════════════════════════════════════════"
echo ""
} | tee "$OUTPUT_FILE"

for onu_file in samples/*.onu; do
    stem=$(basename "$onu_file" .onu)

    # Check skip list
    skip=false
    for s in "${SKIP_LIST[@]}"; do
        if [[ "$stem" == "$s" ]]; then
            skip=true
            break
        fi
    done
    if $skip; then
        msg="  [SKIP] $stem"
        echo "$msg" | tee -a "$OUTPUT_FILE"
        (( SKIP++ )) || true
        continue
    fi

    ll_file="${stem}.ll"
    bin_file="${stem}_bin"

    # Compile to LLVM IR
    if ! ./target/release/onu_refactor "$onu_file" 2>/dev/null; then
        msg="  [FAIL] $stem  (compile error)"
        echo "$msg" | tee -a "$OUTPUT_FILE"
        (( FAIL++ )) || true
        continue
    fi

    # Link with clang
    if ! clang "$ll_file" -O0 -o "$bin_file" -Wno-override-module 2>/dev/null; then
        msg="  [FAIL] $stem  (link error)"
        echo "$msg" | tee -a "$OUTPUT_FILE"
        rm -f "$ll_file"
        (( FAIL++ )) || true
        continue
    fi

    # Run under Valgrind
    {
        echo "────────────────────────────────────────────────────────────────────"
        echo "  Sample: $stem"
        echo "────────────────────────────────────────────────────────────────────"
    } >> "$OUTPUT_FILE"

    valgrind_out=$(valgrind \
        --leak-check=full \
        --show-leak-kinds=all \
        --track-origins=yes \
        --error-exitcode=1 \
        "./$bin_file" 2>&1)
    valgrind_rc=$?

    echo "$valgrind_out" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"

    if [ $valgrind_rc -eq 0 ]; then
        msg="  [PASS] $stem"
        (( PASS++ )) || true
    else
        msg="  [FAIL] $stem  (valgrind reported errors — see $OUTPUT_FILE)"
        (( FAIL++ )) || true
    fi
    echo "$msg" | tee -a "$OUTPUT_FILE"

    rm -f "$ll_file" "$bin_file"
done

{
echo ""
echo "════════════════════════════════════════════════════════════════════"
echo "  Summary:  PASS=$PASS  FAIL=$FAIL  SKIP=$SKIP"
echo "════════════════════════════════════════════════════════════════════"
} | tee -a "$OUTPUT_FILE"

echo ""
echo "Full report saved to $OUTPUT_FILE."
