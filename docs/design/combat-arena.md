# Combat Arena Design

The Combat Arena is Tidebreak's self-contained battle simulator. It handles tactical combat for both player-facing gameplay and DRL training.

See [architecture.md](architecture.md) for how the arena fits into the larger system.

## Canonical Types Used

- **BattlePackage**: Input contract (ships, terrain, weather, teams)
- **BattleResult**: Output contract (winner, outcomes, events, replay)
- **LayerState**: SURFACE, SUBMERGED, TRANSITIONING (ABYSSAL post-MVP)
- **Track**: Fused contact estimate (not ground truth)

## Goals

- Deterministic, headless-capable simulation suitable for DRL training
- Accept battle snapshots via stable, versioned data contracts
- Return results the full simulation can apply (damage, consumption, casualties)
- Support both in-process and service deployment

## Non-Goals (MVP)

- Campaign layer, economy, diplomacy, governance
- Full ship roster, full weapon taxonomy, carrier operations, boarding
- Rendered graphics (debug visualization optional)

## MVP Scope

### Combat Model

**Arena**:
- 2D top-down with fixed timestep
- Two depth states: `SURFACE` and `SUBMERGED` (+ `TRANSITIONING` between them)
- `ABYSSAL` layer is post-MVP

**Ship Archetypes** (two for MVP):
- Surface Corvette: Guns only, cannot dive
- Attack Sub: Torpedoes only, must stay submerged

**Terrain**:
- Solid obstacles (reefs, islands) as circles or polygons
- Soft zones (sensor interference, algae blooms) with intensity
- Currents as vector field (coarse grid)
- Hazardous surface regions (storm belts) with severity

**Sensors** (simplified for MVP):
- Surface sensors (radar/visual) effective at surface
- Sonar for submerged detection
- Range-limited contacts, optional terrain occlusion

**Weapons**:
- Gun shells: Fast projectiles with cooldown and dispersion
- Torpedoes: Slower homing/lead-seeking with acquisition limits

**Damage**:
- Hitpoints with optional component health (stretch goal)
- See [damage-and-boarding.md](damage-and-boarding.md) for tiered approach

**Crew Influence** (aggregated stats):
- Gunnery: Accuracy/reload modifiers
- Engineering: Repair/damage control rates
- Morale: Reaction latency, failure chances, surrender behavior

### Layer Transitions

Ships transitioning between layers:
- Take time (configurable dive/surface durations)
- Have limited weapon availability
- Generate detectable sonar signatures
- Consume energy proportional to hull size
- Can be interrupted by damage
- Fail if exceeding crush depth

## Data Contracts

### Strategic State Flows

The combat arena is a pure function: `BattlePackage → BattleResult`. Strategic layer state flows **into** battles and results flow **back out**:

| Strategic State | Flows Into | Flows Out Of | Purpose | Priority |
|-----------------|------------|--------------|---------|----------|
| **Faction morale** | `TeamContext.faction_context.morale_state` | `TeamOutcome.morale_delta` | Surrender thresholds, accuracy | P1 |
| **Weather** | `BattlePackage.map.weather` | — | Sensor/movement modifiers | P1 |
| **Crew state** | `ShipSnapshot.crew` | `ShipOutcome.crew_casualties` | Morale, skills, fatigue | P1 |
| **Supply state** | `ShipSnapshot.supply` | `ShipOutcome.consumption` | Ammo limits, crew penalties | P2 |
| **Ship damage** | `ShipSnapshot.damage_state` | `ShipOutcome.damage_report` | Persistent component/compartment damage | P2 |
| **Leadership** | `ShipSnapshot.leadership` | — | Captain competence affects crew | P2 |
| **Siege state** | `ShipSnapshot.siege_state` | `BoardingOutcome` | Active boarding on megaships | P3 |

**Contract Versioning**:
- **arena.v1** (MVP): Weather, crew, faction morale. P2/P3 fields are `null`/default.
- **arena.v2** (P2): Adds supply, damage_state, leadership. Backward-compatible—v1 packages work.
- **arena.v3** (P3): Adds siege_state, full boarding. Backward-compatible.

See [contracts.md](../technical/contracts.md) for implementation-grade schemas. Design extensions in this document may not yet be reflected in contracts.md.

See related documents:
- [factions.md](factions.md) — Faction morale system
- [damage-and-boarding.md](damage-and-boarding.md) — Damage tiers, boarding, surrender
- [economy.md](economy.md) — Supply and resource system
- [people.md](people.md) — Leadership and competence
- [weather.md](weather.md) — Weather effects

### BattlePackage (Input)

```
schema_version: string (e.g., "arena.v1")
battle_id: string
seed: integer
torch_seed: integer (optional, for DRL policy sampling)
time_step_s: float (e.g., 0.1)           // Physics substep, NOT tactical tick
time_limit_s: float

teams: [TeamContext]

map:
  bounds: {min_x, min_y, max_x, max_y}
  obstacles: [{shape, material}]
  zones: [{area, modifier_type, intensity}]
  currents: {grid definition or analytic parameters}
  weather: {wind, wave_state, visibility, lightning}
  layers: {surface, submerged definitions}

ships: [ShipSnapshot]
```

**Time Model**: The arena uses two time scales:
- **Substep** (`time_step_s`): Physics integration step, typically 0.1s. Used for movement, collision, projectile simulation.
- **Tactical tick**: 1 second. Game mechanics (cooldowns, durations, DRL observations/actions) reference tactical ticks. Each tactical tick contains `1.0 / time_step_s` substeps.

See [glossary](../vision/glossary.md#time--ticks) for canonical definitions.

**TeamContext**:
```
team_id: string
name: string
faction_context: FactionContext (from factions.md)
```

Where `FactionContext` provides strategic layer context:
```
FactionContext {
    faction_id:     string
    philosophy:     Philosophy      // Affects AI behavior
    tech_tags:      [string]        // Affects ship capabilities
    morale_state:   0.0–1.0         // Faction-wide morale (affects surrender)
    at_war_with:    [string]        // IFF context
}
```

See [factions.md](factions.md) for full definition.

**ShipSnapshot**:
```
ship_id: string (stable ID from main sim)
team_id: string
hull: {mass, radius, max_speed, turn_rate, ...}
capabilities: {can_surface, can_submerge, max_depth_m, dive_rate}
sensors: [{type, range, noise_model}]
weapons: [{type, cooldown, ammo, projectile_params}]
crew: CrewSnapshot
initial_state: ShipState

// P2+ extensions (null/default in arena.v1)
defenses: {armor, shields}?              // P2: damage reduction
leadership: LeadershipContext?           // P2: captain competence
supply: SupplyContext?                   // P2: supply effects on performance
damage_state: DamageReport?               // P2: persistent Tier 1/2 damage
siege_state: SiegeState?                 // P3: megaships under active boarding

extras: {}                               // Reserved for future expansion
```

**ShipState** (initial position and basic condition):
```
x, y: float
heading: float (radians)
speed: float
depth_state: SURFACE | SUBMERGED | TRANSITIONING
hp: float (for Tier 0) or null (use damage_state for Tier 1/2)
ammo: {weapon_type: count}
```

**DamageReport** (for Tier 1/2 ships, persists between battles):
```
damage_tier: 0 | 1 | 2
# Tier 1: Component damage
components: [{component_id, status: OPERATIONAL|DAMAGED|DESTROYED, health: 0.0-1.0}]
# Tier 2: Compartment damage (megaships)
compartments: [{compartment_id, hp, breached, flooded, on_fire, crew_count}]
district_damage: {district_id: 0.0-1.0}  # For arcologies
```

**SiegeState** (for megaships with active boarding):
```
under_siege: boolean
beachhead_established: boolean
attacker_troops: integer
breach_quality: 0.0-1.0
entry_points: [string]
siege_duration_ticks: integer
contested_systems: [string]           # e.g., ["engineering", "cic", "weapons_port"]
```

**Siege Effects on Combat**: A besieged megaship suffers penalties based on contested systems:

| Contested System | Combat Effect |
|------------------|---------------|
| Engineering | Reduced max speed, slower repairs, possible immobilization |
| CIC | Slower reaction time, reduced sensor effectiveness |
| Weapons (sector) | That weapon arc disabled or degraded |
| Power | Rolling blackouts, system failures |
| Flight Deck | No launch/recovery operations |

If attackers control engineering outright, the megaship cannot maneuver—even in an external battle against a third party. The arena applies these penalties automatically based on `contested_systems`.

**Tactical Implications**:
- **Immobile fortress**: A besieged megaship with working weapons but contested engineering is dangerous but static. Enemies can kite it, avoid firing arcs, or ignore it entirely if objectives lie elsewhere.
- **Multi-faction boarding**: A third party can insert troops onto an already-besieged megaship, creating a three-way internal skirmish. The `SiegeState` tracks multiple attacker factions:
  ```
  attacker_forces: [{faction_id, troops, entry_points, controlled_systems}]
  ```
- **Opportunistic capture**: If Faction A has softened up a megaship and Faction B arrives, B might race to insert troops and steal the prize—or negotiate a split.

This creates meaningful tactical decisions beyond "shoot until dead."

**SupplyContext** (affects combat performance):
```
days_of_supply: float
supply_quality: 0.0-1.0              # Degraded supplies = penalties
ammo_reserves: {weapon_type: count}  # Total available, not just loaded
critical_shortages: [ResourceType]   # Food, water, fuel shortages
```

**LeadershipContext** (from people.md, affects crew performance):
```
captain_id: string (optional, for named captains)
command_competence: 0.0-1.0          # Affects tactical decisions
operations_competence: 0.0-1.0       # Affects maneuver efficiency
traits: [string]                     # e.g., "reckless", "cautious"
```

**CrewSnapshot**:
```
crew_count: integer
gunnery: 0.0–1.0
engineering: 0.0–1.0
morale: 0.0–1.0
fatigue: 0.0–1.0 (optional)
extras: {}
```

**Morale Calculation**: Ship morale in `CrewSnapshot` is computed by the strategic layer before battle:

```
ship_morale = faction_morale_state        // From FactionContext
            × supply_modifier             // Days of supply, quality
            × ship_condition_modifier     // Recent damage, repairs
            × leadership_modifier         // Captain competence, traits
            × recent_events_modifier      // Victories, defeats, casualties
```

This pre-computed morale affects in-battle behaviors (reaction latency, accuracy under fire, surrender threshold). See [damage-and-boarding.md](damage-and-boarding.md) for surrender mechanics.

### BattleResult (Output)

```
schema_version: string
battle_id: string
seed: integer (echo for auditability)
torch_seed: integer (if provided)
winner_team_id: string (or null for draw/timeout)
duration_s: float

ships: [ShipOutcome]
teams: [TeamOutcome]
event_summary: {hits, kills, transitions, hazard_damage, surrenders}

replay:
  seed: integer
  trace_version: string
  action_log_ref: string
  action_log_hash: string (optional, for integrity verification)
  initial_state_hash: string (optional, for debugging divergence)
```

**ShipOutcome**:
```
ship_id: string
final_state: ShipState                   // Position, depth, basic HP (Tier 0)
final_morale: 0.0–1.0                    // Morale at battle end (for strategic layer update)
fate: ShipFate                           // OPERATIONAL | DISABLED | DESTROYED | SCUTTLED | CAPTURED
capture_method: CaptureMethod?           // Only if fate=CAPTURED: BOARDED | SURRENDERED
consumption: {ammo_used, fuel_used}
crew_casualties: {killed, wounded}

// P2+ extensions (null/default in arena.v1)
damage_report: DamageReport?              // Component/compartment state (Tier 1/2)
boarding_outcome: BoardingOutcome?       // If boarding occurred
```

**CaptureMethod**: How a ship was captured (only present if `fate = CAPTURED`):
- `BOARDED` — Taken by force via boarding action
- `SURRENDERED` — Crew surrendered due to morale collapse

For megaships (Class XL), the strategic layer also receives `MegashipDamageReport` with district-level damage, population casualties, and siege implications. See [damage-and-boarding.md](damage-and-boarding.md) for the full contract.

**TeamOutcome** (per team summary):
```
team_id: string
morale_delta: float                      // Change to apply to faction morale (-0.3 to +0.2)
ships_lost: integer
ships_surrendered: integer
decisive_victory: boolean                // Affects morale impact
```

The strategic layer uses `TeamOutcome.morale_delta` to update `FactionState.morale` after battles. Decisive victories boost morale; catastrophic losses (especially surrenders) cause morale shock.

## DRL Integration

### Environment API

Gymnasium/PettingZoo-style interface:

```python
reset(battle_package) → observations
step(actions) → observations, rewards, terminated, truncated, info
```

**Multi-agent mapping**: `agent_id → observation/action/reward`
**Action masking**: Cannot fire during cooldown, cannot dive if incapable
**Headless mode**: No rendering for fast rollouts

### Observation Space (MVP)

Per-agent observation includes:
- Own ship: position, velocity, heading, depth_state, hp, cooldowns, ammo
- Contacts: Up to N nearest (relative bearing, range, closing speed, depth_state if known)
- Environment: Current vector, hazard severity, visibility
- Context: Team ID, time remaining

### Action Space (MVP)

Hybrid continuous/discrete:
- Continuous: `throttle [-1..1]`, `turn_rate [-1..1]`
- Discrete: `fire_primary`, `fire_torpedo`, `surface`, `submerge`

### Reward Structure (MVP)

Combine sparse and shaped:
- Terminal: Win/loss/draw
- Shaped: Damage dealt − damage taken, survival bonus, time penalty
- Penalties: Hazardous zones, wasted ammo, collisions

### Reproducibility

Every episode logs:
- Seed and scenario parameters
- Action/state trace sufficient for replay
- Deterministic replay for debugging training regressions

## Step Loop

```
1. Environment   Update currents, weather, hazard damage
2. Sensors       Detect contacts, update track tables
3. Controllers   Collect scripted/human/DRL action inputs
4. Movement      Integrate velocity, handle collisions and transitions
5. Weapons       Process fire commands, spawn projectiles
6. Damage        Resolve hits, apply crew modifiers
7. Termination   Check win/loss/timeout conditions
8. Telemetry     Log events and metrics
```

All steps are deterministic given seed and inputs.

## Extensibility (Post-MVP)

- Additional depth bands (shallow, abyssal) with crush depth modeling
- Carriers and launched craft
- ECM/ECCM and sensor occlusion
- Component damage model with repair mechanics
- Objective-based battles (convoy escort, capture points)

## MVP Deliverables

- [ ] BattlePackage/BattleResult schema (arena.v1) with validation
- [ ] Deterministic step loop with headless mode
- [ ] Two ship archetypes, two weapon types
- [ ] Terrain: Obstacles, currents, hazardous zones
- [ ] Multi-agent DRL wrapper with scripted baselines
- [ ] Telemetry and replay hooks
