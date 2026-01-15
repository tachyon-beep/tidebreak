# Layers and Terrain Design

Depth is modeled as discrete **strategic states** rather than continuous 3D position. A unit is always in exactly one layer. Changing layers is a major commitment, not a tactical dodge.

**Important**: Layers are **rule states**, not physical depth ranges. "Surface" means "surface interaction rules apply," not "0-100m depth." The depth descriptions below are illustrative flavor, not simulated oceanography.

## Canonical Types Used

- **LayerState**: SURFACE, SUBMERGED, ABYSSAL, TRANSITIONING
- **Signature**: Modality-specific detectability (radar/sonar/RF/visual)
- **StateBlocker**: Terrain-layer validity rules

## Design Principles

- Layers create distinct tactical domains with different rules
- Transitions are slow and vulnerable—strategic decisions
- Terrain constrains which layers are available
- 2D movement within each layer, with layer-dependent physics

## The Three Layers

### Surface ("The Arena")

The main combat layer where most action occurs.

| Property | Value |
|----------|-------|
| Flavor | Open ocean, wave action |
| Sensors | Radar, visual, communications at full effectiveness |
| Weapons | Full arsenal: railguns, lasers, missiles, torpedoes |
| Vulnerability | All weapon types |

**Characteristics**:
- Maximum sensor range and variety
- Affected by weather (wind, waves, storms)
- Highest top speed
- Vulnerable to air threats

### Submerged ("The Stealth Layer")

The concealment and ambush layer.

| Property | Value |
|----------|-------|
| Flavor | Thermocline / acoustic shadow zone |
| Sensors | Sonar-primary, passive preferred |
| Weapons | Torpedoes, mines, missiles via pop-up |
| Vulnerability | Sonar detection, ASW weapons |

**Targeting Rule**: Ballistic and surface line-of-sight energy weapons cannot target `SUBMERGED` units unless explicitly tagged as sub-capable.

**Characteristics**:
- Stealth-focused operations
- No wind/wave effects
- Slower top speed, better acceleration/braking
- Thermal layers provide additional concealment

**Pop-Up Maneuver**: Submerged units can briefly rise to fire surface-grade weapons (missiles) before retreating. This creates:
- Temporary detection spike
- Brief vulnerability window
- High-impact strike capability for submarines

### Abyssal ("The Flank")

The deep strategic layer for transit and bypass.

| Property | Value |
|----------|-------|
| Flavor | Deep ocean floor / near crush depth |
| Sensors | Minimal—pressure and specialized sonar only |
| Weapons | Specialized deep-pressure ordnance only |
| Vulnerability | Almost none from standard weapons |

**Targeting Rule**: Only specialized deep-pressure ordnance can target `ABYSSAL` units.

**Characteristics**:
- Strategic transit, not combat
- Bypass blockades and contested zones
- Access deep resource nodes and salvage
- Retreat from hopeless surface battles
- Slowest movement, high pressure strain
- Requires specialized hulls

## Layer Transitions

Changing layers is a **strategic commitment**, not instant evasion.

### Timing

| Transition | Duration |
|------------|----------|
| Surface → Submerged | 30–60+ seconds |
| Submerged → Surface | 30–60+ seconds |
| Submerged → Abyssal | 60–90+ seconds |
| Abyssal → Submerged | 60–90+ seconds |

Exact values are tuning parameters. The key: transitions take long enough to be interruptible and punishable.

### The Transition State

During transition, a unit exists in a vulnerable intermediate state.

**Targetability Contract**: While `TRANSITIONING`, a unit may be **detected and targeted by any sensor/weapon that can target either the origin or destination layer**, and it suffers a large **signature penalty**. It cannot fire weapons.

**Vulnerability**:
- Receives damage from **both** origin and destination layers
- Maximum exposure—the worst of both worlds

**Capability**:
- Cannot fire weapons (zero offense)
- Limited maneuvering

**Signature**:
- Generates massive noise/sensor spike
- Breaks stealth completely
- Visible to both layer's sensors

### Interruption

Heavy damage during transition can **botch** the maneuver:

**Possible Outcomes**:
- Forced return to origin layer
- Stuck in transition (continuing vulnerability)
- Catastrophic failure (ballast failure, hull breach)
- Successful completion but with damage

This creates tension: commit to the dive knowing you're vulnerable, or stay and fight?

## Terrain as State Blocker

Since depth is a state, terrain determines which states are valid:

| Terrain Type | Surface | Submerged | Abyssal |
|--------------|---------|-----------|---------|
| Deep Ocean | ✓ | ✓ | ✓ |
| Continental Shelf | ✓ | ✓ | ✗ |
| Shallows/Reef | ✓ | ✗ | ✗ |
| Land/Island | ✗ | ✗ | ✗ |

**Implications**:
- Coastal areas force surface combat
- Deep water enables full tactical flexibility
- Chokepoints can be created by terrain
- Submarines can be "trapped" in shallow regions

## Movement Physics

2D movement within each layer, with layer-dependent parameters:

### Surface Layer

- Affected by wind and wave drag
- Highest top speed
- Weather impacts handling (drift, reduced turn rate in storms)
- Current effects apply

### Submerged Layer

- No wind/wave effects (underwater)
- Slower top speed than surface
- Better acceleration and braking (water resistance)
- Thermocline currents may apply

### Abyssal Layer

- Slowest movement overall
- High pressure strain on systems
- Thermal vent "highways" may provide faster travel lanes
- Crush depth considerations for hull stress

### Common Properties

All layers use Newtonian-ish physics:
- Inertia (momentum carries through maneuvers)
- Turn radius (cannot pivot in place)
- Acceleration/deceleration curves
- Drifting during hard turns

Similar feel to Starsector's movement model, adapted for naval context.

## Integration Notes

### Combat Arena

The arena tracks layer state per ship:
- `depth_state`: SURFACE, SUBMERGED, ABYSSAL, TRANSITIONING
- `transition_progress`: 0.0–1.0 during transitions
- `transition_origin` and `transition_destination`

### Sensors

Layer determines available sensor modalities:
- Surface: Radar + visual + sonar
- Submerged: Sonar only (passive preferred for stealth)
- Abyssal: Specialized deep sensors only

Cross-layer detection uses appropriate modalities with penalties.

### Weapons

Layer determines weapon availability:
- Surface: Full arsenal
- Submerged: Torpedoes, mines, pop-up missiles
- Abyssal: Deep-pressure ordnance only (rare, specialized)

### Engageability is Weapon-Tagged

Layers do not universally "engage each other". Instead, **weapons declare which depth states they can target**.

| Weapon Category | Can Target |
|-----------------|------------|
| Ballistic (guns, railguns) | `SURFACE` only |
| Line-of-sight energy (lasers) | `SURFACE` only (unless tagged sub-capable) |
| ASW weapons (depth charges, ASW torpedoes) | `SUBMERGED`, `TRANSITIONING` |
| Sub-launched torpedoes | `SUBMERGED`, optionally `SURFACE` (with acquisition penalties) |
| Abyssal ordnance (rare) | `ABYSSAL`, `TRANSITIONING` |

This keeps the mental model clean: **layer defines what sensors/weapons are available**, and **weapon tags define what they can hit**.

### Weather

Weather primarily affects surface layer:
- Sensor degradation
- Weapon accuracy penalties
- Movement handling changes
- Hazard damage in extreme conditions

Submerged and Abyssal layers are largely weather-immune.
