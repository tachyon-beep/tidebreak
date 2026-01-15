# Requirements

This folder contains functional requirements derived from design documents. Requirements describe **what** the system must do, not **how** to implement it.

## How to Read Requirements

### Format

Requirements use the pattern: **"Support [capability] with [constraints]"**

Example:
> Support layer transitions that take 30–60+ seconds and leave units vulnerable to both origin and destination layers during transit.

This format:
- States capability clearly
- Includes measurable constraints where applicable
- Avoids implementation details

### Priority Levels

| Priority | Meaning | When to Implement |
|----------|---------|-------------------|
| **P0** | MVP-critical | Must have for first playable |
| **P1** | Core experience | Required for complete game loop |
| **P2** | Enhanced experience | Adds depth after core works |
| **P3** | Nice to have | If time/resources permit |

Priority is assigned based on:
1. Dependency (does other work depend on this?)
2. Design pillar alignment (does this reinforce core identity?)
3. Implementation risk (should we prove this early?)

### Status Markers

| Marker | Meaning |
|--------|---------|
| (none) | Not started |
| `[DESIGNED]` | Design doc exists in `/design/` |
| `[PROTOTYPE]` | Rough implementation exists |
| `[IMPLEMENTED]` | Production-quality implementation |
| `[TESTED]` | Has automated tests |

## Requirement Documents

| Document | Covers |
|----------|--------|
| [combat.md](combat.md) | Combat Arena, weapons, damage, boarding |
| [layers.md](layers.md) | Depth layers, transitions, terrain |
| [sensors.md](sensors.md) | Detection, tracks, fog of war, EW |
| [ships.md](ships.md) | Fleet hierarchy, capabilities, arcologies |
| [world.md](world.md) | Economy, factions, governance, weather |
| [people.md](people.md) | Named individuals, roles, leadership |
| [drl.md](drl.md) | AI training, curriculum, observations |
| [entity-framework.md](entity-framework.md) | Entities, plugins, resolvers, contracts |
| [cross-cutting.md](cross-cutting.md) | Determinism, replay, performance |

## Deriving Requirements

Requirements should be derived from design documents:

1. Read the design doc in `/design/`
2. Extract "must do" statements
3. Add measurable constraints where the design specifies them
4. Assign priority based on MVP scope
5. Link back to design doc for context

**Do not** invent requirements without a corresponding design. If a requirement seems necessary but has no design, create the design first.

## Changing Requirements

Requirements change when:
- Design changes (update requirement to match)
- Implementation reveals impossibility (update design, then requirement)
- Priority shifts (update priority marker)

Always update the design document first, then propagate to requirements.

## Open Questions

Requirements that need clarification before implementation should link to the design doc's open questions section or be marked with `[TBD]`.

Example:
> Support boarding actions that can be interrupted by [TBD: movement, reinforcements, or disengagement—see design/damage-and-boarding.md].
