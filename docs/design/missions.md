# Mission System Design

This document specifies the mission system that bridges strategic faction missions to tactical ship objectives, with DRL agent integration.

## Overview

**Problem**: The governance system (`governance.md`) handles strategic missions (faction goals → mission generation → fleet assignment), but there's no specification for:
- How fleets decompose missions into ship-level objectives
- How ships know what they're trying to accomplish in battle
- How objectives translate to DRL observation/reward signals

**Solution**: A two-layer system:
- **Operational layer**: Fleet commanders decompose strategic missions into tactical objectives for subordinate ships
- **Tactical layer**: Ships pursue objectives within battles, with objective-conditioned DRL policies

**Scope boundaries**:

| Layer | Responsibility | Documented In |
|-------|---------------|---------------|
| Strategic | Faction goals → mission generation → fleet assignment | governance.md |
| Operational | Mission decomposition → ship objective assignment | **This document** |
| Tactical | Objective pursuit → DRL observation/reward | **This document** |

**MVP vs Target**:
- MVP: Pre-assigned objectives (ships enter battle with fixed objectives)
- Target: Flagship command (dynamic reassignment mid-battle)

Data structures support both; MVP implements the simpler path.

## Canonical Types Used

- **TacticalObjective**: Ship-level goal with target/zone/destination parameters
- **ObjectiveState**: Current objective + progress tracking (component)
- **ObjectiveProgress**: Metrics for reward computation
- **ObjectiveOutcome**: Result of objective pursuit (in BattleResult)
- **RewardConfig**: Per-mission reward weight overrides

## Tactical Objective Taxonomy

Ships pursue **tactical objectives** — concrete goals within a battle or patrol. Fourteen objective types span five categories:

### Combat Objectives

| Type | Intent | Success State |
|------|--------|---------------|
| **DESTROY** | Eliminate specific target(s) | Target destroyed |
| **DISABLE** | Mission-kill target (neutralize without sinking) | Target cannot fight/maneuver |
| **RAID** | Maximize value extracted (damage, captures, cargo) | Survive with accumulated value |
| **INTERCEPT** | Engage threats approaching protected asset | Threats neutralized before reaching asset |

### Defense Objectives

| Type | Intent | Success State |
|------|--------|---------------|
| **PROTECT** | Keep specific asset alive | Asset survives battle |
| **SCREEN** | Hold zone relative to asset, intercept threats | Maintain position, threats engaged |

### Seizure Objectives

| Type | Intent | Success State |
|------|--------|---------------|
| **CAPTURE** | Board and take control of target | Boarding established |

### Survival Objectives

| Type | Intent | Success State |
|------|--------|---------------|
| **EVADE** | Escape battle area alive | Exit designated zone |

### Surveillance Objectives

| Type | Intent | Success State |
|------|--------|---------------|
| **SHADOW** | Covertly trail target | Maintain track without detection |
| **LOITER** | Hold area for intel/ambush | Duration met or ambush sprung |
| **SCOUT** | Detect and report contacts in area | Zone scanned, survive to report |

### Logistics Objectives

| Type | Intent | Success State |
|------|--------|---------------|
| **DELIVER** | Transport cargo/troops to destination | Cargo arrives at target |
| **RECOVER** | Pick up asset/personnel | Payload aboard |
| **EVACUATE** | Extract under hostile pressure | Payload extracted to safety |

**Key distinctions**:
- DESTROY vs DISABLE: Sink vs neutralize (preserves capture potential)
- DESTROY vs RAID: Target-specific vs value-maximizing (privateering)
- SHADOW vs SCOUT: Covert + specific target vs active + area coverage
- RECOVER vs EVACUATE: Permissive vs contested extraction

## Objective Data Structures

### TacticalObjective

The core data structure for specifying what a ship should accomplish:

```rust
TacticalObjective {
    objective_id:   ObjectiveId,
    type:           ObjectiveType,

    // Target specification
    target:         Option<EntityId>,       // Primary target (DESTROY, SHADOW, CAPTURE, DISABLE)
    targets:        Option<Set<EntityId>>,  // Multiple valid targets (RAID, INTERCEPT)
    asset:          Option<EntityId>,       // What you're protecting (PROTECT, SCREEN, INTERCEPT)

    // Location specification
    zone:           Option<Zone>,           // Area of responsibility (LOITER, SCOUT, SCREEN, RAID)
    destination:    Option<Destination>,    // Where to go (DELIVER, EVADE, EVACUATE)

    // Payload (logistics)
    cargo:          Option<CargoManifest>,  // What you're carrying (DELIVER, RECOVER)

    // Priority and timing
    priority:       f32,                    // 0.0-1.0, for multi-objective weighting
    deadline:       Option<Tick>,           // Time limit (if any)

    // Reward tuning
    reward_config:  Option<RewardConfig>,   // Override default reward weights
}
```

### Supporting Types

```rust
Destination = Entity { id: EntityId }
            | Position { x: f32, y: f32 }
            | Rendezvous { entity: EntityId, fallback: Position }

Zone {
    center:      Position,
    radius:      f32,
    shape:       ZoneShape,          // Circle, Rectangle, Polygon
    relative_to: Option<EntityId>,   // If set, zone moves with this entity
}

CargoManifest {
    items: Vec<CargoItem>,
}

CargoItem = Units { unit_type: UnitType, count: u32 }
          | Goods { resource_type: ResourceType, quantity: f32 }
```

### RewardConfig

Per-mission override for reward shaping weights:

```rust
RewardConfig {
    // Component weights (multipliers on default)
    progress_weight:    f32,    // Dense progress signal
    completion_bonus:   f32,    // Sparse completion reward
    failure_penalty:    f32,    // Sparse failure penalty
    time_pressure:      f32,    // Per-tick cost
    survival_mult:      f32,    // Multiplier if ship survives

    // RAID-specific
    damage_weight:      Option<f32>,
    capture_bonus:      Option<f32>,
    cargo_value_mult:   Option<f32>,
}
```

## DRL Integration

### Observation Encoding

Objectives appear in the agent's observation space as a fixed-shape tensor:

```
objective_obs = [
    type:              one_hot(14),      # Objective type
    has_target:        bool,
    target_relative:   [bearing, range, heading_delta, speed_delta],
    has_asset:         bool,
    asset_relative:    [bearing, range, heading_delta, speed_delta],
    has_zone:          bool,
    zone_relative:     [bearing_to_center, dist_to_center, radius],
    has_destination:   bool,
    dest_relative:     [bearing, range],
    priority:          f32,
    time_remaining:    f32,              # Normalized, if deadline set
]
```

Approximately 35-40 floats. Sparse but simple.

**Upgrade path**:
1. MVP: One-hot + flat params
2. V2: Learned goal embeddings
3. V3: Attention-based encoding
4. V4: Hierarchical objective trees

### Reward Shaping

Each objective type generates rewards from five components:

| Component | Type | Signal |
|-----------|------|--------|
| Progress | Dense | Continuous approach toward success |
| Completion | Sparse | Bonus on objective achieved |
| Failure | Sparse | Penalty when objective impossible |
| Time | Dense | Slight negative per tick (urgency) |
| Survival | Multiplier | Bonus if ship survives |

### Per-Objective Reward Tables

**Combat Objectives**:

| Objective | Progress (dense) | Completion (sparse) | Failure (sparse) |
|-----------|------------------|---------------------|------------------|
| DESTROY | Damage dealt to target | Target destroyed | Target escapes / you die |
| DISABLE | System damage to target | Target mission-killed | Target escapes |
| RAID | Value extracted (damage + captures + cargo) | Escape with loot | Die before extracting |
| INTERCEPT | Threats engaged; asset health preserved | All threats neutralized | Threat reaches asset |

**Defense Objectives**:

| Objective | Progress | Completion | Failure |
|-----------|----------|------------|---------|
| PROTECT | Asset health maintained; threats engaged | Asset survives battle | Asset destroyed |
| SCREEN | Time in screen zone; threats intercepted | Battle ends, asset intact | Leave zone; asset hit |

**Seizure**:

| Objective | Progress | Completion | Failure |
|-----------|----------|------------|---------|
| CAPTURE | Approach; disable target; boarding progress | Boarding established | Target destroyed / escapes |

**Survival**:

| Objective | Progress | Completion | Failure |
|-----------|----------|------------|---------|
| EVADE | Distance toward exit; avoided engagements | Exit battle area | Destroyed |

**Surveillance**:

| Objective | Progress | Completion | Failure |
|-----------|----------|------------|---------|
| SHADOW | Maintain track on target; stay undetected | Duration met | Detected / lose track |
| LOITER | Time in zone; contacts detected | Duration met / ambush sprung | Forced out of zone |
| SCOUT | Contacts detected; area coverage | Zone fully scanned | Destroyed before reporting |

**Logistics**:

| Objective | Progress | Completion | Failure |
|-----------|----------|------------|---------|
| DELIVER | Distance to destination; cargo intact | Cargo delivered | Cargo lost / destroyed |
| RECOVER | Approach target; pickup progress | Payload aboard | Target lost / destroyed |
| EVACUATE | Approach; pickup; egress progress | Payload extracted to safety | Payload lost |

### RAID Special Case

RAID is unique — it's value-maximization, not goal-completion:

```
raid_reward = (
    damage_dealt × damage_weight
  + ships_disabled × disable_bonus
  + ships_captured × capture_bonus
  + cargo_seized × cargo_value
  + survived × survival_multiplier
) − time_penalty × ticks_elapsed
```

No single "completion" — reward accumulates continuously. The agent learns to balance aggression vs extraction timing. This is the "privateering" objective: profitable violence with timely withdrawal.

## Entity Framework Integration

### Components

```rust
// Attached to ships
ObjectiveState {
    current:            Option<TacticalObjective>,
    assigned_by:        Option<EntityId>,
    assigned_at:        Tick,
    progress:           ObjectiveProgress,
    status:             ObjectiveStatus,    // ACTIVE | COMPLETED | FAILED | SUPERSEDED
    cumulative_reward:  f32,
}

ObjectiveProgress {
    // Universal
    time_on_objective:  Ticks,

    // Combat
    damage_dealt:       f32,
    damage_to_target:   f32,

    // Spatial
    time_in_zone:       Ticks,
    closest_approach:   f32,

    // Surveillance
    track_maintained:   Ticks,
    contacts_detected:  u32,

    // Logistics
    cargo_delivered:    f32,      // 0.0 - 1.0
    payload_aboard:     bool,

    // Computed
    completion_pct:     f32,      // Normalized 0.0 - 1.0
}

// Attached to flagships
FleetObjectiveState {
    fleet_mission:           Option<MissionId>,  // Link to strategic mission
    subordinate_assignments: Map<EntityId, ObjectiveId>,
}
```

### Plugins

| Plugin | Attached To | Reads | Emits |
|--------|-------------|-------|-------|
| ObjectiveTrackerPlugin | Ship | WorldView, ObjectiveState | ObjectiveProgressUpdate, ObjectiveCompleted, ObjectiveFailed |
| ObjectiveRewardPlugin | Ship | ObjectiveState, Events | RewardSignal |
| FleetCoordinatorPlugin | Flagship | FleetObjectiveState, WorldView | AssignObjective |

### Resolver

`ObjectiveResolver` handles:
- Validating `AssignObjective` outputs
- Writing to `ObjectiveState`
- Updating progress from `ObjectiveProgressUpdate`
- Transitioning status on completion/failure
- Marking old objectives as `SUPERSEDED` on reassignment

### Objective Lifecycle

```
1. ASSIGNMENT
   - FleetCoordinatorPlugin emits AssignObjective(ship_id, objective)
   - ObjectiveResolver validates and writes to ship's ObjectiveState

2. TRACKING (each tick)
   - ObjectiveTrackerPlugin reads WorldView
   - Emits ObjectiveProgressUpdate based on events (damage, position, detections)
   - ObjectiveResolver updates ObjectiveState.progress

3. COMPLETION/FAILURE
   - ObjectiveTrackerPlugin detects terminal condition
   - Emits ObjectiveCompleted or ObjectiveFailed
   - ObjectiveResolver updates status, records in history

4. REASSIGNMENT (flagship command, post-MVP)
   - FleetCoordinatorPlugin emits AssignObjective with new objective
   - ObjectiveResolver marks old as SUPERSEDED, installs new
```

## Strategic → Operational → Tactical Flow

```
STRATEGIC (governance.md)
│
│ Mission: "Escort convoy Alpha from port X to port Y"
│ Assigned to: Fleet 7
│
▼
OPERATIONAL (this doc)
│
│ Fleet 7 flagship decomposes:
│   Ship A → SCREEN(asset=convoy, zone=north_flank)
│   Ship B → SCREEN(asset=convoy, zone=south_flank)
│   Ship C → SCOUT(zone=forward_arc)
│   Ship D → PROTECT(asset=convoy)
│
▼
TACTICAL (combat arena)
│
│ Battle triggered → ships enter with objectives
│ DRL agents execute objective-conditioned policies
│
▼
RESULT
│
│ ObjectiveOutcome per ship flows back up
│ Fleet mission progress updated
│ Strategic mission status updated
```

### BattlePackage Extension

```rust
ShipSnapshot {
    // ... existing fields from combat-arena.md ...

    // NEW: Tactical objective
    objective:      Option<TacticalObjective>,

    // NEW: Fleet context
    fleet_id:       Option<EntityId>,
    is_flagship:    bool,
    subordinates:   Vec<EntityId>,
}
```

### BattleResult Extension

```rust
ShipOutcome {
    // ... existing fields from combat-arena.md ...

    // NEW: Objective outcome
    objective_outcome: Option<ObjectiveOutcome>,
}

ObjectiveOutcome {
    objective_type:     ObjectiveType,
    status:             ObjectiveStatus,
    progress:           ObjectiveProgress,
    reward_earned:      f32,
    key_events:         Vec<EventId>,
}
```

## Implementation Milestones

### MVP (P1)

- [ ] `TacticalObjective` and `ObjectiveState` data structures
- [ ] 4 core objective types: DESTROY, PROTECT, RAID, EVADE
- [ ] `ObjectiveTrackerPlugin` with progress tracking
- [ ] `ObjectiveRewardPlugin` with basic shaping
- [ ] `ObjectiveResolver` (assignment only, no mid-battle reassignment)
- [ ] Observation encoding (one-hot + flat params)
- [ ] BattlePackage/BattleResult extensions
- [ ] Integration tests with scripted scenarios

### V2 (P2)

- [ ] Remaining 10 objective types
- [ ] `FleetCoordinatorPlugin` with pre-battle decomposition
- [ ] Learned goal embeddings for observation
- [ ] Reward config tuning based on training results

### V3 (P3)

- [ ] Mid-battle objective reassignment (flagship command)
- [ ] Attention-based observation encoding
- [ ] Multi-objective support (primary + fallback)
- [ ] Hierarchical objective trees

## Related Documents

- [Governance](governance.md) — Strategic mission system
- [Combat Arena](combat-arena.md) — Tactical battle simulator
- [Entity Framework](entity-framework.md) — Plugin/resolver architecture
- [DRL Requirements](../requirements/drl.md) — Training requirements
- [Factions](factions.md) — Faction goals driving missions
- [Glossary](../vision/glossary.md) — Canonical terminology
