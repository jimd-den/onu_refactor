# Ọ̀nụ Language Syntax Guide

> **Ọ̀nụ** (pronounced "oh-noo") is a statically-typed, functional systems language with an English-flavoured syntax designed around Clean Architecture principles.

---

## Table of Contents

1. [Program Structure](#1-program-structure)
2. [Types](#2-types)
3. [Behaviors (Functions)](#3-behaviors-functions)
4. [Expressions](#4-expressions)
5. [Control Flow](#5-control-flow)
6. [Variables (Derivations)](#6-variables-derivations)
7. [Arrays and Tuples](#7-arrays-and-tuples)
8. [Matrix Literals](#8-matrix-literals)  ← *New*
9. [SVO Input/Output](#9-svo-inputoutput)  ← *New*
10. [Shapes (Records)](#10-shapes-records)
11. [Standard Library](#11-standard-library)
12. [REPL](#12-repl)  ← *New*
13. [Comments](#13-comments)

---

## 1. Program Structure

Every Ọ̀nụ program begins with a **module declaration** followed by one or more **behaviors** (functions).

```
the-module-called <ModuleName>
    with-concern: <description>

the-behavior-called main
    with-intent: <description>
    takes: nothing
    delivers: an integer
    as:
        0
```

- Module and behavior names use `kebab-case`.
- All keywords use hyphenated multi-word forms (`the-behavior-called`, `with-intent`, etc.).

---

## 2. Types

| Ọ̀nụ Type  | Description                         | LLVM Backing |
|------------|-------------------------------------|--------------|
| `integer`  | 64-bit signed integer               | `i64`        |
| `i8`       | 8-bit signed integer                | `i8`         |
| `i16`      | 16-bit signed integer               | `i16`        |
| `i32`      | 32-bit signed integer               | `i32`        |
| `i64`      | 64-bit signed integer               | `i64`        |
| `i128`     | 128-bit signed integer              | `i128`       |
| `u8`       | 8-bit unsigned integer              | `i8`         |
| `u16`      | 16-bit unsigned integer             | `i16`        |
| `u32`      | 32-bit unsigned integer             | `i32`        |
| `u64`      | 64-bit unsigned integer             | `i64`        |
| `u128`     | 128-bit unsigned integer            | `i128`       |
| `float`    | 32-bit float                        | `f32`        |
| `f64`      | 64-bit float                        | `f64`        |
| `boolean`  | `true` or `false`                   | `i1`         |
| `text`     | UTF-8 string (heap-allocated)       | `i8*`        |
| `nothing`  | Unit type (no value)                | `void`       |

### Type Annotations

Types are introduced with **articles**: `a`, `an`, or `the`.

```
a integer
an i64
a boolean
a text
```

---

## 3. Behaviors (Functions)

### Pure Behavior

```
the-behavior-called add
    with-intent: compute the sum of two integers
    takes:
        an integer called x
        an integer called y
    delivers: an integer
    as:
        x added-to y
```

### Effect Behavior (impure — may perform I/O)

```
the-effect-behavior-called greet
    with-intent: print a greeting
    takes:
        a text called name
    delivers: nothing
    as:
        broadcasts name
```

### Memoized Behavior (with diminishing returns / Fibonacci-style)

```
the-behavior-called fib
    with-intent: compute Fibonacci number
    with-diminishing: n
    takes:
        an integer called n
    delivers: an integer
    as:
        if n exceeds 1
        then (fib (n decreased-by 1)) added-to (fib (n decreased-by 2))
        else n
```

### Non-terminating Behaviors

For behaviors that may not terminate (e.g., infinite loops), add:

```
the-effect-behavior-called loop-forever
    with-intent: run indefinitely
    no-guaranteed-termination
    takes: nothing
    delivers: nothing
    as:
        ...
```

---

## 4. Expressions

### Arithmetic Operators

| Ọ̀nụ Syntax            | Meaning                |
|-------------------------|------------------------|
| `x added-to y`          | `x + y`               |
| `x decreased-by y`      | `x - y`               |
| `x scales-by y`         | `x * y`               |
| `x partitions-by y`     | `x / y`               |

### Comparison Operators

| Ọ̀nụ Syntax            | Meaning                |
|-------------------------|------------------------|
| `x matches y`           | `x == y`              |
| `x exceeds y`           | `x > y`               |
| `x falls-short-of y`    | `x < y`               |

### Function Application

```
-- single argument
fib 10

-- multiple arguments (infix 'utilizes')
x utilizes add y

-- explicit call with parens
(add x y)
```

---

## 5. Control Flow

### If / Then / Else

Ọ̀nụ has a single, always-exhaustive conditional:

```
if <condition>
then <true-expression>
else <false-expression>
```

**Example:**

```
if n matches 0
then 1
else n scales-by (factorial (n decreased-by 1))
```

---

## 6. Variables (Derivations)

Bindings are immutable by default and are introduced with `derivation`:

```
derivation: <name> derives-from <type-annotation> <value-expression>
<body-that-uses-name>
```

**Example:**

```
derivation: result derives-from a integer (fib 20)
broadcasts result
```

Derivations chain naturally:

```
derivation: x derives-from a integer 10
derivation: y derives-from a integer 20
x added-to y
```

---

## 7. Arrays and Tuples

### Arrays

```
-- A homogeneous array of integers
[1, 2, 3, 4, 5]
```

### Tuples

Tuples are expressed via parentheses (engine-level, for multi-value returns):

```
(42, true, "hello")
```

---

## 8. Matrix Literals

> **New in this release.**

Matrices are first-class values with a two-dimensional `[row ; row]` syntax.  Each row is a comma-separated list of numeric literals; rows are separated by `;`.

### Syntax

```
[ elem, elem, ... ; elem, elem, ... ; ... ]
```

Each `;` starts a new row; commas separate elements within a row.

### Examples

```
-- 2×2 identity matrix
[1, 0; 0, 1]

-- 2×3 matrix
[1, 2, 3; 4, 5, 6]

-- 1×4 row vector
[10, 20, 30, 40]
```

### Rules

- Rows must all have the same number of columns (rectangular).
- Elements must be numeric literals (`integer` or `float`).
- Whitespace around `,` and `;` is ignored.

### AST Representation

A matrix literal lowers to `Expression::Matrix { rows: usize, cols: usize, data: Vec<Expression> }` in the AST, where `data` stores elements in **row-major order**.

---

## 9. SVO Input/Output

> **New in this release.**

Ọ̀nụ supports **Subject-Verb-Object** English-flavoured I/O syntax as an alternative to the `broadcasts` keyword.

### Write to Console

```
write <expr> to console
```

This is semantically equivalent to `broadcasts <expr>` and lowers to `Expression::Emit`.

**Examples:**

```
write "Hello, world!" to console
write result to console
write 42 to console
```

### Read from Console

```
read <name> from console
```

Lowers to a call to the built-in `receives-line` behavior, which reads one line from standard input as a `text` value.

**Example:**

```
derivation: input derives-from a text (read line from console)
broadcasts input
```

### SVO vs. Traditional Syntax

| SVO Syntax                   | Traditional Syntax            |
|------------------------------|-------------------------------|
| `write x to console`         | `broadcasts x`                |
| `read line from console`     | `receives-line`               |

Both forms can be freely mixed within a program.

---

## 10. Shapes (Records)

Shapes are nominal product types (like structs):

```
the-shape-called Point
    takes:
        an integer called x
        an integer called y
```

**Construction:**

```
Point 3 4
```

**Field access (automatically generated accessor behaviors):**

```
-- Access `x` field of a Point
x my-point
```

**Shape behaviors (methods):**

```
the-behavior-called translate
    with-intent: move a point by an offset
    takes:
        a Point called p via observation
        an integer called dx
        an integer called dy
    delivers: a Point
    as:
        Point ((x p) added-to dx) ((y p) added-to dy)
```

---

## 11. Standard Library

### Core Math

| Name           | Signature                        | Description                   |
|----------------|----------------------------------|-------------------------------|
| `added-to`     | `integer × integer → integer`    | Addition                      |
| `decreased-by` | `integer × integer → integer`    | Subtraction                   |
| `scales-by`    | `integer × integer → integer`    | Multiplication                |
| `partitions-by`| `integer × integer → integer`    | Integer division              |

### Text / String

| Name          | Signature                        | Description                    |
|---------------|----------------------------------|--------------------------------|
| `len`         | `text → integer`                 | String length in bytes         |
| `char-at`     | `text × integer → text`          | Character at index             |
| `joined-with` | `text × text → text`             | Concatenation                  |
| `as-text`     | `integer → text`                 | Integer to string conversion   |

### I/O

| Name              | Signature              | Description                     |
|-------------------|------------------------|---------------------------------|
| `broadcasts`      | `text → nothing`       | Print to stdout (no newline)    |
| `receives-line`   | `→ text`               | Read a line from stdin          |
| `receives-argument`| `integer → text`      | Read a CLI argument by index    |
| `argument-count`  | `→ integer`            | Number of CLI arguments         |

---

## 12. REPL

> **New in this release.**

Ọ̀nụ ships with an interactive JIT REPL that compiles and executes programs in real time, printing an execution time benchmark.

### Starting the REPL

```bash
onu --repl
```

### REPL Workflow

1. Type (or paste) a complete Ọ̀nụ program at the `onu>` prompt.
2. Press **Enter**.
3. The REPL compiles the program through the full pipeline (lex → parse → HIR → MIR → LLVM IR) and JIT-executes the result using Inkwell's `ExecutionEngine`.
4. The return value of `main` and the wall-clock execution time are printed.

### Example Session

```
onu> the-module-called Hello with-concern: greet
...  the-behavior-called main takes: nothing delivers: an integer as: 42
=> 42
[JIT benchmark: 312µs]

onu> quit
Farewell.
```

### REPL State Machine

The REPL is implemented using the **State Pattern** to cleanly separate idle wait time from active evaluation, preventing REPL infrastructure from polluting the core compiler domain.

```
ReplState::Idle  ──(input received)──►  ReplState::Evaluating  ──(done)──►  ReplState::Idle
```

---

## 13. Comments

Single-line comments begin with `--`:

```
-- This is a comment
the-behavior-called fib  -- inline comment
```

Multi-line comments are not currently supported; use multiple `--` lines.

---

## Quick Reference Card

```
-- Module
the-module-called <Name> with-concern: <text>

-- Pure function
the-behavior-called <name>
    with-intent: <text>
    takes: [nothing | (a <type> called <param>)+]
    delivers: [nothing | a <type>]
    as:
        <expression>

-- Effect function (I/O allowed)
the-effect-behavior-called <name> ...

-- Conditional
if <cond> then <expr> else <expr>

-- Binding
derivation: <name> derives-from a <type> <value>

-- Arithmetic
x added-to y     x decreased-by y
x scales-by y    x partitions-by y

-- Comparison
x matches y      x exceeds y      x falls-short-of y

-- I/O (traditional)
broadcasts <text-expr>

-- I/O (SVO)
write <expr> to console
read  <name> from console

-- Matrix
[r0c0, r0c1; r1c0, r1c1]
```

---

*Generated for Ọ̀nụ language — Clean Architecture Compiler Pipeline.*
