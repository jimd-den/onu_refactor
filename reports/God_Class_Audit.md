# AUDIT REPORT: God Classes and Architectural Violations

## 1. The Essential Experience (Goal)
The Onu architecture must protect the **"Compilation Narrative."** Each stage should be a discrete, replaceable "Black Box." Current violations create "Tangled Narratives" where backend machine details force rewrites of frontend logic.

---

## 2. Identified God Classes (Policy Bloat)

### A. The Orchestration God: `CompilationPipeline` (`src/lib.rs`)
*   **Violations:** **SRP**, **DIP**.
*   **The Responsibility:** It acts as both the **Composition Root** and the **Execution Engine**. It knows about specific Infrastructure modules (`OnuIoModule`) and every compilation stage.
*   **Impact:** Mixing "How to build" with "How to run" makes the pipeline rigid and hard to test.

### B. The Grammar God: `OnuParser` (`src/adapters/parser/mod.rs`)
*   **Violations:** **SRP**, **OCP**.
*   **The Responsibility:** Manages token streams, line-counting, grammar rules for all constructs, and direct symbol registration.
*   **Impact:** A change in how "Peanuts" are parsed shouldn't require touching the logic for "Shapes."

### C. The Omniscient Service: `RegistryService` (`src/application/use_cases/registry_service.rs`)
*   **Violations:** **DIP**, **Leaky Boundaries**.
*   **The Responsibility:** Stores symbols AND calculates target-specific memory layouts (size/alignment).
*   **Impact:** High-level application policy is "infected" with low-level LLVM layout details.

### D. The Strategy Monolith: `src/adapters/codegen/strategies.rs`
*   **Violations:** **OCP**, **DRY**.
*   **The Responsibility:** Houses 1100+ lines of instruction translation logic in a single file.
*   **Impact:** Fragile boundary prone to merge conflicts and "Shotgun Surgery" for every language expansion.

---

### 3. Core Architectural Failures

| Bug | Violation Type | Architectural Explanation |
| :--- | :--- | :--- |
| **Intent Greediness** | **SRP** | Documentation (Prose) is not isolated from Grammar (Logic). Parser enters a "sink" mode that swallows keywords. |
| **Article Collision** | **KISS / DRY** | Greedy article check lacks a Lookahead Guard. It assumes every 'a' is a type indicator, stealing the programmer's namespace. |
| **Operator Identity** | **OCP** | Parser identity mapping is incomplete. New operators require manual surgery in the central `match` arms. |

---

## 4. Refactoring Blueprint (The "Clear Window" Initiative)

### Phase 1: Lexical Purity
*   Simplify `OnuLexer` by moving complex phrase matching to a `TokenStreamFilter`.
*   Ensure Lexer is "Meaning-Blind" and only produces physical tokens.

### Phase 2: Parser Decomposition
*   Extract `ParserInternal` into isolated Strategy files: `header_parser.rs`, `type_parser.rs`, and `expression_parser.rs`.
*   Introduce `SymbolQueryPort` to decouple the Parser from the `RegistryService`.

### Phase 3: Implementation of Bug Fixes
*   **Stop-Word Strategy:** Stop unquoted intent consumption if a structural keyword is peeked.
*   **Lookahead Guard:** Only consume `a/an/the` if followed by a recognized type name.
*   **Identity Mapping:** Exhaustively map all operator tokens to behavior strings.

---

**Boss's Verdict:** The "Brain" is a genius, but the "Sensory Boundary" is clogged with technical debt. Dismantle these God Classes to restore the literate promise of the language.

**Status:** AUDIT LOGGED. **Next Step:** Execute Implementation Phase.
