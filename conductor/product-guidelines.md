# Product Guidelines

## Prose Style
- **Formal/Academic**: Documentation and comments should be written in a precise, academic, and detailed manner, similar to a formal specification or a scientific whitepaper. The goal is to provide a clear and unambiguous understanding of the language's mechanics and design rationale.

## Architectural Principles
- **Strict Clean Architecture**: The project rigorously adheres to Clean Architecture principles. This means maintaining clear boundaries between Entities, Use Cases, Interface Adapters, and Frameworks/Drivers.
- **Single Responsibility Principle (SRP)**: Avoid the "God Class" code smell. No single module or service should have multiple responsibilities or grow beyond a manageable complexity. Large transformations must be decomposed into smaller, specialized services.
- **Linear Type Safety**: All transformations across IR stages must maintain strict ownership and linear type invariants to ensure the native memory management remains correct.

## Coding Principles
- **SOLID, KISS, and DRY**: These are non-negotiable standards for all code. We prioritize simplicity and maintainability without compromising on architectural robustness.
- **Functional Purity**: We strive for small, composable, and deterministic pure functions. Side effects should be isolated and minimized.
- **Performance-First**: While maintaining readability, we prioritize O(1) or O(n) complexity and a minimal memory footprint. Performance is a core feature of the `onu` language.

## Development Workflow
- **Test-First (TDD)**: We follow a strict Test-Driven Development flow. A feature or bug fix is incomplete without a corresponding test case that reproduces the failure state before the implementation is applied.
