# Implementation Plan: Onu Compiler Refactor (Arity & Architecture)

## Phase 1: Semantic Binary Operations (TDD)
- [x] Task: Create failing tests for `HirBinOp` and `HirExpression::BinaryOp` in `hir.rs`
- [x] Task: Define `HirBinOp` and `HirExpression::BinaryOp` in `domain/entities/hir.rs` (Green)
- [x] Task: Create failing tests for `LoweringService` to map strings to `HirBinOp` (Red)
- [x] Task: Implement string-to-enum mapping in `LoweringService` (Green)
- [x] Task: Create failing tests for `MirLoweringService` to map `HirBinOp` to `MirBinOp` (Red)
- [x] Task: Update `MirLoweringService` to map `HirBinOp` to `MirBinOp` exhaustively (Green)
- [x] Task: Verify all samples pass with new `HirBinOp` structure
- [~] Task: Conductor - User Manual Verification 'Phase 1: Semantic Binary Operations' (Protocol in workflow.md)

## Phase 2: Two-Pass Parsing (TDD)
- [ ] Task: Create failing tests for `Parser::scan_headers()` to register signatures (Red)
- [ ] Task: Implement `Parser::scan_headers()` to register all function/behavior signatures (Green)
- [ ] Task: Create failing tests for arity-bounded `utilizes` collection in `Parser::parse_expression()` (Red)
- [ ] Task: Update `Parser::parse_expression()` to use `SymbolTable::get_arity()` (Green)
- [ ] Task: Create failing tests that fail *without* `TokenLineStart` (Red)
- [ ] Task: Remove `TokenLineStart`, `TokenIndent`, `TokenDedent`, and `LayoutService` (Green)
- [ ] Task: Verify all samples compile and execute correctly
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Two-Pass Parsing' (Protocol in workflow.md)

## Phase 3: Pipeline Decomposition & Clean Arch (TDD)
- [ ] Task: Create failing tests for individual `CompilationPipeline` stages (Red)
- [ ] Task: Decompose `CompilationPipeline::compile()` into stage methods (Green)
- [ ] Task: Create failing tests for injected `EnvironmentPort` in `AnalysisService` and `ModuleService` (Red)
- [ ] Task: Inject `&dyn EnvironmentPort` and update logging in services (Green)
- [ ] Task: Create failing tests for `OwnershipRule` using `BehaviorRegistryPort` (Red)
- [ ] Task: Define `BehaviorRegistryPort` and refactor `OwnershipRule` (Green)
- [ ] Task: Verify all tests and samples pass with the refactored architecture
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Pipeline Decomposition & Clean Arch' (Protocol in workflow.md)

## Phase 4: Final Validation & Cleanup
- [ ] Task: Run full regression suite on all samples and unit tests
- [ ] Task: Verify >80% test coverage for new compiler stages
- [ ] Task: Final code review and documentation update
- [ ] Task: Conductor - User Manual Verification 'Phase 4: Final Validation & Cleanup' (Protocol in workflow.md)