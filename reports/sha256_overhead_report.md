# SHA-256 Overhead Analysis: Pure LLVM Onu vs C

## Problem Statement

Diagnose the ~2× overhead of the Onu SHA-256 benchmark vs C, design
memory-safe pure-LLVM optimizations, and measure their impact.

---

## Baseline Measurements (before optimizations)

| Implementation | User time (100 × 1000 hashes) | vs C   |
|----------------|-------------------------------|--------|
| C (`-O3`)      | 0.096 s                       | 1.00×  |
| Onu (original) | 0.186 s                       | 1.94×  |

---

## Overhead Root-Cause Analysis (LLVM IR inspection)

### 1. K-constant lookup — **biggest single source of overhead**

**C behaviour**: `K[t]` compiles to one `movl (%rax,%rcx,4), %edx` — a
single indexed memory load from a 256-byte cache-resident static array.

**Onu (original) behaviour**: The `sha256-k` function was a 200-line nested
if-else tree.  After LLVM's optimization passes it became a 4-level
switch-tree: 64 separate basic blocks (`bb16`…`bb111`) each holding one
constant, with a 6-branch binary decision path to reach each constant.

IR evidence:
```
bb140:                  ; preds = %bb111, %bb110, %bb109 … (64 predecessor BBs!)
```

**Impact**: Per compression round: 6 conditional branches instead of
1 load.  Over 1000 hashes × 64 rounds = 64 000 lookups, this is ≈384 000
unnecessary branch instructions (each also consuming BTB / predictor
entries and contributing icache pressure from the 64-BB expansion).

### 2. String hex encoding — **secondary overhead**

**C behaviour**: `printf("%08x", word)` — buffered libc output, no heap.

**Onu (original) behaviour**: `hash-hex` called `word-hex8` 8 times as
separate `fastcc` function calls.  Each `word-hex8` call:
- Bumped the arena pointer once (+39 bytes)
- Built a 16-char intermediate string in arena

`hash-hex` then stitched the 8 results together with 5 `llvm.memcpy` calls:
```
tail call void @llvm.memcpy…(…16…) × 4
tail call void @llvm.memcpy…(…32…) × 2
```
Arena per hash: 8 × 39 + 199 = **511 bytes**, with 5 memcpy calls.

### 3. Newline alloca in hot loop — **minor**

Each of the 1000 loop iterations contained `%nl_ptr = alloca i8` — a
stack allocation inside the loop body.  Normally hoisted by LLVM but
adds unnecessary IR noise.

### 4. Memoization compatibility analysis

| Function         | Args              | Memoizable? | Verdict                                      |
|------------------|-------------------|-------------|----------------------------------------------|
| `sha256-compress`| W16 + HashState   | No          | Shape args unsupported by HashMemoStrategy   |
| `sha256-k`       | Integer `t`       | Technically | But already inlined → memo wrapper is slower |
| `rotr32`         | 2 × Integer       | No benefit  | Too many unique (x,n) pairs per hash         |
| `hash-loop`      | Effect            | No          | Effect behaviors excluded                    |

**Conclusion**: SHA-256 is inherently non-memoizable for performance.
Every message is unique, every W16 schedule is unique.  Adding `with
diminishing` to `sha256-k` would prevent inlining and add cache-lookup
overhead instead of saving it.

**TCO assessment**: Recursion is handled correctly.  Both the outer
hash-loop (1000 iterations) and the inner 64-round compression loop
compile to proper LLVM phi-loops with zero recursive calls.  The 64
phi-nodes in `bb11` carry all 24 schedule+state words as registers.

---

## Optimizations Applied

### Optimization 1: K-table via `ConstantTableLoad` MIR instruction

**New infrastructure** (`MirInstruction::ConstantTableLoad`):
- Emits `@sha256_K = internal constant [64 x i64] [1116352408, …]` once
- Emits `getelementptr inbounds [64 x i64], …, i64 0, i64 %round`
- Emits `load i64, i64* %gep`

**Memory safety**: The global is `constant` — LLVM enforces read-only at
the IR level.  Bounds: `round` ∈ [0,63], all within the 64-entry table.
No arena allocation. No libc.

**IR before**: 64 basic blocks, 6-level branch tree per lookup  
**IR after**: 2 instructions per lookup (`getelementptr` + `load`)

### Optimization 2: `write-hex-word` stdlib op — single-buffer hex encoding

**New stdlib op** `write-hex-word(buf, word, base_offset)`:
- Takes the pre-allocated 64-char arena buffer
- Emits 8 MIR instruction groups (nibble-extract + branchless nibble→ASCII + byte-store) **inline** (not as a function call)
- Branchless nibble→ASCII: `char = nibble + 87 − (nibble_lt_10 × 39)` — no branches, compiles to `select`/`cmov`
- Returns the same `buf` pointer (in-place, no new allocation)

**IR before**: 8 × `call fastcc @word-hex8` + 5 × `llvm.memcpy`, 511 bytes/hash  
**IR after**: 1 arena bump (64-byte buffer) + 64 byte stores, 0 `memcpy`, 0 function calls  
**Functions eliminated**: `word-hex8` is gone from the IR entirely

---

## Results After Optimization

| Implementation   | User time (100 × 1000 hashes) | vs C   | vs Onu baseline |
|------------------|-------------------------------|--------|-----------------|
| C (`-O3`)        | 0.089 s                       | 1.00×  | —               |
| Onu (original)   | 0.186 s                       | 2.09×  | —               |
| Onu (optimized)  | 0.141 s                       | **1.58×**  | **+24% faster** |

**IR statistics**:

| Metric               | Before | After |
|----------------------|--------|-------|
| Functions            | 4      | 3     |
| K-lookup BBs         | 64+    | 0     |
| `llvm.memcpy` calls  | 5/hash | 0     |
| Arena bumps          | 9/hash | 1/hash|
| K-table access       | 6 branches | 1 GEP + 1 load |

---

## Remaining Overhead (1.58× vs C)

1. **Arena bump per hash** (the 64-byte buffer allocation): one global
   pointer load + GEP + store.  Unavoidable without a pre-allocated buffer
   reused across hashes (would require effect-level state).

2. **`hash-hex` as a separate function**: The write-hex-word stdlib op
   makes `hash-hex` a 200-instruction function body. At that size, LLVM's
   cost model may decline to inline it into `hash-loop`, leaving a `call
   fastcc` boundary.  C has no such boundary since it uses `printf` from
   a shared library anyway.

3. **x86_64 syscall overhead**: Onu uses raw `syscall` asm for each write
   (write(1, buf, 64) + write(1, "\n", 1) × 1000 = 2000 syscalls).
   The C version uses buffered `printf` (1–2 syscalls total after stdio
   flushing).  This syscall cost is visible in the `sys` time metric.

---

## Pure-LLVM Memory Safety Summary

All SHA-256 computation uses:
- **Arena allocator** (`@onu_arena = internal global [16MiB x i8]`) — no heap, no leaks, no use-after-free
- **`@sha256_K = internal constant`** — read-only by LLVM enforcement
- **`x86_64 inline asm syscall`** — no libc, no printf, no malloc
- **Zero `llvm.memcpy` on hot path** (after optimizations)
- **Zero heap allocations** — confirmed by valgrind (0 allocs, 0 leaks)

---

## Recommendations for Further Improvement

1. **Reuse hex buffer across hashes**: Pre-allocate one 64-byte buffer
   before `hash-loop` and pass it in.  This eliminates the 1 arena bump
   per hash (saves ~1000 global pointer loads/stores).

2. **Batch syscall output**: Collect all 1000 hex strings in a single
   64 KB arena buffer and issue a single `write` syscall at the end.
   This would eliminate 2000 syscalls and likely close the gap to C.

3. **Vectorize the compression loop**: With explicit SIMD operations
   (future `vec-and`, `vec-xor` etc.), eight hashes could be computed in
   parallel using AVX2 registers — matching the throughput of
   multi-buffer SHA-256 in OpenSSL.

4. **TCO + SROA confirmation**: Current phi-forest (24 phi nodes for
   W16+HashState) is already register-resident.  No further action needed.
