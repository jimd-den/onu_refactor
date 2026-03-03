# Product Guidelines: Ọ̀nụ Project

## Documentation Standards
- **Literate Whitepaper Philosophy:** Treat comments and documentation as a whitepaper for stakeholders. Every file must explain the "why" and the underlying business logic in plain English, ensuring that a non-technical reader can understand the intent.
- **Why over How:** Focus on the architectural rationale and domain rules rather than just describing the code's mechanics.

## Semantic & Naming Principles
- **Semantic Discourse:** Variable naming and high-level logic flow must be semantic and readable by domain experts.
- **No Obscure Abbreviations:** Avoid all cryptic or industry-specific jargon that obscures meaning. Favor descriptive, long-form names that align with the "discourse" concept of the Ọ̀nụ language.

## Architectural Governance
- **Strict Clean Architecture:** All solutions must adhere to the four distinct layers: Entities, Use Cases, Interface Adapters, and Frameworks/Drivers.
- **SOLID / KISS Equilibrium:** Maintain architectural robustness (SOLID) while ensuring that abstractions are simple and necessary (KISS). Never create a layer unless it reduces duplication or protects against volatility.

## Observability & Telemetry
- **Granular Traceability:** Implement detailed logging for all functions, including ISO 8601 timestamps, input arguments, and return values.
- **Transparency:** The compiler's inner workings should be fully observable through traces, providing a clear map of transformation from source to binary.
