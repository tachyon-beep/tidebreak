# Technical Specifications

Implementation-grade specifications for Tidebreak systems. These documents define formal contracts, invariants, and interfaces that code must satisfy.

## Documents

| Document | Purpose |
|----------|---------|
| [architecture.md](architecture.md) | Entity-Plugin-Resolver framework specification |
| [contracts.md](contracts.md) | State component schemas and mutation rules |

## Relationship to Design Docs

**Design docs** (`design/`) describe *intent and approach*—why we chose this architecture, what tradeoffs we accepted.

**Technical specs** (`technical/`) define *formal contracts*—what the implementation must satisfy, what invariants must hold, what interfaces look like.

When design and spec conflict, the spec is authoritative for implementation; the design doc should be updated to reflect decisions.

## Reading These Documents

- **Invariants**: Conditions that must always hold. Violations are bugs.
- **Contracts**: Interface definitions with pre/post conditions.
- **Schemas**: Data structure definitions with field-level constraints.
- **Rules**: Deterministic procedures that must be followed exactly.
