# ADR 0004: Combat Arena as Isolated Subsystem

## Status
Accepted

## Context

Tidebreak has two major simulation layers:

1. **Combat Arena**: Real-time tactical battles (seconds to minutes).
2. **Full Simulation**: Strategic campaign with economy, factions, governance (days to years).

These layers have different:
- **Timescales**: Combat in seconds, campaign in days.
- **Fidelity**: Combat needs physics; campaign needs aggregate models.
- **Development priority**: Combat Arena is MVP; Full Simulation is later.
- **Runtime modes**: Combat needs headless DRL training; campaign needs save/load.

The question: How tightly coupled should these layers be?

## Decision

The **Combat Arena is an isolated subsystem** that:

1. **Knows nothing about the campaign layer**. No references to factions, economy, governance, or world state.
2. **Communicates via versioned data contracts**: `BattlePackage` (input) and `BattleResult` (output).
3. **Is a pure function**: `simulate(BattlePackage) → BattleResult` with no side effects.
4. **Can run in-process or as a service** without changing contracts.

### Data Contracts

**BattlePackage** (Campaign → Arena):
- Battle configuration (seed, time limit, teams)
- Map definition (bounds, obstacles, zones, currents, weather)
- Ship snapshots (hull, weapons, sensors, crew, initial state)
- Faction context (morale, tech tags, philosophy) — as **data**, not references

**BattleResult** (Arena → Campaign):
- Winner and duration
- Per-ship outcomes (fate, final state, consumption, casualties)
- Per-team outcomes (morale delta, losses)
- Event summary (hits, kills, surrenders)
- Replay data (seed, action log reference)

### Contract Versioning

Contracts are versioned (`arena.v1`, `arena.v2`, `arena.v3`) with forward compatibility:
- Old clients ignore new fields (`extra="ignore"`).
- New features (supply effects, persistent damage, boarding) add fields without breaking old packages.

```
arena.v1 (MVP):   weather, crew, basic morale
arena.v2 (P2):    supply, damage_state, leadership
arena.v3 (P3):    siege_state, full boarding
```

## Consequences

### Enables

- **Independent development**: Arena can be built and tested without campaign layer.
- **DRL training isolation**: Train agents without loading economy/faction systems.
- **Testability**: Arena tests use BattlePackage fixtures, not full world state.
- **Service deployment**: Arena could run as microservice for multiplayer or distributed training.
- **Clear ownership**: Arena team owns arena; campaign team owns campaign; contracts are the interface.

### Costs

- **Serialization overhead**: Campaign must serialize ship state into snapshots. Not free, but bounded.
- **Contract maintenance**: Changes to what flows between layers require versioned contract updates.
- **No live campaign queries**: Arena cannot ask "what's the current diplomatic state?" — must be baked into BattlePackage.
- **Potential duplication**: Some types (ship stats, weapon definitions) appear in both layers.

### Trade-off Accepted

The isolation overhead is worth paying because:
1. DRL training will run millions of battles; isolation enables this without campaign overhead.
2. The contract boundary forces explicit design of what crosses layers.
3. Campaign layer isn't built yet; arena shouldn't wait for it or depend on it.

### Follow-ups Required

- Define `BattlePackage` and `BattleResult` schemas in `contracts.md` (done).
- Implement schema validation for contract compliance.
- Define migration path for contract version upgrades.
- Document which campaign state flows into which BattlePackage fields.

## Alternatives Considered

### Tight Integration

Arena directly accesses campaign world state. Ships are the same objects in both layers.

**Rejected because**:
- Cannot run arena without campaign infrastructure.
- DRL training would need to mock entire campaign layer.
- Changes to campaign types break arena.
- Harder to reason about what state affects combat.

### Event-Driven Integration

Arena emits events; campaign subscribes and updates.

**Rejected because**:
- Adds complexity without clear benefit over pure function model.
- Event ordering and delivery guarantees are another thing to get right.
- The current model (return BattleResult, campaign applies it) is simpler and sufficient.

### Shared Library, Separate Processes

Arena as separate process communicating via IPC.

**Rejected because**:
- The decision is about logical isolation, not physical.
- In-process is simpler for MVP.
- The contract design allows service deployment *later* if needed.
- "Can run as service" is enabled by the contract design, not required by it.
