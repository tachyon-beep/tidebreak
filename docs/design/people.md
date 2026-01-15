# People System Design

Model a limited set of **key individuals** whose decisions, competence, loyalties, and relationships materially affect fleets, arcologies, factions, and missions—without simulating whole populations at person-level.

**Core Principle**: Named faces exist at the point of interaction; otherwise they're rolled up into aggregate politics. This matches the "world goes on" principle and keeps the simulation lean.

## Canonical Types Used

- **PersonId**: Stable identifier for save/replay
- **FaceId**: Handle for presence-gated person instantiation
- **Role**: XO, Captain, Fleet Commander, Arcology Leader, Security Chief, Diplomat, Spymaster
- **Trait**: Behavioural/competence modifier
- **Loyalty**: 0.0–1.0 alignment to current employer/leader
- **Ambition**: 0.0–1.0 drive for power/status
- **Reputation**: Per-faction standing + global notoriety
- **Grudge**: Recorded grievance with decay
- **PersonStatus**: Employed, Dismissed, Exiled, Missing, Dead

## Non-Goals (Early)

What the people system does NOT model:

- Full crew roster simulation (populations are aggregate)
- Romance/social-sim mechanics
- Procedural dialogue trees
- Detailed skill levelling grind
- Individual citizen tracking

## Scope: Who Gets Modeled?

Hard cap by category (tunable):

| Context | Named People |
| ------- | ------------ |
| **Player fleet** | XO + captains of ships in current fleet + 1–2 advisors |
| **Factions** | Leader + 2–5 cabinet roles (military chief, trade chief, security chief) |
| **Fleets** | Commander + key specialists (optional) |
| **Arcologies** | Leader + internal security head + logistics head (optional) |
| **Internal factions** | One leader per faction type (Militarists have a face, etc.) |

Everyone else is part of `PopulationState` and `InternalFactionsState` (aggregate).

This keeps the simulation sparse but narratively punchy.

## Data Model (Shared Contract)

### PersonState Component

```rust
PersonState {
    person_id:          PersonId
    name:               String
    age:                Option<u16>          // Flavour
    origin_faction:     Option<FactionId>    // Where they "came from"

    // Core stats (keep small!)
    competence:         CompetenceProfile
    traits:             Vec<Trait>           // Small, curated list
    loyalty:            f32                  // 0.0–1.0 to current employer/leader
    ambition:           f32                  // 0.0–1.0

    // Social/political
    reputation:         Map<FactionId, f32>  // -1.0 to 1.0 per faction
    ideology_affinity:  Map<FactionTag, f32> // Links to InternalFactions
    grudges:            Vec<GrudgeRecord>    // Small list, decays

    // Career
    current_role:       Option<RoleAssignment>
    history:            Vec<RoleRecord>      // Recent only, for size
    status:             PersonStatus
}
```

### CompetenceProfile

Keep it tight to avoid "RPG spreadsheet disease" — five domains:

```rust
CompetenceProfile {
    command:        f32     // 0.0–1.0: Fleet tactics, morale effects
    operations:     f32     // 0.0–1.0: Navigation, logistics efficiency
    engineering:    f32     // 0.0–1.0: Repairs, reliability management
    intelligence:   f32     // 0.0–1.0: Sensor doctrine, counter-intel, influence ops
    politics:       f32     // 0.0–1.0: Legitimacy management, faction wrangling, negotiation
}
```

That's enough to cover XO/captains/faction leaders without creating skill trees.

### RoleAssignment

Links person to entity:

```rust
RoleAssignment {
    role:           RoleType
    entity_id:      EntityId         // Ship / fleet / arcology / faction
    start_tick:     Tick
    authority:      f32              // How much they can act without approval
}

enum RoleType {
    // Fleet roles
    FleetCommander,
    Captain,
    XO,
    ChiefEngineer,

    // Arcology roles
    Governor,
    SecurityChief,
    TradeChief,
    LogisticsHead,

    // Faction roles
    FactionLeader,
    MilitaryChief,
    Diplomat,
    Spymaster,

    // Internal faction roles
    InternalFactionLeader { faction_type: FactionType },
}
```

### GrudgeRecord

```rust
GrudgeRecord {
    target:         EntityId         // Person, faction, or player
    cause:          GrudgeCause      // Dismissed, Betrayed, Defeated, etc.
    intensity:      f32              // 0.0–1.0
    created_tick:   Tick
    decay_rate:     f32              // Per-tick reduction
}
```

Grudges decay over time unless reinforced by further grievances.

### PersonStatus

```rust
enum PersonStatus {
    Employed { employer: EntityId },
    Dismissed { former_employer: EntityId, tick: Tick },
    Exiled { from_faction: FactionId },
    Missing,
    Dead { cause: DeathCause, tick: Tick },
}
```

## Traits

Keep trait list curated (~20 max), each with clear mechanical hooks:

### Temperament

| Trait | Effect |
| ----- | ------ |
| **Cautious** | +decision latency, −risk of catastrophic failure |
| **Reckless** | −decision latency, +risk of catastrophic failure |
| **Ruthless** | +crisis suppression effectiveness, −legitimacy on use |
| **Compassionate** | +morale recovery, −willingness to use force |

### Governance Style

| Trait | Effect |
| ----- | ------ |
| **Proceduralist** | +legitimacy from following process, −speed |
| **Populist** | +support from Populist faction, −from Technocrats/Traders |
| **Technocratic** | +efficiency bonuses, −popular appeal |

### Reliability

| Trait | Effect |
| ----- | ------ |
| **Meticulous** | +repair quality, +logistics efficiency, −speed |
| **Improviser** | +crisis response speed, −long-term reliability |

### Social

| Trait | Effect |
| ----- | ------ |
| **Charismatic** | +legitimacy buffer, +recruitment success |
| **Intimidating** | +compliance, +coup deterrence, −morale |
| **Divisive** | +faction polarization, ±intense loyalty/hatred |

### Pathology (Optional, Use Carefully)

| Trait | Effect |
| ----- | ------ |
| **Paranoid** | +coup detection, +false positives, −trust building |
| **Vengeful** | Grudges decay slower, +retaliation priority |
| **Ambitious** | +self-promotion, +coup risk if passed over |

## Integration: Roles as Plugins, People as Inputs

Avoid making people "do everything." Instead:

- Ships/fleets/arcologies/factions have **plugins that read PersonState** of their assigned leaders and apply modifiers.
- People entities can run a small number of behavioural plugins (ambition/loyalty drift, networking) if needed.

### Example Plugin Interactions

```yaml
FleetCommandPlugin:
  reads: [FleetState, AssignedCommander.PersonState]
  emits: [FormationModifier, ReactionLatencyModifier]
  behavior: "Commander's 'command' competence and traits affect fleet cohesion and response time"

ArcologyGovernancePlugin:
  reads: [GovernanceState, Governor.PersonState]
  emits: [LegitimacyModifier, CrisisResponseModifier]
  behavior: "Governor's 'politics' competence and traits affect legitimacy drift and crisis handling"

SecurityChiefPlugin:
  reads: [GovernanceState, SecurityChief.PersonState, InternalFactionsState]
  emits: [CoupDetectionModifier, PurgeEffectivenessModifier]
  behavior: "Security chief's 'intelligence' affects coup detection; ruthlessness affects suppression"

CaptainPlugin:
  reads: [ShipState, Captain.PersonState]
  emits: [CrewMoraleModifier, RepairEfficiencyModifier]
  behavior: "Captain's traits and competence affect ship-level performance"
```

This keeps causality clean: the **role plugin** is the mechanism; the person is the parameter source.

## Presence-Gated Fidelity (Faces)

Named people exist at full detail only when the player is present. Otherwise, they're abstracted into aggregate variables.

### Two-Layer Representation

**Abstract Layer (Always Exists)**

Stored on arcologies/factions:

```rust
FaceRoster {
    governor_face_id:       FaceId
    faction_faces:          Map<FactionType, FaceId>  // One per internal faction
    security_face_id:       Option<FaceId>
    trade_face_id:          Option<FaceId>
}
```

A `FaceId` is a stable handle, not necessarily a live Person entity.

**Concrete Layer (Player Present)**

Instantiate Person entities for:

- Governor
- Each internal faction leader
- Mission-giver "faces"
- Security/trade chiefs (if relevant)

### FaceRecord (Persistent Identity)

```rust
FaceRecord {
    face_id:            FaceId
    person_id:          PersonId            // Stable identity
    name_seed:          u64                 // Reproducible name/appearance
    competence_hint:    CompetenceProfileLite
    traits_hint:        Vec<TraitTag>
    reputation_hint:    f32
    last_known_role:    RoleType
}
```

When player arrives, Person entity is instantiated from this record + current political context.

### Spawning and Despawning

**On Player Arrival**

`SpawnFacesResolver` runs:

1. Reads `FaceRoster` + current political state
2. Materialises Person entities for each FaceId
3. Applies pending "offscreen changes" (e.g., faction leader replaced during coup)
4. Emits events with `trace_id` for visibility/debug

**On Player Departure**

**Recommended**: Persist Person entities but mark them dormant:

- Keep them in world state (cheap, count is small)
- Mark `active = false` and stop running behavioural plugins
- Existence remains stable for replays and saves

Alternative: Despawn but keep FaceRecords (more complex, saves entity count).

### Offscreen Leadership Changes

When player isn't present, leadership changes happen in abstract model:

- Internal faction leader replaced
- Governor assassinated
- Board reshuffle
- Election outcome

Mechanically:

- Arcology updates its `FaceRoster` to point to new `FaceId`
- Logs causality chain event for attribution

When player arrives:

- SpawnFacesResolver instantiates new face
- Surfaces news/event summary: "Governor replaced after emergency election"

## Dismissal, Defection, and Rise

The emergent story potential: **fire your XO → they pop up later as a faction leader**.

### State Transitions Supported

- **Dismissed** (player fires XO/captain)
- **Recruited by rival faction**
- **Assigned to arcology leadership**
- **Elected/appointed leader**
- **Stages a coup**
- **Forms splinter faction**

### Mechanism: Discontent Pressure + Opportunity

When you fire your XO, you don't script their destiny. You create conditions:

1. Grudge increases
2. Reputation shifts (your faction −, rivals potentially +)
3. Loyalty to you becomes irrelevant; ambition persists
4. They become available in the "labour market"
5. Factions looking for leaders may "bid" based on competence and alignment

### Integration with Strategic AI

Handled via existing Mission and Strategic AI system:

- Faction goal: "Improve fleet leadership" or "Destabilise rival arcology"
- Mission/opportunity spawns: "Recruit high-skill exile officer"
- If not taken by player, AI resolves internally

### Election/Coup Hooks

Tie into governance:

- **Democracies**: Candidates compete; person's `politics`, reputation, ideology affinity influence vote
- **Autocracies/Juntas**: `intelligence` + network + disgruntled internal faction influence coup probability
- **Corporate**: Board appointment based on profit performance + reputation among traders/technocrats

## Missions and Faces

### Mission Sponsorship

Every mission gets a sponsor face:

```rust
Mission {
    // ... existing fields ...
    sponsor_face_id: Option<FaceId>  // Who is offering (for UI + modifiers)
}
```

- If face exists (player present): used for UI, dialogue flavour, negotiation modifiers
- If absent: mission listed as "Arcology Council", "Militarist Bloc", etc.

### Delegates

A faction leader doesn't personally hand out all missions:

- **Leader face**: Big politics, sensitive missions
- **Delegate face**: Operations officer, fixer, quartermaster

Keeps "the governor" special while allowing variety.

## Mechanical Effects Summary

### When Embodied (Player Present)

- Negotiation shifts (better rewards, lower penalties)
- Credibility/Intel quality for missions
- Political consequences (insult the governor → legitimacy hit)
- Unique mission chains tied to grudges/ambition
- Detailed trait effects

### When Abstracted (Player Absent)

Effects rolled into aggregate:

- "Mission quality" scalar
- "Faction cohesion" scalar
- "Governor competence" scalar

World remains consistent; faces just add richness when present.

## Attribution and Explainability

Every major people event emits:

- `source_id`, `cause_id`, `trace_id`
- Short "why" summary (for UI/debug)

Example event:

```rust
PersonDefected {
    person_id: PersonId,
    from: EntityId,
    to: FactionId,
    causes: [
        "DismissedAtTick(1234)",
        "GrudgeHigh(0.8)",
        "FactionYNeededLeader",
        "IdeologyAffinityMatch(Militarists: 0.7)"
    ],
    trace_id: TraceId,
}
```

No mystery teleports.

## MVP Staging

### P2 (Minimum Viable People)

- [ ] Person entity + PersonState
- [ ] Roles: XO, Captain, Faction Leader, Arcology Leader
- [ ] Traits: 8–12 core traits
- [ ] Effects: Small modifiers to decision latency, morale, crisis response
- [ ] Basic dismissal and recruitment
- [ ] FaceRecord persistence

### P3 (The Fun Escalates)

- [ ] Succession rules per government type
- [ ] Elections / board appointments / coup attempts
- [ ] Grudges + revenge missions
- [ ] "Exile market" where factions recruit talent
- [ ] Person-to-person networks
- [ ] Delegate faces for mission variety

## Related Documents

- [Governance Design](governance.md) — How people integrate with government systems
- [Entity Framework](entity-framework.md) — People as entities with PersonState
- [Damage and Boarding](damage-and-boarding.md) — Key personnel as boarding objectives
- [Glossary](../vision/glossary.md) — Canonical terminology
