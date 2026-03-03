# Technology Stack: Ọ̀nụ Compiler

## Core Language & Runtime
- **Rust (2024 Edition):** The primary implementation language, selected for its safety guarantees, performance, and excellent tooling.
- **Pure LLVM IR:** The target output of the compiler, designed for freestanding execution without dependence on external standard libraries (libc).

## Frameworks & Libraries
- **Inkwell (LLVM 14.0):** A safe wrapper around the LLVM C API, used for intermediate representation generation and machine code emission.
- **Chrono:** Leveraged for high-precision ISO 8601 timestamps in observability logs.
- **Either:** Used for expressive error handling and branching logic in the compiler's transformation stages.

## Infrastructure & Tooling
- **Cargo:** The standard Rust build system and package manager.
- **Valgrind:** Used for exhaustive memory safety validation of both the compiler and its generated binaries.
- **Clang:** Utilized as the linker and driver for realizing the final discourse binaries from LLVM IR.
