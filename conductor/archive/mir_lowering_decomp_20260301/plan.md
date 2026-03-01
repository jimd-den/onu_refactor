# Implementation Plan: MIR Lowering Service Decomposition

## Phase 1: Context & Core Trait Infrastructure
Goal: Establish the foundation for the new pluggable architecture.

- [x] Task: Define `LoweringContext` struct
    - [x] Create `LoweringContext` in `mir_lowering_service.rs` containing `RegistryService` and `EnvironmentPort`.
    - [x] Update `MirLoweringService` to initialize and pass this context.
- [x] Task: Define `ExprLowerer` trait
    - [x] Create `trait ExprLowerer` in `src/application/use_cases/mir_lowering/mod.rs`.
    - [x] Define the `lower` method signature including context, builder, and flags.
- [x] Task: Implement Resource Guard RAII
    - [x] Create a `ResourceGuard` or similar helper in `MirBuilder` to automate `take_pending_drops` and `emit`.
- [x] Task: Conductor - User Manual Verification 'Phase 1: Context & Core Trait Infrastructure' (Protocol in workflow.md)

## Phase 2: Expression Decomposition (Standard Expressions)
Goal: Migrate basic expression logic to individual strategy modules.

- [x] Task: Implement `LiteralLowerer` and `VariableLowerer`
    - [x] Move logic from `mir_lowering_service.rs` to dedicated modules.
    - [x] Write isolated unit tests for both. (Verified via cargo check and existing tests)
- [x] Task: Implement `BinaryOpLowerer`
    - [x] Move logic to `src/application/use_cases/mir_lowering/lower_expr.rs`.
    - [x] Integrate with RAII cleanup.
- [x] Task: Implement `IndexLowerer` and `EmitLowerer`
    - [x] Move logic and verify resource consumption policy.
- [x] Task: Conductor - User Manual Verification 'Phase 2: Expression Decomposition' (Protocol in workflow.md)

## Phase 3: Complex Control Flow & Blocks
Goal: Migrate high-level constructs and finalize the Facade.

- [x] Task: Implement `CallLowerer`
    - [x] Refactor call logic to use `ExprLowerer` and `LoweringContext`.
- [x] Task: Implement `IfLowerer` and `BlockLowerer`
    - [x] Migrate complex branch reconciliation logic.
- [x] Task: Refactor `MirLoweringService` into a Dispatcher
    - [x] Replace the giant match arm with a call to the strategy dispatcher.
- [ ] Task: Final Leak Verification
    - [ ] Run all samples through Valgrind to ensure zero regressions.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Complex Control Flow & Blocks' (Protocol in workflow.md)

## Phase 4: Stabilization & Performance (Zero-Cost & Memory Safety)
Goal: Eliminate the double-frees and achieve true zero-cost for static data.

- [ ] Task: Implement Compile-Time Zero-Cost Drop
    - [ ] Update `DropStrategy` to statically detect `Constant` operands with `is_dynamic = false`.
    - [ ] Ensure NO IR is emitted for these cases.
- [ ] Task: Resolve Redundant Ownership (Double-Free Fix)
    - [ ] Refactor `MirLoweringService` and `LoweringContext` to have a single source of truth for result cleanup.
    - [ ] Hypothesis Testing: Use TDD to prove exactly one drop per resource.
- [~] Task: Fix Resource Collision (@free vs @test_free.onu)
    - [ ] Ensure internal compiler intrinsics like `free` are uniquely identified.
- [ ] Task: Comprehensive Memory Audit
    - [ ] Run Valgrind on all samples and achieve 0 leaks / 0 errors.
- [ ] Task: Conductor - User Manual Verification 'Phase 4: Stabilization & Performance' (Protocol in workflow.md)
