# Tidebreak

**A naval strategy game where fleets battle across ocean layers and mobile city-ships vie for survival in a drowned world.**

## The Premise

In a post-catastrophe world, most livable land is gone. What remains is heavily contested—and critically, the remaining land factions control the **mega shipyards**: the only facilities capable of building carriers, supertankers, and purpose-built arcology-ships. Everyone else must make do with converted vessels or what they can salvage.

But land factions have their own vulnerability: **food comes from the ocean**. Kelp farms, fisheries, and aquaculture operations are ocean-based. If blockaded, land factions starve. This creates a strategic interdependence—land controls shipbuilding, ocean controls food—and makes piracy a genuine threat to both sides.

Most of humanity now lives on the waves. The largest vessels aren't just warships—they're floating cities called **Arcology-Ships**, carrying tens of thousands of people. Some are purpose-built marvels from the shipyards; others are refactored supertankers jury-rigged into mobile nations. Each has governments, economies, and internal politics.

Combat spans three dimensions: surface, sub-surface, and abyssal depths. Fleets must coordinate across layers, with submarines lurking below and surface ships vulnerable to weather above. Diving isn't just evasion—it's a tactical choice with sensor, weapon, and survival tradeoffs.

## Core Experience

Tidebreak combines:

- **Tactical combat** across ocean depth layers, where positioning in 3D space matters
- **Fleet command** ranging from swarm jetskis to massive dreadnoughts
- **Mobile nation management** where your arcology-ship is a society, not just a vessel
- **AI opponents** trained via deep reinforcement learning to adapt and challenge

The game draws inspiration from Starsector's fleet-scale combat but reimagines it for an ocean setting with unique mechanics around depth, sensors, and weather.

## Fleet Hierarchy

| Class | Examples | Role |
|-------|----------|------|
| Wave Skimmers | Jetskis, small boats | Swarm harassment, point defense |
| Cutters | Patrol boats, LCS | Fast attack, escort, scouting |
| Corvettes | Multi-role combatants | Main line combat |
| Frigates | Heavy gunboats | Small fleet flagships |
| Dreadnoughts | Battleships, carriers | Fleet anchors, massive firepower |
| Arcology-Ships | Nomad-cities | Mobile bases, economic hubs, faction capitals |

## What Makes It Different

### Depth as Strategic State

Unlike traditional naval games with flat oceans or continuous depth, Tidebreak uses three discrete **strategic states**:

- **Surface** ("The Arena"): Full sensors and weapons, vulnerable to everything, affected by weather
- **Submerged** ("The Stealth Layer"): Sonar-only, torpedoes and mines, immune to ballistic/energy weapons
- **Abyssal** ("The Flank"): Strategic transit and bypass, minimal combat, requires specialized hulls

Changing layers takes 30–60+ seconds—a strategic commitment, not a tactical dodge. During transition, ships are maximally vulnerable (exposed to both layers) and cannot fire. This creates tension: commit to the dive knowing you're exposed, or stay and fight?

### Arcology-Ships as Nations

The largest vessels aren't military assets—they're societies:

- Populations with needs (food, water, morale)
- Internal factions with competing goals
- Government types that affect decision-making speed and stability
- Economic production (manufacturing, farms, markets)
- Catastrophic consequences if damaged or captured

Losing an arcology isn't losing a ship. It's losing a nation.

### Fog of War Through Sensors

Combat operates on uncertain information:

- Ships maintain **track tables** of contacts with varying quality and age
- Sensor types (radar, sonar, visual, passive RF) work differently by layer
- Teams share tactical pictures over contested data links
- Electronic warfare degrades sensing and networking
- Identification is uncertain—friendly fire is possible

### Trained AI Opponents

Enemy commanders use deep reinforcement learning agents trained across:

- Ship-level tactics (maneuvering, weapon timing)
- Fleet-level coordination (formations, combined arms)
- Strategic decisions (resource allocation, risk assessment)

**Design goal**: Agents trained to discover tactics rather than follow scripts. Scripted AI used only as baselines and curriculum bootstrapping. Emergent behaviors (wolfpack tactics, crossing the T) are training targets, not guarantees.

## Target Audience

Players who enjoy:

- Fleet-scale tactical combat (Starsector, Highfleet, Naval Action)
- Strategic management with real consequences (Dwarf Fortress, RimWorld)
- Emergent complexity from simple rules
- AI that adapts rather than follows patterns

## Non-Goals

To prevent scope creep, these are explicitly **not** part of the vision:

**No global tech tree**
: Campaign timescale is years, not centuries. There's no "research laser weapons" over 50 turns. Technology exists; the question is who has access to it.

**Progression through integration, not invention**
: Power comes from supply chains, crew specialization, doctrine, political leverage, and procurement relationships—not unlocking new tiers of gear.

**Limited sources of new capital ships**
: New carriers, supertankers, and purpose-built arcologies come only from mega shipyards (controlled by land factions). Everyone else uses: OTS procurement, variants, salvage, jury-rig upgrades, and converted vessels.

**Not a 4X game**
: No "expand to every corner of the map" imperative. The ocean is big; control is local and contested. Factions coexist, trade, raid, and occasionally go to war—but total domination isn't the default win condition.

## Current Status

Early prototype focused on the **Combat Arena MVP**: a deterministic, headless-capable battle simulator for both player-facing combat and DRL training. The full game vision (economy, factions, diplomacy, governance) is documented but not yet implemented.
