# Architecture Overview

This document describes Tidebreak's high-level architecture. It focuses on system boundaries and interactions, not implementation details.

## Canonical Types Used

- **BattlePackage**: Input contract for Combat Arena
- **BattleResult**: Output contract from Combat Arena
- **Entity**: Fundamental container (see entity-framework.md)
- **WorldView**: Immutable state snapshot for plugins

## System Boundaries

Tidebreak consists of two major systems that can evolve independently:

```
┌─────────────────────────────────────────────────────────────────┐
│                        Full Simulation                          │
│  (Campaign, Economy, Factions, Diplomacy, Governance, Time)     │
│                                                                 │
│    ┌─────────────────────────────────────────────────────┐     │
│    │                   Combat Arena                       │     │
│    │  (Tactical battles, DRL training, Deterministic)     │     │
│    └─────────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────────┘
```

**Combat Arena**: Self-contained battle simulator. Receives a snapshot of ships, terrain, and conditions. Returns battle outcomes. Designed for both player-facing combat and DRL training. This is the current development focus.

**Full Simulation**: Strategic layer managing the persistent world—time progression, economy, faction relations, governance, and population. Invokes the Combat Arena when battles occur. Not yet implemented.

The arena can run **in-process** (library call) or as a **service** (RPC) without changing data contracts.

## Data Contracts

Systems communicate through versioned, self-contained data structures:

**BattlePackage** (Full Sim → Arena)
- Battle configuration (seed, time limit, teams)
- Map definition (bounds, obstacles, zones, currents, weather)
- Ship snapshots (hull, weapons, sensors, crew, initial state)
- No references back to the main simulation

**BattleResult** (Arena → Full Sim)
- Winner and duration
- Per-ship outcomes (final state, consumption, casualties)
- Event summary (hits, kills, transitions)
- Replay data (seed, action log reference)

Contracts are **forward-compatible**: old clients ignore new fields via `extra="ignore"` semantics.

## Entity Framework

Both systems use a common entity model (see [entity-framework.md](entity-framework.md) for details):

**Core Concepts**:
- **Entity**: Container with identity, state components, and plugins
- **Plugin**: Capability module that reads state and emits proposals
- **Resolver**: Adjudicator that applies proposals deterministically
- **Shared Contract**: Schema definitions for all state and events

**Execution Model**:
```
1. SNAPSHOT: Freeze state into immutable WorldView
2. PLUGINS:  Each plugin reads WorldView, emits Outputs (parallelizable)
3. RESOLVE:  Collect outputs, resolve conflicts, write to NextState
4. APPLY:    Swap NextState → CurrentState, emit telemetry
```

Plugins never mutate state directly. They propose; resolvers decide.

## Combat Arena Internals

The arena runs a deterministic step loop:

```
┌───────────────────────────────────────────────────────────┐
│                    Combat Arena API                        │
│  simulate(BattlePackage) → BattleResult                   │
│  env.reset() / env.step() (Gymnasium interface)           │
└─────────────────────────────┬─────────────────────────────┘
                              │
                              ▼
┌───────────────────────────────────────────────────────────┐
│                      Battle State                          │
│  (deterministic, serializable, seeded RNG)                 │
└─────────────────────────────┬─────────────────────────────┘
                              │ step(dt)
                              ▼
┌───────────────────────────────────────────────────────────┐
│              Systems (ordered, pure updates)               │
│                                                           │
│  1. Environment (currents, weather, hazards)              │
│  2. Sensors (detect contacts, update tracks)              │
│  3. Controllers (scripted / human / DRL inputs)           │
│  4. Movement (physics, collisions, layer transitions)     │
│  5. Weapons (fire, spawn projectiles, homing)             │
│  6. Damage (resolve hits, crew modifiers, cascades)       │
│  7. Termination (win/loss/timeout checks)                 │
│  8. Telemetry (events, metrics, replay logging)           │
└───────────────────────────────────────────────────────────┘
```

**Key Properties**:
- Fixed timestep (configurable, default 0.1s)
- Seeded RNG for all randomness
- Headless mode for fast DRL rollouts
- Controllers are isolated from simulation rules

## DRL Integration

The arena exposes a Gymnasium/PettingZoo-style interface:

```python
env.reset(battle_package) → observations
env.step(actions) → observations, rewards, terminated, truncated, info
```

**Observations**: Per-agent view including own ship state, sensed contacts (not ground truth), local environment.

**Actions**: Hybrid space—continuous (throttle, turn) and discrete (fire, dive, surface).

**Rewards**: Sparse (win/loss) plus shaped (damage dealt, survival, efficiency).

**Training Curriculum**:
1. 1v1 in calm conditions
2. Add currents and obstacles
3. Add weather hazards
4. Add sensor occlusion
5. Multi-ship coordination
6. Asymmetric fleet compositions

## Full Simulation (Future)

The strategic layer will manage:

| System | Responsibility |
|--------|----------------|
| World State | Time, map, persistent entities |
| Economy | Resources, production, trade routes |
| Factions | Relations, treaties, reputation |
| Governance | Government types, decisions, legitimacy |
| Population | Needs, morale, internal factions |
| Events | Crises, weather patterns, migrations |

When combat occurs, the full simulation:
1. Builds a BattlePackage from current world state
2. Invokes the Combat Arena
3. Applies BattleResult to update world state

The arena doesn't know about the campaign layer. The campaign layer doesn't know about combat mechanics. They communicate only through contracts.

## Cross-Cutting Concerns

**Determinism**: Same build + same platform + same seed + same inputs → identical results and replay. Required for DRL training, replay debugging, and multiplayer. Cross-platform determinism is a stretch goal requiring deterministic math modes (fixed-point or constrained integrators).

**Serialization**: All state is serializable. Supports save/load, replay, and debugging.

**Telemetry**: Structured event logging with causal chain IDs. Enables tracing cause-and-effect through complex interactions.

**Performance**: Headless mode targets faster-than-real-time simulation. Fleet-scale battles (dozens of ships) must not degrade stability.

## Current Status

| Component | Status |
|-----------|--------|
| Combat Arena (MVP) | Prototype exists, needs redesign |
| BattlePackage/BattleResult contracts | Drafted |
| Entity Framework | Designed, not implemented |
| DRL Environment | Basic wrapper exists |
| Full Simulation | Not started |

Development priority is rebuilding the Combat Arena from clean specifications before expanding to the full simulation.
