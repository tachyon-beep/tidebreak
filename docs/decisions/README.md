# Decisions (ADRs)

This folder contains **Architecture Decision Records (ADRs)** for Tidebreak: short documents that capture a significant decision, its context, and its consequences.

## When to write an ADR

- A design choice has meaningful tradeoffs and should be recorded.
- A decision affects multiple systems, schemas, or workflows.
- A choice is hard to reverse later (architecture, contracts, determinism approach, etc.).

## Format (template)

```markdown
# ADR 000X: Title

## Status
Proposed | Accepted | Superseded | Deprecated

## Context
What problem are we solving? What constraints matter?

## Decision
What are we doing?

## Consequences
What does this enable? What does it cost? What follow-ups are required?

## Alternatives Considered
What else did we consider, and why not?
```

## Naming & workflow

- Name files like `adr-0001-short-title.md` (monotonic ID).
- If a decision changes, add a new ADR and mark the old one as **Superseded** (link both ways).
- Link ADRs from the relevant `docs/design/` and/or `docs/technical/` documents.
