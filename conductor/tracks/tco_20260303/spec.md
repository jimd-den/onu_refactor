# Specification: Tail Call Optimization (TCO)

## Overview
Implement guaranteed Tail Call Optimization (TCO) in the Ọ̀nụ compiler using the LLVM `musttail` attribute. This track focuses on enabling deep recursion without stack overflow for both self-recursive and mutually recursive functions, while strictly adhering to Ọ̀nụ's linear type safety rules.

## Functional Requirements
- **TCO Identification:** The compiler must identify tail calls within the Ọ̀nụ MIR (Middle Intermediate Representation).
- **LLVM musttail Integration:** Tail calls must be marked with the `musttail` attribute in the generated LLVM IR to guarantee optimization by the LLVM backend.
- **Recursion Support:** Support both self-recursion (functions calling themselves) and mutual recursion (cycles of function calls).
- **Argument Handling:** Support tail calls where arguments are re-ordered, resized, or passed with complex calling conventions.
- **Linear Type Safety:** Ensure that all linear types (resources) are explicitly dropped *before* the tail call is executed to prevent memory leaks and ensure ownership rules are met.
- **Cross-Module Support:** Ensure TCO works for tail calls across different Ọ̀nụ modules.

## Non-Functional Requirements
- **Memory Efficiency:** Tail-recursive functions must execute in O(1) stack space.
- **Zero-Leak Guarantee:** All TCO-enabled programs must pass Valgrind with zero heap leaks, verified via TDD.
- **Clean Architecture:** Implementation must reside within the `Lowering Service (Adapters)` layer to separate TCO identification from IR generation logic where possible.

## Acceptance Criteria
- [ ] A test suite with deep recursion (e.g., 1,000,000+ calls) passes successfully without stack overflow.
- [ ] Generated LLVM IR shows the `musttail` attribute on all identified tail calls.
- [ ] Valgrind confirms zero memory leaks for tail-recursive programs.
- [ ] TCO is verified for both self-recursion and mutual recursion across modules.
- [ ] Linear type "Legal Custody" rules are maintained (resources dropped before `musttail`).

## Out of Scope
- Implementing TCO for targets other than LLVM.
- Non-tail call optimizations (e.g., inlining) that are not directly related to TCO.
