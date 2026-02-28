# Implementation Plan: Onu Compiler Refactor (Arity & Architecture)

## Phase 1: Semantic Binary Operations (TDD) [checkpoint: 54f2f5b]
- [x] Task: Create failing tests for `HirBinOp` and `HirExpression::BinaryOp` in `hir.rs`
- [x] Task: Define `HirBinOp` and `HirExpression::BinaryOp` in `domain/entities/hir.rs` (Green)
- [x] Task: Create failing tests for `LoweringService` to map strings to `HirBinOp` (Red)
- [x] Task: Implement string-to-enum mapping in `LoweringService` (Green)
- [x] Task: Create failing tests for `MirLoweringService` to map `HirBinOp` to `MirBinOp` (Red)
- [x] Task: Update `MirLoweringService` to map `HirBinOp` to `MirBinOp` exhaustively (Green)
- [x] Task: Verify all samples pass with new `HirBinOp` structure
- [x] Task: Conductor - User Manual Verification 'Phase 1: Semantic Binary Operations' (Protocol in workflow.md) 54f2f5b

## Phase 2: Two-Pass Parsing (TDD) [checkpoint: d387639]
- [x] Task: Create failing tests for `Parser::scan_headers()` to register signatures (Red)
- [x] Task: Implement `Parser::scan_headers()` to register all function/behavior signatures (Green)
- [x] Task: Create failing tests for arity-bounded `utilizes` collection in `Parser::parse_expression()` (Red)
- [x] Task: Update `Parser::parse_expression()` to use `SymbolTable::get_arity()` (Green)
- [x] Task: Create failing tests that fail *without* `TokenLineStart` (Red) (Covered by existing samples)
- [x] Task: Remove `TokenLineStart`, `TokenIndent`, `TokenDedent`, and `LayoutService` (Green)
- [x] Task: Verify all samples compile and execute correctly
- [x] Task: Conductor - User Manual Verification 'Phase 2: Two-Pass Parsing' (Protocol in workflow.md) d387639

## Phase 3: Pipeline Decomposition ## Phase 3: Pipeline Decomposition & Clean Arch (TDD) Clean Arch (TDD) [checkpoint: eed1d89]
- [x] Task: Create failing tests for individual `CompilationPipeline` stages (Red)
- [x] Task: Decompose `CompilationPipeline::compile()` into stage methods (Green)
- [x] Task: Create failing tests for injected `EnvironmentPort` in `AnalysisService` and `ModuleService` (Red)
- [x] Task: Inject `&dyn EnvironmentPort` and update logging in services (Green)
- [x] Task: Create failing tests for `OwnershipRule` using `BehaviorRegistryPort` (Red)
- [x] Task: Define `BehaviorRegistryPort` and refactor `OwnershipRule` (Green)
- [x] Task: Verify all tests and samples pass with the refactored architecture
- [x] Task: Conductor - User Manual Verification 'Phase 3: Pipeline Decomposition - [~] Task: Conductor - User Manual Verification 'Phase 3: Pipeline Decomposition & Clean Arch' (Protocol in workflow.md) Clean Arch' (Protocol in workflow.md) eed1d89

## Phase 4: Final Validation & Cleanup
- [ ] Task: Run full regression suite on all samples and unit tests
- [ ] Task: Verify >80% test coverage for new compiler stages
- [ ] Task: Final code review and documentation update
- [ ] Task: Conductor - User Manual Verification 'Phase 4: Final Validation & Cleanup' (Protocol in workflow.md)