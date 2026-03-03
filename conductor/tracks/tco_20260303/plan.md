# Implementation Plan: Tail Call Optimization (TCO)

## Phase 1: Setup & TCO Identification (Adapters/Lowering)
- [x] Task: Add TCO metadata to MIR representation
    - [x]
    - [x]
    - [x]
    - [x]
    - [x]
- [x] Task: Implement tail-call identification in MIR analysis
    - [x]
    - [x]
    - [x]
    - [x]
    - [x]
- [x] Task: Conductor - User Manual Verification 'Phase 1: Setup & TCO Identification' (Protocol in workflow.md)

## Phase 2: LLVM musttail Marking (Adapters/Codegen)
- [x] Task: Update the LLVM lowering service to handle `musttail` (using set_tail_call)
    - [x]
    - [x]
    - [x]
    - [x]
    - [x]
- [x] Task: Conductor - User Manual Verification 'Phase 2: LLVM musttail Marking' (Protocol in workflow.md)

## Phase 3: Mutual Recursion & Complex Arguments
- [x] Task: Implement `musttail` marking for mutual recursion (Refactored MIR lowering)
    - [x]
    - [x]
    - [x]
    - [x]
    - [x]
- [x] Task: Handle argument re-ordering/resizing for tail call
    - [x]
    - [x]
    - [x]
    - [x]
- [x] Task: Conductor - User Manual Verification 'Phase 3: Mutual Recursion & Complex Arguments' (Protocol in workflow.md)

## Phase 4: Linear Type Safety & Verification
- [x] Task: Ensure all resources are dropped before the tail call
    - [x] **Hypothesis:** The compiler must emit `drop` calls for all linear types *before* the `musttail` call to prevent memory leaks, as `musttail` prohibits any instructions after it.
    - [x] Create a test with resources and a tail call (Red), verifying leaks or invalid IR if drops are missing or misplaced.
    - [x] Audit and adjust the lowering service for correct drop placement.
    - [x] Verify test (Green) and run through Valgrind for final confirmation.
- [x] Task: Conductor - User Manual Verification 'Phase 4: Linear Type Safety & Verification' (Protocol in workflow.md)
