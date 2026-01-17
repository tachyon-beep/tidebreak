# State Component Contracts

This document defines the canonical state component schemas for Tidebreak. All components are part of the Shared Contract and must be serializable, deterministic, and well-typed.

## Schema Conventions

### Field Types

| Type | Description | Serialization |
|------|-------------|---------------|
| `EntityId` | Unique entity identifier | u64 |
| `PersonId` | Stable person identifier (see people.md) | String |
| `Quantity` | Non-negative amount | f64, >= 0 |
| `Ratio` | Bounded 0.0-1.0 | f64, clamped |
| `Modifier` | Multiplicative factor | f64, >= 0 |
| `Position` | 2D coordinate | (f64, f64) |
| `Heading` | Radians, CCW from +X | f64, normalized to [0, 2π) |
| `Layer` | Depth layer | enum { Surface, Submerged, Abyssal } |

### Time Model

Tidebreak uses three time scales:

| Type | Real-Time Equivalent | Used In | Serialization |
|------|---------------------|---------|---------------|
| `StrategicTick` | 1 day | Campaign layer, governance, economy | u64 |
| `TacticalTick` | 1 second | Combat arena, cooldowns, DRL observations | u64 |
| `Substep` | 0.1 seconds (configurable) | Physics integration within arena | f64 |

**Usage rules**:
- Components in `docs/technical/contracts.md` default to `StrategicTick` unless noted
- Components used in combat arena (CombatState, WeaponState, SensorState) use `TacticalTick`
- `BattlePackage.time_step_s` defines the substep duration; tactical tick = 1.0s always
- Duration fields inherit the tick type of their containing component

**Duration types**:

| Type | Description | Context |
|------|-------------|---------|
| `StrategicDuration` | Tick count in strategic ticks | Governance decisions, treaties |
| `TacticalDuration` | Tick count in tactical ticks | Cooldowns, transitions, combat timers |

**Examples**:
- `GovernanceState.decision_latency`: `StrategicDuration` (days to make a decision)
- `WeaponState.ready_at`: `TacticalTick` (when weapon can fire again)
- `LayerState.transition_end`: `TacticalTick` (when dive/surface completes)

### Mutation Rules

Each component specifies:
- **Owner**: Which resolver(s) may write to it
- **Observers**: Which plugins may read it
- **Mutation**: Which output types can affect it

---

## Combat Components

### CombatState

Primary combat status for ships and platforms.

```
CombatState {
    posture:        CombatPosture       # Aggressive, Defensive, Evasive, Withdrawn
    engagement:     EngagementStatus    # Engaged, Disengaging, Clear
    heat:           Ratio               # Weapon/system thermal load
    stress:         Ratio               # Crew combat stress
    suppression:    Ratio               # Incoming fire suppression effect
    cooldowns:      Map<WeaponSlot, TacticalTick>  # Per-weapon ready time
}
```

**Owner**: CombatResolver
**Observers**: WeaponPlugin, TacticsPlugin, AIControllerPlugin
**Mutation**: `FireWeapon` (cooldowns), `ApplyModifier` (heat, stress, suppression)

### WeaponState

Per-weapon status (component per slot or nested in CombatState).

```
WeaponState {
    slot:           WeaponSlot
    weapon_type:    WeaponTypeId
    ammunition:     Quantity
    condition:      Ratio               # Degradation
    ready_at:       TacticalTick        # Next available tick
    target_lock:    EntityId?           # Current tracking target
}
```

**Owner**: CombatResolver
**Mutation**: `FireWeapon` (ammunition, ready_at), `ApplyModifier` (condition)

---

## Movement Components

### MovementState

Position and velocity for mobile entities.

```
MovementState {
    position:       Position
    velocity:       (f64, f64)          # m/s
    heading:        Heading
    throttle:       Ratio               # Current engine power
    turn_rate:      f64                 # rad/s, current
    max_speed:      f64                 # m/s, hull limit
    acceleration:   f64                 # m/s², current capability
}
```

**Owner**: PhysicsResolver
**Observers**: All navigation and tactical plugins
**Mutation**: `SetThrottle`, `SetHeading`, `SetCourse`, `HelmOrder`, `ApplyModifier` (max_speed, acceleration for damage effects)

### LayerState

Current depth layer and transition status.

```
LayerState {
    current:        Layer
    transitioning:  bool
    target:         Layer?              # If transitioning
    transition_start: TacticalTick?     # When transition began
    transition_end:   TacticalTick?     # When transition completes
}
```

**Owner**: PhysicsResolver
**Mutation**: `LayerTransition` output

**Contract**: During transition (`transitioning == true`):
- Entity can be detected by sensors targeting either `current` or `target` layer
- Entity cannot fire weapons
- Entity generates signature spike (see SensorState)

---

## Sensor Components

### SensorState

Sensor suite status and emissions configuration.

```
SensorState {
    emissions_mode: EmissionsMode       # Active, Passive, EMCON
    radar_power:    Ratio               # Radar output level (0 = off)
    sonar_mode:     SonarMode           # Active, Passive, Off
    signature:      SignatureProfile    # Current detectability
    clutter:        Ratio               # Local environmental noise
    jamming_recv:   Ratio               # Incoming jamming pressure
}

EmissionsMode = Active | Passive | EMCON

SignatureProfile {
    radar_cross:    f64                 # Radar cross-section
    acoustic:       f64                 # Acoustic signature
    thermal:        f64                 # IR signature
    wake:           f64                 # Visual/radar wake
}
```

**Owner**: SensorResolver
**Observers**: DetectionPlugin, EWPlugin, StealthPlugin
**Mutation**: `ApplyModifier`, sensor configuration outputs

### TrackTableState

Fused contact information maintained by an entity.

```
TrackTableState {
    tracks:         Map<TrackId, Track>
    max_tracks:     u32                 # Capacity limit
    last_fusion:    TacticalTick        # When tracks were last fused
}

Track {
    id:             TrackId
    target:         EntityId?           # Null if unidentified
    position:       Position            # Estimated
    velocity:       (f64, f64)?         # Estimated, null if unknown
    uncertainty:    f64                 # Position error radius
    quality:        TrackQuality        # Q0-Q3
    classification: Classification      # Unknown, Friendly, Hostile, Neutral
    iff_confidence: Ratio               # How sure of classification
    age:            TacticalTick        # Ticks since last update
    source:         Set<SensorType>     # Contributing sensors
    layer:          Layer?              # Detected layer, null if unknown
}

TrackQuality = Q0 | Q1 | Q2 | Q3
# Q0: Bearing-only cue
# Q1: Position estimate, not fire-control
# Q2: Fire-control quality, local engagement
# Q3: Shareable for remote engagement
```

**Owner**: SensorResolver
**Observers**: TargetingPlugin, TacticsPlugin, AIControllerPlugin
**Mutation**: Detection outputs from SensorPlugin

---

## Inventory Components

### InventoryState

Consumable resources carried by an entity.

```
InventoryState {
    fuel:           Quantity            # Propulsion fuel
    ammunition:     Map<AmmoType, Quantity>
    spares:         Quantity            # Repair materials
    food:           Quantity            # Crew sustenance
    water:          Quantity            # Crew sustenance
    cargo:          Map<CargoType, Quantity>  # Trade goods, salvage
    capacity:       Map<ResourceType, Quantity>  # Max storage
}
```

**Owner**: LogisticsResolver
**Observers**: SupplyPlugin, ProductionPlugin, TradePlugin
**Mutation**: Consumption outputs, transfer outputs, `ApplyModifier`

### ReadinessState

Maintenance and operational readiness.

```
ReadinessState {
    maintenance_debt: Ratio             # Accumulated deferred maintenance
    reliability:    Modifier            # Failure probability multiplier
    fatigue:        Ratio               # Crew fatigue
    morale:         Ratio               # Crew morale
    efficiency:     Modifier            # Overall operational efficiency
}
```

**Owner**: ReadinessResolver
**Observers**: All operational plugins
**Mutation**: `ApplyModifier`, repair outputs

---

## Governance Components

### GovernanceState

Political system of a faction or arcology.

```
GovernanceState {
    government_type:    GovernmentType
    constitution_id:    ConstitutionId  # Ruleset for decisions
    decision_queue:     List<QueuedDecision>
    decision_latency:   StrategicDuration   # Days per decision cycle
    last_decision:      StrategicTick
    succession:         SuccessionRule
    leader:             EntityId?       # Current leader entity
}

GovernmentType = Autocracy | Oligarchy | DirectDemocracy |
                 RepresentativeDemocracy | CorporateMeritocracy |
                 MilitaryJunta | Theocracy | Anarchy

QueuedDecision {
    type:           DecisionType
    params:         DecisionParams
    queued_at:      StrategicTick
    sponsor:        EntityId            # Who proposed it
    urgency:        Priority
}
```

**Owner**: GovernanceResolver
**Observers**: DecisionPlugin, LegitimacyPlugin, FactionPlugin
**Mutation**: `QueueDecision`, government transition events

### CivicsState

Legitimacy and political capital.

```
CivicsState {
    legitimacy:         Ratio           # Government authority
    political_capital:  Quantity        # Ability to push unpopular decisions
    compliance:         Ratio           # Population willingness to follow
    stability:          Ratio           # Resistance to upheaval
    dissent:            Ratio           # Active opposition level
}
```

**Owner**: GovernanceResolver
**Observers**: All governance plugins
**Mutation**: `ApplyModifier`, crisis events

---

## Population Components

### PopulationState

Demographics for arcologies and enclaves.

```
PopulationState {
    total:              u32                 # Total population
    growth_rate:        f64                 # Per-tick growth
    casualties:         u32                 # Recent losses
    refugees:           u32                 # Incoming/outgoing
    density:            f64                 # Population per unit area

    # P2+: Demographic distribution (see governance.md for derivation)
    vocational_makeup:  Map<VocationType, Ratio>?   # What people do (sums to 1.0)
    ideological_makeup: Map<IdeologyType, Ratio>?   # What people believe (sums to 1.0)
    demographic_blocks: List<DemographicBlock>?     # Cross-tabulation for key combos
}

VocationType = Engineering | Agriculture | Logistics | Security | Administration | Medical | Skilled | Unskilled

IdeologyType = Militarist | Trader | Populist | Technocrat | Traditionalist | Expansionist | Isolationist

DemographicBlock {
    vocation:       VocationType
    ideology:       IdeologyType
    fraction:       Ratio               # % of total population in this block
}
```

**Owner**: PopulationResolver
**Observers**: EconomyPlugin, MoralePlugin, GovernancePlugin, FactionPlugin
**Mutation**: Population events, `ApplyModifier`, demographic drift

### InternalFactionsState

Political factions within an arcology. Internal factions have two orthogonal axes: vocational (what you do) and ideological (what you believe).

```
InternalFactionsState {
    factions:       Map<FactionId, InternalFaction>
}

InternalFaction {
    id:                     FactionId
    name:                   String
    faction_axis:           FactionAxis     # Ideological or Vocational

    # Derived from PopulationState demographics
    population_share:       Ratio           # % of population in this faction
    structural_multiplier:  Modifier        # Vocational leverage (1.0-2.0)
    effective_influence:    Ratio           # population_share × structural_multiplier, normalized

    # Faction sentiment
    goals:                  Set<FactionGoal>    # Expansion, Isolation, Reform, etc.
    satisfaction:           Ratio               # With current government
    radicalization:         Ratio               # Willingness to act outside system

    # Leadership
    leader:                 PersonId?           # Named leader (see people.md)
}

FactionAxis = Ideological | Vocational
# Ideological: Influence = population % (no leverage bonus)
# Vocational: Influence = population % × structural leverage (control critical systems)

FactionGoal = Expansion | Isolation | Reform | Stability | Recognition | Autonomy | Resources
```

**Owner**: FactionResolver
**Observers**: GovernancePlugin, PropagandaPlugin, InfluencePlugin
**Mutation**: `ApplyModifier`, influence operations, demographic drift

---

## Diplomacy Components

### DiplomacyState

External relations for factions.

```
DiplomacyState {
    relations:      Map<EntityId, Relation>
    treaties:       Map<TreatyId, Treaty>
    reputation:     Map<ReputationType, Ratio>
    council_seats:  Map<CouncilId, SeatInfo>
}

Relation {
    target:         EntityId
    standing:       f64                 # -1.0 (hostile) to +1.0 (allied)
    trust:          Ratio
    fear:           Ratio
    trade_value:    Quantity            # Economic interdependence
    last_contact:   StrategicTick
}

Treaty {
    id:             TreatyId
    type:           TreatyType          # Alliance, NonAggression, Trade, etc.
    parties:        Set<EntityId>
    terms:          TreatyTerms
    expires:        StrategicTick?
    enforcement:    EnforcementMechanism
}
```

**Owner**: DiplomacyResolver
**Observers**: DiplomacyPlugin, TradePlugin, WarPlugin
**Mutation**: Diplomatic outputs, treaty events

---

## Environment Components

### EnvironmentState

Local environmental conditions for an entity.

```
EnvironmentState {
    layer:          Layer               # Current strategic state
    weather:        WeatherCondition
    sea_state:      SeaState            # Wave height category
    visibility:     Ratio               # Visual range modifier
    current:        (f64, f64)          # Local water current vector
    hazards:        Set<HazardType>     # Nearby hazards
    terrain_type:   TerrainType         # DeepOcean, Shelf, Shallows, etc.
}

WeatherCondition {
    precipitation:  PrecipitationType   # None, Rain, Storm
    wind_speed:     f64                 # m/s
    wind_direction: Heading
    fog:            bool
}
```

**Owner**: EnvironmentResolver
**Observers**: All plugins (environmental factors affect most systems)
**Mutation**: Weather system outputs, time progression

---

## Communication Components

### CommsState

Communication system status.

```
CommsState {
    links:          Map<EntityId, CommLink>
    bandwidth_used: Quantity
    bandwidth_max:  Quantity
    latency_base:   TacticalDuration    # Minimum message delay (tactical ticks)
    jamming_out:    Ratio               # Outgoing jamming power
    mesh_connected: bool                # Part of tactical data mesh
}

CommLink {
    target:         EntityId
    quality:        Ratio               # Signal quality
    bandwidth:      Quantity            # Available throughput
    latency:        TacticalDuration    # Round-trip ticks
    encrypted:      bool
    last_contact:   TacticalTick
}
```

**Owner**: CommsResolver
**Observers**: TDMPlugin, EWPlugin, CommandPlugin
**Mutation**: Comm outputs, jamming effects

---

## Battle Contracts

Cross-boundary contracts between the Full Simulation and Combat Arena.

### Battle Supporting Types

Types used by BattlePackage and BattleResult.

```
Bounds {
    min_x:      f64
    min_y:      f64
    max_x:      f64
    max_y:      f64
}

Obstacle {
    shape:      Shape               # Circle, Polygon
    position:   Position
    material:   ObstacleMaterial    # Solid, Soft (sensor interference)
}

Shape = Circle { radius: f64 }
      | Polygon { vertices: List<Position> }

ObstacleMaterial = Solid | Soft { intensity: Ratio }

MapZone {
    area:           Shape
    position:       Position
    modifier_type:  ZoneModifier
    intensity:      Ratio
}

ZoneModifier = SensorInterference | Hazard | Current | SpeedBonus

CurrentField = Grid { cells: List<List<(f64, f64)>>, cell_size: f64 }
             | Analytic { base_vector: (f64, f64), variance: f64 }

LayerConfig {
    surface:    LayerParams
    submerged:  LayerParams
    abyssal:    LayerParams?        # Post-MVP
}

LayerParams {
    sensor_modifiers:   Map<SensorType, Modifier>
    speed_modifier:     Modifier
    visibility:         Ratio
}

HullParams {
    mass:           f64             # kg
    radius:         f64             # m, collision radius
    max_speed:      f64             # m/s
    acceleration:   f64             # m/s²
    turn_rate:      f64             # rad/s
    armor:          Ratio?          # Damage reduction (P2+)
}

Capabilities {
    can_surface:    bool
    can_submerge:   bool
    max_depth_m:    f64
    dive_rate:      f64             # m/s
    surface_rate:   f64             # m/s
}

SensorConfig {
    sensor_type:    SensorType      # Radar, Sonar, Visual, ESM
    range:          f64             # m
    arc:            f64?            # rad, null = 360°
    noise_floor:    f64             # Detection threshold
    update_rate:    TacticalDuration  # Ticks between updates
}

SensorType = Radar | Sonar | Visual | ESM | Thermal

WeaponConfig {
    slot:           WeaponSlot
    weapon_type:    WeaponTypeId
    range:          f64             # m, max effective range
    cooldown:       TacticalDuration  # Ticks between shots
    ammunition:     u32             # Starting ammo
    projectile:     ProjectileParams
}

ProjectileParams {
    speed:          f64             # m/s
    damage:         f64             # HP damage on hit
    guidance:       GuidanceType    # Unguided, Homing, LeadPursuit
    blast_radius:   f64?            # m, for area weapons
}

GuidanceType = Unguided | Homing { turn_rate: f64 } | LeadPursuit { lead_factor: f64 }

ShipState {
    x:              f64
    y:              f64
    heading:        Heading
    speed:          f64
    layer:          Layer
    hp:             f64?            # Tier 0 ships; null for Tier 1/2
    ammo:           Map<WeaponSlot, u32>
}

Consumption {
    ammo_used:      Map<WeaponSlot, u32>
    fuel_used:      f64
}

Casualties {
    killed:         u32
    wounded:        u32
}
```

### BattlePackage (Input)

Configuration and state for a battle instance.

```
BattlePackage {
    schema_version: String              # e.g., "arena.v1"
    battle_id:      String
    seed:           u64
    torch_seed:     u64?                # Optional, for DRL policy sampling
    time_step_s:    f64                 # Physics substep in seconds (e.g., 0.1); tactical tick = 1.0s
    time_limit_s:   f64

    teams:          List<Team>
    map:            MapDefinition
    ships:          List<ShipSnapshot>
}

Team {
    team_id:        String
    name:           String
    faction_context: FactionContext?    # P1+: strategic context
}

FactionContext {
    faction_id:     String
    philosophy:     Philosophy?         # Affects AI behavior
    tech_tags:      List<String>?       # Affects ship capabilities
    morale_state:   Ratio               # Faction-wide morale (0.0-1.0)
    at_war_with:    List<String>?       # IFF context
}

MapDefinition {
    bounds:         Bounds
    obstacles:      List<Obstacle>
    zones:          List<MapZone>       # Area effects (hazards, sensor interference)
    currents:       CurrentField
    weather:        WeatherCondition
    layers:         LayerConfig
}

ShipSnapshot {
    ship_id:        String              # Stable ID from main sim
    team_id:        String
    hull:           HullParams          # mass, radius, max_speed, turn_rate
    capabilities:   Capabilities        # can_surface, can_submerge, max_depth_m
    sensors:        List<SensorConfig>
    weapons:        List<WeaponConfig>
    crew:           CrewSnapshot
    initial_state:  ShipState           # x, y, heading, speed, layer, hp, ammo
}

CrewSnapshot {
    crew_count:     u32
    gunnery:        Ratio               # 0.0–1.0
    engineering:    Ratio
    morale:         Ratio
    fatigue:        Ratio?              # Optional
}
```

**Invariants**:
- All `ship_id` values must be unique within the package
- All `team_id` references must resolve to defined teams
- `seed` and `time_step_s` must be positive

### BattleResult (Output)

Outcome and replay data from a battle.

```
BattleResult {
    schema_version: String
    battle_id:      String
    seed:           u64                 # Echo for auditability
    torch_seed:     u64?
    winner_team_id: String?             # Null for draw/timeout
    duration_s:     f64

    ships:              List<ShipOutcome>
    teams:              List<TeamOutcome>           # P1+: per-team summary
    event_summary:      EventSummary
    replay:             ReplayData

    # Extensions for boarding and megaship battles
    boarding_outcomes:  List<BoardingOutcome>       # Empty if no boarding attempted
    megaship_reports:   List<MegashipDamageReport>  # Empty if no megaships
}

TeamOutcome {
    team_id:            String
    morale_delta:       f64             # Change to apply to faction morale (-0.3 to +0.2)
    ships_lost:         u32
    ships_surrendered:  u32
    decisive_victory:   bool            # Affects morale impact magnitude
}

ShipOutcome {
    ship_id:        String
    final_state:    ShipState           # position, heading, speed, layer, hp, ammo
    consumption:    Consumption         # ammo_used, fuel_used
    crew_casualties: Casualties         # killed, wounded
}

EventSummary {
    hits:           u32
    kills:          u32
    surrenders:     u32
    transitions:    u32
    hazard_damage:  f64
}

ReplayData {
    seed:               u64
    trace_version:      String
    action_log_ref:     String          # Reference to action log
    action_log_hash:    String?         # Optional, for integrity verification
    initial_state_hash: String?         # Optional, for debugging divergence
}
```

**Invariants**:
- `ships` list must match input `BattlePackage.ships` by `ship_id`
- `seed` must match input seed
- `duration_s` must be <= input `time_limit_s`

### ShipOutcome (Extended)

Extended outcome for ships with fate and damage reporting.

```
ShipOutcome {
    ship_id:            String
    fate:               ShipFate            # See below
    final_state:        ShipState
    consumption:        Consumption
    crew_casualties:    Casualties

    # P2+ extensions (null in arena.v1)
    capture_method:     CaptureMethod?      # Only if fate=CAPTURED
    final_morale:       Ratio?              # Morale at battle end (0.0–1.0)
    damage_report:      DamageReport?       # Tier 1+ ships only
    salvage_value:      Ratio?              # Estimated recovery potential (0.0–1.0)
    boarding_outcome:   BoardingOutcome?    # If boarding occurred
}

ShipFate = OPERATIONAL | DISABLED | DESTROYED | SCUTTLED | CAPTURED

CaptureMethod = BOARDED | SURRENDERED
# BOARDED: Taken by force via boarding action
# SURRENDERED: Crew surrendered due to morale collapse

DamageReport {
    components:         Map<ComponentId, ComponentStatus>
    compartments:       Map<CompartmentId, CompartmentState>?  # Tier 2 only
}

ComponentStatus = OPERATIONAL | DAMAGED | DESTROYED
```

### BoardingOutcome

Per-target result for boarding attempts (usually one per arcology).

```
BoardingOutcome {
    target_ship_id:     String
    attacker_team_id:   String
    status:             BoardingStatus

    # Tactical mechanics summary
    attached_duration_s: f64               # Total time docked
    transfer_progress:   Ratio             # 0.0–1.0 progress bar
    troops_inserted:     u32

    # Casualties (arena has limited internal knowledge)
    attacker_casualties: u32
    defender_casualties: u32

    # How hard for defenders to purge in strategic layer
    breach_quality:      Ratio             # 0.0–1.0

    # Localization for strategic layer siege initialization
    entry_point:         String?           # dock node / quadrant / district
}

BoardingStatus = NONE | ATTEMPTED | ESTABLISHED | REPULSED
```

**Invariants**:
- `target_ship_id` must reference a ship in the battle
- `transfer_progress` of 1.0 implies `status: ESTABLISHED`
- `breach_quality` only meaningful when `status: ESTABLISHED`

### MegashipDamageReport

Extended damage reporting for Class XL ships (arcologies, sea cities).

```
MegashipDamageReport {
    ship_id:            String

    # Key system states
    propulsion_status:  ComponentStatus
    cic_status:         ComponentStatus
    primary_weapons:    ComponentStatus
    power_systems:      ComponentStatus

    # District-level damage (percentages)
    district_damage:    Map<String, Ratio>

    # Population impact
    casualty_estimate:  u32
    evacuation_status:  Ratio              # How many evacuated (0.0–1.0)

    # For strategic layer siege initialization
    morale_shock:       Ratio              # Impact on defender morale
    legitimacy_impact:  Ratio              # Impact on government legitimacy
}
```

**Design Note**: The arena returns facts it can measure (component states, casualties, damage percentages). The campaign layer interprets these using governance and morale systems. The arena does not leak campaign semantics like legitimacy calculations—it only provides the inputs.

---

## Schema Versioning

All components include implicit versioning:

```
ComponentEnvelope {
    schema_version: u32
    component_type: ComponentId
    data:           ComponentData       # Type-specific payload
}
```

**Migration Rule**: When loading older schemas:
1. Missing fields use documented defaults
2. Unknown fields are ignored (`extra="ignore"`)
3. Type mismatches fail loudly (no silent coercion)

---

## Validation Requirements

### At Entity Creation

- All required components for entity tags are present
- Component values within documented bounds
- References (EntityId) resolve to valid entities

### At Tick Boundary

- No component violates invariants after resolution
- All cross-references still valid
- Resource quantities non-negative

### At Serialization

- All components serialize without loss
- Private components included with owning entities
- Causal chain metadata preserved
