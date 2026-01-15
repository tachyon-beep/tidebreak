# Combat Requirements

Requirements for the Combat Arena and tactical battle system.

See: [design/combat-arena.md](../design/combat-arena.md), [design/damage-and-boarding.md](../design/damage-and-boarding.md)

## Arena Core (P0)

- Support a 2D top-down combat arena with fixed timestep simulation
- Support deterministic simulation given the same seed and inputs
- Support headless mode for DRL training and batch evaluation
- Support BattlePackage input and BattleResult output contracts with schema versioning
- Support faster-than-real-time simulation for training

## Weapons (P0)

- Support gun-type weapons with cooldown, dispersion, and projectile travel
- Support torpedo-type weapons with homing/lead-seeking behavior
- Support weapon availability constraints by layer (guns surface-only, torpedoes cross-layer)
- Support ammo tracking and consumption

## Weapons (P1)

- Support missiles with pop-up launch from Submerged layer
- Support mines as deployable hazards
- Support weapon component damage affecting availability

## Damage Model (P0)

- Support Tier 0 damage: single HP pool with status flags (mobility, weapons, sensors disabled)
- Support critical hits that toggle status flags without full component model

## Damage Model (P1)

- Support Tier 1 damage: independent component health (propulsion, power, sensors, weapons)
- Support component damage affecting ship capabilities directly
- Support damage control as resource allocation problem

## Damage Model (P2)

- Support Tier 2 damage: compartment graph with dependencies for capitals/arcologies
- Support localized damage (losing one section doesn't immediately affect others)
- Support cascading failures (fire, flooding, power loss)
- Support bulkhead containment mechanics

## Damage Types (P1)

- Support kinetic/explosive damage causing HP loss and breach chance
- Support fire/thermal damage spreading to adjacent compartments
- Support flooding damage blocking movement and repairs

## Boarding (P1)

- Support boarding preconditions (target disabled, attacker attached, or target overwhelmed)
- Support Tier 0 boarding: quick deterministic resolution for small ships

## Boarding (P2)

- Support Tier 1+ boarding: multi-phase process (attach, breach, secure objectives)
- Support per-compartment control states (attacker/contested/defender)
- Support objective-based victory (Bridge, CIC, Engineering)
- Support morale-driven surrender thresholds

## Boarding (P3)

- Support arcology-specific objectives (government seat)
- Support civilian resistance mechanics
- Support boarding as strategic crisis event

## Connection State Machine (P1)

- Support connection state progression: APPROACHING → LATCHING → DOCKED → TRANSFERRING
- Support LATCHING as wind-up phase before dock established
- Support connection breaking on:
  - Hit above damage threshold
  - Collision with other entities
  - Forced maneuver (target or attacker)
- Support LATCHING break resetting to APPROACHING
- Support TRANSFERRING break pausing transfer (not resetting)

## Arcology Boarding - Tactical (P1)

- Support "Suppress → Dock → Hold → Establish" tactical objective flow
- Support transfer rate as function of:
  - Dock integrity (degrades under fire/velocity)
  - Suppression factor (local defenses suppressed)
  - Sea state factor (storms affect surface docking)
  - Command link factor (EW/comms disruption)
- Support transfer progress tracking (0.0–1.0)
- Support troops_inserted count
- Support breach_quality metric (how hard to purge, 0.0–1.0)
- Support entry_point localization (dock node / quadrant / district)

## Arcology Boarding - Strategic (P2)

- Support beachhead status as handoff to strategic layer
- Support siege resolution over strategic time (days/weeks)
- Support defender actions: quarantine, purge teams, propaganda, negotiate
- Support attacker actions: reinforce, expand foothold, subvert factions, isolate

## Ship Outcomes (P0)

- Support ShipFate enum: OPERATIONAL, DISABLED, DESTROYED, SCUTTLED, CAPTURED
- Support fate assignment based on ship class:
  - Small (Class S): DESTROYED on defeat (no wreck)
  - Medium (Class M): DESTROYED, DISABLED, or CAPTURED
  - Large (Class L): DISABLED or CAPTURED (too big to sink easily)
  - Megaship (Class XL): Degraded only (cannot be destroyed conventionally)

## Ship Outcomes (P1)

- Support salvage_value estimation for disabled/destroyed ships
- Support component/compartment damage reports for Tier 1+ ships

## Megaship Damage Reports (P1)

- Support MegashipDamageReport schema for Class XL ships
- Support reporting key system states (propulsion, CIC, weapons, power)
- Support district-level damage percentages
- Support casualty estimates
- Support morale_shock and legitimacy_impact metrics
- Support damage report as input to strategic siege resolution

## BoardingOutcome Schema (P1)

- Support BoardingOutcome in BattleResult for each boarding attempt
- Support fields:
  - target_ship_id, attacker_team_id
  - status (BoardingStatus enum)
  - attached_duration_s
  - transfer_progress (0.0–1.0)
  - troops_inserted
  - attacker_casualties, defender_casualties
  - breach_quality (0.0–1.0)
  - entry_point (nullable)

## Crew Influence (P0)

- Support gunnery stat affecting weapon accuracy and reload
- Support engineering stat affecting repair and damage control rates
- Support morale stat affecting reaction latency and failure chances

## Crew Influence (P1)

- Support fatigue affecting crew performance over time
- Support crew casualties affecting capability
- Support surrender behavior based on morale thresholds

## Terrain (P0)

- Support solid obstacles (reefs, islands) blocking movement
- Support collision detection and resolution
- Support map bounds

## Terrain (P1)

- Support soft zones (sensor interference, algae blooms) with intensity modifiers
- Support hazardous surface zones (storm belts) with damage over time
- Support currents as vector field affecting movement
