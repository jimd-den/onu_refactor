# Track Specification: MIR Type Enrichment

## Goal
Enrich the `MirInstruction::Call` instruction with resolved return and argument type information. This eliminates the need for the `CallStrategy` (LLVM backend) to infer types using fragile string-matching on function names like "as-text" or "joined-with".

## Context
The current `CallStrategy::generate()` method in `src/adapters/codegen/strategies.rs` uses a hardcoded list of strings to determine if a function returns a "string struct" or a 64-bit integer. This is a significant source of bytecode flakiness and maintenance overhead.

## Technical Requirements
- **Enrich MIR Instruction**: Update `MirInstruction::Call` in `src/domain/entities/mir.rs` to include `return_type: OnuType` and `arg_types: Vec<OnuType>`.
- **Lowering Update**: Modify `MirLoweringService::lower_expression` in `src/application/use_cases/mir_lowering_service.rs` to resolve these types from the `RegistryService` at the point of MIR generation.
- **Backend Refactor**: Update `CallStrategy::generate` in `src/adapters/codegen/strategies.rs` to read the return and argument types directly from the instruction, removing all string-matching logic.

## Desired Outcome
The LLVM backend should be entirely agnostic of function names for type resolution. Adding a new builtin to `CoreModule` should automatically propagate its type information correctly through the entire pipeline without requiring any changes to the backend.
