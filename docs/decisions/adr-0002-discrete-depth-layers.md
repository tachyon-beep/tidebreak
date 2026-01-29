# ADR 0002: Discrete Depth Layers

## Status
Accepted

## Context

Naval games handle depth in various ways:

1. **Flat ocean**: No depth at all (most naval RTS games).
2. **Continuous depth**: Submarines at arbitrary depths with realistic physics.
3. **Discrete layers**: A small number of strategic states with distinct properties.

Tidebreak's design pillars emphasize:
- "Depth Creates Tactical Space" — depth should fundamentally change combat, not just be a number.
- Determinism and DRL-trainable — observation/action spaces should be tractable.
- Meaningful commitment — changing depth should be a decision, not a twitch action.

## Decision

Use **three discrete depth layers** as strategic states:

| Layer | Name | Characteristics |
|-------|------|-----------------|
| **Surface** | "The Arena" | Full sensor suite (radar, visual, sonar). All weapon types. Vulnerable to everything. Affected by weather. |
| **Submerged** | "The Stealth Layer" | Sonar only. Torpedoes and mines. Immune to ballistic/energy weapons. Invisible to radar. |
| **Abyssal** | "The Flank" | Strategic transit and bypass. Minimal combat capability. Requires specialized hulls. Post-MVP. |

### Transition Mechanics

- Transitions take **30-60+ seconds** (hull-dependent).
- During transition, ships are **maximally vulnerable**: detectable by both layers, cannot fire weapons.
- Transitions generate a **signature spike** (sonar bloom).
- Transitions can be **interrupted by damage**.
- Transitions are a **commitment**, not a dodge.

Layer state is tracked via `LayerState` component with `current`, `target`, `transitioning`, and timing fields.

## Consequences

### Enables

- **Clear tactical states**: Each layer has a distinct sensor/weapon/hazard profile. No ambiguity about "how submerged."
- **Meaningful decisions**: Committing to a dive is risky. Timing matters.
- **Tractable DRL**: Layer is a small discrete observation. Transition is a discrete action.
- **Fleet composition**: Ships specialized for different layers create interesting fleet design.
- **Layer control as objective**: "Control the submerged layer" is a clear tactical goal.

### Costs

- **Less realism**: Real submarines operate at continuous depths with thermocline effects.
- **Reduced granularity**: No "periscope depth" vs "deep running" vs "crush depth" subtlety (in MVP).
- **Potential expansion complexity**: Adding thermal layers within Submerged later may require rework.

### Trade-off Accepted

The loss of continuous depth realism is acceptable because:
1. Discrete states create cleaner decision points for players and AI.
2. Three layers already generate significant tactical complexity (see system-interactions.md).
3. Thermocline mechanics can be added as modifiers *within* Submerged layer post-MVP without changing the layer model.

### Follow-ups Required

- Define per-layer sensor modifiers in `LayerConfig`.
- Define transition durations per hull class.
- Implement signature spike during transition.
- Document Abyssal layer requirements for post-MVP.

## Alternatives Considered

### Continuous Depth

Submarines at arbitrary meter depths with realistic sonar propagation, thermoclines, and crush depth.

**Rejected because**:
- Observation space explodes (depth as continuous float, thermal layers, etc.).
- Most depth values are tactically equivalent — only a few "bands" matter.
- Thermocline hunting is interesting but adds complexity before core systems are proven.
- Can be approximated via modifiers within Submerged layer later.

### Two Layers (Surface/Submerged only)

Simpler model without Abyssal.

**Rejected because**:
- Abyssal enables strategic bypass and flanking maneuvers.
- Three layers create rock-paper-scissors interactions.
- Abyssal is deferred to post-MVP anyway, so the cost is low.

### Five+ Layers

Surface, Shallow, Deep, Abyssal, Trench, etc.

**Rejected because**:
- Diminishing returns on tactical differentiation.
- Increases cognitive load and DRL observation space.
- Three layers is the minimum to create layer-control dynamics; more can be added if needed.
