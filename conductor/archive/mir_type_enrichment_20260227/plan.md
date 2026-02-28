# Implementation Plan: MIR Type Enrichment

## Phase 1: Preparation and Testing
- [x] Task: Reproduce the current flakiness in a failing test case. 494591
    - [x] Create a new test case in `tests/pipeline_test.rs` that calls a builtin function not in the current `CallStrategy` string-match list.
    - [x] Verify that the generated LLVM IR has the wrong return type (i64 instead of string struct).

## Phase 2: Domain Layer Update
- [x] Task: Update the `MirInstruction::Call` in `src/domain/entities/mir.rs`. 495675
    - [x] Add `return_type: OnuType` and `arg_types: Vec<OnuType>` to the `Call` variant.
    - [x] Update any existing tests or code that construct `MirInstruction::Call` to provide placeholder or default types.

## Phase 3: Application Layer Enrichment
- [x] Task: Enrich `MirInstruction::Call` during lowering. 496956
    - [x] In `MirLoweringService::lower_expression` in `src/application/use_cases/mir_lowering_service.rs`, use `registry.get_signature(name)` to resolve the return and argument types.
    - [x] Populate the `MirInstruction::Call` fields with these resolved types.

## Phase 4: Backend Strategy Refactor
- [x] Task: Update `CallStrategy::generate` in `src/adapters/codegen/strategies.rs`. 498236
    - [x] Replace the hardcoded string-matching logic with code that reads `inst.return_type` and `inst.arg_types`.
    - [x] Map these `OnuType` variants to the correct LLVM types.

## Phase 5: Verification
- [x] Task: Verify the fix with the reproduction test case. 498236
    - [x] Run the test created in Phase 1 and confirm it now passes with the correct LLVM IR.
- [x] Task: Conductor - User Manual Verification 'MIR Type Enrichment' (Protocol in workflow.md)

## Phase: Review Fixes
- [x] Task: Apply review suggestions 0d9e472
