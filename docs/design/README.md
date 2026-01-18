# Design Documents

Technical design documents describing how Tidebreak systems work.

## Reading Order

For newcomers, read in this order:

### Core Systems (MVP)
1. [architecture.md](architecture.md) — High-level system boundaries and data flow
2. [layers-and-terrain.md](layers-and-terrain.md) — Depth as strategic state (core differentiator)
3. [combat-arena.md](combat-arena.md) — The MVP battle simulator
4. [sensors-and-fog.md](sensors-and-fog.md) — Fog of war and track-based detection
5. [damage-and-boarding.md](damage-and-boarding.md) — Scaled damage and capture mechanics
6. [entity-framework.md](entity-framework.md) — The plugin/resolver architecture

### Strategic Layer
7. [governance.md](governance.md) — Governments, legitimacy, political systems
8. [economy.md](economy.md) — Resources, production, trade routes
9. [factions.md](factions.md) — Faction framework and AI
10. [missions.md](missions.md) — Mission → objective decomposition (strategy ↔ tactics)
11. [people.md](people.md) — Named individuals, roles, leadership

### Environment & Interactions
12. [weather.md](weather.md) — Weather effects on combat and sensors
13. [system-interactions.md](system-interactions.md) — Combat arena cascade chains and loops
14. [strategic-system-interactions.md](strategic-system-interactions.md) — Strategic layer cascades and combat handoffs
15. [sandbox-design.md](sandbox-design.md) — Constraint design and replayability

## Document Index

### Core Systems

| Document | Scope | Status |
|----------|-------|--------|
| [architecture.md](architecture.md) | System boundaries, data contracts, DRL integration | Designed |
| [layers-and-terrain.md](layers-and-terrain.md) | Depth layers, transitions, terrain as state blocker | Designed |
| [combat-arena.md](combat-arena.md) | MVP battle simulator, weapons, step loop | Designed |
| [sensors-and-fog.md](sensors-and-fog.md) | Tracks, sensors, tactical mesh, EW | Designed |
| [damage-and-boarding.md](damage-and-boarding.md) | Tiered damage, boarding phases, morale | Designed |
| [entity-framework.md](entity-framework.md) | ECS variant, plugins, resolvers, contracts | Designed |

### Strategic Layer

| Document | Scope | Status |
|----------|-------|--------|
| [governance.md](governance.md) | Government types, legitimacy, political capital, decisions | Designed |
| [economy.md](economy.md) | Resources, production chains, trade routes, supply | Designed |
| [factions.md](factions.md) | Faction framework, philosophy, disposition, treaties | Designed |
| [missions.md](missions.md) | Mission → tactical objective decomposition and DRL reward hooks | Designed |
| [people.md](people.md) | Named individuals, competence, traits, roles, faces | Designed |

### Environment & Interactions

| Document | Scope | Status |
|----------|-------|--------|
| [weather.md](weather.md) | Weather state, storms, hazards, tactical effects | Designed |
| [system-interactions.md](system-interactions.md) | Combat arena cascade chains, loops, and coupling | Designed |
| [strategic-system-interactions.md](strategic-system-interactions.md) | Strategic layer cascades, loops, and combat handoffs | Designed |
| [sandbox-design.md](sandbox-design.md) | Constraint design, onboarding, replayability | Designed |

## Design Principles

From [vision/pillars.md](../vision/pillars.md):

1. **Depth creates tactical space** — Layers aren't cosmetic
2. **Imperfect information drives decisions** — Fog of war is core
3. **Scale matters** — Jetski and arcology need different mechanics
4. **Nations, not just ships** — Arcologies are societies
5. **Determinism enables learning** — Same seed = same outcome
6. **Systems interlock** — Weather affects sensors affects targeting...

## Writing Design Documents

### When to Write One

Create a design doc when:
- A feature spans multiple systems
- Multiple implementation approaches exist
- Tradeoffs need to be documented
- Future maintainers need context

### Template

```markdown
# [Feature] Design

Brief description of what this covers.

## Goals
- What must this achieve?

## Non-Goals
- What is explicitly out of scope?

## Design
[The actual design]

## Alternatives Considered
[What else was considered and why it was rejected]

## Open Questions
[What remains undecided]
```

### After Writing

1. Update this README index
2. Derive requirements in `/requirements/`
3. Create ADR in `/decisions/` if a major choice was made
