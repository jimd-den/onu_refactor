#!/usr/bin/env bash
# =============================================================================
# turing_bench.sh — Time every Onu sample against its C equivalent, then run
#                   Valgrind on each Onu binary.  All output is written to
#                   turing_bench_results.txt.
#
# Usage:  bash turing_bench.sh
#
# Requires:  cargo, clang, valgrind, LLVM_SYS_140_PREFIX (or whichever LLVM
#            version is selected via Cargo features).
# =============================================================================

set -euo pipefail

OUT="turing_bench_results.txt"
TIMEOUT=30          # seconds per binary
PASS=0; FAIL=0; SKIP=0

# ── helpers ──────────────────────────────────────────────────────────────────

ts() { date '+%Y-%m-%dT%H:%M:%S'; }

sep()  { printf '═%.0s' {1..68}; echo; }
sep2() { printf '─%.0s' {1..68}; echo; }

log()  { echo "$*" | tee -a "$OUT"; }
log_raw() { echo "$*" >> "$OUT"; }

# Compile a .c file with clang -O3.  Returns 0 on success.
compile_c() {
    local src="$1" bin="$2"
    clang "$src" -O3 -o "$bin" -Wno-override-module 2>/dev/null
}

# Compile an Onu sample to a binary via cargo run (release).
compile_onu() {
    local src="$1"
    cargo run --release --quiet -- "$src" >/dev/null 2>&1
}

# Time a binary; append result to OUT.  $1=label  $2=binary  $3=timeout_sec
time_bin() {
    local label="$1" bin="./$2" t="${3:-$TIMEOUT}"
    if [ ! -x "$bin" ]; then
        log "  [SKIP] $label — binary not found"
        return
    fi
    log_raw ""
    log_raw "  [$label]"
    # /usr/bin/time for portable format; fall back to shell built-in
    if command -v /usr/bin/time &>/dev/null; then
        { timeout "$t" /usr/bin/time -f "    wall=%e s  user=%U s  sys=%S s  maxRSS=%M kB" \
            "$bin" 2>&1; } >> "$OUT" || true
    else
        { timeout "$t" bash -c "time $bin" 2>&1; } >> "$OUT" || true
    fi
}

# Run Valgrind on a binary; append summary to OUT.
valgrind_bin() {
    local label="$1" bin="./$2"
    if [ ! -x "$bin" ]; then return; fi
    local vout
    vout=$(timeout "$TIMEOUT" valgrind \
        --leak-check=full --show-leak-kinds=all \
        --track-origins=yes --error-exitcode=99 \
        "$bin" 2>&1) || true
    # Extract the summary line
    local summary
    summary=$(echo "$vout" | grep -E "ERROR SUMMARY|definitely lost|total heap" | tail -5 || true)
    if echo "$vout" | grep -q "ERROR SUMMARY: 0 errors"; then
        log_raw "  [VALGRIND-PASS] $label — 0 errors, 0 leaks"
        (( PASS++ )) || true
    else
        log_raw "  [VALGRIND-FAIL] $label"
        log_raw "$summary"
        (( FAIL++ )) || true
    fi
}

# =============================================================================
# Main
# =============================================================================

{
sep
echo "  TURING BENCH — Onu vs C  |  $(ts)"
echo "  Timing every sample; then Valgrind memory check on all Onu binaries."
sep
echo ""
} | tee "$OUT"

# ── 1. Build Onu compiler (release) ──────────────────────────────────────────
log "Building Onu compiler (cargo build --release) …"
if ! cargo build --release --quiet 2>/dev/null; then
    log "ERROR: cargo build failed — aborting."
    exit 1
fi
log "  OK"
echo "" | tee -a "$OUT"

# ── 2. Compile C benchmarks ───────────────────────────────────────────────────
log "Compiling C reference benchmarks (clang -O3) …"
declare -A C_BINS
c_sources=(
    cbench_fib_naive.c:c_fib_naive_bin
    cbench_collatz.c:c_collatz_bin
    cbench_ackermann.c:c_ackermann_bin
    cbench_gcd.c:c_gcd_bin
    cbench_tak.c:c_tak_bin
    cbench_mccarthy91.c:c_mccarthy91_bin
    cbench_rule110.c:c_rule110_bin
    cbench_pepin.c:c_pepin_bin
    cbench_hanoi.c:c_hanoi_bin
    cbench_primal_scroll.c:c_primal_scroll_bin
)
for pair in "${c_sources[@]}"; do
    src="${pair%%:*}"; bin="${pair##*:}"
    if [ -f "$src" ]; then
        if compile_c "$src" "$bin"; then
            log "  OK  $src → $bin"
            C_BINS["$bin"]=1
        else
            log "  FAIL $src"
        fi
    else
        log "  SKIP $src (not found)"
    fi
done
echo "" | tee -a "$OUT"

# ── 3. Compile Onu samples ────────────────────────────────────────────────────
log "Compiling Onu samples …"
onu_samples=(
    samples/fib_naive_only.onu
    samples/fib_bench.onu
    samples/collatz_bench.onu
    samples/ackermann_bench.onu
    samples/gcd_measure.onu
    samples/tak_balance.onu
    samples/mccarthy91.onu
    samples/rule110.onu
    samples/pepin_test.onu
    samples/hanoi.onu
    samples/primal_scroll.onu
)
for src in "${onu_samples[@]}"; do
    name=$(basename "$src" .onu)
    if compile_onu "$src"; then
        log "  OK  $src → ${name}_bin"
    else
        log "  FAIL $src"
    fi
done
echo "" | tee -a "$OUT"

# =============================================================================
# SECTION A: TIMING BENCHMARKS
# =============================================================================

{
sep
echo "  SECTION A — TIMING  (wall time / user time / RSS)"
sep
} | tee -a "$OUT"

# ── Naive Fibonacci ───────────────────────────────────────────────────────────
{
sep2; echo "  Naive Fibonacci  fib(40) = 102334155"; sep2
} >> "$OUT"
time_bin "C  (cbench_fib_naive)" "c_fib_naive_bin"
time_bin "Onu (fib_naive_only)"  "fib_naive_only_bin"

# ── Fibonacci Benchmark (naive + TCO + range) ─────────────────────────────────
{
sep2; echo "  Fibonacci Benchmark (naive / tco / range)"; sep2
} >> "$OUT"
time_bin "Onu (fib_bench)" "fib_bench_bin"

# ── Collatz Conjecture ────────────────────────────────────────────────────────
{
sep2; echo "  Collatz  steps(1..1,000,000) = 131,434,424"; sep2
} >> "$OUT"
time_bin "C  (cbench_collatz)"  "c_collatz_bin"
time_bin "Onu (collatz_bench)"  "collatz_bench_bin"

# ── Ackermann ─────────────────────────────────────────────────────────────────
{
sep2; echo "  Ackermann(3,11) = 16381"; sep2
} >> "$OUT"
time_bin "C  (cbench_ackermann)"  "c_ackermann_bin"
time_bin "Onu (ackermann_bench)"  "ackermann_bench_bin"

# ── GCD (Euclidean Measure) ───────────────────────────────────────────────────
{
sep2; echo "  GCD — Euclidean Measure  gcd(1M, 1M-1) = 1"; sep2
} >> "$OUT"
time_bin "C  (cbench_gcd)"   "c_gcd_bin"
time_bin "Onu (gcd_measure)" "gcd_measure_bin"

# ── Tak Function ──────────────────────────────────────────────────────────────
{
sep2; echo "  Tak Function  tak(18,12,6) = 7"; sep2
} >> "$OUT"
time_bin "C  (cbench_tak)"    "c_tak_bin"
time_bin "Onu (tak_balance)"  "tak_balance_bin"

# ── McCarthy 91 ───────────────────────────────────────────────────────────────
{
sep2; echo "  McCarthy 91  mc91(100) = 91"; sep2
} >> "$OUT"
time_bin "C  (cbench_mccarthy91)"  "c_mccarthy91_bin"
time_bin "Onu (mccarthy91)"        "mccarthy91_bin"

# ── Rule 110 ──────────────────────────────────────────────────────────────────
{
sep2; echo "  Rule 110 — 16 cells × 16 generations"; sep2
} >> "$OUT"
time_bin "C  (cbench_rule110)"  "c_rule110_bin"
time_bin "Onu (rule110)"        "rule110_bin"

# ── Pépin Test ────────────────────────────────────────────────────────────────
{
sep2; echo "  Pépin Primality  F(1)..F(4) all prime"; sep2
} >> "$OUT"
time_bin "C  (cbench_pepin)"  "c_pepin_bin"
time_bin "Onu (pepin_test)"   "pepin_test_bin"

# ── Tower of Hanoi ────────────────────────────────────────────────────────────
{
sep2; echo "  Tower of Hanoi  n=12 disks = 4095 moves"; sep2
} >> "$OUT"
time_bin "C  (cbench_hanoi)"  "c_hanoi_bin"
time_bin "Onu (hanoi)"        "hanoi_bin"

# ── Primal Scroll (BF interpreter) ───────────────────────────────────────────
{
sep2; echo "  Primal Scroll (Brainfuck)  ++++++++[>++++++<-]>. → '0'"; sep2
} >> "$OUT"
time_bin "C  (cbench_primal_scroll)"  "c_primal_scroll_bin"
time_bin "Onu (primal_scroll)"        "primal_scroll_bin"

# =============================================================================
# SECTION B: VALGRIND MEMORY CHECKS  (Onu binaries only)
# =============================================================================

{
echo ""
sep
echo "  SECTION B — VALGRIND  (Onu binaries — expected: 0 errors, 0 leaks)"
sep
} | tee -a "$OUT"

onu_bins=(
    fib_naive_only_bin
    fib_bench_bin
    collatz_bench_bin
    ackermann_bench_bin
    gcd_measure_bin
    tak_balance_bin
    mccarthy91_bin
    rule110_bin
    pepin_test_bin
    hanoi_bin
    primal_scroll_bin
)

for bin in "${onu_bins[@]}"; do
    label="${bin%_bin}"
    if [ -x "./$bin" ]; then
        valgrind_bin "$label" "$bin"
    else
        log_raw "  [SKIP-VALGRIND] $label — binary not found"
        (( SKIP++ )) || true
    fi
done

# =============================================================================
# SUMMARY
# =============================================================================

{
echo ""
sep
echo "  SUMMARY  |  $(ts)"
printf "  Valgrind:  PASS=%-4d  FAIL=%-4d  SKIP=%d\n" "$PASS" "$FAIL" "$SKIP"
sep
} | tee -a "$OUT"

echo ""
log "Results written to $OUT"
