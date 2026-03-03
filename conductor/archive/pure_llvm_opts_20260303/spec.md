# Specification: Pure LLVM Optimization Suite

## Overview
Implement a suite of three key optimizations in the Ọ̀nụ compiler to bridge the performance gap with C, focusing on deep recursion scenarios like the Ackermann function. The implementation must adhere to Pure LLVM principles, Clean Architecture, and maintain zero-leak memory purity.

## Functional Requirements

### 1. Direct Return Path (Multiple Exit Points)
- **Optimization:** Refactor MIR lowering and LLVM codegen to emit multiple `ret` instructions directly from terminal blocks.
- **Goal:** Eliminate the jump to a `common.ret` block and the use of `phi` nodes for function results in tail or simple return scenarios.
- **Scope:** All behaviors should support distributed return points.

### 2. Built-in Integer Specialization
- **Optimization:** Implement MIR-level inlining for standard integer operations.
- **Operations:** `added-to`, `decreased-by`, `exceeds`, `falls-short-of`, and `matches`.
- **Logic:** When the compiler identifies these operations applied to native integer types (e.g., `i64`), it should lower them directly to LLVM primitive instructions (`add`, `sub`, `icmp`) instead of behavior calls.

### 3. Stack Frame Elision (Leaf Function Optimization)
- **Optimization:** Implement automatic detection of "Leaf Functions"—behaviors that do not use linear types (resources) and perform only primitive operations.
- **Behavior:** For detected leaf functions, skip the creation of the standard Ọ̀nụ scope/frame bookkeeping, allowing for minimal LLVM stack usage.
- **Safety:** Must strictly verify that no resources are present in the function's scope before eliding the frame.

## Non-Functional Requirements
- **Performance:** Ackermann(3, 11) performance should improve significantly, aiming to approach C's execution time.
- **Memory Safety:** Every optimization must pass Valgrind with zero heap leaks.
- **Clean Architecture:** Optimizations should be implemented in the appropriate layers (e.g., MIR analysis in Use Cases, special lowering in Adapters).

## Acceptance Criteria
- [ ] Ackermann(3, 11) benchmark shows measurable speedup (target < 0.3s).
- [ ] LLVM IR for specialized behaviors shows primitive instructions instead of `call`.
- [ ] LLVM IR shows multiple `ret` instructions and reduced `phi` usage.
- [ ] Valgrind confirms no memory leaks for all TCO and optimization samples.
- [ ] Standard test suite passes without regressions.

## Out of Scope
- Global whole-program optimizations.
- Non-integer built-in specialization (e.g., string operations).
- Profile-guided optimizations.
