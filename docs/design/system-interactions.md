# System Interaction Analysis: Tidebreak Combat Arena

This document maps the interactions between Tidebreak's core combat systems, identifies feedback loops, traces cascade chains, and evaluates emergence potential.

## Executive Summary

| Metric | Assessment |
| ------ | ---------- |
| **Interaction Density** | High — Most systems touch 3+ other systems |
| **Orthogonality Score** | 8/10 — Each system has a clear primary role |
| **Emergence Potential** | High — Constraint interactions create discovery space |
| **Loop Balance** | Healthy — Mix of positive (damage cascade) and negative (resource depletion) |

## Mechanic Inventory

| Mechanic | Primary Function | Systems Touched | Orthogonal? |
| -------- | ---------------- | --------------- | ----------- |
| **Depth Layers** | Strategic positioning states | Movement, Sensors, Weapons, Damage | Yes |
| **Layer Transitions** | Commitment/vulnerability mechanic | Movement, Sensors, Damage, Time | Yes |
| **Track Quality** | Information uncertainty | Sensors, Weapons, TDM | Yes |
| **Sensors** | Detection and tracking | Layers, Weather, EW, Crew | Yes |
| **Weapons** | Damage delivery | Layers, Tracks, Crew, Damage | Yes |
| **Damage** | State degradation | Components, Crew, Morale, Movement | Yes |
| **Crew** | Modifier source | Weapons, Sensors, Damage Control, Morale | Yes |
| **Morale** | Surrender threshold | Damage, Supply, Leadership, Surrender | Yes |
| **Weather** | Environmental modifier | Sensors, Movement, Weapons, Transitions | Yes |
| **TDM (Data Mesh)** | Track sharing | Sensors, EW, Fleet Coordination | Yes |
| **EW (Electronic Warfare)** | Sensor/comms degradation | Sensors, TDM, Tracks | Yes |
| **EMCON** | Emission control | Sensors, Signature, Detection | Yes |
| **Currents** | Movement modifier | Movement, Fuel, Positioning | Yes |
| **Terrain** | Movement/layer constraints | Layers, Movement, Sensors | Yes |
| **Boarding** | Capture mechanic | Damage, Morale, Crew, Position | Partial* |

*Boarding has some overlap with damage/morale but serves a distinct capture objective.

## Interaction Matrix

```text
               Layers Trans Track Sensor Weapon Damage Crew Morale Weather TDM   EW   EMCON Curr Terr Board
Layers           -     ●     ○      ●      ●      ○     ○     ○      ○     ○    ○     ○     ○    ●     ○
Transitions      ●     -     ○      ●      ●      ●     ○     ○      ○     ○    ○     ○     ○    ○     ○
Track Quality    ○     ○     -      ●      ●      ○     ○     ○      ○     ●    ●     ●     ○    ○     ○
Sensors          ●     ●     ●      -      ●      ●     ●     ○      ●     ●    ●     ●     ○    ●     ○
Weapons          ●     ●     ●      ●      -      ●     ●     ○      ●     ○    ○     ○     ○    ○     ○
Damage           ○     ●     ○      ●      ●      -     ●     ●      ○     ○    ○     ○     ○    ○     ●
Crew             ○     ○     ○      ●      ●      ●     -     ●      ○     ○    ○     ○     ○    ○     ●
Morale           ○     ○     ○      ○      ○      ●     ●     -      ○     ○    ○     ○     ○    ○     ●
Weather          ○     ○     ○      ●      ●      ○     ○     ○      -     ●    ○     ○     ●    ○     ○
TDM              ○     ○     ●      ●      ○      ○     ○     ○      ●     -    ●     ○     ○    ○     ○
EW               ○     ○     ●      ●      ○      ○     ○     ○      ○     ●    -     ○     ○    ○     ○
EMCON            ○     ○     ●      ●      ○      ○     ○     ○      ○     ○    ○     -     ○    ○     ○
Currents         ○     ○     ○      ○      ○      ○     ○     ○      ●     ○    ○     ○     -    ○     ○
Terrain          ●     ○     ○      ●      ○      ○     ○     ○      ○     ○    ○     ○     ○    -     ○
Boarding         ○     ○     ○      ○      ○      ●     ●     ●      ○     ○    ○     ○     ○    ○     -

Legend: ● = strong interaction, ○ = weak/no interaction, - = self
```

### Key Interaction Clusters

**Information Warfare Cluster**: Sensors ↔ Track Quality ↔ TDM ↔ EW ↔ EMCON

These systems form a tight web where each decision affects the others. Going active improves tracks but reveals position. Sharing tracks requires links that can be jammed. EMCON reduces signature but blinds you.

**Combat Resolution Cluster**: Weapons ↔ Damage ↔ Crew ↔ Morale ↔ Boarding

Damage cascades through crew efficiency to morale thresholds and ultimately surrender/capture. The chain is clear: hit → damage → crew loss → morale drop → surrender consideration.

**Positioning Cluster**: Layers ↔ Transitions ↔ Terrain ↔ Weather ↔ Currents

Where you are matters. Layers restrict weapons/sensors. Terrain restricts layers. Weather affects surface operations. Currents affect fuel economy.

## Interaction Details

### Layers × Sensors (Strong)

**Interaction**: Each layer has distinct sensor profiles. Surface gets radar+visual+sonar. Submerged gets sonar only. Abyssal gets minimal sensing.

**Designed**: Yes — core pillar "Depth Creates Tactical Space"

**Player Discovery**: Common — learned in first hours

**Depth**: Deep — drives fleet composition, doctrine, EMCON decisions

### Track Quality × Weapons (Strong)

**Interaction**: Fire-control quality (Q2/Q3) required to engage. Low-quality tracks mean you can't shoot, or you waste ammo on ghosts.

**Designed**: Yes — core pillar "Imperfect Information Drives Decisions"

**Player Discovery**: Common — learned when first shot misses

**Depth**: Deep — creates tension between patience and opportunity

### Transitions × Damage (Strong)

**Interaction**: Heavy damage during transition can botch the dive (forced return, stuck, catastrophic failure). Transitioning ships are maximally vulnerable.

**Designed**: Yes — creates commitment cost for layer changes

**Player Discovery**: Uncommon — learned when dive gets punished

**Depth**: Medium — tactical consequence, not strategic

### Weather × TDM (Medium)

**Interaction**: Storms degrade surface sensors AND can disrupt data links, fragmenting the tactical mesh.

**Designed**: Yes — makes weather tactically meaningful beyond visibility

**Player Discovery**: Rare — requires fleet-scale play

**Depth**: Medium — creates windows for coordinated action

### Damage × Morale × Boarding (Strong Chain)

**Interaction**: Damage degrades crew efficiency → morale drops → surrender threshold approaches → boarding becomes viable. The chain is deterministic and traceable.

**Designed**: Yes — pillar "Explainable Causality"

**Player Discovery**: Common — core combat loop

**Depth**: Deep — every engagement involves this chain

## Feedback Loop Map

### Positive Loops

#### Damage Cascade

```text
Damage → Component Loss → Reduced Capability → Harder to Avoid Damage → More Damage
```

- **Risk**: Runaway spiral, battles decided by first hit
- **Mitigation**: Damage control crew allocation, retreat options, morale surrender before total loss

#### Sensor Dominance

```text
Better Tracks → Earlier Engagement → First Strike → Enemy Sensor Loss → Even Better Relative Tracks
```

- **Risk**: Sensor superiority too decisive
- **Mitigation**: EMCON options, EW countermeasures, layer transitions to break contact

#### Information Sharing

```text
More Ships in Mesh → Better Fused Picture → Better Engagement → More Ships Survive → Larger Mesh
```

- **Risk**: Larger fleets always win
- **Mitigation**: EW disruption of mesh, high-value node targeting (fusion nodes)

### Negative Loops

#### Ammunition Depletion

```text
Engagement → Ammo Consumption → Reduced Fire Rate → Reduced Damage Output → Extended Battle → More Ammo Consumption
```

- **Purpose**: Prevents infinite engagement, forces disengagement decisions
- **Risk**: Could cause stagnation if too aggressive

#### Crew Fatigue/Casualty

```text
Combat → Crew Casualties → Reduced Efficiency → Slower Repairs/Firing → Extended Exposure → More Casualties
```

- **Purpose**: Creates time pressure, rewards decisive action
- **Risk**: Could feel punishing; balanced by damage control allocation choices

#### Emission Exposure

```text
Active Sensors → Better Tracks → But Also Detected → Enemy Targeting → Damage → Sensor Loss → Worse Tracks
```

- **Purpose**: Creates EMCON decision tension
- **Risk**: None — this is the core decision loop

### Loop Interactions

The **Damage Cascade** positive loop is balanced by the **Ammunition Depletion** and **Crew Fatigue** negative loops. You can't just keep shooting forever, and taking damage makes your damage control crew work harder.

The **Sensor Dominance** loop is countered by the **Emission Exposure** loop — going active to dominate the picture also exposes you.

## Cascade Chain Analysis

### Chain 1: Storm-Masked Dive Strike

```text
Trigger: Storm enters battle area
→ Step 1: Surface sensors degraded (weather → sensors)
→ Step 2: Surface ships can't track submerged contacts (sensors → tracks)
→ Step 3: Sub approaches undetected (tracks → positioning)
→ Step 4: Sub executes pop-up missile strike (layers → weapons)
→ Step 5: Surface ships damaged before response (weapons → damage)
→ Final: Sub re-submerges in storm cover (weather → escape)
```

**Designed/Emergent**: Emergent from designed interactions

**Player Agency Points**: Surface fleet could have stayed in EMCON and used passive sonar; could have positioned pickets differently; could have retreated from storm zone

### Chain 2: Mesh Collapse Cascade

```text
Trigger: EW attack on fusion node
→ Step 1: Fusion node loses track quality (EW → TDM)
→ Step 2: Distributed ships lose curated picture (TDM → tracks)
→ Step 3: Ships fall back to local sensors only (tracks → sensors)
→ Step 4: Coordination breaks down, fire on same targets or miss others (sensors → weapons)
→ Step 5: Concentrated enemy fire on isolated ships (weapons → damage)
→ Final: Fleet fragments, piecemeal destruction (damage → defeat)
```

**Designed/Emergent**: Designed — this is why TDM tiers and fusion nodes exist

**Player Agency Points**: Could protect fusion node with escorts; could designate backup nodes; could use EMCON to reduce EW targeting data

### Chain 3: Boarding Cascade

```text
Trigger: Capital ship takes critical propulsion damage
→ Step 1: Ship immobilized (damage → movement)
→ Step 2: Cannot escape boarding approach (movement → positioning)
→ Step 3: Assault ships dock and begin transfer (positioning → boarding)
→ Step 4: Internal security engaged, casualties mount (boarding → crew)
→ Step 5: Morale drops below threshold (crew → morale)
→ Final: Surrender or capture (morale → outcome)
```

**Designed/Emergent**: Designed — the damage tier/boarding tier system

**Player Agency Points**: Could scuttle before capture; could call for relief fleet; could negotiate surrender terms

### Chain 4: Transition Ambush

```text
Trigger: Surface ship begins dive to escape
→ Step 1: Massive signature spike during transition (transitions → sensors)
→ Step 2: Enemy tracks quality jumps to Q3 (sensors → tracks)
→ Step 3: Enemy fires during vulnerability window (tracks → weapons)
→ Step 4: Heavy damage during transition (weapons → damage)
→ Step 5: Transition botched, ship stuck at surface (damage → transitions)
→ Final: Ship destroyed unable to complete dive or fight (transitions → outcome)
```

**Designed/Emergent**: Designed — the transition vulnerability window is intentional

**Player Agency Points**: Could have committed earlier before enemy was in range; could have had escort cover the dive; could have accepted surface combat

## Emergence Evaluation

### Designed Interactions

Count: **~45** (from interaction matrix strong connections)

### Emergent Interactions Found (Predicted)

Based on system analysis, these emergent patterns should appear:

1. **Weather-window exploitation** — timing attacks to coincide with sensor-degrading weather
2. **Picket sacrifice** — using cheap units to absorb first strike and reveal enemy positions
3. **Mesh fragmentation tactics** — deliberately separating to survive EW attacks on links
4. **Pop-up missile boats** — subs that specialize in brief surface strikes then immediate dive
5. **Transition screening** — formations where some ships cover others during dive
6. **Bait-and-switch layers** — surface decoy while submerged striker approaches
7. **EMCON wolfpacks** — coordinated silent approach with burst engagement
8. **Arcology shield doctrine** — using the arcology itself as sensor/mesh anchor

Count: **8+** predicted emergent strategies

### Emergence Ratio

8+ / 45 ≈ **0.18** (pre-implementation estimate)

Target: > 1.5 for mature game

**Note**: This ratio will increase dramatically once players and DRL agents explore the system. The interaction density suggests high emergence potential that won't be realized until implementation.

### Community-Discovered Techniques (Predicted)

Based on similar naval/tactical games:

- Optimal transition timing windows for specific ship matchups
- Sensor range "dead zones" created by terrain + layer combinations
- Fleet compositions that counter specific enemy doctrines
- Weather prediction and positioning for storm-masked operations
- TDM link topology optimization for EW resilience

## Gaps and Opportunities

### Isolated Mechanics

| Mechanic | Currently Touches | Could Connect To |
| -------- | ----------------- | ---------------- |
| **Currents** | Movement, Weather | Sensors (sonar propagation), Torpedoes (drift) |
| **Terrain** | Layers, Movement | TDM (line-of-sight relay), Sensors (acoustic shadow) |

### Missing Loops

**Supply Pressure Loop** (campaign layer):

```text
Combat → Ammo/Fuel Consumption → Must Resupply → Leaves Combat Zone → Enemy Recovers
```

This would add strategic depth but belongs in campaign, not arena.

**Reputation/Intel Loop** (campaign layer):

```text
Victory → Intelligence on Enemy Doctrine → Counter-Doctrine Development → Advantage
```

Again, strategic layer concern.

### Cascade Opportunities

**Thermal Layer Exploitation**: Submarines could use thermal layers (thermoclines) to hide from sonar, creating another layer-within-layer positioning decision. Currently folded into "submerged" state; could be expanded.

**Component Repair Chains**: Damaged components could have repair dependencies — can't fix weapons until power is restored, can't restore power until fire is out. Currently implicit in damage control allocation.

## Recommendations

### High Priority

1. **Ensure transition vulnerability feels consequential** — The transition → damage interaction is core to the commitment mechanic. If transitions are too safe, the layer system loses meaning.

2. **Make TDM degradation visible** — Players need to see when their mesh is under attack and fragmenting. UI feedback for "link quality" per ship.

### Medium Priority

1. **Add sonar propagation to currents** — Currents could affect torpedo acquisition and sonar detection ranges, creating another positioning consideration.

2. **Add acoustic shadow zones to terrain** — Islands and reefs could block sonar, creating ambush positions for submarines.

### Low Priority

1. **Expand thermal layer mechanics** — Finer-grained depth positioning within the "submerged" state for advanced play.

2. **Add weather forecasting** — Let players see incoming weather to plan around it, creating anticipation gameplay.

## Technical Notes

### Implementation Order

Systems should be implemented in dependency order:

1. **Movement + Layers** — Foundation for everything
2. **Sensors + Tracks** — Core information warfare
3. **Weapons + Damage** — Combat resolution
4. **TDM + EW** — Fleet-scale play
5. **Weather + Currents** — Environmental variety
6. **Boarding** — Capture mechanics (post-MVP)

### Performance Implications

- Track management scales with O(n²) for n ships — needs efficient spatial indexing
- TDM mesh updates could be batched to reduce per-tick cost
- Weather/current fields can use coarse grids with interpolation

### Testing Requirements

- **Interaction coverage**: Every strong interaction in the matrix needs test scenarios
- **Loop stability**: Verify positive loops don't cause runaway outcomes in < 30 seconds
- **Emergence validation**: Record DRL agent behaviors to find unexpected strategies

## Related Documents

- [Sandbox Design](sandbox-design.md) — How constraints create meaningful choices
- [Sensors and Fog](sensors-and-fog.md) — Information warfare details
- [Layers and Terrain](layers-and-terrain.md) — Depth layer mechanics
- [Damage and Boarding](damage-and-boarding.md) — Combat resolution systems
- [Design Pillars](../vision/pillars.md) — Guiding principles
