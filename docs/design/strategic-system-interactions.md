# System Interaction Analysis: Tidebreak Strategic Layer

This document maps the interactions between Tidebreak's strategic layer systems (Economy, Governance, People, Factions), identifies feedback loops, traces cascade chains, and analyzes the handoffs to/from the combat arena.

## Executive Summary

| Metric | Assessment |
| ------ | ---------- |
| **Interaction Density** | High — All four strategic systems touch each other |
| **Orthogonality Score** | 8/10 — Each system has a clear primary role |
| **Emergence Potential** | High — Political/economic interactions create emergent narratives |
| **Loop Balance** | Healthy — Mix of growth, decay, and balancing loops |
| **Combat Handoff** | Well-defined — Clear contracts in both directions |

## Mechanic Inventory

| Mechanic | Primary Function | Systems Touched | Orthogonal? |
| -------- | ---------------- | --------------- | ----------- |
| **Resources** | Material constraints | Economy, Factions, Combat (supply) | Yes |
| **Trade Routes** | Economic connections | Economy, Factions, Governance | Yes |
| **Action Points** | Decision pacing | Governance, Factions | Yes |
| **Strategic Goals** | Faction intent | Governance, Factions, People | Yes |
| **Missions** | Work decomposition | Governance, Factions, Combat | Yes |
| **Disposition** | Faction relationships | Factions, People, Governance | Yes |
| **Treaties** | Formal agreements | Factions, Governance, Economy | Yes |
| **Morale (Faction)** | Collective confidence | Factions, Combat, People | Yes |
| **Population** | Labor/consumption | Economy, People, Governance | Yes |
| **Internal Factions** | Political pressure | Governance, People, Economy | Yes |
| **Leadership** | Performance modifiers | People, Factions, Combat | Yes |
| **Reputation (Player)** | Access/pricing | Factions, Economy, Missions | Yes |
| **Crew** | Ship performance | People, Combat, Economy (supply) | Yes |
| **Supply** | Combat readiness | Economy, Combat, Logistics | Yes |
| **Arcologies** | Population centers | All systems | Partial* |

*Arcologies are hubs that touch everything—by design, they're where systems intersect.

## Interaction Matrix

```text
               Rsrc Trade ActPt Goals Miss  Disp Treat Morl  Pop  IntFac Lead  Rep  Crew Supp Arcol
Resources        -    ●     ○     ○     ○     ○    ●     ○    ●     ○     ○     ○    ●    ●    ●
Trade Routes     ●    -     ○     ●     ●     ●    ●     ○    ○     ○     ○     ●    ○    ●    ●
Action Points    ○    ○     -     ●     ●     ●    ●     ○    ○     ○     ○     ○    ○    ○    ●
Strategic Goals  ○    ●     ●     -     ●     ●    ●     ●    ○     ○     ●     ○    ○    ○    ●
Missions         ○    ●     ●     ●     -     ●    ○     ●    ○     ○     ○     ●    ●    ●    ●
Disposition      ○    ●     ●     ●     ●     -    ●     ●    ○     ○     ○     ●    ○    ○    ○
Treaties         ●    ●     ●     ●     ○     ●    -     ○    ○     ○     ○     ○    ○    ●    ●
Morale (Faction) ○    ○     ○     ●     ●     ●    ○     -    ○     ●     ●     ○    ●    ○    ●
Population       ●    ○     ○     ○     ○     ○    ○     ○    -     ●     ○     ○    ●    ○    ●
Internal Factions○    ○     ○     ○     ○     ○    ○     ●    ●     -     ●     ○    ○    ○    ●
Leadership       ○    ○     ○     ●     ○     ○    ○     ●    ○     ●     -     ○    ●    ○    ●
Reputation       ○    ●     ○     ○     ●     ●    ○     ○    ○     ○     ○     -    ○    ○    ○
Crew             ●    ○     ○     ○     ●     ○    ○     ●    ●     ○     ●     ○    -    ●    ○
Supply           ●    ●     ○     ○     ●     ○    ●     ○    ○     ○     ○     ○    ●    -    ●
Arcologies       ●    ●     ●     ●     ●     ○    ●     ●    ●     ●     ●     ○    ○    ●    -

Legend: ● = strong interaction, ○ = weak/no interaction, - = self
```

### Key Interaction Clusters

**Political Cluster**: Disposition ↔ Treaties ↔ Goals ↔ Action Points

These systems form the diplomacy engine. Disposition affects what treaties are possible, treaties constrain goals, goals consume action points, and all diplomatic actions affect disposition.

**Economic Cluster**: Resources ↔ Trade Routes ↔ Supply ↔ Arcologies

Material flows between production (arcologies), distribution (trade routes), consumption (supply), and storage (resources). Blockades disrupt trade routes, which starves supply, which degrades combat readiness.

**People Cluster**: Population ↔ Internal Factions ↔ Leadership ↔ Crew ↔ Morale

Human capital flows from arcology populations through internal political dynamics, manifests as leaders and crews, and affects faction morale. Casualties cascade backward into population loss.

**Mission Cluster**: Goals ↔ Missions ↔ Reputation ↔ Crew ↔ Combat Handoff

Strategic goals generate missions, missions involve crews, crews fight in combat, results affect reputation and morale, which flows back to faction goals.

## Interaction Details

### Economy ↔ Factions (Strong)

**Interaction**: Faction economic strength determines action point accumulation, fleet sustainability, and diplomatic leverage. Trade agreements between factions create interdependence.

**Flow Direction**: Bidirectional — Economy enables faction actions; faction relationships shape trade access.

**Designed**: Yes — core pillar "Economics Drive Politics"

**Depth**: Deep — factions with poor economies can't sustain wars; wealthy factions have more options

### Governance ↔ People (Strong)

**Interaction**: Government type affects leader selection, internal faction influence, and population happiness. Leadership competence affects governance efficiency.

**Flow Direction**: Bidirectional — Governance structures select leaders; leaders shape governance outcomes.

**Designed**: Yes — core pillar "People Matter"

**Depth**: Deep — same resources with different governance/leadership yields different outcomes

### Factions ↔ Combat (Strong: via BattlePackage/BattleResult)

**Interaction**: Faction state flows into battles (morale, tech, supplies); battle outcomes flow back (casualties, morale delta, captures). This is the primary strategic↔tactical handoff.

**Data Contracts**:

```
INTO COMBAT (BattlePackage):
├─ FactionContext.morale_state → Ship morale calculation
├─ FactionContext.philosophy → AI behavior
├─ FactionContext.tech_tags → Ship capabilities
├─ ShipSnapshot.crew → Gunnery, engineering, morale
├─ ShipSnapshot.supply → Ammo limits, penalties
└─ ShipSnapshot.leadership → Command modifiers

OUT OF COMBAT (BattleResult):
├─ TeamOutcome.morale_delta → FactionState.morale update
├─ ShipOutcome.crew_casualties → Population loss
├─ ShipOutcome.consumption → Resource depletion
├─ ShipOutcome.fate → Ship roster update
└─ ShipOutcome.capture_method → Faction ship transfers
```

**Flow Direction**: Bidirectional — faction state conditions combat; combat results update faction state.

**Designed**: Yes — explicit contract versioning (arena.v1, v2, v3)

**Depth**: Deep — cascading strategic consequences from tactical outcomes

### Morale × Disposition (Strong)

**Interaction**: Faction morale affects willingness to make peace (low morale → seek treaties) or war (high morale → aggression). Disposition affects morale through alliance victories/defeats.

**Designed**: Yes — creates political narrative arcs

**Depth**: Medium — primarily affects AI faction behavior

### Supply × Combat Readiness (Strong)

**Interaction**: Days of supply affects crew penalties, ammo availability, and morale. Prolonged blockades degrade combat effectiveness without firing a shot.

**Designed**: Yes — logistics matters

**Depth**: Deep — creates strategic layer gameplay (protect convoys, break blockades)

## Feedback Loop Map

### Positive Loops

#### Economic Snowball

```text
Territory → Resources → Fleets → Victory → More Territory → More Resources
```

- **Risk**: Runaway faction dominance
- **Mitigation**: Multiple faction alliances against the leader; overextension penalties; distance decay on control

#### Morale Spiral

```text
Victory → Morale Boost → Better Combat Performance → Victory
OR
Defeat → Morale Drop → Worse Combat Performance → Defeat
```

- **Risk**: Death spiral for losing factions
- **Mitigation**: Leader traits can buffer morale; respawn mechanics (exiles, insurgencies); faction philosophy affects resilience

#### Alliance Momentum

```text
Shared Victory → Disposition Increase → Stronger Alliance → Joint Operations → Shared Victory
```

- **Risk**: Permanent alliance blocs
- **Mitigation**: Contested captures create friction (−0.10 to −0.25 on prize disputes); internal faction pressure against allies

### Negative Loops

#### Resource Depletion

```text
War → Resource Consumption → Economic Strain → Reduced War Capacity → Forced Peace
```

- **Purpose**: Prevents endless wars; creates victory windows
- **Risk**: Could cause stagnation; balanced by war objective (defeat enemy before depletion)

#### Overextension Decay

```text
Expansion → More Holdings → More Garrisons → Fewer Offensive Forces → Harder Expansion
```

- **Purpose**: Prevents single-faction domination
- **Risk**: Could make expansion unrewarding; balanced by economic gains from territory

#### Reputation Consequences

```text
Broke Treaty → Disposition Drop → Trade Restrictions → Economic Damage → Reduced Capability
```

- **Purpose**: Makes betrayal costly; encourages alliance stability
- **Risk**: Could lock factions into suboptimal alliances; balanced by changing circumstances

### Balancing Loops

#### Contested Capture Politics

```text
Allied Victory → Both Claim Prize → Disposition Tension → Negotiation/Withdrawal → Alliance Preserved (or Fractures)
```

- **Purpose**: Creates emergent political drama from tactical outcomes
- **Designed**: Yes — explicit in disposition events (−0.01/day standoff decay)

#### Internal Faction Pressure

```text
External Policy → Internal Faction Reaction → Government Stability → Policy Modification
```

- **Purpose**: Prevents AI factions from ignoring "public opinion"
- **Designed**: Yes — internal factions push back on unpopular decisions

## Cascade Chain Analysis

### Chain 1: Blockade Starvation

```text
Trigger: Faction A interdicts Faction B's trade routes
→ Step 1: Trade route disruption (Economy → Trade)
→ Step 2: Supply shortages at Faction B arcologies (Trade → Supply)
→ Step 3: Fleet supply levels drop (Supply → Combat readiness)
→ Step 4: Crew morale penalties from shortages (Supply → Crew → Morale)
→ Step 5: Next battle, ships fight at reduced effectiveness (Morale → Combat)
→ Step 6: Defeat compounds morale drop (Combat → Faction Morale)
→ Final: Internal factions demand peace (Morale → Governance → Diplomacy)
```

**Designed/Emergent**: Designed — this is the supply pressure loop

**Strategic Agency Points**: Could protect trade routes with escorts; could find alternative routes; could raid enemy supply lines in retaliation; could negotiate before starvation

### Chain 2: Leadership Assassination

```text
Trigger: Faction leader killed/captured in battle
→ Step 1: Leadership vacuum (People → Leadership)
→ Step 2: Succession crisis (Leadership → Governance)
→ Step 3: Internal faction power struggle (Governance → Internal Factions)
→ Step 4: Morale shock from leader loss (Leadership → Morale)
→ Step 5: Decision paralysis (action point efficiency drops) (Morale → Governance)
→ Step 6: Opportunistic enemy attack during crisis (Governance → Defense)
→ Final: Faction fragmentation or collapse (Defense → Faction defeat)
```

**Designed/Emergent**: Emergent from designed interactions

**Strategic Agency Points**: Could protect leaders (not on front lines); could establish succession; could have charismatic backup leaders

### Chain 3: Diplomatic Betrayal Cascade

```text
Trigger: Faction A breaks treaty with Faction B
→ Step 1: Massive disposition drop (−0.30) (Diplomacy → Disposition)
→ Step 2: Other factions note broken treaty (Disposition → Reputation)
→ Step 3: Alliance offers to Faction A dry up (Reputation → Diplomacy)
→ Step 4: Factions form counter-alliance against Faction A (Reputation → Disposition network)
→ Step 5: Trade partners restrict access (Disposition → Trade)
→ Step 6: Economic isolation (Trade → Economy)
→ Final: Faction A surrounded by enemies with weak economy (Economy → Strategic position)
```

**Designed/Emergent**: Designed — betrayal is meant to have lasting consequences

**Strategic Agency Points**: Could time betrayal to gain decisive advantage; could offer concessions to repair reputation; could ally with other "pariah" factions

### Chain 4: Victory Morale Surge

```text
Trigger: Decisive battle victory with enemy flagship captured
→ Step 1: Large morale_delta from BattleResult (+0.15) (Combat → Morale)
→ Step 2: Crews in next battles fight harder (Morale → Combat performance)
→ Step 3: Enemy faction morale drops (−0.20) (Combat → Enemy Morale)
→ Step 4: Enemy ships more likely to surrender (Enemy Morale → Surrender threshold)
→ Step 5: Further victories compound effect (Surrender → Captures → Morale)
→ Final: Enemy faction morale collapse, mass surrenders (Morale → Faction defeat)
```

**Designed/Emergent**: Designed — morale cascade is intentional

**Strategic Agency Points**: Could distribute flagship value across flotilla (no single high-value target); could accept surrender terms before collapse; could keep reserves for counter-attack

### Chain 5: Prize Dispute Alliance Fracture

```text
Trigger: Allied factions both insert troops onto captured megaship
→ Step 1: Contested capture state (Combat → Politics)
→ Step 2: Daily disposition decay (−0.01/day) (Politics → Disposition)
→ Step 3: Neither faction withdraws (Disposition → Standoff)
→ Step 4: Disposition drops below alliance threshold (Standoff → Disposition < 0.7)
→ Step 5: Alliance treaty lapses (Disposition → Treaties)
→ Step 6: Former allies become neutral/hostile (Treaties → Faction relations)
→ Final: War between former allies over prize (Faction relations → Conflict)
```

**Designed/Emergent**: Designed — explicit in disposition events table

**Strategic Agency Points**: Could negotiate split (trade concessions, salvage rights); could withdraw for goodwill (+0.05); could escalate to claim prize and accept alliance loss

## Combat Handoff Analysis

### Strategic → Tactical (BattlePackage)

| Data Flow | Source | Destination | Purpose |
| --------- | ------ | ----------- | ------- |
| Faction morale | `FactionState.morale.effective` | `FactionContext.morale_state` | Surrender thresholds, accuracy |
| Tech level | `FactionState.tech_tags` | `FactionContext.tech_tags` | Ship capability modifiers |
| Crew state | `ShipState.crew` | `CrewSnapshot` | Gunnery, engineering, morale |
| Supply state | Logistics system | `ShipSnapshot.supply` | Ammo limits, penalties |
| Leadership | People system | `ShipSnapshot.leadership` | Command competence |
| Damage state | Previous battles | `ShipSnapshot.damage_state` | Persistent damage (P2) |
| Siege state | Ongoing boarding | `ShipSnapshot.siege_state` | Megaship penalties (P3) |

**Contract Versioning**:
- **arena.v1** (MVP): morale, crew, weather only
- **arena.v2** (P2): adds supply, damage_state, leadership
- **arena.v3** (P3): adds siege_state, full boarding

### Tactical → Strategic (BattleResult)

| Data Flow | Source | Destination | Purpose |
| --------- | ------ | ----------- | ------- |
| Morale delta | `TeamOutcome.morale_delta` | `FactionState.morale.battle_modifier` | Update faction confidence |
| Casualties | `ShipOutcome.crew_casualties` | Population loss | Reduce labor pool |
| Consumption | `ShipOutcome.consumption` | Resource depletion | Update economy |
| Ship fates | `ShipOutcome.fate` | Fleet roster | Update asset lists |
| Captures | `CaptureMethod` | Faction ship transfer | Prize mechanics |
| Damage reports | `ShipOutcome.damage_report` | Ship repair queue | Persistent damage |
| Boarding outcomes | `BoardingOutcome` | Siege state | Megaship control |

### Handoff Timing

```text
STRATEGIC TICK (1 day)
│
├─ Faction AI evaluates goals, accumulates action points
├─ Economic production/consumption cycles
├─ Missions assigned to fleets
│
└─ IF battle triggered:
    │
    ├─ FREEZE strategic state into BattlePackage
    │
    ├─ COMBAT ARENA (N tactical ticks @ 1 second each)
    │   └─ (Substeps @ 0.1s for physics)
    │
    ├─ RECEIVE BattleResult
    │
    └─ APPLY results to strategic state
        ├─ Update faction morale
        ├─ Update ship roster
        ├─ Update resource pools
        └─ Update disposition (battle against/with other factions)
```

## Gaps and Opportunities

### Isolated Mechanics

| Mechanic | Currently Touches | Could Connect To |
| -------- | ----------------- | ---------------- |
| **Currents** (Combat) | Movement, Weather | Trade routes (shipping lanes), Supply (transit time) |
| **Terrain** (Combat) | Layers, Movement | Strategic chokepoints, Trade route planning |
| **Intel** | Sensors, Tracks | Strategic reconnaissance, Disposition (knowing enemy plans) |

### Missing Loops

**Technology Development Loop** (post-MVP):

```text
Resources → Research → Tech Advancement → Better Ships → Victory → Resources
```

Currently technology is static (tech_tags on factions). Could add progression.

**Refugee/Migration Loop** (post-MVP):

```text
War → Population Displacement → Arcology Overcrowding → Instability → More Conflict
```

Currently populations are tied to arcologies. Could add movement.

### Cascade Opportunities

**Economic Espionage**: Factions could steal trade intelligence, disrupting routes without military action. Currently, trade disruption requires blockade.

**Cultural Influence**: Factions could spread ideology to enemy internal factions, causing unrest. Currently, internal factions are isolated to their arcology.

**Technology Capture**: Capturing ships could yield tech_tag unlocks. Currently, capture only transfers the ship.

## Recommendations

### High Priority

1. **Make supply consequences visible** — Players/AI need to see "days of supply" prominently and understand impending penalties. The supply → combat cascade is core to strategic depth.

2. **Surface disposition changes clearly** — When actions affect disposition, show the delta and reason. Political consequences should be predictable.

3. **Ensure morale flows bidirectionally feel balanced** — Victory shouldn't be too snowbally; defeat shouldn't be unrecoverable. Test the morale multipliers extensively.

### Medium Priority

1. **Add strategic chokepoints** — Terrain features that affect trade routes would create natural flashpoints and strategic geography.

2. **Implement intel system** — Strategic-level sensors (spy networks, recon missions) that affect faction decision-making and disposition.

3. **Add war weariness** — Long wars should generate internal faction pressure for peace, preventing eternal conflicts.

### Low Priority

1. **Technology progression** — Let factions advance tech over time, creating catch-up dynamics.

2. **Population migration** — Let civilians move between arcologies based on safety/prosperity, creating refugee crises.

## MVP Staging

### P1 (Core Handoffs)

- [ ] BattlePackage/BattleResult contract implementation
- [ ] Faction morale flow (in and out of combat)
- [ ] Basic crew state propagation
- [ ] Ship fate tracking (roster updates)

### P2 (Strategic Loops)

- [ ] Supply system with combat penalties
- [ ] Disposition events from combat outcomes
- [ ] Leadership competence affecting performance
- [ ] Trade route mechanics

### P3 (Full Strategic Layer)

- [ ] Internal faction politics
- [ ] Treaty system
- [ ] Contested capture resolution
- [ ] Action point economy

## Related Documents

- [System Interactions (Combat)](system-interactions.md) — Tactical layer analysis
- [Economy Design](economy.md) — Resource and trade systems
- [Governance Design](governance.md) — Political decision-making
- [People Design](people.md) — Crew, leadership, population
- [Factions Design](factions.md) — Faction state and relationships
- [Combat Arena Design](combat-arena.md) — Battle contracts
- [Missions Design](missions.md) — Strategic → Tactical objective flow
- [Contracts](../technical/contracts.md) — Implementation-grade schemas
