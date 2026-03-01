# Initial Concept
Readable pure solid, kiss enforcing language that runs faster than c, reads like a whitepaper

# Product Guide

## Vision
The `onu` language is a readable, pure, SOLID, and KISS-enforcing language. It aims to provide a syntax that reads like a whitepaper while achieving performance that surpasses C. It focuses on clarity, simplicity, and high-level expressiveness without sacrificing the raw performance of a system-level language.

## Target Audience
- **Systems Engineers**: Who require low-level control and performance.
- **High-level Developers**: Who seek powerful abstractions and readable code.
- **Domain Experts**: Who need a clear and expressive language to describe complex logic.

## Core Features
- **Concurrency & Parallelism**: Built-in support for safe and efficient multi-threaded operations.
- **Memory Safety & Correctness**: A strong focus on ensuring code integrity. This is now achieved through a native linear types policy that generates explicit, optimized `malloc`/`free` calls at compile-time, eliminating common runtime errors.
- **SOLID & KISS Principles**: The language is designed to enforce clean architecture and simplicity.

## Differentiation
- **Syntax Innovation**: `onu` distinguishes itself through a unique, whitepaper-like syntax that prioritizes readability and semantic clarity.
- **Runtime-Free Performance**: By implementing memory management natively in the compiler IR, `onu` achieves a true runtime-free execution model, potentially surpassing C's performance through superior static analysis and optimization.
