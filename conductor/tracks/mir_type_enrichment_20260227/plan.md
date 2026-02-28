# Implementation Plan: MIR Type Enrichment

## Phase 1: Preparation and Testing
- [ ] Task: Reproduce the current flakiness in a failing test case.
    - [ ] Create a new test case in `tests/pipeline_test.rs` that calls a builtin function not in the current `CallStrategy` string-match list.
    - [ ] Verify that the generated LLVM IR has the wrong return type (i64 instead of string struct).

## Phase 2: Domain Layer Update
- [ ] Task: Update the `MirInstruction::Call` in `src/domain/entities/mir.rs`.
    - [ ] Add `return_type: OnuType` and `arg_types: Vec<OnuType>` to the `Call` variant.
    - [ ] Update any existing tests or code that construct `MirInstruction::Call` to provide placeholder or default types.

## Phase 3: Application Layer Enrichment
- [ ] Task: Enrich `MirInstruction::Call` during lowering.
    - [ ] In `MirLoweringService::lower_expression` in `src/application/use_cases/mir_lowering_service.rs`, use `registry.get_signature(name)` to resolve the return and argument types.
    - [ ] Populate the `MirInstruction::Call` fields with these resolved types.

## Phase 4: Backend Strategy Refactor
- [ ] Task: Update `CallStrategy::generate` in `src/adapters/codegen/strategies.rs`.
    - [ ] Replace the hardcoded string-matching logic with code that reads `inst.return_type` and `inst.arg_types`.
    - [ ] Map these `OnuType` variants to the correct LLVM types.

## Phase 5: Verification
- [ ] Task: Verify the fix with the reproduction test case.
    - [ ] Run the test created in Phase 1 and confirm it now passes with the correct LLVM IR.
- [ ] Task: Conductor - User Manual Verification 'MIR Type Enrichment' (Protocol in workflow.md)
