# ISSUE: [CRITICAL] Segfault in N-Dim Memoization and Sub-optimal Pipeline Ordering

## 1. The Essential Experience (Impact)
The **Onu Compiler**'s promise of "Zero-Cost Complexity Collapse" is currently broken. Users expecting high-performance recursive execution (Ackermann, Collatz) are encountering **Segmentation Faults** instead of sub-second results. This violates the core pillar of **Predictable High-Performance**.

## 2. Structural Audit (Root Cause Analysis)

### A. Interface Adapter Corruption (The Crash)
*   **Location:** `src/adapters/codegen/strategies.rs` (`TypedStoreStrategy`)
*   **The Violation:** The `CompoundMemoStrategy` (Application Layer) instructs the Codegen (Interface Adapter) to store a value into the occupancy flag.
*   **The Detail:** 
    *   The occupancy flag is typed as **`OnuType::I8`**.
    *   The constant value being stored is `1`, which the MIR Builder emits as a **64-bit integer (`I64`)**.
    *   The `TypedStoreStrategy` bitcasts the destination pointer to `i8*` but attempts to `store` the raw `i64` value without a `Trunc` instruction.
*   **Result:** Corrupted LLVM IR that triggers a **Segmentation Fault** at runtime.

### B. Policy Ordering Conflict (The Performance Gap)
*   **Location:** `src/application/use_cases/stages/mir_stage.rs`
*   **The Violation:** `TcoPass` (Tail-Call Optimization) is executing **BEFORE** `MemoPass`.
*   **The Detail:** 
    *   `TcoPass` identifies the tail-recursive call in Ackermann and converts it into a `Branch` (loop jump).
    *   `MemoPass` only searches for `MirInstruction::Call` to wrap in cache lookups.
    *   Because the call has been erased by the TCO pass, the cache lookup is never injected for the most frequent path.
*   **Result:** The algorithm retains $O(2^n)$ complexity for all tail-recursive steps, preventing sub-second execution.

## 3. Evidence Report

*   **Crash Trace:** `bash: line 1: 680542 Segmentation fault (core dumped) ./ackermann_bench_bin`
*   **Codegen Mismatch (Line 858 of strategies.rs):**
    ```rust
    // Logic missing truncation guard for narrower destination types
    builder.build_store(typed_ptr, val).unwrap(); 
    ```
*   **Pipeline Sequence (Line 30 of mir_stage.rs):**
    ```rust
    let mir_program = TcoPass::run(mir_program); // Erases calls
    // ...
    let mir_program = MemoPass::run(mir_program, self.registry); // Misses optimized loops
    ```

## 4. Prioritized Action Plan

### Phase 1: Stabilization (Boundary Protection)
1.  **Refactor `TypedStoreStrategy`:** Update the strategy to compare the bit-width of the source `MirOperand` against the target `OnuType`. If `source_bits > target_bits`, emit an LLVM `Trunc` instruction before the `store`.
2.  **Explicit Typings:** Update `CompoundMemoStrategy.rs` to use `MirLiteral::I8(1)` if possible, or rely on the `TypedStore` fix.

### Phase 2: Inversion (Strategic Pipeline Reordering)
1.  **Reorder `MirStage.rs`:** Move `MemoPass::run` to execute immediately after `MirLoweringService::lower_program` and before any `TcoPass` or `InlinePass` runs.
2.  **Reasoning:** All recursive calls must be "Captured" by the memoization logic while they are still in `Call` form. The subsequent `TcoPass` will then optimize the recursion *inside* the generated `.inner` function.

## 5. Boss's Verdict
This is a classic failure at the **Policy vs. Detail** boundary. The compiler's "Brain" (MIR Pass) made assumptions that the "Hands" (Codegen) could not fulfill. Tighten the type-safety at the boundary and correct the pipeline sequence to unlock the requested performance.

**Status:** IDENTIFIED / PENDING FIX.
