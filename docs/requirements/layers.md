# Layer Requirements

Requirements for the depth layer system and terrain.

See: [design/layers-and-terrain.md](../design/layers-and-terrain.md)

## Layer States (P0)

- Support discrete layer states as rule sets, not physical depth ranges
- Support units existing in exactly one layer at a time (except during transition)
- Support layer determining available sensors, weapons, and vulnerabilities
- MVP: Surface and Submerged layers only (+ Transitioning between them)

## Layer States (P1)

- Support Abyssal layer as third strategic state

## Surface Layer (P0)

- Support full sensor access (radar, visual, sonar)
- Support full weapon access (guns, missiles, torpedoes)
- Support vulnerability to all weapon types
- Support weather effects on sensors, weapons, and movement

## Submerged Layer (P0)

- Support sonar-primary detection
- Support torpedo and mine weapons
- Support immunity to ballistic and energy weapons
- Support vulnerability to torpedoes and depth charges

## Submerged Layer (P1)

- Support pop-up maneuver for briefly firing surface weapons (missiles)
- Support pop-up creating temporary detection and vulnerability spike
- Support thermal layers providing concealment bonuses

## Abyssal Layer (P1)

- Support strategic transit with minimal combat capability
- Support immunity to most standard weapons
- Support specialized deep-pressure ordnance only
- Support hull pressure strain mechanics

## Abyssal Layer (P2)

- Support thermal vent highways for faster travel
- Support deep salvage and resource access
- Support crush depth limits based on hull rating

## Layer Transitions (P0)

- Support layer transitions taking 30–60+ seconds (configurable)
- Support transition state where units cannot fire weapons
- Support transition state generating massive sensor signatures
- Support transition state receiving damage from both origin and destination layers

## Layer Transitions (P1)

- Support transition interruption by heavy damage
- Support botched transitions (forced return, stuck, catastrophic failure)
- Support transition energy cost proportional to hull size

## Terrain as State Blocker (P0)

- Support terrain types determining valid layer states
- Support Deep Ocean tiles allowing all three layers
- Support Shelf/Coastal tiles blocking Abyssal layer
- Support Shallows/Reef tiles blocking Submerged and Abyssal layers
- Support Land/Island tiles blocking all layers (impassable)

## Movement Physics (P0)

- Support 2D movement within each layer
- Support inertia (momentum through maneuvers)
- Support turn radius (no instant pivoting)
- Support acceleration and deceleration curves

## Movement Physics (P1)

- Support layer-dependent physics parameters
- Support Surface layer affected by wind and waves
- Support Submerged layer with slower top speed, better acceleration
- Support Abyssal layer with slowest movement and pressure strain
