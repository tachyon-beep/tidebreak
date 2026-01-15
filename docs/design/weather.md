# Weather Design

Weather affects surface combat, sensors, and layer transitions. It creates tactical windows, forces adaptation, and makes the ocean feel alive.

**Core Principle**: Weather is an environmental modifier that creates interesting decisions, not a simulation of meteorology. It should be predictable enough to plan around, variable enough to disrupt plans.

## Canonical Types Used

- **WeatherState**: Current conditions at a location
- **WeatherCell**: Discrete weather zone with properties
- **StormSystem**: Moving weather pattern
- **SeaState**: Wave conditions affecting surface operations
- **Visibility**: Detection range modifier

## Goals

- Weather creates tactical opportunities (storm-masked attacks, calm-weather engagement windows)
- Weather affects sensors differentially (radar degraded, sonar less affected)
- Weather affects surface layer primarily, submerged less
- Weather is deterministic and seedable for replay/DRL

## Non-Goals

- Realistic meteorological simulation
- Climate modeling
- Long-term weather prediction systems
- Weather manipulation mechanics

## Weather Model

### Weather State

Each location has a weather state:

```rust
WeatherState {
    // Precipitation
    precipitation:      PrecipitationType   // None, Rain, Storm
    precipitation_intensity: f32            // 0.0-1.0

    // Wind
    wind_speed:         f32                 // m/s
    wind_direction:     f32                 // radians

    // Waves
    sea_state:          SeaState            // 0-9 scale
    wave_height:        f32                 // meters
    wave_period:        f32                 // seconds

    // Visibility
    visibility:         f32                 // km, 0.0-20.0+
    fog_density:        f32                 // 0.0-1.0

    // Special
    lightning_risk:     f32                 // 0.0-1.0
    magnetic_anomaly:   f32                 // 0.0-1.0 (for "blueout" events)
}
```

### Sea State Scale

Simplified Douglas Sea Scale:

| Sea State | Wave Height | Surface Effects |
|-----------|-------------|-----------------|
| 0-1 | < 0.5m | Calm. Full sensor effectiveness. |
| 2-3 | 0.5-1.5m | Slight. Minor penalties. |
| 4-5 | 1.5-4m | Moderate. Radar degraded, small craft penalty. |
| 6-7 | 4-9m | Rough. Major radar/visual degradation, small craft danger. |
| 8-9 | > 9m | Severe. Surface combat nearly impossible, small craft destroyed. |

### Precipitation Types

```rust
enum PrecipitationType {
    None,
    LightRain,      // Minor visibility reduction
    HeavyRain,      // Major visibility reduction, radar clutter
    Storm,          // All sensors degraded, hazard damage
}
```

## Weather Effects

### Surface Layer Effects

| Condition | Effect |
|-----------|--------|
| High sea state | Movement penalty, accuracy penalty, small craft damage |
| Low visibility | Visual sensor range reduced |
| Heavy rain | Radar clutter, increased false contacts |
| Storm | All sensors degraded, hull stress damage |
| Lightning | Electronics damage risk, EMP-like effects |

### Submerged Layer Effects

| Condition | Effect |
|-----------|--------|
| Surface storm | Sonar noise increased (wave action) |
| Thermocline disruption | Sonar propagation changed |
| Surface currents | Affects transition timing |

Submerged layer is largely sheltered from surface weather—one reason to dive.

### Layer Transition Effects

| Condition | Effect |
|-----------|--------|
| High sea state | Transition takes longer, higher failure risk |
| Storm | Transition dangerous, possible damage |
| Calm | Transition easier, faster |

### Sensor-Specific Effects

From sensors-and-fog.md, weather affects sensor modalities differently:

| Sensor | Weather Sensitivity |
|--------|---------------------|
| **Radar** | High. Rain clutter, sea return, reduced range in storms |
| **Visual** | High. Fog, rain, spray all reduce range |
| **Sonar** | Low-Medium. Surface noise affects passive; active less affected |
| **ESM/RF** | Medium. Lightning interference, propagation effects |

## Weather Patterns

### Static Zones

Some areas have persistent weather characteristics:

```rust
WeatherZone {
    zone_id:        ZoneId
    bounds:         Polygon
    base_state:     WeatherState        // Default conditions
    variability:    f32                 // How much it fluctuates
    seasonal:       Option<SeasonalPattern>
}
```

Examples:

- **Calm belts**: Consistently low sea state, good for trade routes
- **Storm belts**: Persistent rough weather, dangerous transit
- **Fog banks**: Low visibility zones near cold currents

### Moving Systems

Storms and fronts move across the map:

```rust
StormSystem {
    system_id:      SystemId
    center:         Position
    velocity:       Vector2
    radius:         f32
    intensity:      f32                 // Peak severity
    profile:        StormProfile        // Eye, gradient, etc.
    lifetime:       Tick                // When it dissipates
}

enum StormProfile {
    Uniform,            // Same intensity throughout
    Gradient,           // Strongest at center
    Cyclonic,           // Eye structure with calm center
}
```

### Weather Forecasting

Players can observe weather patterns:

```rust
WeatherForecast {
    location:       Position
    time_horizon:   Tick                // How far ahead
    confidence:     f32                 // 0.0-1.0, decreases with time
    predicted:      WeatherState
}
```

Forecasting requires:

- Sensor platforms with meteorological equipment
- TDM connectivity to share data
- Time for pattern analysis

Without forecasting, players only see current conditions.

## Tactical Weather Use

### Storm-Masked Operations

From system-interactions.md, weather creates tactical windows:

```text
Trigger: Storm enters battle area
→ Surface sensors degraded (weather → sensors)
→ Surface ships can't track submerged contacts (sensors → tracks)
→ Sub approaches undetected (tracks → positioning)
→ Sub executes pop-up strike (layers → weapons)
→ Sub re-submerges in storm cover (weather → escape)
```

### Weather Windows

Phases of combat advantage:

| Weather Transition | Advantage For |
|--------------------|---------------|
| Storm arriving | Submarines (surface sensors degrading) |
| Storm peak | Neither (all combat dangerous) |
| Storm clearing | Surface (regaining sensor advantage) |
| Calm period | Surface-dominant (full sensor effectiveness) |

### Fleet Weather Doctrine

Different fleet compositions favor different weather:

| Fleet Type | Preferred Weather | Reason |
|------------|-------------------|--------|
| Surface-heavy | Calm to moderate | Full sensor advantage |
| Sub-heavy | Moderate to rough | Surface sensors degraded |
| Mixed | Variable | Can adapt |
| Small craft | Calm only | High sea state is deadly |

## Weather Hazards

### Direct Damage

Severe weather can damage ships:

```rust
WeatherDamage {
    threshold:      SeaState            // When damage starts
    damage_rate:    f32                 // Per tick above threshold
    damage_type:    DamageType          // Hull stress, flooding
    affected_class: Vec<SizeClass>      // Small ships more vulnerable
}
```

| Sea State | Effect on Size Class |
|-----------|----------------------|
| 6-7 | Small craft take damage |
| 8-9 | Small craft destroyed, medium ships take damage |
| 9+ | All surface ships take damage |

### Lightning Strikes

In storms with lightning risk:

```rust
LightningEvent {
    probability:    f32                 // Per tick
    damage:         f32                 // Electronics damage
    emp_effect:     bool                // Temporary sensor blackout
}
```

### Special Events: Blueout

Setting-specific magnetic storms:

```rust
BlueoutEvent {
    center:         Position
    radius:         f32
    intensity:      f32
    effects:
      - All electronics degraded
      - Navigation systems unreliable
      - Communications disrupted
      - InfoIntegrity drops (governance effect)
}
```

Blueouts are rare but devastating—they can isolate arcologies and disrupt fleets.

## Weather in Combat Arena

### BattlePackage Weather

Weather is part of the map definition:

```yaml
map:
  weather:
    initial_state: WeatherState
    zones: [WeatherZone]
    systems: [StormSystem]      # Optional moving weather
    forecast_available: bool
```

### Weather Update System

In the combat arena step loop:

```yaml
step_order:
  1. Environment:
     - Update storm system positions
     - Calculate local weather per cell
     - Apply weather damage to exposed ships
  2. Sensors:
     - Apply weather modifiers to detection
     # ... rest of loop
```

### Observation Space

DRL agents observe weather:

```yaml
observation.environment:
  local_weather:
    sea_state: int
    visibility: float
    precipitation: enum
    wind_vector: (float, float)
  nearby_storms:
    - direction: float
    - distance: float
    - intensity: float
```

## Data Contracts

### WeatherState Component

```rust
WeatherState {
    // See above for full definition
}
```

### WeatherUpdate Output

```rust
WeatherUpdate {
    location:       Position
    old_state:      WeatherState
    new_state:      WeatherState
    cause:          WeatherCause    // StormArrival, StormDeparture, Natural
}
```

### WeatherDamageEvent

```rust
WeatherDamageEvent {
    target:         EntityId
    damage:         f32
    damage_type:    DamageType
    weather_cause:  WeatherState
    trace_id:       TraceId
}
```

## Plugins and Resolvers

### Weather Plugins

```yaml
WeatherEvolutionPlugin:
  reads: [WeatherZones, StormSystems, Tick]
  emits: [WeatherUpdate]
  behavior: "Advance storm positions, calculate local conditions"

WeatherDamagePlugin:
  reads: [WeatherState, ShipState, Position]
  emits: [WeatherDamageEvent]
  behavior: "Apply weather damage to exposed ships"

WeatherForecastPlugin:
  reads: [WeatherState, StormSystems, SensorState]
  emits: [WeatherForecast]
  behavior: "Generate forecasts for ships with meteorological capability"
```

### Weather Resolver

The `WeatherResolver` handles:

- Applying weather updates to zones
- Processing weather damage events
- Updating sensor modifiers for weather conditions

## MVP Staging

### P1 (Core Weather)

- [ ] WeatherState with sea state, visibility, precipitation
- [ ] Static weather zones
- [ ] Weather effects on sensors (modifiers)
- [ ] Weather effects on small craft (damage)
- [ ] Weather in BattlePackage/observation space

### P2 (Dynamic Weather)

- [ ] Moving storm systems
- [ ] Weather forecasting
- [ ] Lightning effects
- [ ] Layer transition weather effects

### P3 (Full Weather)

- [ ] Blueout events
- [ ] Seasonal patterns
- [ ] Weather affecting TDM (InfoIntegrity)
- [ ] Strategic weather (affects trade routes, blockades)

## Related Documents

- [Layers and Terrain](layers-and-terrain.md) — Layer-weather interactions
- [Sensors and Fog](sensors-and-fog.md) — Weather effects on detection
- [Combat Arena](combat-arena.md) — Weather in battle simulation
- [System Interactions](system-interactions.md) — Weather in cascade chains
- [World Requirements](../requirements/world.md) — Weather requirements
- [Glossary](../vision/glossary.md) — Canonical terminology
