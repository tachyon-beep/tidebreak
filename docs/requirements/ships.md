# Ship Requirements

Requirements for the fleet hierarchy, ship capabilities, and arcologies.

See: [vision/pitch.md](../vision/pitch.md), [design/damage-and-boarding.md](../design/damage-and-boarding.md)

## Fleet Hierarchy (P0)

- Support multiple ship classes with distinct roles:
  - Wave Skimmers: Swarm harassment, point defense
  - Cutters: Fast attack, escort, scouting
  - Corvettes: Main line combat
  - Frigates: Heavy gunboats, small fleet flagships
  - Dreadnoughts: Fleet anchors, massive firepower
  - Carriers: Launch and recover smaller craft
  - Arcology-Ships: Mobile nations, population centers

## Ship Capabilities (P0)

- Support layer capability per ship class (can_surface, can_submerge, can_dive_abyssal)
- Support maximum operating depth (crush depth)
- Support movement parameters (speed, acceleration, turn rate) per class
- Support weapon mount configurations per class

## Ship Capabilities (P1)

- Support ship size/crew determining damage and boarding tier
- Support carrier-class ships launching and recovering craft
- Support purpose-built vs. converted arcology distinctions

## Arcology-Ships (P1)

- Support arcologies as mobile nations with populations of 10,000+
- Support internal economy (manufacturing, markets, hydroponics)
- Support arcology damage using Tier 2 compartment model
- Support arcology loss as strategically catastrophic

## Arcology-Ships (P2)

- Support government types affecting decision-making
- Support internal factions with competing goals
- Support arcology boarding as multi-phase crisis
- Support civilian resistance mechanics
- Support government seat as boarding objective

## Stationary Platforms (P1)

- Support stationary platforms across scale range:
  - Monitoring stations (sensors, minimal crew)
  - Ocean farms (food production)
  - Industrial rigs (processing, fabrication)
  - Fortified platforms (defense, control)
  - Sea cities (faction capitals, arcology-scale)

## Stationary Platforms (P2)

- Support platforms contributing to tactical mesh
- Support platform combat (cannot maneuver, can mount defenses)
- Support platform siege and blockade mechanics
- Support platform capture objectives
