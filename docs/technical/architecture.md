# Entity Framework Specification

This document defines the formal contracts for Tidebreak's Entity-Plugin-Resolver architecture. All implementations must satisfy these invariants and interfaces.

## Overview

Tidebreak uses a **Reactive ECS variant** where:
- **Entities** are containers with identity, state, and plugins
- **Plugins** read state and emit proposals (never mutate directly)
- **Resolvers** adjudicate proposals and apply state changes deterministically

This architecture serves three requirements: determinism (for DRL training and replays), modularity (systems interact through contracts, not coupling), and traceability (every outcome has an attributable cause).

---

## Entity Specification

### Definition

An **Entity** is a uniquely identified container comprising:

```
Entity {
    id:         EntityId        # Globally unique, immutable after creation
    tags:       Set<Tag>        # Classification (Ship, Faction, Arcology, Platform)
    owner:      EntityId?       # Owning entity (null for top-level)
    components: Map<ComponentId, Component>
    plugins:    List<PluginId>  # Ordered for determinism
    children:   List<EntityId>  # Nested entities (compartments, squadrons, factions)
}
```

### Invariants

**INV-E1**: Entity IDs are unique across the entire simulation lifetime.

**INV-E2**: An entity's `tags` set is immutable after creation.

**INV-E3**: `plugins` list ordering is stable and deterministic (sorted by plugin_id).

**INV-E4**: Child entities inherit owner from parent unless explicitly overridden.

### Entity Types

| Tag | Semantics | Typical Components |
|-----|-----------|-------------------|
| `Ship` | Mobile combat unit | Combat, Sensor, Movement, Inventory |
| `Platform` | Fixed installation | Sensor, Defense, Production |
| `Arcology` | Mobile nation | Governance, Civics, Population, Factions, Economy |
| `Faction` | Political entity | Diplomacy, Relations, Resources |
| `Enclave` | Population group | Civics, InternalFactions, Population |

---

## Plugin Specification

### Definition

A **Plugin** is a capability module that:
1. Reads from an immutable `WorldView`
2. Emits typed `Output` proposals
3. Stores persistent memory only via serialized `PrivateComponent`

```
Plugin {
    id:              PluginId
    required_tags:   Set<Tag>           # Entity must have these to attach
    reads:           Set<ComponentId>   # Components this plugin observes
    emits:           Set<OutputType>    # Output types this plugin may produce
    phases:          Set<Phase>         # Execution phases this plugin runs in
}
```

### Plugin Interface

```python
class Plugin(Protocol):
    """Contract all plugins must satisfy."""

    def tick(self, view: WorldView, entity: EntityId) -> List[Output]:
        """
        Generate outputs based on current world state.

        Pre:  view is immutable snapshot of world state
        Pre:  entity is valid and has all required_tags
        Post: returns list of typed Outputs
        Post: no side effects (pure function of inputs)
        """
        ...

    @property
    def declaration(self) -> PluginDeclaration:
        """Static metadata about this plugin's dependencies and outputs."""
        ...
```

### Invariants

**INV-P1**: Plugins MUST NOT mutate any state. They only emit outputs.

**INV-P2**: Plugins MUST NOT access state outside their declared `reads` set.

**INV-P3**: Plugins MUST NOT emit output types outside their declared `emits` set.

**INV-P4**: Plugin execution order within a phase is deterministic (sorted by plugin_id, then entity_id).

**INV-P5**: Plugins with persistent memory MUST store it in `PrivateComponent<PluginType>` attached to the entity. Local variables do not survive serialization.

### Private Components

When plugins require memory between ticks:

```
PrivateComponent<T> {
    plugin_type: TypeId     # Which plugin owns this
    data:        T          # Plugin-specific serializable state
}
```

**Rule**: Only the owning plugin may write to its private component. The component is serialized with entity state, ensuring replay determinism.

---

## Shared Contract Specification

### Purpose

The **Shared Contract** is the anti-chaos mechanism. It defines:
- All state component schemas
- All event schemas
- All output schemas and legal targets
- Resolution rules and conflict handling
- Determinism constraints
- Serialization requirements

**Nothing important exists outside the contract.**

### Contract Structure

```
SharedContract {
    components:     Map<ComponentId, ComponentSchema>
    events:         Map<EventType, EventSchema>
    outputs:        Map<OutputType, OutputSchema>
    resolutions:    Map<OutputType, ResolutionRule>
    determinism:    DeterminismConstraints
    serialization:  SerializationRules
}
```

### Invariants

**INV-C1**: All state accessed by plugins MUST be defined in the contract.

**INV-C2**: All outputs emitted MUST conform to contract schemas.

**INV-C3**: Contract changes require version increment and migration path.

**INV-C4**: Resolution rules MUST be pure functions of inputs (no hidden state).

---

## Output Specification

### Definition

An **Output** is a typed proposal from a plugin. Outputs do not take effect immediately—they are collected and resolved.

```
Output {
    output_id:  OutputId        # Unique within tick (for deterministic ordering)
    type:       OutputType      # Discriminator for resolution routing
    source_id:  EntityId        # Entity that emitted this
    plugin_id:  PluginId        # Plugin that emitted this
    cause_id:   EventId?        # Upstream event that triggered this (causal chain)
    trace_id:   TraceId         # Root identifier for cause chain
    payload:    OutputPayload   # Type-specific data
    tick:       TickNumber      # When emitted (for ordering)
}
```

### Output Categories

#### Commands (Attempt Actions)

```
# Movement
SetThrottle     { throttle: Ratio }                     # 0.0–1.0 engine power
SetHeading      { target_heading: Heading }             # Desired heading
SetCourse       { throttle: Ratio, heading: Heading }   # Combined command
HelmOrder       { throttle: Ratio, turn_rate: f64 }     # Direct control (DRL)

# Combat
FireWeapon      { target: EntityId, slot: WeaponSlot, mode: FireMode }
LayerTransition { target_layer: Layer }
LaunchCraft     { squadron: EntityId, mission: MissionType }

# Strategic
QueueDecision   { decision_type: DecisionType, params: DecisionParams }
AllocateSupplies{ route: RouteId, resource: ResourceType, amount: Quantity }
```

#### Modifiers (State Changes)

```
ApplyModifier {
    target:     EntityId
    component:  ComponentId
    field:      FieldPath
    delta:      NumericDelta | EnumTransition
    duration:   Duration?       # None = permanent
}
```

#### Events (Facts for Reaction/Telemetry)

```
CrisisTriggered     { crisis_type: CrisisType, severity: Severity }
InfluenceDetected   { actor: EntityId, vector: InfluenceVector, confidence: Float }
TransitionComplete  { entity: EntityId, from_layer: Layer, to_layer: Layer }
DamageApplied       { target: EntityId, amount: Float, source: EntityId }
```

#### Intents (Planning-Level Goals)

```
Intent {
    kind:       IntentKind      # DefendPosition, AttackTarget, Escort, etc.
    target:     EntityId?
    location:   Position?
    priority:   Priority
}
```

#### Reservations (Resource Contention)

```
Reserve {
    resource:   ResourceType    # bandwidth, power, ammunition
    amount:     Quantity
    duration:   Duration
}
```

### Causal Chain Requirements

**REQ-CC1**: Every output MUST have a valid `source_id` and `plugin_id`.

**REQ-CC2**: Every output MUST have a `trace_id`. Outputs triggered by events inherit the event's trace_id.

**REQ-CC3**: Outputs triggered by other events SHOULD set `cause_id` to enable chain reconstruction.

---

## Resolver Specification

### Definition

A **Resolver** is a deterministic system that:
1. Collects outputs of specific types
2. Resolves conflicts using stable rules
3. Writes results to `NextState` (never `CurrentState`)

```
Resolver {
    id:             ResolverId
    handles:        Set<OutputType>     # Which outputs this resolver processes
    priority:       Int                 # Execution order (lower = earlier)
}
```

### Resolver Interface

```python
class Resolver(Protocol):
    """Contract all resolvers must satisfy."""

    def resolve(
        self,
        outputs: List[Output],
        current: WorldView,
        next_state: MutableState,
        rng: SeededRNG
    ) -> List[Event]:
        """
        Process collected outputs and apply results.

        Pre:  outputs contains only types in self.handles
        Pre:  outputs are sorted deterministically (tick, source_id, output_id)
        Pre:  current is immutable
        Post: mutations applied only to next_state
        Post: returned events have valid causal chain metadata
        Post: deterministic given same inputs and RNG state
        """
        ...
```

### Standard Resolvers

| Resolver | Handles | Purpose |
|----------|---------|---------|
| `EffectResolver` | `ApplyModifier` | Generic stat changes |
| `PhysicsResolver` | Movement outputs | Position, velocity, collisions |
| `CombatResolver` | `FireWeapon`, `DamageApplied` | Weapon fire and damage propagation |
| `SensorResolver` | Detection outputs | Track fusion and quality updates |
| `GovernanceResolver` | `QueueDecision`, transitions | Decision procedures and government changes |
| `ReservationResolver` | `Reserve` | Resource contention and allocation |

### Conflict Resolution Rules

**RULE-CR1**: When multiple outputs target the same field, apply in deterministic order (tick, then source_id, then output_id).

**RULE-CR2**: Additive modifiers stack. Multiplicative modifiers compound.

**RULE-CR3**: Conflicting reservations are resolved by priority, then by first-emitted.

**RULE-CR4**: All tie-breakers use entity_id comparison (lower wins).

---

## Execution Model

### Phase-Based Loop

```
FRAME N:

1. SNAPSHOT PHASE (Serial)
   - Freeze CurrentState into immutable WorldView
   - Establish deterministic iteration order
   - Record frame RNG seed

2. PLUGIN PHASE (Parallelizable)
   - For each entity (sorted by entity_id):
     - For each plugin (sorted by plugin_id):
       - plugin.tick(world_view, entity_id) → outputs
   - Collect all outputs into OutputBuffer

3. RESOLUTION PHASE (Serial, Deterministic)
   - Sort outputs by (type, tick, source_id, output_id)
   - For each resolver (sorted by priority):
     - Filter outputs matching resolver.handles
     - resolver.resolve(outputs, world_view, next_state, rng) → events
   - Collect all events

4. APPLY PHASE (Serial)
   - Swap NextState → CurrentState
   - Emit telemetry events with causal metadata
   - Advance tick counter
   - Advance RNG stream
```

### Determinism Invariants

**INV-D1**: Given identical `seed` and `inputs`, the simulation produces identical `outputs` on the same platform/build.

**INV-D2**: All iteration orders are defined by sorted ID comparisons, never by hash order or insertion order.

**INV-D3**: All randomness uses explicit RNG streams from the seeded generator.

**INV-D4**: Floating-point operations use consistent precision (or fixed-point for cross-platform determinism).

**INV-D5**: Plugin phase parallelization must not introduce race conditions—outputs are collected, not applied.

---

## WorldView Specification

### Definition

A **WorldView** is an immutable snapshot of simulation state provided to plugins.

```
WorldView {
    tick:       TickNumber
    entities:   ImmutableMap<EntityId, EntitySnapshot>
    globals:    ImmutableMap<GlobalId, GlobalState>
    rng_seed:   Seed            # For this frame (plugins should not use directly)
}
```

### Access Patterns

```python
# Entity access
view.entity(id) -> EntitySnapshot?
view.entities_with_tag(tag) -> Iterator[EntitySnapshot]
view.entities_in_radius(pos, radius) -> Iterator[EntitySnapshot]

# Component access
view.component(entity_id, component_id) -> Component?

# Relationship traversal
view.children(entity_id) -> Iterator[EntityId]
view.owner(entity_id) -> EntityId?
```

### Invariants

**INV-W1**: WorldView is immutable. Any attempt to mutate raises an error.

**INV-W2**: WorldView reflects state at start of frame, not partial updates.

**INV-W3**: Plugins cannot access entities/components outside their declared `reads`.

---

## DRL Mapping

The architecture maps directly to reinforcement learning:

| Framework Concept | DRL Equivalent | Notes |
|-------------------|----------------|-------|
| `WorldView` | Observation | Immutable state snapshot |
| `Output` list | Action | Plugin proposals |
| Resolver step | Transition function | Deterministic state update |
| Reward emitter | Reward signal | Specialized resolver or post-step evaluator |
| `PrivateComponent` | Agent memory | Serialized for episode continuity |

**A DRL policy is just a plugin that emits outputs.**

```python
class DRLPolicy(Plugin):
    """Neural network policy as a plugin."""

    def tick(self, view: WorldView, entity: EntityId) -> List[Output]:
        obs = self.extract_observation(view, entity)
        action = self.network.forward(obs)
        return self.decode_action(action, entity)
```

---

## Required Tooling

This architecture requires supporting infrastructure:

| Tool | Purpose | Priority |
|------|---------|----------|
| Contract Schema Validator | Validate component, event, output schemas | P0 |
| Plugin Dependency Validator | Check required tags/components at attach time | P0 |
| Replay Inspector | Step through tick → outputs → resolution → state delta | P0 |
| Causal Graph Viewer | Visualize trace_id chains | P1 |
| Determinism Diff Tool | Compare two runs, pinpoint divergence tick | P1 |
| State Serializer | Save/load full simulation state | P0 |

**Without these tools, "clever architecture" becomes "mysterious architecture."**

---

## Implementation Milestones

### Milestone 1: Skeleton

- [ ] Entity container with component storage
- [ ] Shared Contract schema definitions
- [ ] Plugin interface and registration
- [ ] Single resolver + deterministic loop
- [ ] Seeded RNG integration
- [ ] Basic telemetry logging
- [ ] Replay from seed + inputs

### Milestone 2: First Slice

- [ ] 2-3 ship plugins (movement, weapon, sensor)
- [ ] 2-3 governance bundles (autocracy, corporate, democracy)
- [ ] Generic EffectResolver
- [ ] Causal chain plumbing
- [ ] Plugin dependency validation

### Milestone 3: Expansion

- [ ] Influence operations + internal factions
- [ ] Mission intents + task offers
- [ ] Council votes + enforcement
- [ ] Fidelity scaling (detail near player focus)
