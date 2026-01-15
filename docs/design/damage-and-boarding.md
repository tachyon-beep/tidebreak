# Damage and Boarding Design

Damage and boarding scale with ship size. Small ships resolve quickly; large ships have modular damage and multi-phase boarding. Arcology-ships add political dimensions.

## Canonical Types Used

- **DamageTier**: Tier 0 (HP), Tier 1 (components), Tier 2 (compartments)
- **BoardingTier**: Tier 0 (quick), Tier 1+ (multi-phase objectives)
- **BoardingStatus**: NONE, ATTEMPTED, ESTABLISHED, REPULSED
- **Component**: Damageable subsystem (propulsion, sensors, weapons, power)
- **Compartment**: Spatial section with health, state, and occupants
- **ShipFate**: OPERATIONAL, DISABLED, DESTROYED, SCUTTLED, CAPTURED

## Goals

- Damage fidelity scales with ship class
- Boarding with clear counterplay and deterministic resolution
- Partial outcomes (disabled subsystems, contested sections)
- "Disable vs. destroy vs. capture" as meaningful choices
- Arcology boarding as strategic crisis, not quick dice roll

## Non-Goals

- Interior FPS simulation
- High-fidelity blast/flooding physics

## Size Classes

Use **crew/population** as the primary complexity driver:

| Class | Examples | Crew/Pop | Damage Tier | Boarding Tier |
|-------|----------|----------|-------------|---------------|
| **S** | Drones, patrol boats, corvettes | 0–150 | Tier 0 | Tier 0 |
| **M** | Frigates, destroyers, cruisers | 150–1,500 | Tier 1 | Tier 0 or light |
| **L** | Carriers, cargo, fortress rigs | 1,500–10,000 | Tier 2 | Tier 1+ |
| **XL** | Arcology-ships, sea cities | 10,000+ | Tier 2 (districts) | Tier 2 |

Tier is an **authoring choice** with recommended defaults based on crew/population, not a hard physical law. Override per hull for special cases (e.g., a high-value frigate might use Tier 1 boarding despite low crew count).

MVP starts with Tier 0 for all ships.

## Damage Model

### Tier 0: Hull + Status (Small Craft)

- Single `hp` value
- Status flags: `mobility_disabled`, `weapons_disabled`, `sensors_disabled`, `dead_in_water`
- Critical hits toggle flags (seeded) without full component model

### Tier 1: Components (Warships)

Independent component health without interior topology:

| Component | Effect When Damaged |
|-----------|---------------------|
| Propulsion | Speed cap reduced |
| Power | System availability |
| Sensors | Detection range reduced |
| Primary Weapons | Firing disabled/degraded |
| Missile Cells | Ammo/firing affected |
| Flight Deck | Launch/recovery disabled |

Components can be partially repaired by damage control.

### Tier 2: Compartments (Capitals/Arcologies)

Ship as a graph of compartments with dependencies:

**Compartment Properties**:
- `hp`, `breached`, `flooded`, `on_fire`
- Crew count
- Hosted systems (reactor, hangar, CIC, magazines)

**Dependencies**:
- Power distribution
- Command/control chains
- Damage control access
- Bulkhead containment

**Damage Localization**:
- Losing hangar doesn't directly kill reactor
- Power loss cascades to dependent systems
- Bulkheads contain (or fail to contain) fire/flooding

### Damage Types

| Type | Effect |
|------|--------|
| Kinetic/Explosive | HP loss, breach chance |
| Fire/Thermal | Spreads to adjacent compartments |
| Flooding | Blocks movement/repairs, pressure damage |
| EMP/EW | Temporary subsystem disruption |

### Damage Control

- Finite resource (crew time + equipment)
- Allocation problem (which compartments first)
- Mitigates cascading failures
- Arcologies have huge capacity but huge surface area

## Outcome by Ship Class

Ship fate scales with size. Small craft are destroyed; megaships are degraded.

| Class | Typical Fates | Salvage Potential |
|-------|---------------|-------------------|
| **S** (Small) | Destroyed, sunk, vaporized | None—write-off |
| **M** (Medium) | Sunk, rendered inserviceable, captured | Hull salvageable if not sunk |
| **L** (Large) | Mission-killed, captured, scuttled | Salvage/repair likely |
| **XL** (Megaship) | Degraded, besieged, captured | Cannot be "destroyed" conventionally |

### Small Craft (Class S)

Jetskis, patrol boats, corvettes. When they lose, they're gone:
- `fate: DESTROYED` — no wreck, no salvage
- Fast, clean outcomes for swarm units

### Medium Ships (Class M)

Frigates, destroyers. Can be sunk or disabled:
- `fate: DESTROYED` — sunk, possibly salvageable wreck
- `fate: DISABLED` — dead in water, can be towed/captured/salvaged
- `fate: CAPTURED` — crew surrendered or eliminated

### Large Ships (Class L)

Carriers, cargo, fortress rigs. Too big to sink easily:
- `fate: DISABLED` — mission-killed but afloat
- `fate: CAPTURED` — tactical boarding resolved in arena
- Component damage report matters for strategic repair costs

### Megaships (Class XL)

Arcology-ships, sea cities. **Cannot be conventionally destroyed**—they're too massive.

Instead, megaships accumulate **degradation**:
- Engines disabled (immobilized)
- CIC destroyed (tactical blind)
- Major weapon systems knocked out
- Hull damage across districts/decks
- Population casualties and morale collapse

**Special-purpose weapons** (nuclear, antimatter, deep-bore charges) can destroy megaships but with catastrophic consequences—usually prohibited by doctrine or treaty.

The `BattleResult` for megaships returns a **damage report**, not a binary fate:
- Component states (propulsion, CIC, weapons, power)
- District/deck damage percentages
- Boarding status (if attempted)
- Casualty estimates

This damage report feeds the strategic layer for repair, legitimacy effects, and siege resolution.

## Boarding Model

### Preconditions

Boarding requires at least one of:
- Target immobilized/disabled
- Attacker attached (grapple/dock) under risk
- Target overwhelmed (no escorts, security depleted)

### Tier 0: Quick Resolution (Small Ships)

Single deterministic contested check:

**Inputs**:
- Attacker: Marines, gear, morale
- Defender: Crew, internal security, damage state

**Modifiers**:
- Target disabled
- Bulkhead integrity
- Fires/flooding
- Leadership quality

**Outputs**:
- Capture
- Repulse
- Stalemate/withdraw
- Scuttle/self-destruct

Deterministic from seed—no hidden dice rolls.

### Tier 1+: Multi-Phase (Large Ships)

Stateful process with partial outcomes:

**Phases**:
1. **Attach/Breach**: Establish entry point (can fail/interrupt)
2. **Secure Objectives**: Capture key compartments
3. **Consolidate**: Eliminate pockets, prevent sabotage

**State Tracking**:
- Per-compartment control: `attacker` / `contested` / `defender`
- Boarding force distribution across entry points
- Time-based attrition with seeded events

### Victory Conditions

**Combat Ship** (carrier, militarized cargo, major platform):

Control all of:
- **Bridge** (command authority)
- **CIC** (tactical systems)
- **Engineering** (propulsion/power)

Plus: Pacify defenders OR force surrender via morale threshold

**Arcology-Ship** (floating city):

Control the above plus:
- **Government Seat** (palace, parliament, corporate HQ—varies by government type)

Plus: Reduce **civilian resistance** below threshold

Civilian resistance is influenced by legitimacy, casualties, communications control, and time—derived from the same morale inputs as military surrender.

### Morale and Surrender

Surrender is a threshold based on **morale**, not elimination:

**Morale Inputs** (computed before battle, flows via `CrewSnapshot.morale`):
- **Faction-wide morale** from `FactionContext.morale_state` (see [factions.md](factions.md))
- Ship condition (flooding, fire, compartment loss)
- Supply quality and level (food, water, medicine, ammo)
- Leadership and training (crew quality, cohesion from [people.md](people.md))
- Government type and events (legitimacy, propaganda, recent battles)
- Information environment (isolation, misinformation)

For arcologies, civilian resistance uses the **same inputs** with different weights/thresholds.

### Specialist Units

| Unit Type | Role |
|-----------|------|
| Marines | Baseline clearing strength |
| Breaching Engineers | Faster door/bulkhead breach, disable scuttling |
| Internal Security | Defender strength, chokepoint advantage |
| Drones | Scouting, decoying |
| Civil Affairs | Reduce civilian resistance escalation |

Abstract as numeric modifiers initially; expand later.

### Arcology Boarding as Crisis

On arcology-ships, boarding is a **strategic crisis**:

- External battle determines if boarding is possible
- Internal phase can outlast the tactical battle
- Involves factions, morale collapse, sabotage
- Humanitarian constraints (civilian decks)
- Political consequences (leadership capture, legitimacy shock)

## Arcology Capture: Tactical Injection vs Strategic Siege

Capturing an Arcology-Ship is **split across layers** to match timescales. Real-time combat happens over minutes; city-scale capture happens over days/weeks.

### Tactical Layer (Combat Arena): Establish Beachhead

In the Combat Arena, attackers do not fight deck-by-deck. Instead, they attempt to establish a **boarding beachhead**—a "King of the Hill" objective.

**Tactical Objective: BOARDING_ESTABLISHED**

1. **Suppress**: Disable the target's defenses (or at least the local quadrant)
2. **Dock**: Physically attach troop transports/assault ships
3. **Hold**: Survive while the "Troop Transfer" bar fills
4. **Establish**: Once transfer threshold is reached, beachhead is established

**Connection State Machine**

Distance-only checks are gamey. Use a connection state machine:

```
APPROACHING → LATCHING (wind-up) → DOCKED → TRANSFERRING
```

- Any hit above threshold, collision, or forced maneuver breaks connection
- Breaking during TRANSFERRING pauses transfer; breaking during LATCHING resets to APPROACHING
- Defenders have a crisp disruption goal: break the connection

**Transfer Rate Function**

Transfer rate is systemic, not static:

```
transfer_rate = base_rate
              × dock_integrity      (degrades under fire/velocity)
              × suppression_factor  (local defenses suppressed?)
              × sea_state_factor    (storms matter on surface)
              × command_link_factor (EW/comms disruption)
```

This makes pillars interlock: sensors, EW, weather, and damage all affect boarding.

**Tactical Outcome**

The arena returns `boarding_status: ESTABLISHED` plus:
- `troops_inserted`: How many got through
- `breach_quality`: How hard it is for defenders to purge (0.0–1.0)
- `entry_point`: Which dock/quadrant (for strategic layer)
- `attached_duration_s`: Total time docked

**Critical UX Note**: Victory state should be named **BEACHHEAD ESTABLISHED**, not "Captured". The tactical end screen should say:

> "Arcology under siege. Control will be decided over time unless relief forces break contact or defenders purge the breach."

### Strategic Layer (Campaign): Resolve the Siege

Once a beachhead exists, control is decided over **days/weeks** using governance, morale, and internal factions.

**Defender Actions** (strategic layer):
- Quarantine districts / bulkhead lockdown
- Purge teams / internal security surge
- Propaganda and legitimacy stabilization
- Negotiate / amnesty / buy-off internal factions
- Request relief fleet / break blockade
- Scuttle critical systems / deny objectives (at huge legitimacy cost)

**Attacker Actions** (strategic layer):
- Reinforce the breach (requires maintaining naval superiority)
- Expand foothold district by district
- Subvert internal factions
- Cut communications / isolate leadership
- Manage legitimacy/political fallout of occupation

**Resolution**: The siege ends when one side achieves control or withdraws. Government type and internal factions determine how hard the siege is and what "winning" looks like.

### Why This Split Works

| Concern | Solution |
|---------|----------|
| Timeline mismatch | Tactical = minutes, Strategic = weeks |
| Interior FPS scope creep | Arena stays about maneuver/sensors/suppression |
| Trivializing city-ships | Capture is an epic, multi-phase campaign |
| Meaningful governance systems | Politics and factions pay rent during siege |
| Trainable AI objective | "Suppress → Dock → Hold" is legible and learnable |

## Data Contract Extensions

### BattlePackage Extensions

Per ship (for boarding-capable battles):
- `components[]` or `compartments[]` layout
- Crew distributions
- Boarding capability (marines, gear, drones)
- Internal security rating
- Bulkhead integrity parameters
- Dock points / entry vectors (for megaships)

### BattleResult Extensions

**ShipOutcome** gains:
- `fate`: OPERATIONAL, DISABLED, DESTROYED, SCUTTLED, CAPTURED
- `damage_report`: Component/compartment states (for Tier 1+)
- `salvage_value`: Estimated recovery potential

**BoardingOutcome** (new, per boarding attempt):

```
BoardingOutcome {
    target_ship_id:     String
    attacker_team_id:   String
    status:             BoardingStatus      # NONE, ATTEMPTED, ESTABLISHED, REPULSED

    # Tactical mechanics summary
    attached_duration_s: f64
    transfer_progress:   Ratio              # 0.0–1.0
    troops_inserted:     u32

    # Casualties (arena has limited internal knowledge)
    attacker_casualties: u32
    defender_casualties: u32

    # How hard for defenders to purge in strategic layer
    breach_quality:      Ratio              # 0.0–1.0

    # Localization for strategic layer
    entry_point:         String?            # dock node / quadrant / district
}
```

**MegashipDamageReport** (for Class XL ships):

```
MegashipDamageReport {
    ship_id:            String

    # Key system states
    propulsion_status:  ComponentStatus     # OPERATIONAL, DAMAGED, DESTROYED
    cic_status:         ComponentStatus
    primary_weapons:    ComponentStatus
    power_systems:      ComponentStatus

    # District-level damage (percentages)
    district_damage:    Map<String, Ratio>  # e.g., {"port_quarter": 0.4, "stern": 0.8}

    # Population impact
    casualty_estimate:  u32
    evacuation_status:  Ratio               # 0.0–1.0 (how many got out)

    # For strategic layer siege initialization
    morale_shock:       Ratio               # Impact on defender morale
    legitimacy_impact:  Ratio               # Impact on government legitimacy
}
```

The arena returns facts it can measure; the campaign layer interprets them using governance/morale systems.

## DRL Considerations

**Observations**:
- Own component/compartment health and status
- Damage control allocation state
- Boarding progress (own and known enemy, subject to intel)

**Actions**:
- Allocate damage control
- Toggle scuttle/lockdown/bulkheads
- Attempt attach/breach
- Allocate boarding forces to objectives

Partial observability: Opponents don't know internal damage without close proximity or intel.

## Staging

1. Tier 0 hull+flags for all ships (fast, stable)
2. Tier 1 components for mid/large ships
3. Tier 0 boarding for small ships and disabled targets
4. Tier 2 compartments + multi-phase boarding for arcologies
