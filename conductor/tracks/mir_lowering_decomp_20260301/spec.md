# Specification: MIR Lowering Service Decomposition

## Overview
This track aims to refactor the `MirLoweringService` from a monolithic "God Class" into a pluggable, trait-based architecture. The primary focus is resolving Single Responsibility Principle (SRP) and DRY violations while maintaining the perfect memory safety (zero leaks) recently achieved.

## Functional Requirements
- **Trait-Based Lowering**: Decompose the `lower_expression` match arm into individual implementations of an `ExprLowerer` trait.
- **Lowering Context**: Introduce a `LoweringContext` struct to encapsulate shared dependencies (`RegistryService`, `EnvironmentPort`) and prevent service-leakage into sub-modules.
- **RAII Resource Management**: Centralize the resource cleanup policy (mark consumed, take/emit pending drops) using a wrapper or RAII-style guard to eliminate duplicated cleanup logic across lowering steps.
- **Strategy Modules**: Create a structured directory `src/application/use_cases/mir_lowering/` containing specialized modules for each expression category.

## Non-Functional Requirements
- **Clean Architecture Adherence**: Maintain strict boundaries between use cases and domain entities.
- **DRY Policy Enforcement**: Eliminate repeated resource management sequences in match arms.
- **Zero-Cost Abstractions**: Ensure the refactor does not introduce significant performance overhead in the compiler.

## Acceptance Criteria
- [ ] `MirLoweringService` is reduced to a thin orchestrator (Facade).
- [ ] Each HIR expression variant has a dedicated lowerer implementation in a separate module.
- [ ] Resource cleanup logic is centralized and automated via RAII/Guard.
- [ ] All existing sample programs compile and execute with **zero memory leaks** (Valgrind verified).
- [ ] Each lowering strategy can be unit-tested in isolation without instantiating the full `MirLoweringService`.
- [ ] Regression test: LLVM IR output remains functionally equivalent to the current baseline.

## Out of Scope
- Adding new language features or HIR expression types.
- Modifying the `MirBuilder` state management (unless required for RAII).
- Refactoring the `OnuCodegen` or `Parser` layers.
