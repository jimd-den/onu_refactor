# Initial Concept
Implementation of the Ọ̀nụ programming language compiler, focusing on safety, memory management (linear types), and performance via pure LLVM.

# Product Definition: Ọ̀nụ Compiler

## Vision
Ọ̀nụ is a high-performance systems programming language that bridges the gap between natural language semantics and hardware-level efficiency. It aims to provide zero-cost safety through linear types and a sophisticated ownership model, allowing developers to write "discourse-driven" code that compiles to pure LLVM IR without the need for a standard library like libc.

## Core Priority Features
- **Standard Library Expansion:** Growing the suite of built-in modules including `Ọ̀nụ-IO`, `StandardMath`, and robust string manipulations.
- **Safety & Ownership Rules:** Implementing and refining "Legal Custody" rules via HIR analysis to ensure memory safety without a garbage collector.
- **Advanced Optimizations:** Developing a robust pipeline of LLVM optimizations (including guaranteed TCO, pure data leaf detection, and fully featured O3 passes), ensuring human-readable "discourse" translates to highly efficient machine code that matches or exceeds Clang -O3 performance.
- **Leak-Free Execution:** Ensuring that all compiled programs pass rigorous memory safety checks (e.g., Valgrind) with zero heap leaks.

## Technical Constraints
- **Pure LLVM Architecture:** The compiler must produce freestanding IR that does not depend on libc or external runtime environments.
- **Strict Clean Architecture:** The codebase must adhere to the four-layer Clean Architecture model (Entities, Use Cases, Adapters, Infrastructure) to ensure long-term maintainability and testability.

## Success Metrics
- **Memory Purity:** Achieving zero heap usage in standard benchmarks, verified via Valgrind.
- **Performance Benchmarking:** Matching or exceeding the performance of equivalent C implementations in core algorithmic tasks (e.g., Fibonacci, Factorial).
- **Comprehensive Documentation:** Maintaining a fully documented compiler pipeline, from Lexer to Realization.
