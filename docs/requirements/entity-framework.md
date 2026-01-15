# Entity Framework Requirements

Requirements for the Entity-Plugin-Resolver architecture.

See: [design/entity-framework.md](../design/entity-framework.md)

## Entities (P0)

- Support Entity as a container with identity, state components, and plugins
- Support entities having unique IDs and type tags (ship, faction, platform, enclave)
- Support entities owning child entities (squadrons, compartments, internal factions)
- Support entities being behavior-light—behavior lives in plugins and resolvers
- Support entity creation and destruction during simulation

## Plugins (P0)

- Support plugins as capability modules attached to entities
- Support plugins reading from bounded WorldView (read-only access)
- Support plugins emitting typed Outputs (proposals, not mutations)
- Support plugins storing memory only via serialized Private Components
- Support plugin declarations specifying:
  - Required tags and components
  - Which components they read
  - Which output types they emit
  - Which phases they run in

## Plugin Invariants (P0)

- **INV-P1**: Plugins MUST NOT mutate any state directly
- **INV-P2**: Plugins MUST NOT read from anything except WorldView
- **INV-P3**: Plugins MUST NOT communicate with other plugins except via Outputs
- **INV-P4**: Plugins MUST be deterministic given same WorldView
- **INV-P5**: Plugin private state MUST be serializable

## Resolvers (P0)

- Support resolvers as adjudicators that collect and process outputs
- Support resolvers writing only to NextState (never CurrentState)
- Support resolvers using stable, documented conflict resolution rules
- Support generic EffectResolver for most stat changes via ApplyModifier
- Support specialized resolvers for:
  - Physics and collisions
  - Detection and track fusion
  - Combat damage propagation

## Resolver Invariants (P0)

- **INV-R1**: Resolvers MUST process outputs in deterministic order
- **INV-R2**: Resolvers MUST use stable tie-breakers (entity_id, then timestamp)
- **INV-R3**: Resolvers MUST NOT create hidden state
- **INV-R4**: Resolvers MUST document conflict resolution rules

## Shared Contract (P0)

- Support Shared Contract defining all legal state, events, and outputs
- Support state component schemas with validation rules
- Support event schemas with required fields
- Support output schemas with allowed targets
- Support serialization rules for all contract types
- Support forward compatibility via `extra="ignore"` semantics

## Execution Flow (P0)

- Support strict four-phase execution per tick:
  1. **SNAPSHOT**: Freeze CurrentState into immutable WorldView
  2. **PLUGIN**: Each plugin reads WorldView, emits Outputs (parallelizable)
  3. **RESOLUTION**: Collect outputs, resolve conflicts, write to NextState
  4. **APPLY**: Swap NextState → CurrentState, emit telemetry

- Support stable iteration order (entity_id, plugin_id) for determinism
- Support RNG stream advancement only in APPLY phase

## WorldView (P0)

- Support WorldView as immutable snapshot of current state
- Support WorldView providing read-only access to:
  - Entity states and components
  - Global environment state
  - Current tick and elapsed time
- Support WorldView filtering based on plugin declarations

## Outputs (P0)

- Support typed output categories:
  - **Commands**: Actions to attempt (FireWeapon, LaunchCraft, QueueDecision)
  - **Modifiers**: Stat changes (ApplyModifier with target, stat, delta, duration)
  - **Events**: Facts for reaction/telemetry (CrisisTriggered, InfluenceDetected)
  - **Intents**: Planning-level goals (Intent with kind, target, priority)
  - **Reservations**: Resource contention (Reserve with resource, amount)

## Causal Chains (P0)

- Support causal chain metadata on all outputs:
  - `source_id`: Which entity/plugin emitted it
  - `cause_id`: The upstream event that triggered it
  - `trace_id`: Root chain identifier
- Support tracing any effect back to its root cause
- Support causal chain preservation through serialization/replay

## State Components (P1)

- Support standard state component schemas:
  - InventoryState (fuel, ammo, spares, food, water)
  - ReadinessState (maintenance debt, reliability, fatigue)
  - CombatState (posture, cooldowns, weapons)
  - SensorState (emissions mode, detections, noise)
  - TrackTableState (contacts with age, quality, IFF)
  - GovernanceState (government type, decision queue)
  - CivicsState (legitimacy, political capital)

- Support each component having defined owner systems and mutation rules

## Private Components (P0)

- Support private components for plugin-specific memory
- Support private components being attached to entities
- Support private components being fully serialized
- Support private components being replay-safe and deterministic
- Support only owning plugin writing to its private components

## Plugin Bundles (P1)

- Support plugin bundles as named collections of plugins
- Support bundle swapping for entity type changes (e.g., government transitions)
- Support bundle swapping preserving entity identity
- Support bundle definitions in data (not code)

## Required Tooling (P1)

- Support Contract Schema Validator for state, outputs, events
- Support Plugin Dependency Validator for required components/tags
- Support Replay Inspector for step → outputs → resolution → state delta

## Required Tooling (P2)

- Support Causal Graph Viewer for trace ID chains
- Support Determinism Diff Tool for pinpointing divergence tick
