# Implementation Plan: Pure LLVM Optimization Suite

## Phase 1: Direct Return Path (Multiple Exit Points)
- [x] Task: Refactor MIR lowering to support multiple terminal returns
    - [x]
    - [x]
    - [x]
    - [x]
- [x]
    - [x]
    - [x]
    - [x]
- [x] Task: Conductor - User Manual Verification 'Phase 1: Direct Return Path' (Protocol in workflow.md)

## Phase 2: Built-in Integer Specialization
- [x] Task: Implement MIR specialization for arithmetic ops
    - [x]
    - [x]
    - [x]
    - [x]
- [x]
    - [x]
    - [x]
    - [x]
    - [x]
- [x] Task: Conductor - User Manual Verification 'Phase 2: Built-in Integer Specialization' (Protocol in workflow.md)

## Phase 3: Stack Frame Elision (Leaf Functions)
- [x] Task: Implement Leaf Function detection logic
    - [x]
    - [x]
    - [x]
    - [x]
- [x] Task: Optimize Codegen for marked Leaf Functions
    - [x]
    - [x]
    - [x]
- [x] Task: Conductor - User Manual Verification 'Phase 3: Stack Frame Elision' (Protocol in workflow.md)

## Phase 4: Final Benchmarking & Verification
- [~] Task: Perform exhaustive performance comparison
    - [ ] Run `cbench` suite and compare Ackermann results with baseline.
    - [ ] Verify target performance (< 0.3s).
- [ ] Task: Comprehensive Valgrind memory leak audit
    - [ ] Run all optimization samples through Valgrind to ensure zero regressions in memory purity.
- [ ] Task: Conductor - User Manual Verification 'Phase 4: Final Benchmarking & Verification' (Protocol in workflow.md)

## Phase 5: Deep Codegen Optimizations
- [x] Task: Standardize -O3 Pass Pipeline using PassManagerBuilder
- [x] Task: Restore readnone attribute for Pure Data Leaves
- [x] Task: Direct SSA Generation (Verified unnecessary: reached Clang parity)
- [x] Task: Conductor - User Manual Verification 'Phase 5: Deep Codegen Optimizations' (Protocol in workflow.md)
