# ADR 0001: Entity-Plugin-Resolver Architecture

## Status
Accepted

## Context

Tidebreak requires an architecture that supports:

1. **Determinism**: Same inputs + same seed must produce identical results for DRL training, replays, and debugging.
2. **Parallelization**: Plugin execution should be parallelizable for performance.
3. **DRL-native design**: The architecture should map cleanly to reinforcement learning concepts (observation, action, transition).
4. **Modularity**: Systems should interact through contracts, not tight coupling.
5. **Traceability**: Every outcome should be attributable to a cause chain.

Traditional ECS (Entity-Component-System) allows systems to mutate components directly, which creates ordering dependencies, race conditions under parallelization, and makes determinism difficult to guarantee.

## Decision

Use a **Reactive ECS variant** called Entity-Plugin-Resolver:

- **Entities** are containers with identity, state components, and attached plugins.
- **Plugins** read from an immutable `WorldView` snapshot and emit typed `Outputs` (proposals). Plugins never mutate state directly.
- **Resolvers** collect outputs by type, resolve conflicts using stable rules, and write to `NextState`.
- **Execution** follows a strict four-phase loop: SNAPSHOT → PLUGIN → RESOLUTION → APPLY.

Key invariant: **Plugins propose, resolvers decide.**

### DRL Mapping

| Framework Concept | DRL Equivalent |
|-------------------|----------------|
| WorldView (immutable snapshot) | Observation |
| Outputs (proposals) | Action |
| Resolver step | Transition function |
| Reward emitter | Reward signal |

A DRL policy is just a plugin that emits outputs.

## Consequences

### Enables

- **Determinism**: Plugins cannot create race conditions; resolution order is explicit and stable.
- **Parallelization**: Plugin phase is embarrassingly parallel since plugins only read and emit.
- **Replay**: Any state can be reconstructed from seed + inputs.
- **Debugging**: Causal chains trace every effect to its source.
- **Testing**: Plugins are pure functions; resolvers have explicit conflict rules.

### Costs

- **Indirection**: Actions require two hops (plugin emits, resolver applies). More complex than direct mutation.
- **Resolver bottleneck risk**: If every action type needs a custom resolver, the resolver layer becomes unwieldy. Mitigated by generic `EffectResolver` for most stat changes.
- **Learning curve**: Contributors must understand the proposal/resolution split.

### Follow-ups Required

- Define the Shared Contract (all component, event, and output schemas).
- Implement generic `EffectResolver` for `ApplyModifier` outputs.
- Build tooling: Replay Inspector, Plugin Dependency Validator, Causal Graph Viewer.

## Alternatives Considered

### Traditional ECS (Unity-style)

Systems directly mutate components in order.

**Rejected because**: Order-dependent mutations make determinism fragile. Parallelization requires careful system scheduling. No natural DRL mapping.

### Actor Model

Entities as actors exchanging messages.

**Rejected because**: Message ordering is harder to make deterministic. Less natural fit for tick-based simulation. Higher complexity for the problem at hand.

### Event Sourcing (Pure)

All state derived from event log.

**Rejected because**: Performance concerns for real-time simulation. The hybrid approach (outputs as proposals, resolvers write state) captures the benefits without full event replay on every tick.
