# Implementation Plan: Pure LLVM Optimization Suite

## Phase 1: Direct Return Path (Multiple Exit Points)
- [ ] Task: Refactor MIR lowering to support multiple terminal returns
    - [ ] **Hypothesis:** Allowing terminal MIR blocks to specify their own return operand will eliminate the need for a central merge block and `phi` node.
    - [ ] Create failing test (Red) where IR shows `phi` nodes for simple branching returns.
    - [ ] Update `MirBuilder` and terminal logic to support distributed `Return` terminators.
    - [ ] Verify test (Green): IR now shows multiple `ret` instructions.
- [ ] Task: Adjust Codegen Strategy for direct LLVM return
    - [ ] **Hypothesis:** Direct emission of LLVM `ret` in terminal blocks will reduce jump overhead.
    - [ ] Modify `OnuCodegen` to emit `ret` immediately upon encountering a `MirTerminator::Return`.
    - [ ] Verify IR and benchmark improvement.
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Direct Return Path' (Protocol in workflow.md)

## Phase 2: Built-in Integer Specialization
- [ ] Task: Implement MIR specialization for arithmetic ops
    - [ ] **Hypothesis:** Inlining `added-to` and `decreased-by` as MIR `BinaryOperation` instructions will bypass behavior call overhead.
    - [ ] Create Red test: Ackermann IR should show `sub` and `add` instead of `call`.
    - [ ] Update `LoweringService` to intercept these behaviors for `i64` types.
    - [ ] Verify Green and Refactor.
- [ ] Task: Implement MIR specialization for comparison ops
    - [ ] **Hypothesis:** Inlining `exceeds`, `falls-short-of`, and `matches` as MIR comparisons will further improve branch performance.
    - [ ] Create Red test for comparison IR.
    - [ ] Implement specialization in lowering.
    - [ ] Verify Green and benchmark.
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Built-in Integer Specialization' (Protocol in workflow.md)

## Phase 3: Stack Frame Elision (Leaf Functions)
- [ ] Task: Implement Leaf Function detection logic
    - [ ] **Hypothesis:** Functions that contain no `Drop` instructions and no behavior calls can safely elide the Ọ̀nụ stack frame.
    - [ ] Create Red test: Ackermann function (once specialized) should qualify as a leaf.
    - [ ] Implement analyzer to mark `is_leaf` on `MirFunction`.
    - [ ] Verify detection accuracy.
- [ ] Task: Optimize Codegen for marked Leaf Functions
    - [ ] **Hypothesis:** Skipping scope setup for Leaf Functions will significantly reduce stack churn.
    - [ ] Update codegen to bypass arena/scope initialization for `is_leaf` functions.
    - [ ] Verify Green: Benchmark Ackermann and verify zero leaks.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Stack Frame Elision' (Protocol in workflow.md)

## Phase 4: Final Benchmarking & Verification
- [ ] Task: Perform exhaustive performance comparison
    - [ ] Run `cbench` suite and compare Ackermann results with baseline.
    - [ ] Verify target performance (< 0.3s).
- [ ] Task: Comprehensive Valgrind memory leak audit
    - [ ] Run all optimization samples through Valgrind to ensure zero regressions in memory purity.
- [ ] Task: Conductor - User Manual Verification 'Phase 4: Final Benchmarking & Verification' (Protocol in workflow.md)
