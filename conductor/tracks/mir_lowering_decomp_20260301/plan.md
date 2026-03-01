# Implementation Plan: MIR Lowering Service Decomposition

## Phase 1: Context & Core Trait Infrastructure
Goal: Establish the foundation for the new pluggable architecture.

- [ ] Task: Define `LoweringContext` struct
    - [ ] Create `LoweringContext` in `mir_lowering_service.rs` containing `RegistryService` and `EnvironmentPort`.
    - [ ] Update `MirLoweringService` to initialize and pass this context.
- [ ] Task: Define `ExprLowerer` trait
    - [ ] Create `trait ExprLowerer` in `src/application/use_cases/mir_lowering/mod.rs`.
    - [ ] Define the `lower` method signature including context, builder, and flags.
- [ ] Task: Implement Resource Guard RAII
    - [ ] Create a `ResourceGuard` or similar helper in `MirBuilder` to automate `take_pending_drops` and `emit`.
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Context & Core Trait Infrastructure' (Protocol in workflow.md)

## Phase 2: Expression Decomposition (Standard Expressions)
Goal: Migrate basic expression logic to individual strategy modules.

- [ ] Task: Implement `LiteralLowerer` and `VariableLowerer`
    - [ ] Move logic from `mir_lowering_service.rs` to dedicated modules.
    - [ ] Write isolated unit tests for both.
- [ ] Task: Implement `BinaryOpLowerer`
    - [ ] Move logic to `src/application/use_cases/mir_lowering/lower_expr.rs`.
    - [ ] Integrate with RAII cleanup.
- [ ] Task: Implement `IndexLowerer` and `EmitLowerer`
    - [ ] Move logic and verify resource consumption policy.
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Expression Decomposition' (Protocol in workflow.md)

## Phase 3: Complex Control Flow & Blocks
Goal: Migrate high-level constructs and finalize the Facade.

- [ ] Task: Implement `CallLowerer`
    - [ ] Refactor call logic to use `ExprLowerer` and `LoweringContext`.
- [ ] Task: Implement `IfLowerer` and `BlockLowerer`
    - [ ] Migrate complex branch reconciliation logic.
- [ ] Task: Refactor `MirLoweringService` into a Dispatcher
    - [ ] Replace the giant match arm with a call to the strategy dispatcher.
- [ ] Task: Final Leak Verification
    - [ ] Run all samples through Valgrind to ensure zero regressions.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Complex Control Flow & Blocks' (Protocol in workflow.md)
