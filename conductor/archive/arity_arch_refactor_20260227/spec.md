# Specification: Onu Compiler Refactor (Arity & Architecture)

## Overview
This track focuses on the foundational refactoring of the Onu compiler to move from string-driven and newline-bounded mechanisms to a purely type-driven, arity-bounded architecture. It also aims to close remaining Clean Architecture violations.

## Functional Requirements
1.  **Semantic Binary Operations:**
    -   Introduce `HirExpression::BinaryOp(HirBinOp, ...)` in `hir.rs`.
    -   Map strings to `HirBinOp` in `LoweringService` (AST→HIR).
    -   Update `MirLoweringService` to map `HirBinOp` to `MirBinOp` exhaustively with no string matching.
2.  **Two-Pass Parsing:**
    -   Implement a header scan pre-pass to register all function/behavior signatures in the `SymbolTable`.
    -   Perform the full parse pass using `SymbolTable::get_arity()` to bound `utilizes` argument collection.
    -   Remove `TokenLineStart` and the `LayoutService` once arity-driven parsing is stable.
3.  **Pipeline Decomposition:**
    -   Refactor `CompilationPipeline::compile()` into discrete methods: `lex()`, `parse()`, `lower_hir()`, `lower_mir()`, and `emit_ir()`.
    -   Ensure each stage is independently testable.
    -   Move the registry "freeze" into the `emit_ir()` stage.
4.  **Clean Architecture Inversion:**
    -   Inject `&dyn EnvironmentPort` into `AnalysisService` and `ModuleService` for logging.
    -   Extract a `BehaviorRegistryPort` trait in the `domain` layer.
    -   Update `OwnershipRule` to depend on the `BehaviorRegistryPort` instead of `RegistryService`.

## Non-Functional Requirements
-   Maintain 100% compatibility with existing `.onu` samples.
-   Adhere to SOLID and KISS principles.
-   Improve compiler stage testability.

## Acceptance Criteria
-   [ ] All `samples/*.onu` files compile and execute correctly.
-   [ ] `MirLoweringService` contains zero string-literal matches for binary operations.
-   [ ] `TokenLineStart` and `LayoutService` are deleted.
-   [ ] `CompilationPipeline` is decomposed into isolated methods.
-   [ ] `OwnershipRule` has no dependency on the `application` layer.

## Out of Scope
-   New language features (e.g., classes, generics).
-   Optimization passes (beyond LLVM's default).