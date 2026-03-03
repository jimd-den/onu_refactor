# Implementation Plan: Tail Call Optimization (TCO)

## Phase 1: Setup & TCO Identification (Adapters/Lowering)
- [ ] Task: Add TCO metadata to MIR representation
    - [ ] **Hypothesis:** Adding TCO metadata to MIR call instructions will allow the lowering service to distinguish tail calls from regular calls.
    - [ ] Create a test case (Red) where a call instruction fails to signal TCO without metadata.
    - [ ] Add a `is_tail_call` boolean or enum to MIR call instructions.
    - [ ] Update MIR builder to propagate tail-call hints.
    - [ ] Verify test (Green) and Refactor.
- [ ] Task: Implement tail-call identification in MIR analysis
    - [ ] **Hypothesis:** Identifying calls at the terminal position of a MIR block, with no subsequent instructions besides a return, will correctly identify tail calls.
    - [ ] Create failing tests for self-recursive and mutual recursion scenarios (Red).
    - [ ] Implement identification service in MIR analysis.
    - [ ] Ensure identification respects Ọ̀nụ's linear type rules (all drops must happen before the call).
    - [ ] Verify tests (Green) and Refactor.
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Setup & TCO Identification' (Protocol in workflow.md)

## Phase 2: LLVM musttail Marking (Adapters/Codegen)
- [ ] Task: Update the LLVM lowering service to handle `musttail`
    - [ ] **Hypothesis:** Applying the `musttail` attribute via Inkwell will guarantee LLVM performs TCO, even at -O0.
    - [ ] Create a reproduction test (Red) that triggers a stack overflow for a deep recursive function without `musttail`.
    - [ ] Modify the `LoweringService` to apply the `musttail` attribute to identified calls.
    - [ ] Verify test (Green): Function now executes without stack overflow.
    - [ ] Refactor.
- [ ] Task: Conductor - User Manual Verification 'Phase 2: LLVM musttail Marking' (Protocol in workflow.md)

## Phase 3: Mutual Recursion & Complex Arguments
- [ ] Task: Implement `musttail` marking for mutual recursion
    - [ ] **Hypothesis:** `musttail` will work correctly across module boundaries if function signatures match exactly.
    - [ ] Create a cross-module mutual recursion test (Red) that fails due to stack overflow or missing `musttail`.
    - [ ] Implement mutual recursion identification and marking.
    - [ ] Verify test (Green).
    - [ ] Refactor.
- [ ] Task: Handle argument re-ordering/resizing for `musttail`
    - [ ] **Hypothesis:** `musttail` semantics require exact signature matches; our lowering service must ensure this before applying the attribute.
    - [ ] Create a test with complex argument passing (Red).
    - [ ] Implement signature validation and argument lowering for `musttail`.
    - [ ] Verify test (Green).
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Mutual Recursion & Complex Arguments' (Protocol in workflow.md)

## Phase 4: Linear Type Safety & Verification
- [ ] Task: Ensure all resources are dropped before the tail call
    - [ ] **Hypothesis:** The compiler must emit `drop` calls for all linear types *before* the `musttail` call to prevent memory leaks, as `musttail` prohibits any instructions after it.
    - [ ] Create a test with resources and a tail call (Red), verifying leaks or invalid IR if drops are missing or misplaced.
    - [ ] Audit and adjust the lowering service for correct drop placement.
    - [ ] Verify test (Green) and run through Valgrind for final confirmation.
- [ ] Task: Conductor - User Manual Verification 'Phase 4: Linear Type Safety & Verification' (Protocol in workflow.md)
