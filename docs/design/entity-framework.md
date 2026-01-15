# Entity Framework Design

Tidebreak uses a reactive ECS (Entity-Component-System) variant where plugins propose actions and resolvers adjudicate outcomes deterministically.

## Canonical Types Used

- **Entity**: Container with identity, state components, and plugins
- **Plugin**: Capability module (reads WorldView, emits Outputs)
- **Resolver**: Adjudicator that applies outputs deterministically
- **SharedContract**: Schema definitions for all state and events
- **WorldView**: Immutable state snapshot (DRL: Observation)
- **Output**: Typed proposal (DRL: Action)
- **CausalChain**: source_id, cause_id, trace_id for traceability

## Why This Architecture

### Unified Entity Model

Ships, governments, factions, enclaves, and platforms are all **entities** with the same fundamental structure. This enables clean cross-system interactions.

**Example**: A mutiny (crew/boarding system) lowers legitimacy (governance system).
- Bad: Mutiny code calls government code directly (tight coupling)
- Good: MutinyPlugin emits an effect; governance resolver applies it

### Native DRL Fit

The architecture maps directly to reinforcement learning:

| Framework Concept | DRL Equivalent |
|-------------------|----------------|
| WorldView (immutable snapshot) | Observation |
| Outputs (proposals) | Action |
| Resolver step | Transition function |
| Reward emitter | Reward |

A DRL policy is just a plugin that emits outputs.

## Core Concepts

### Entities

An entity is a container with:
- **Identity**: ID, tags (ship/enclave/faction/platform), ownership
- **State Components**: Defined by the Shared Contract
- **Plugins**: Capability modules (configured in data)
- **Children**: Optional nested entities (squadrons, compartments, factions)

Entities are behavior-light. Behavior lives in plugins and resolvers.

### Plugins

A plugin is a capability module that:
- **Reads** from a bounded WorldView (read-only)
- **Emits** typed Outputs (proposals)
- **Stores memory** only via serialized Private Components

Plugins must declare:
- Required tags/components
- Which components they read
- Which output types they emit
- Which phases they run in

### Shared Contract

The anti-chaos mechanism. Defines:
- State component schemas and invariants
- Event schemas
- Output schemas and allowed targets
- Resolution rules (including conflict handling)
- Determinism constraints (ordering, RNG, tie-breakers)
- Serialization rules

Nothing important exists outside the contract.

### Resolvers

Adjudicators that:
- Collect outputs by type
- Resolve conflicts using stable rules
- Write to NextState only (never CurrentState)

## Execution Flow

Strict phase order for determinism:

```
FRAME N:

1. SNAPSHOT PHASE (Serial)
   - Freeze CurrentState into immutable WorldView
   - Establish stable iteration order (entity_id, plugin_id)

2. PLUGIN PHASE (Parallelizable)
   - Each plugin reads WorldView
   - Each plugin emits Outputs (no state mutation)

3. RESOLUTION PHASE (Serial, Deterministic)
   - Collect outputs by type/category
   - Resolve conflicts using stable rules
   - Write to NextState only

4. APPLY & CLEANUP (Serial)
   - Swap NextState → CurrentState
   - Emit telemetry / causal traces
   - Advance RNG stream / tick counter
```

**Key principle**: Plugins never "do" things directly. They propose. Resolvers decide.

## State Components

Keep shared state small, stable, and meaningful:

| Component | Contents |
|-----------|----------|
| InventoryState | Fuel, ammo, spares, food, water |
| ReadinessState | Maintenance debt, reliability, fatigue |
| CombatState | Posture, cooldowns, heat/stress, weapons |
| SensorState | Emissions mode, detections, noise factors |
| TrackTableState | Contacts with age, quality, classification, IFF |
| CommsState | Links, bandwidth, latency, jamming pressure |
| GovernanceState | Government type, decision queue, constitution |
| CivicsState | Legitimacy, political capital, compliance |
| InternalFactionsState | Factions with size, goals, satisfaction |
| DiplomacyState | Relations, treaties, council membership |
| EnvironmentState | Layer, local weather/hazard fields |
| PersonState | Competence, traits, loyalty, ambition, grudges |
| FaceRoster | Named positions linked to stable FaceIds |
| InfoEnvironmentState | Comms integrity, internal trust, isolation |

Each component has: schema, owner system(s), mutation rules.

## Output Types

Outputs are typed proposals:

### Commands (Attempt Actions)
- `FireWeapon(target, slot, mode)`
- `QueueDecision(decision_type, params)`
- `LaunchCraft(squadron, mission)`
- `AllocateSupplies(route, amount)`

### Modifiers/Effects (Stat Changes)
- `ApplyModifier(target, stat, delta, duration)`

### Events (Facts for Reaction/Telemetry)
- `CrisisTriggered(type, severity)`
- `InfluenceDetected(actor, vector, confidence)`

### Intents (Planning-Level Goals)
- `Intent(kind, target, priority)`

### Reservations (Resource Contention)
- `Reserve(resource, amount)`

## Critical Guardrails

### Risk 1: Resolver Bottleneck

**Problem**: A resolver per action type becomes unmaintainable.

**Solution**: Generic Effect System. Most changes route through `EffectResolver` applying `ApplyModifier(target, stat, delta)`.

Reserve specialized resolvers for:
- Physics/collisions
- Detection/track fusion
- Vote tallying
- Combat damage propagation
- Major transitions (government changes, treaties)

### Risk 2: Event Hell

**Problem**: Hidden causality makes debugging impossible.

**Solution**: Causal Chains. Every output carries:
- `source_id`: Which entity/plugin emitted it
- `cause_id`: The upstream event that triggered it
- `trace_id`: Root chain identifier

Example:
```
ApplyModifier(
  target = Enclave_12,
  stat = "legitimacy",
  delta = -5,
  source_id = Ship_101::MutinyPlugin,
  cause_id = Event_88::MutinyAttemptStarted,
  trace_id = Trace_4001
)
```

Enables debugging: "Why did enclave 12 flip?" with attributable chain.

### Risk 3: Split Brain State

**Problem**: Plugin keeps internal variables that aren't serialized.

**Solution**: Private Components. When plugins need memory:
- Store in `PrivateComponent<PluginType>` attached to entity
- Only that plugin writes it
- Fully serialized, replay-safe, deterministic

Examples:
- `PrivateComponent::InfluenceOpsMemory`: Active operations, cooldowns
- `PrivateComponent::WeaponFireControl`: Salvo staging, last track

## Example: Government Change

Government change is a state transition + plugin bundle swap:

**Flow**:
1. Influence ops, shortages, propaganda emit effects changing legitimacy, satisfaction, cohesion
2. TransitionEvaluator checks thresholds
3. On success:
   - Apply transition effects (purges, instability)
   - Swap government bundle (remove old plugins, add new)
   - Update `GovernanceState.constitution_id`
4. Next tick, new government's plugins run with their latency/constraints

Causal chains produce clear narratives for debugging and player feedback.

## Example: People as Entities

People are entities with `PersonState`, but with presence-gated instantiation:

**Flow**:
1. Arcologies/factions maintain a `FaceRoster` with stable `FaceId` handles
2. When player arrives at arcology, `SpawnFacesResolver` instantiates Person entities
3. Role plugins (e.g., `GovernorPlugin`, `CaptainPlugin`) read PersonState and emit modifiers
4. When player departs, Person entities go dormant (persist but don't run behavioural plugins)

**Why this works**:
- Named people exist when narratively relevant (player interaction)
- Aggregate politics handle the rest (`GovernanceState`, `InternalFactionsState`)
- Stable FaceIds preserve continuity across visits
- Causal chains explain "who did what and why they're now faction leader"

See [People System](people.md) for full details.

## Required Tooling

This architecture requires investment in:

| Tool | Purpose |
|------|---------|
| Contract Schema Validator | State, outputs, events |
| Plugin Dependency Validator | Required components/tags |
| Replay Inspector | Step → outputs → resolution → state delta |
| Causal Graph Viewer | Trace ID chains |
| Determinism Diff Tool | Pinpoint divergence tick |

Without these, "clever architecture" becomes "mysterious architecture."

## Implementation Milestones

### 1. Skeleton
- Entity container + component storage
- Shared Contract schemas
- Plugin interface (WorldView in, Outputs out)
- One resolver + deterministic loop + seeded RNG
- Telemetry log + replay from seed

### 2. First Slice
- 2–3 ship plugins (movement, weapon, sensor)
- 2–3 governance bundles (autocracy, corporate, democracy)
- Generic EffectResolver
- Causal chain plumbing

### 3. Expansion
- Influence ops + internal factions
- Mission intents + task offers
- Council votes + enforcement hooks
- Fidelity scaling (high detail near player)
