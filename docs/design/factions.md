# Factions Design

Factions are the political actors of the world—organizations that control arcologies, fleets, and territory. They have philosophies, resources, relationships, and goals that drive the strategic layer.

**Core Principle**: Factions are autonomous agents that pursue their own interests. The player can ally with, oppose, or ignore factions, but the factions act regardless.

**MVP Note**: The faction system is designed for expansion but initially stubbed with 2 test factions. The framework supports many factions; content is intentionally minimal until other systems are proven.

## Canonical Types Used

- **FactionId**: Stable identifier
- **Philosophy**: Faction worldview affecting decisions
- **FactionState**: Resources, holdings, goals, relationships
- **Disposition**: Faction-to-faction relationship (-1.0 to 1.0)
- **Treaty**: Formal agreement between factions

## Faction vs. Arcology vs. Internal Faction

Three related but distinct concepts:

| Concept | Scope | Example |
|---------|-------|---------|
| **Faction** | Multi-entity political organization | "The Thalassic Accord" |
| **Arcology** | Single city-ship with its own government | "New Providence" (governed by Representative Democracy) |
| **Internal Faction** | Population subgroup within an arcology | "The Militarists of New Providence" |

A faction may control multiple arcologies, each with different government types and internal factions. Arcologies can defect, be conquered, or rebel without destroying the faction.

## Faction Framework

### FactionState Component

```rust
FactionState {
    faction_id:         FactionId
    name:               String
    philosophy:         Philosophy

    // Holdings
    capital:            Option<ArcologyId>  // Primary seat of power
    arcologies:         Vec<ArcologyId>
    fleets:             Vec<FleetId>
    platforms:          Vec<PlatformId>

    // Resources (aggregate of holdings)
    resources:          ResourcePool
    action_points:      ActionPoints        // From governance.md

    // Strategy
    strategic_goals:    Vec<StrategicGoal>
    active_missions:    Vec<MissionId>

    // Relationships
    dispositions:       Map<FactionId, Disposition>
    treaties:           Vec<Treaty>

    // Player relationship
    player_reputation:  f32                 // -1.0 to 1.0
    player_history:     Vec<ReputationEvent>

    // Leadership (links to People system)
    leader_face_id:     FaceId
    cabinet:            Vec<(RoleType, FaceId)>
}
```

### Philosophy

Philosophy defines faction worldview and decision biases:

```rust
Philosophy {
    // Core values (0.0-1.0 weight)
    militarism:         f32     // Preference for force
    commercialism:      f32     // Preference for trade
    isolationism:       f32     // Preference for independence
    expansionism:       f32     // Desire for growth
    traditionalism:     f32     // Resistance to change

    // Derived behaviors
    aggression_bias:    f32     // How easily they go to war
    alliance_preference: f32    // How readily they ally
    risk_tolerance:     f32     // Strategic risk-taking
}
```

Philosophy affects:

- Strategic goal selection
- Disposition drift rates
- Treaty preferences
- Crisis response style

### Strategic Goals

What factions pursue (from governance.md):

```rust
enum GoalType {
    Expand,         // Acquire new territory
    Defend,         // Protect existing holdings
    Trade,          // Maximize economic output
    Weaken,         // Damage a specific rival
    Survive,        // Faction under existential threat
}

StrategicGoal {
    goal_type:      GoalType
    target:         Option<Target>  // Specific faction, region, or resource
    priority:       f32             // 0.0-1.0
    progress:       f32             // How close to completion
}
```

### Disposition

Relationship between factions:

```rust
Disposition {
    faction_a:      FactionId
    faction_b:      FactionId
    value:          f32             // -1.0 (war) to 1.0 (alliance)
    trend:          f32             // Recent direction of change
    recent_events:  Vec<EventId>    // What's affecting it
}
```

Disposition thresholds (from governance.md):

| Range | Status | Behaviors |
|-------|--------|-----------|
| 0.7+ | Allied | Joint missions, shared intel, mutual defense |
| 0.3–0.7 | Friendly | Trade, non-aggression, occasional cooperation |
| −0.3–0.3 | Neutral | Cautious interaction, no commitments |
| −0.7–−0.3 | Unfriendly | Trade restrictions, border incidents |
| < −0.7 | Hostile | Active conflict, war |

### Disposition Events

Disposition shifts based on actions (strategic tick = 1 day, see [glossary](../vision/glossary.md#time--ticks)):

| Event | Disposition Change |
|-------|-------------------|
| Joint victory | +0.05 to +0.10 immediate |
| Honored treaty | +0.02 per week (~+0.005/day) |
| Broke treaty | −0.30 immediate |
| Attacked faction ships | −0.20 to −0.50 immediate |
| **Contested capture** | −0.10 to −0.25 immediate |
| Withdrew from contested capture | +0.05 immediate (goodwill) |
| Negotiated split | No change or +0.02 |
| Standoff (contested capture) | −0.01 per day until resolved |

**Contested Capture**: When allied factions both have troops aboard a captured megaship, it generates disposition tension. Someone has to blink:
- **Withdraw**: One faction pulls out, takes the reputation hit but preserves alliance
- **Negotiate**: Split the prize (one gets the ship, other gets salvage rights, trade concessions, or debt)
- **Standoff**: Neither blinks, disposition decays (−0.01 per day) until alliance fractures or fighting breaks out

This creates emergent political drama from mechanical systems—alliances can shatter over a prize neither wants to give up.

### Treaties

Formal agreements:

```rust
Treaty {
    treaty_id:      TreatyId
    parties:        Vec<FactionId>
    treaty_type:    TreatyType
    terms:          Vec<TreatyTerm>
    duration:       Option<Tick>    // None = indefinite
    signed_tick:    Tick
}

enum TreatyType {
    NonAggression,
    TradeAgreement,
    MutualDefense,
    Ceasefire,
    Vassalage,
}
```

## Faction Archetypes

Factions have archetypes that combine philosophy, tech focus, and starting conditions.

### Archetype Framework

```rust
FactionArchetype {
    name:               String
    philosophy:         Philosophy
    tech_focus:         Vec<TechTag>        // What they're good at
    economic_focus:     Vec<ResourceType>   // What they produce
    military_doctrine:  DoctrineType        // How they fight
    starting_holdings:  HoldingsTemplate
}
```

### Archetype Examples

| Archetype | Philosophy | Tech Focus | Military Doctrine |
|-----------|------------|------------|-------------------|
| **Maritime League** | Commercial, expansionist | Trade, logistics | Convoy protection, economic warfare |
| **Steel Covenant** | Militarist, traditionalist | Weapons, armor | Direct assault, attrition |
| **Horizon Collective** | Isolationist, technocratic | Sensors, stealth | Ambush, asymmetric |
| **Free Ports** | Commercial, neutral | Salvage, repair | Defensive, mercenary |
| **Land Dominion** | Expansionist, militarist | Shipbuilding, heavy industry | Fleet actions, blockade |

## Test Factions (MVP)

For initial testing, two minimal factions:

### Faction A: "The Accord"

```yaml
name: "The Thalassic Accord"
philosophy:
  militarism: 0.3
  commercialism: 0.7
  isolationism: 0.2
  expansionism: 0.4
  traditionalism: 0.3
tech_focus: [trade, logistics]
economic_focus: [food, fuel]
military_doctrine: defensive
starting_holdings:
  arcologies: 1
  fleets: 1
  platforms: 2
notes: "Ocean-based trading faction. Good for testing trade, diplomacy, missions."
```

### Faction B: "The Dominion"

```yaml
name: "The Iron Dominion"
philosophy:
  militarism: 0.8
  commercialism: 0.2
  isolationism: 0.3
  expansionism: 0.7
  traditionalism: 0.5
tech_focus: [weapons, shipbuilding]
economic_focus: [materials, salvage]
military_doctrine: aggressive
starting_holdings:
  arcologies: 1
  fleets: 2
  platforms: 1
  land_base: 1  # Controls a shipyard
notes: "Land-based military faction. Good for testing conflict, blockades, conquest."
```

### Test Scenarios

These two factions enable testing:

| System | Test |
|--------|------|
| **Governance** | Different faction philosophies affect AI decisions |
| **Missions** | Factions generate different mission types |
| **Economy** | Trade routes between factions, blockade mechanics |
| **Combat** | Fleet encounters, territory disputes |
| **Diplomacy** | Treaties, disposition drift, war/peace |
| **People** | Faction leaders, defection, grudges |

## Faction AI

Factions use the Strategic AI Economy from governance.md:

1. **Accumulate action points** based on economic/military strength
2. **Evaluate goals** against current situation
3. **Spend points** on actions (invasions, diplomacy, missions, etc.)
4. **React to events** (attacks, opportunities, crises)

### Decision Process

```yaml
per_tick:
  1. Update strategic assessment (threats, opportunities)
  2. Adjust goal priorities based on situation
  3. If action_points >= threshold:
     - Select highest-priority affordable action
     - Execute (spawn mission, launch fleet, send diplomat)
  4. Process incoming events (attacks, treaty offers)
  5. Update dispositions based on recent events
```

### Grace Periods

From governance.md, factions have grace periods:

```yaml
grace_periods:
  invasion:           90 days     # No faction invasions until day 90
  alliance_formation: 60 days     # Alliances can't form immediately
  player_targeting:   30 days     # Factions won't target new players
```

## Faction Morale

Faction-wide morale affects all ships and populations. It flows into individual ship morale calculations before battles.

### Morale State

```rust
FactionMorale {
    base_morale:        f32             // 0.0-1.0, from economy and legitimacy
    battle_modifier:    f32             // Recent victories/defeats
    attrition_modifier: f32             // Sustained losses over time
    effective_morale:   f32             // Combined value, clamped 0.0-1.0
}
```

### Morale Sources

| Source | Effect |
|--------|--------|
| Economic prosperity | +morale when prosperous, −morale when struggling |
| Battle victories | +0.05 to +0.15 per victory (scaled by decisiveness) |
| Battle defeats | −0.05 to −0.20 per defeat (worse for surrenders) |
| Ship losses | −morale proportional to fleet percentage lost |
| Territory gained | +morale for expansion |
| Territory lost | −morale for contraction |
| Leader traits | Charismatic leaders buffer morale losses |

### Battle Results Flow

After each battle, `TeamOutcome.morale_delta` from [combat-arena.md](combat-arena.md) is applied:

```rust
faction.morale.battle_modifier += team_outcome.morale_delta
faction.morale.battle_modifier *= 0.95  // Decay toward baseline over time
```

Catastrophic defeats (many surrenders, flagship lost) can cause faction-wide morale collapse, affecting all subsequent battles until recovery.

### Morale Effects

| Morale Level | Effects |
|--------------|---------|
| > 0.8 | Crews fight to the death, +accuracy, −surrender chance |
| 0.5–0.8 | Normal operations |
| 0.3–0.5 | Crews surrender earlier, −accuracy, recruitment harder |
| < 0.3 | Mass surrenders, desertion risk, missions fail more often |

## Player-Faction Interaction

### Reputation

Player reputation with each faction (-1.0 to 1.0):

| Range | Status | Effects |
|-------|--------|---------|
| 0.8+ | Trusted | Inner circle missions, best prices, alliance possible |
| 0.6–0.8 | Friendly | Sensitive missions, good prices |
| 0.3–0.6 | Neutral | Standard missions, standard prices |
| 0.0–0.3 | Suspicious | Limited missions, poor prices |
| −0.3–0.0 | Unwelcome | No missions, hostile pricing |
| < −0.3 | Hostile | Attacked on sight, bounties |

### Reputation Events

```rust
ReputationEvent {
    tick:           Tick
    event_type:     ReputationEventType
    delta:          f32
    description:    String
}

enum ReputationEventType {
    MissionComplete,
    MissionFailed,
    MissionAbandoned,
    AttackedFactionShip,
    DefendedFactionShip,
    TradedWith,
    BrokeAgreement,
    SharedIntel,
    // etc.
}
```

### Working for Multiple Factions

Players can work for multiple factions, but:

- Attacking one faction's ships hurts reputation
- Completing missions that harm Faction B while working for Faction A has consequences
- Factions notice and react to double-dealing

## Faction Defeat and Respawn

From governance.md:

### Defeat Conditions

A faction is defeated when:

- All arcologies conquered or destroyed
- No remaining fleets
- Leader killed/captured with no succession

### Respawn Conditions

```yaml
respawn_conditions:
  min_days_dead:      120         # Must be gone this long
  max_respawns:       3           # Per faction per game
  requires:
    - Surviving population somewhere
    - Allied faction willing to host
    - OR hidden base/exile fleet
```

Factions return as insurgents, exiles, or reconstituted governments.

## Data Contracts

See `FactionState` above. Additional contracts:

### FactionSnapshot (for Combat Arena)

When battles occur, faction context travels in BattlePackage:

```rust
FactionContext {
    faction_id:     FactionId
    philosophy:     Philosophy          // Affects AI behavior
    tech_tags:      Vec<TechTag>        // Affects ship capabilities
    morale_state:   f32                 // Faction-wide effective morale (0.0-1.0)
    at_war_with:    Vec<FactionId>      // IFF context
}
```

The `morale_state` here is `FactionMorale.effective_morale` from the Faction Morale section. It affects individual ship morale calculations (see [combat-arena.md](combat-arena.md)) and surrender thresholds (see [damage-and-boarding.md](damage-and-boarding.md)).

## MVP Staging

### P2 (Stub Factions)

- [ ] FactionState component
- [ ] 2 test factions with minimal content
- [ ] Basic disposition system
- [ ] Player reputation tracking
- [ ] Integration with governance action points

### P3 (Full Factions)

- [ ] 4-6 distinct factions with unique philosophies
- [ ] Treaty system
- [ ] Faction AI goal pursuit
- [ ] Faction defeat and respawn
- [ ] Complex multi-faction diplomacy

## Related Documents

- [Governance Design](governance.md) — How factions interact with governments
- [Economy Design](economy.md) — Faction economic specializations
- [People Design](people.md) — Faction leaders and personnel
- [World Requirements](../requirements/world.md) — Faction requirements
- [Glossary](../vision/glossary.md) — Canonical terminology
