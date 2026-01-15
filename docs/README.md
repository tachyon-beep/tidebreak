# Tidebreak Documentation

Professional documentation for Tidebreak, a naval strategy game with depth-layer combat and mobile city-ships.

## Documentation Structure

```
docs/
├── vision/           What and why
│   ├── pitch.md      Product vision and core experience
│   ├── pillars.md    Design principles guiding decisions
│   └── glossary.md   Canonical terminology
│
├── design/           How it works (intent and approach)
│   ├── README.md     Reading order and index
│   ├── architecture.md
│   ├── layers-and-terrain.md
│   ├── combat-arena.md
│   ├── sensors-and-fog.md
│   ├── damage-and-boarding.md
│   └── entity-framework.md
│
├── technical/        Formal specifications (contracts and invariants)
│   ├── README.md     How to read specs
│   ├── architecture.md   Entity-Plugin-Resolver framework spec
│   └── contracts.md      State component schemas
│
├── requirements/     What it must do
│   ├── README.md     How to read requirements
│   ├── combat.md
│   ├── layers.md
│   ├── sensors.md
│   ├── ships.md
│   ├── world.md
│   ├── drl.md
│   ├── entity-framework.md
│   └── cross-cutting.md
│
├── decisions/        Why we chose this (ADRs - future)
│
├── research/         Ideas and explorations (future)
│
└── development/      How to contribute
    └── setup.md      Development environment
```

## Quick Links

**Start here:**
- [vision/pitch.md](vision/pitch.md) — What is Tidebreak?
- [vision/pillars.md](vision/pillars.md) — What makes it unique?
- [design/README.md](design/README.md) — Technical designs

**For developers:**
- [development/setup.md](development/setup.md) — Environment setup
- [requirements/README.md](requirements/README.md) — What to build
- [technical/architecture.md](technical/architecture.md) — Entity-Plugin-Resolver spec

## Document Types

| Type | Purpose | Example |
|------|---------|---------|
| **Vision** | Product direction, principles | "Depth creates tactical space" |
| **Design** | Technical approach and intent | "Layers are discrete states, not continuous depth" |
| **Technical** | Formal specs and contracts | "INV-P1: Plugins MUST NOT mutate any state" |
| **Requirements** | Verifiable capabilities | "Support layer transitions taking 30-60+ seconds" |
| **Decision** | Recorded choices (ADR) | "We chose discrete layers because..." |
| **Development** | Contributor guides | "Run `uv run pytest` to test" |

## Conventions

- **Terminology**: Use terms from [glossary.md](vision/glossary.md)
- **Requirements**: Use "Support [capability]" format with priority (P0–P3)
- **Links**: Use relative paths within docs/
- **Code examples**: Only in design docs, never in requirements
