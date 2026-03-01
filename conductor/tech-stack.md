# Tech Stack

## Programming Language
- **Rust (v2024 edition)**: The core language for the `onu` compiler, chosen for its memory safety, performance, and robust ecosystem.

## Primary Libraries & Frameworks
- **inkwell (LLVM 14)**: Used for high-level LLVM bindings to generate optimized machine code.
- **chrono**: Utilized for time-related operations.
- **either**: Used for functional-style error handling and branching.

## Compiler Architecture
- **Two-Pass Parsing**: Implements a header scan pre-pass to register behavior signatures before full body parsing, enabling arity-bounded argument collection.
- **Type-Driven Lowering**: Moves from string-based resolution to purely type-driven transformations across HIR and MIR stages.
- **Decoupled Transformation Services**: Following a major refactor to eliminate "God class" code smells, the `MirLoweringService` and `ModuleService` are now decomposed into focused, single-responsibility components.
- **Native Memory Management**: Implements a native linear types policy directly within the LLVM IR via `malloc`/`free` calls, completely eliminating the need for an external `runtime.c`.

## Build & Test Infrastructure
- **cargo**: The standard Rust build tool and package manager.
- **rustc**: The underlying Rust compiler.
- **Rust built-in test framework**: Used for all unit and integration testing.

## Architectural Patterns
- **Clean Architecture**: A layered architecture that ensures separation of concerns between core logic, use cases, adapters, and infrastructure.
- **SOLID / KISS / DRY**: Core design principles applied across the codebase.
- **Anti-God-Class Enforcement**: Actively decomposing large, centralized modules into smaller, composable units to maintain the KISS and SRP principles.

## Execution Environment
- **Native OS Environment**: Designed for native execution across various operating systems with no external runtime dependencies (self-hosted linear type management).
