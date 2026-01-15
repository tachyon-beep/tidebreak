# Governance Design

Governance models how arcology-ships and factions make decisions, interact with each other, and offer opportunities to the player. The world is politically alive—factions pursue goals, form alliances, and go to war whether or not the player is involved.

**Core Principle**: The player can interact with politics, but politics happens around them. Factions have their own ships, resources, and agendas. The player is an actor in the world, not the center of it.

## Canonical Types Used

- **GovernmentType**: Autocracy, Junta, Corporate, DirectDemocracy, Representative
- **Legitimacy**: 0.0–1.0 government authority measure
- **PoliticalCapital**: Spendable resource for unpopular decisions
- **InternalFaction**: Population subgroup with goals and influence
- **Mission**: Task offered by a faction, fulfillable by player or faction assets
- **Disposition**: Faction-to-faction or faction-to-player relationship

## Governance Scope

Governance applies at two scopes:

- **Arcology Governance**: How a specific arcology-ship makes internal decisions (civil policy, security posture, crisis response). Modeled via `GovernanceState` component attached to arcology entities.
- **Faction Governance**: How a faction allocates strategic resources across its holdings (fleets, platforms, diplomacy). Modeled via `FactionPolicyState` component attached to faction entities.

By default, a small faction may have a 1:1 relationship (one arcology is the faction). Larger factions may have multiple arcologies with local governance that feeds into faction-level strategy.

**Example**: The Thalassic Accord (faction) contains three arcology-ships, each with its own government type. The faction sets strategic priorities; individual arcologies execute within their political constraints. If one arcology is conquered, it may defect without collapsing the entire faction.

## Time Basis

Governance operates on the **strategic layer tick**. For design purposes:

- **1 strategic tick = 1 day** (see [glossary](../vision/glossary.md#time--ticks))
- Decision latency is expressed in strategic ticks
- A "10-tick decision" takes 10 days of game time

Combat Arena ticks are separate (faster). Governance doesn't run during tactical combat—the `BattlePackage` carries relevant modifiers, and `BattleResult` returns facts for governance to interpret.

## Non-Goals (Scope Control)

What governance does NOT model:

- **Detailed ideology spectra**: Internal factions capture political variation; no need for multi-axis political compass
- **Full constitutional law**: No parliament simulator or legislative drafting mini-game
- **Long-horizon technological progress**: No global R&D tech tree; technology is about salvage exploitation and efficiency
- **Perfectly realistic economics**: Economy supports gameplay pacing and meaningful choices, not academic accuracy
- **Individual citizen simulation**: Populations are aggregated; no tracking of 10,000 individual opinions
- **Real-time speech/debate**: Governance decisions resolve over ticks, not interactive dialogue trees

## Design Principles

### The World Goes On

Factions are autonomous agents with goals, resources, and decision procedures. They:

- Pursue strategic objectives (expand territory, secure resources, weaken rivals)
- Assign their own ships and assets to missions
- Form and break alliances based on interests
- React to player actions but don't depend on them

The player is one actor among many. If the player ignores a faction's problems, that faction solves them (or fails) using its own resources.

### Governments as Entity Bundles

Following the entity framework, government types are **plugin bundles** attached to arcology entities:

```text
Arcology Entity
├── Hull components (physical ship)
├── Population components (crew, civilians)
├── Economy components (production, trade)
└── Governance bundle (varies by type)
    ├── DecisionPlugin (how decisions are made)
    ├── LegitimacyPlugin (authority calculations)
    ├── FactionPlugin (internal politics)
    └── MissionPlugin (external interactions)
```

Changing government type = swapping the governance bundle.

**Parameterisation**: Bundles are not fixed templates. They have parameters (latency ranges, thresholds, override costs, legitimacy curves) that can vary based on leader traits, institutional history, and cultural factors. Two autocracies can behave quite differently based on their configuration.

### Missions as the Interaction Layer

Missions are the primary way factions interact with each other and the player:

- Factions generate missions based on their goals and problems
- Missions can be fulfilled by **faction assets** (NPC ships) or **offered to the player**
- Player access to missions depends on reputation and physical presence
- Completed missions affect faction relationships, resources, and world state

## Government Types

Each government type has distinct decision-making characteristics:

| Type | Decision Speed | Legitimacy Curve | Crisis Response | Vulnerability |
| ---- | -------------- | ---------------- | --------------- | ------------- |
| **Autocracy** | Instant | Fragile (personality-dependent) | Fast but erratic | Assassination, coup |
| **Military Junta** | Fast (military), Slow (civil) | Stable under threat | Excellent | Overextension, civil unrest |
| **Corporate Meritocracy** | Fast (profitable), Slow (else) | Profit-dependent | Reactive | Market collapse, corruption |
| **Direct Democracy** | Very slow | Very high | Slow but resilient | Paralysis, populism |
| **Representative Democracy** | Moderate | Moderate | Deliberate | Gridlock, scandal |

### Decision Procedures

Governments don't just have different speeds—they have different **procedures**:

**Autocracy**:

```yaml
procedure: single_authority
decision_latency: 0-1 ticks
constraints:
  - Leader personality affects all decisions
  - No consultation required
  - High variance in quality
override_modes: [decree]  # Can always decree, but may trigger coup
legitimacy_cost: Low (if successful), Catastrophic (if failed)
```

**Military Junta**:

```yaml
procedure: command_council
decision_latency: 1-3 ticks (military), 5-10 ticks (civil)
constraints:
  - Military decisions fast-tracked
  - Civil decisions require consensus
  - Factions = military branches
override_modes: [martial_law, emergency_powers]  # Military can override civil, high legitimacy cost
legitimacy_cost: Low (military), High (civil overreach)
```

**Corporate Meritocracy**:

```yaml
procedure: profit_analysis
decision_latency: 2-5 ticks
constraints:
  - ROI calculation for all decisions
  - Shareholder interests weighted
  - Short-term bias
override_modes: [executive_action, hostile_takeover]  # Board can override, but stakeholders react
legitimacy_cost: Tied to quarterly performance
```

**Direct Democracy**:

```yaml
procedure: popular_vote
decision_latency: 10-20 ticks
constraints:
  - All major decisions require referendum
  - High participation legitimacy boost
  - Vulnerable to misinformation
override_modes: [emergency_vote]  # Fast referendum only, no executive override
legitimacy_cost: Very low (people chose this)
```

**Representative Democracy**:

```yaml
procedure: legislative_process
decision_latency: 5-15 ticks
constraints:
  - Coalition building required
  - Opposition can delay/block
  - Compromise outcomes
override_modes: [executive_order, state_of_emergency]  # Limited executive override, requires ratification
legitimacy_cost: Moderate (mandate-dependent)
```

### Decision Queue

Each government maintains a **decision queue**—pending choices that move through the procedure:

```rust
DecisionQueue {
    pending:    Vec<Decision>       // Awaiting procedure
    in_process: Option<Decision>    // Currently being decided
    blocked:    Vec<Decision>       // Stalled (gridlock, veto, etc.)
    completed:  Vec<Decision>       // Recently resolved (for history)
}
```

The GovernanceResolver advances the queue each tick based on government type rules.

## Legitimacy

Legitimacy measures government authority. High legitimacy = compliance. Low legitimacy = resistance, unrest, potential collapse.

### Legitimacy Sources

| Source | Effect | Government Sensitivity |
| ------ | ------ | ---------------------- |
| **Victory** | +Major | All (especially Junta) |
| **Defeat** | −Major | All (especially Autocracy) |
| **Prosperity** | +Moderate | Corporate, Democracy |
| **Hardship** | −Moderate | All |
| **Crisis Response** | ±Variable | Depends on outcome |
| **Popular Decision** | +Minor | Democracy types |
| **Unpopular Decision** | −Minor to −Major | Democracy types |
| **Time in Power** | Decay toward baseline | All |

### Legitimacy Thresholds

```yaml
thresholds:
  stable: 0.6+        # Normal operations
  strained: 0.4-0.6   # Unrest, protests, inefficiency
  crisis: 0.2-0.4     # Active resistance, coup risk
  collapse: <0.2      # Government transition imminent
```

### Political Capital

Political capital is a spendable resource for pushing through unpopular decisions:

```rust
PoliticalCapital {
    current:    f32         // Available to spend
    max:        f32         // Cap (varies by government type)
    regen_rate: f32         // Per-tick recovery
}
```

- Spending political capital reduces legitimacy cost of a decision
- Regenerates slowly over time
- Some governments have more capacity (Autocracy) but higher risk

### Mechanical Effects

Variables must do something or they become decorative numbers.

**Legitimacy modifies**:

- Compliance rate (how quickly population follows directives)
- Productivity efficiency (output multiplier for economy)
- Unrest risk (probability of spontaneous protests, strikes)
- Coup/mutiny resistance (threshold for internal faction action)
- Surrender negotiation (governments with high legitimacy don't surrender easily)

**Political Capital enables**:

- Spending capital to reduce legitimacy loss from unpopular decisions
- Accelerating decisions through procedural bottlenecks
- Overriding procedural blockers (where government type allows)
- Forcing through crisis responses without normal consultation

**Baseline drift**: Legitimacy tends to drift toward a government-type baseline over time unless reinforced by events (victories, prosperity) or degraded by failures (defeats, scandals, hardship). Autocracies have lower baselines but higher variance; democracies have higher baselines but slower recovery.

## Internal Factions

Arcology populations aren't monolithic. Internal factions represent interest groups:

### Faction Types

Internal factions come in two flavors that can overlap:

**Ideological Factions** (what you believe):

| Faction | Goals | Supports | Opposes |
| ------- | ----- | -------- | ------- |
| **Militarists** | Security, expansion | Defense spending, aggressive policy | Trade deals, demilitarization |
| **Traders** | Profit, open routes | Trade agreements, neutrality | War, isolationism |
| **Technocrats** | Efficiency, systems optimization | Infrastructure, automation, salvage exploitation | Tradition, populism |
| **Populists** | Equality, welfare | Social programs, redistribution | Elite privilege, austerity |
| **Traditionalists** | Stability, heritage | Status quo, slow change | Radical reform, foreign influence |

**Vocational Factions** (what you do):

| Faction | Domain | Goals | Leverage |
| ------- | ------ | ----- | -------- |
| **Engine Collective** | Propulsion, power | Recognition, safety standards, fair shifts | Movement stops without them |
| **Life Support Guild** | Air, water, waste | Funding, autonomy, hazard pay | Everyone dies without them |
| **Dock Workers Union** | Cargo, trade, repairs | Fair wages, job security | Trade halts without them |
| **Agricultural Syndicate** | Kelp farms, fisheries, food | Land rights (deck space), water allocation | Food supply |
| **Security Corps** | Internal policing, defense | Authority, equipment, respect | Order collapses without them |

**Orthogonal Membership**: A person can belong to both axes—an engineer might be ideologically Populist but also a member of the Engine Collective. Sometimes these align (Populists + Lower Decks unions), sometimes they conflict (a Traditionalist dock worker whose guild is striking).

**Vocational vs Ideological Demands**:
- Ideological factions want *policy changes* (more defense spending, wealth redistribution)
- Vocational factions want *recognition and resources* (better conditions, more say in decisions affecting their work)
- A government can satisfy one without the other—giving Populists welfare programs doesn't help the Engine Collective if their shifts are still 16 hours

### Population Demographics

We don't model 20,000 individuals on an arcology—we model **aggregate demographics** (Stellaris-style pops):

```rust
PopulationState {
    total_population:   u32

    // Vocational distribution (sums to 1.0)
    vocational_makeup:  Map<VocationType, f32>   // e.g., {Engineering: 0.15, Agriculture: 0.20, ...}

    // Ideological distribution (sums to 1.0)
    ideological_makeup: Map<IdeologyType, f32>   // e.g., {Populist: 0.35, Trader: 0.25, ...}

    // Cross-tabulation for important combinations
    demographic_blocks: Vec<DemographicBlock>
}

DemographicBlock {
    vocation:       VocationType
    ideology:       IdeologyType
    population:     u32
    satisfaction:   f32
    radicalization: f32
}
```

**What This Enables**:
- "40% of engineers are Populists" → Engine Collective strikes have Populist political flavor
- "The Docks are mostly Traders" → Dock Workers Union supports trade agreements
- "Security Corps is split between Militarists and Traditionalists" → Internal tension during policy debates

**Demographic Drift**:
- **Policy effects**: Pro-labor policies shift workers toward satisfaction; austerity radicalizes them
- **Economic conditions**: Prosperity reduces radicalization; scarcity increases it
- **Migration**: People move between arcologies, bringing their demographics with them
- **Generational**: Over long timescales, ideology shifts based on lived experience

**Simplified Tracking** (MVP):
- Track vocational and ideological distributions separately
- Only compute cross-tabulation for narratively important moments (strikes, coups, elections)
- Named individuals (Faces) represent demographic blocks for player interaction

This gives us Stellaris-style population politics without simulating individuals—we know the Engine Collective is 60% Populist without tracking each engineer's beliefs.

### Structural Power

Not all political power comes from ideology or numbers—some factions derive influence from **physical control of critical systems**:

| System Control | Leverage |
|----------------|----------|
| **Engineering/Propulsion** | "We move the ship. Ignore us at your peril." |
| **Life Support** | "Everyone breathes because we keep the scrubbers running." |
| **Power Generation** | "We can make decisions... difficult to implement." |
| **Food Production** | "The kelp farms feed everyone. Remember that." |
| **Docks/Trade** | "All your imports come through us." |

This creates **deck-based factions** where lower decks (engineering, life support, heavy industry) often develop distinct political identities from upper decks (command, administration, luxury quarters).

**Lower Decks Politics**:
- May align with Populists ideologically but have independent leverage
- Can threaten work stoppages or "efficiency reductions" without open rebellion
- In Condominium arcologies, may become a third faction: "We don't care who controls the bridge—we control whether it goes anywhere"
- Historical parallel: shipboard unions, engine room crews, essential workers

**Upper Decks vs Lower Decks** tension is a recurring theme—those who give orders vs those who make the ship actually function. Government types handle this differently:
- **Autocracy**: Suppresses lower deck organizing; risks catastrophic mutiny
- **Democracy**: Lower decks have voting power proportional to population
- **Corporate**: Lower decks are "labor costs" to be minimized; breeds resentment
- **Junta**: Military engineering corps may bridge the divide

### Faction Mechanics

```rust
InternalFaction {
    faction_type:   FactionType
    faction_axis:   FactionAxis     // Ideological or Vocational

    // Derived from PopulationState demographics
    population_share: f32           // What % of population belongs to this faction
    structural_multiplier: f32      // Vocational factions get leverage bonus (1.0-2.0)
    effective_influence: f32        // population_share × structural_multiplier

    // Tracked per faction
    satisfaction:   0.0-1.0         // With current government
    radicalization: 0.0-1.0         // Willingness to act outside system
    leader:         Option<PersonId> // Named leader (for events)
}

enum FactionAxis {
    Ideological,    // Influence = population %
    Vocational,     // Influence = population % × structural leverage
}
```

**Influence is Demographic**:
- Ideological faction influence = percentage of population with that ideology
- Vocational faction influence = percentage of workforce in that vocation × structural multiplier
- If 35% of the population is Populist, Populists have 35% ideological influence
- If 15% work in Engineering but they control propulsion, Engine Collective might have 15% × 1.5 = 22.5% effective influence

**Structural Multiplier** (for vocational factions):

| Vocation | Multiplier | Rationale |
|----------|------------|-----------|
| Engineering | 1.5 | Ship doesn't move without them |
| Life Support | 1.8 | Everyone dies without them (but hard to actually use) |
| Docks | 1.3 | Trade and resupply disrupted |
| Agriculture | 1.4 | Food supply threatened |
| Security | 1.2 | Order maintained (but using leverage risks mutiny label) |
| Administration | 1.0 | No structural leverage |

This means a small but critical workforce (Engineering at 10% of population) can punch above their weight politically.

LeaderTraits {
    risk_tolerance: f32     // -1.0 (risk averse) to 1.0 (gambler)
    cruelty:        f32     // 0.0 (merciful) to 1.0 (ruthless)
    pragmatism:     f32     // 0.0 (ideological) to 1.0 (practical)
    paranoia:       f32     // 0.0 (trusting) to 1.0 (suspicious)
    charisma:       f32     // 0.0 (uninspiring) to 1.0 (magnetic)
}
```

**LeaderTraits** parameterize government bundle behavior. A paranoid autocrat makes different decisions than a trusting one—even with the same procedure and constraints. Traits modify:

- Decision latency (paranoia increases deliberation)
- Override willingness (risk tolerance affects when leaders push through)
- Legitimacy sensitivity (charisma provides buffer)
- Crisis response style (cruelty affects suppression vs. accommodation)
- Faction management (pragmatism affects coalition-building)

**Influence** determines voting weight in democracies and coup likelihood in autocracies.

**Satisfaction** affects legitimacy. Unhappy factions erode government authority.

**Radicalization** determines whether factions work within the system (protests, voting) or outside it (sabotage, mutiny, coup).

### Faction Drift

Factions respond to government decisions:

```yaml
on_decision:
  if faction.supports(decision):
    satisfaction += 0.05
    radicalization -= 0.02
  if faction.opposes(decision):
    satisfaction -= 0.05
    radicalization += 0.02
  if faction.ignored(decision):  # Their issue, not addressed
    satisfaction -= 0.02
    radicalization += 0.01
```

Over time, radicalized factions may attempt coups, defections, or civil unrest.

### Faction Presentation by Radicalization

Radicalization changes how factions present themselves to the world. The same underlying goals manifest through different organizational forms:

#### Low Radicalization (0.0–0.3): Civic Organizations

Factions operate as legitimate interest groups within the system:

| Faction | Presentation | Activities |
| ------- | ------------ | ---------- |
| **Militarists** | Veterans' associations, defense policy institutes, naval academies | Lobbying, white papers, memorial events |
| **Traders** | Merchant guilds, chamber of commerce, trade associations | Trade fairs, business networking, market reports |
| **Technocrats** | Engineering societies, salvage analysis guilds, efficiency consultancies | Conferences, technical review, advisory boards |
| **Populists** | Labor unions, mutual aid societies, civic clubs | Community events, petitions, public forums |
| **Traditionalists** | Heritage societies, historical preservation groups, cultural clubs | Festivals, museums, oral history projects |

*Tone*: Professional, respectable, working within the system. Members are open about affiliation.

#### Medium Radicalization (0.3–0.6): Ideological Movements

Factions develop stronger identity and insider/outsider dynamics:

| Faction | Presentation | Activities |
| ------- | ------------ | ---------- |
| **Militarists** | Warrior lodges, honor brotherhoods, "old guard" networks | Initiation rituals, loyalty oaths, informal command structures |
| **Traders** | Merchant cartels, "old money" families, exclusive trading houses | Secret deals, market manipulation, blacklists |
| **Technocrats** | Meritocratic orders, "enlightened" circles, optimization cults | Manifestos, selective recruitment, systems redesign |
| **Populists** | Revolutionary councils, workers' communes, underground presses | Strikes, sabotage threats, parallel governance |
| **Traditionalists** | Religious revivals, ancestral cults, purity movements | Sermons, moral panics, shunning of "outsiders" |

*Tone*: Us-vs-them mentality emerging. Meetings behind closed doors. Symbols and rituals matter. Members may hide affiliation from outsiders.

#### High Radicalization (0.6–1.0): Extremist Cells

Factions abandon the political process for direct action:

| Faction | Presentation | Activities |
| ------- | ------------ | ---------- |
| **Militarists** | Coup plotters, junta-in-waiting, death squads | Assassination, military takeover, purges |
| **Traders** | Pirate syndicates, smuggling cartels, economic warlords | Extortion, blockade running, hostile takeovers |
| **Technocrats** | Techno-fascists, "necessary measures" cabals, automation absolutists | Infrastructure sabotage, forced "optimization," population "efficiency" |
| **Populists** | Revolutionary cells, mob rule, guillotine enthusiasts | Riots, show trials, wealth seizure, class warfare |
| **Traditionalists** | Inquisitions, ethnic cleansers, fundamentalist militias | Pogroms, book burning, forced conversion |

*Tone*: Violence is justified. The ends justify the means. Members use code names. Open warfare against the current order.

### Radicalization Signals

Players can read faction radicalization through observable cues:

```yaml
low_radicalization:
  - Public membership rolls
  - Open meeting announcements
  - Cooperation with government
  - Professional language in communications

medium_radicalization:
  - Membership by invitation only
  - Coded meeting locations
  - Criticism of government legitimacy
  - Symbolic language, in-group references

high_radicalization:
  - Cell structure, compartmentalized knowledge
  - Secret communications, dead drops
  - Calls for direct action, "by any means necessary"
  - Dehumanizing language about opponents
```

These signals appear in news events, intercepted communications, and NPC dialogue—giving players warning before a faction acts.

## Information Environment

Arcologies track an abstract **InfoIntegrity (0.0–1.0)** representing trust in communications, news, and internal coordination.

```rust
InfoEnvironment {
    integrity:          f32         // 0.0-1.0, overall information health
    comms_quality:      f32         // External communication reliability
    internal_trust:     f32         // Population trust in official sources
    rumor_velocity:     f32         // How fast misinformation spreads
}
```

### Sources of Degradation

- **External isolation**: Cut data links, no allied contact, blockade
- **EW jamming**: Active electronic warfare degrades comms
- **Propaganda**: Enemy influence operations spread misinformation
- **Crisis chaos**: During emergencies, rumors outpace official channels
- **Damaged infrastructure**: Comms arrays destroyed, internal networks compromised

### Effects of Low InfoIntegrity

- Increased decision latency (governments can't get accurate information)
- Higher misidentification rate (internal factions blamed for external problems)
- Accelerated faction radicalization (uncertainty breeds extremism)
- Reduced crisis response effectiveness
- Susceptibility to enemy influence operations

### Effects of High InfoIntegrity

- Faster crisis response
- Reduced radicalization drift
- Better internal faction coordination
- Resistance to propaganda and misinformation

This creates a systems interlock: combat EW and isolation affect governance outcomes. An arcology cut off from allies doesn't just lose tactical data—its government becomes less effective and its factions more radical.

## Mission System

Missions are how factions pursue goals and interact with the world. The key insight: **factions generate missions whether or not a player exists to take them**.

### Mission Generation

Factions generate missions based on:

1. **Strategic goals** (expand, defend, trade, sabotage)
2. **Current problems** (shortage, threat, damaged reputation)
3. **Opportunities** (weak rival, valuable salvage, alliance opening)

```rust
Mission {
    mission_id:     MissionId
    issuer:         FactionId       // Who's offering
    target:         MissionTarget   // What/who/where
    objective:      Objective       // What must be done
    reward:         Reward          // What you get
    deadline:       Option<Tick>    // Time limit (if any)

    // Assignment
    assigned_to:    Option<Assignee>  // Player, NPC fleet, or unassigned
    status:         MissionStatus

    // Visibility
    public:         bool            // Visible to all, or faction-only
    player_eligible: bool           // Can player take this
    reputation_req: f32             // Minimum rep to see/accept
}

enum Assignee {
    Player { player_id: PlayerId },
    Fleet { fleet_id: FleetId },    // Faction's own ships
    Mercenary { merc_id: MercId },  // Hired third party
}
```

### Mission Flow

```text
┌─────────────┐
│  Generated  │ ← Faction has goal/problem
└──────┬──────┘
       │
       ▼
┌─────────────┐     ┌─────────────────┐
│  Evaluate   │────►│ Assign to Fleet │ ← Faction has capable assets
└──────┬──────┘     └────────┬────────┘
       │                     │
       │ (no capable         │
       │  assets OR          │
       │  wants deniability) │
       ▼                     │
┌─────────────┐              │
│ Offer to    │              │
│ Player/Merc │              │
└──────┬──────┘              │
       │                     │
       ▼                     ▼
┌─────────────┐     ┌─────────────────┐
│  In Progress │◄───│   In Progress   │
└──────┬──────┘     └────────┬────────┘
       │                     │
       ▼                     ▼
┌─────────────┐     ┌─────────────────┐
│  Completed  │     │    Completed    │
│  / Failed   │     │    / Failed     │
└─────────────┘     └─────────────────┘
```

### Mission Types

| Type | Example | Typical Issuer |
| ---- | ------- | -------------- |
| **Escort** | Protect convoy to destination | Traders, any faction |
| **Patrol** | Secure area for time period | Military, territorial faction |
| **Strike** | Destroy/disable target | Militarists, rival factions |
| **Delivery** | Transport cargo/passengers | Any |
| **Salvage** | Recover wreck/artifact | Technocrats, scavengers |
| **Reconnaissance** | Scout area, report contacts | Any |
| **Smuggling** | Deliver contraband undetected | Criminals, rebels |
| **Extraction** | Rescue/kidnap person | Political factions |
| **Sabotage** | Disable target without open war | Rivals, internal factions |
| **Blockade** | Prevent traffic through zone | Aggressors |

### Player Access to Missions

Players see missions when:

1. **Physical presence**: At a port/station controlled by the faction
2. **Reputation threshold**: Faction trusts player enough to offer work
3. **Mission visibility**: Mission is marked `player_eligible`

```yaml
visibility_rules:
  public_missions:
    # Visible to anyone at faction ports
    requires: physical_presence

  faction_missions:
    # Standard faction work
    requires: physical_presence AND reputation >= 0.3

  sensitive_missions:
    # Deniable ops, political tasks
    requires: physical_presence AND reputation >= 0.6

  inner_circle:
    # Faction's most important goals
    requires: physical_presence AND reputation >= 0.8
```

### NPC Mission Execution

When factions assign missions to their own fleets:

1. **Fleet Selection**: Faction AI picks appropriate ships
2. **Route Planning**: Path to objective, considering threats
3. **Execution**: Fleet attempts mission (may encounter player!)
4. **Outcome**: Success/failure affects faction resources and relationships

The player may observe, assist, or oppose NPC mission fleets.

### The World Moves Without You

If a player doesn't take a mission:

- **High-priority missions**: Faction assigns own assets
- **Low-priority missions**: May expire unfulfilled
- **Desperate missions**: Faction offers to rivals, mercenaries, or... pirates

The consequence: opportunities pass. That convoy you didn't escort? It got raided. That salvage you didn't recover? A rival faction got it. The world doesn't wait.

## Strategic AI Economy

Factions don't act randomly—they accumulate resources and spend them on actions. This creates pacing and strategic planning without scripted behavior.

*Inspired by [Nexerelin](https://github.com/Histidine91/Nexerelin)'s point-based invasion and diplomacy systems.*

### Action Points

Each faction accumulates **action points** based on their economic and military strength:

```rust
ActionPoints {
    current:            f32
    max:                f32         // Cap prevents hoarding
    base_rate:          f32         // Per-tick generation
    economy_mult:       f32         // Scales with controlled resources
    military_mult:      f32         // Scales with fleet strength
}
```

**Generation factors**:

- Base rate (all factions get something)
- Controlled arcologies and platforms
- Trade route income
- Fleet size and quality
- Alliance contributions

### Spending Actions

Factions spend action points on strategic moves:

| Action | Cost | Effect |
| ------ | ---- | ------ |
| **Launch Invasion** | High | Spawn invasion fleet against target |
| **Reinforce Position** | Medium | Add ships to defensive garrison |
| **Diplomatic Overture** | Low | Attempt alliance, treaty, or ceasefire |
| **Generate Mission** | Low | Create mission from current goals |
| **Economic Investment** | Medium | Improve production capacity |
| **Covert Operation** | Medium | Sabotage, espionage, or destabilization |

Higher-cost actions require more accumulation time, creating natural pacing.

### Grace Periods

To prevent early-game chaos, certain actions have grace periods:

```yaml
grace_periods:
  invasion:           90 days     # No faction invasions until day 90
  alliance_formation: 60 days     # Alliances can't form immediately
  player_targeting:   30 days     # Factions won't target new players
```

This gives players (and small factions) time to establish before the world's power dynamics fully engage.

### Strategic Goals

Factions have **strategic goals** that guide how they spend action points:

```rust
StrategicGoal {
    goal_type:      GoalType        // Expand, Defend, Trade, Weaken, Survive
    target:         Option<Target>  // Specific faction, region, or resource
    priority:       f32             // 0.0-1.0
    progress:       f32             // How close to completion
}
```

**Goal types**:

- **Expand**: Acquire new territory (prioritize invasion)
- **Defend**: Protect existing holdings (prioritize reinforcement)
- **Trade**: Maximize economic output (prioritize diplomacy, trade routes)
- **Weaken**: Damage a specific rival (prioritize covert ops, targeted missions)
- **Survive**: Faction under threat (prioritize alliances, defense)

Goals shift based on faction state, relationships, and world events.

### Vengeance Tracking

When a faction is attacked, they accumulate **vengeance points** against the aggressor:

```rust
VengeanceTracker {
    target:             FactionId   // Who wronged us
    points:             f32         // Accumulated grievance
    threshold:          f32         // When we act
    decay_rate:         f32         // Forgiveness over time
}
```

When vengeance exceeds threshold, the faction prioritizes retaliation—launching attacks, supporting enemies of the aggressor, or offering bounty missions.

This creates consequences for player aggression that feel earned, not arbitrary.

### Faction Respawn

Destroyed factions can return if conditions allow:

```yaml
respawn_conditions:
  min_days_dead:      120         # Must be gone this long
  max_respawns:       3           # Per faction per game
  requires:
    - Surviving population somewhere
    - Allied faction willing to host
    - OR hidden base/exile fleet
```

The world heals. Factions you crushed may return as insurgents, exiles, or reconstituted governments.

## Faction Relationships

Factions have relationships with each other that affect missions, trade, and conflict:

### Disposition Matrix

```rust
Disposition {
    faction_a:      FactionId
    faction_b:      FactionId
    value:          f32             // -1.0 (war) to 1.0 (alliance) — aggregate score
    treaty:         Option<Treaty>  // Formal agreement (if any)
    recent_events:  Vec<EventId>    // What's affecting the relationship
}
```

**Note**: `value` is an aggregate score. A future expansion could decompose this into axes (trust, fear, interest, grievance) for richer dynamics—e.g., a faction you trade with but ideologically hate. For MVP, the single float suffices.

### Relationship Thresholds

```yaml
thresholds:
  allied:     0.7+      # Joint missions, shared intel, mutual defense
  friendly:   0.3-0.7   # Trade, non-aggression, occasional cooperation
  neutral:    -0.3-0.3  # Cautious interaction, no commitments
  unfriendly: -0.7--0.3 # Trade restrictions, border incidents
  hostile:    <-0.7     # Active conflict, war
```

### Relationship Drift

Relationships change based on:

- **Shared enemies** ("enemy of my enemy...")
- **Competing interests** (same resources, same territory)
- **Treaty obligations** (honored or broken)
- **Player actions** (if player works for one faction against another)
- **Mission outcomes** (faction A's mission harmed faction B)

## Government Transitions

Governments can change through several mechanisms:

### Transition Types

| Type | Trigger | Speed | Legitimacy Effect |
| ---- | ------- | ----- | ----------------- |
| **Election** | Scheduled or called | Planned | Neutral (mandate refresh) |
| **Reform** | Government-initiated | Slow | Positive (if popular) |
| **Revolution** | Popular uprising | Fast | High (new mandate) then decay |
| **Coup** | Internal faction action | Instant | Low (must be earned) |
| **Conquest** | External takeover | Instant | Very low (occupation) |

### Transition Process

```text
1. Trigger event (coup, election, etc.)
2. Old governance bundle detached
3. Transition state (chaos, uncertainty)
4. New governance bundle attached
5. Legitimacy reset to type-appropriate baseline
6. Internal factions realign
```

During transition, the arcology has:

- Reduced decision-making capability
- Increased vulnerability to boarding/capture
- Internal faction instability

### Conquest State Machine

Conquest isn't a boolean—it's a process. The `ConquestState` tracks occupation progression:

```rust
enum ConquestState {
    None,                           // Normal governance
    Occupied {                      // Enemy controls government seat
        occupier: FactionId,
        resistance: f32,            // 0.0-1.0, how much population resists
        days_occupied: u32,
    },
    Contested {                     // Active resistance, unclear control
        claimants: Vec<FactionId>,
        stability: f32,
    },
    Pacified {                      // Resistance suppressed, puppet regime
        overlord: FactionId,
        puppet_legitimacy: f32,
    },
    Integrated,                     // Fully absorbed into conquering faction
    Condominium {                   // Stable multi-faction control
        controlling_factions: Vec<FactionId>,
        district_control: Map<DistrictId, FactionId>,
        movement_requires_consensus: bool,
    },
}
```

**Progression**:

- `None → Occupied`: Government seat captured in boarding action
- `Occupied → Contested`: Resistance exceeds threshold, guerrilla warfare
- `Occupied → Pacified`: Resistance suppressed, collaborator government installed
- `Contested → Occupied/Pacified`: One side wins the internal conflict
- `Contested → Condominium`: Stalemate formalized into power-sharing (see below)
- `Pacified → Integrated`: Long-term stability, population accepts new faction (rare)

### Condominium Arcologies

When a `Contested` state stabilizes without resolution, factions may formalize a **Condominium**—permanent shared control with hard boundaries:

- **District sovereignty**: "Faction A law applies on decks 1-40, Faction B on 41-80"
- **Movement consensus**: Bridge and Engineering controlled by different factions means the arcology only moves when both agree
- **Internal borders**: Checkpoints, customs, potentially different currencies
- **Shared infrastructure**: Life support, power, and hull integrity require cooperation regardless of politics

**Why Condominiums Form**:
- Neither faction can dislodge the other without unacceptable losses
- The arcology is too valuable to destroy fighting over
- External threats make cooperation necessary
- Population prefers stability to continued conflict

**Condominium Instability**:
- Any faction can attempt to break the arrangement (triggers return to `Contested`)
- External attack on one faction's districts may or may not trigger mutual defense
- Economic disputes over shared resources
- Gradual demographic shifts changing the balance of power

This creates arcologies that are politically fascinating—floating cities with internal borders, multiple legal systems, and crews who need passports to visit other decks.

This dovetails with the "tactical injection → strategic siege" model from boarding.

## Integration with Other Systems

### Boarding and Capture

When an arcology is boarded (see [damage-and-boarding.md](damage-and-boarding.md)):

- **Government Seat** is a key objective
- Capturing government seat triggers **Conquest** transition
- Legitimacy of new rulers starts very low
- Internal factions may resist or collaborate

### Morale and Surrender

Governance affects surrender calculations:

- **Legitimacy** modifies morale thresholds
- **Government type** affects surrender conditions (Autocrats rarely surrender; Democracies may vote to)
- **Internal factions** may force or prevent surrender

### Economy

Governance affects economic decisions:

- Trade agreements require government approval
- Resource allocation follows government priorities
- Corporate governments optimize for profit; military for security

### Combat Arena

The Combat Arena doesn't simulate governance directly. Instead:

- `BattlePackage` includes relevant modifiers (decision latency, morale bonuses)
- `BattleResult` returns facts for governance to interpret
- Siege resolution happens in the strategic layer using governance systems

## Faces and Fidelity

Arcologies and internal factions have named **faces** (governor, faction leaders, mission delegates) used for player interaction.

### Presence-Gated Instantiation

- When the player is **present** at an arcology/station, faces are instantiated as Person entities with traits, competence, and roles.
- When the player is **absent**, faces are abstracted into aggregate politics (`GovernanceState`, `InternalFactionsState`), and only stable `FaceRecord`s persist.

This preserves continuity—the same people can reappear and evolve—without simulating full character dynamics everywhere at all times.

### Face Roster

Each arcology maintains a `FaceRoster` linking named positions to stable identities:

```rust
FaceRoster {
    governor_face_id:       FaceId
    faction_faces:          Map<FactionType, FaceId>  // One per internal faction
    security_face_id:       Option<FaceId>
    trade_face_id:          Option<FaceId>
}
```

### Leadership and Governance

Named leaders modify governance outcomes:

- **Governor competence** affects decision quality, crisis response, and legitimacy drift
- **Faction leader traits** affect radicalization velocity and faction cohesion
- **Security chief** affects coup detection and suppression effectiveness

When leaders change (election, coup, assassination), the `FaceRoster` updates and emits causal chain events. Players arriving later see the outcome with attribution.

### Succession and Churn

Leadership succession follows government type:

- **Autocracy**: Designated heir, or power struggle on death/incapacity
- **Junta**: Senior officer assumes command, or factional contest
- **Corporate**: Board appointment based on profit track record
- **Democracy**: Election triggered, candidates drawn from internal faction leaders

See [People System](people.md) for full details on named individuals.

## Attribution and Explainability

All governance events carry causal chain metadata for debugging and player explanation:

- **Missions**: Record `source_id` (generating faction), `cause_id` (triggering goal/problem), `trace_id` (root cause)
- **Legitimacy changes**: Record which event, decision, or action caused the shift
- **Government transitions**: Record full causal chain from trigger to outcome
- **Faction radicalization**: Record which decisions or events drove the change

This supports:

- **Debugging**: "Why did this coup happen?" with attributable chain
- **Player feedback**: "Your attack on the convoy caused the Traders to radicalize"
- **Replay analysis**: Deterministic reconstruction of political cascades
- **DRL training**: Reward attribution for strategic decisions

## Data Contracts

### GovernanceState Component

```rust
GovernanceState {
    government_type:    GovernmentType
    leader:             Option<PersonId>
    leader_traits:      LeaderTraits        // Parameterises government behavior
    legitimacy:         Ratio               // 0.0-1.0
    political_capital:  PoliticalCapital
    decision_queue:     DecisionQueue

    // Transition tracking
    transition_state:   Option<TransitionState>
    conquest_state:     ConquestState       // Occupation progression
    previous_type:      Option<GovernmentType>

    // History for causality
    recent_decisions:   Vec<DecisionRecord>
    legitimacy_events:  Vec<LegitimacyEvent>
}

// Separate component - allows factions to exist outside arcologies (diaspora, exile groups)
InternalFactionsState {
    factions:           Vec<InternalFaction>
    dominant_faction:   Option<FactionType>  // Currently most influential
    tension_level:      f32                  // Inter-faction conflict risk
}

// Information environment affects decision quality and faction dynamics
InfoEnvironmentState {
    integrity:          f32         // 0.0-1.0, overall information health
    comms_quality:      f32         // External communication reliability
    internal_trust:     f32         // Population trust in official sources
    isolation_days:     u32         // Days since last external contact
}
```

### Mission Contract

```rust
Mission {
    mission_id:         MissionId
    issuer:             FactionId
    mission_type:       MissionType

    target:             MissionTarget
    objective:          Objective
    success_criteria:   Vec<Criterion>

    reward:             Reward
    reputation_reward:  f32
    reputation_penalty: f32             // If failed/abandoned

    deadline:           Option<Tick>

    assigned_to:        Option<Assignee>
    status:             MissionStatus
    progress:           MissionProgress

    visibility:         MissionVisibility
    reputation_req:     f32
}
```

### FactionState Component

```rust
FactionState {
    faction_id:         FactionId
    name:               String
    philosophy:         Philosophy

    // Resources
    ships:              Vec<ShipId>
    platforms:          Vec<PlatformId>
    arcologies:         Vec<ArcologyId>
    resources:          ResourcePool

    // Goals
    strategic_goals:    Vec<Goal>
    active_missions:    Vec<MissionId>

    // Relationships
    dispositions:       Map<FactionId, Disposition>
    treaties:           Vec<Treaty>

    // Player relationship
    player_reputation:  f32
    player_history:     Vec<ReputationEvent>
}
```

## Plugins and Resolvers

### Governance Plugins

```yaml
plugins:
  AutocracyDecisionPlugin:
    reads: [GovernanceState, WorldView]
    emits: [Decision]
    behavior: "Instant decisions based on leader personality"

  DemocracyDecisionPlugin:
    reads: [GovernanceState, InternalFactions, WorldView]
    emits: [Vote, Decision]
    behavior: "Queue decisions for referendum, tally votes"

  LegitimacyPlugin:
    reads: [GovernanceState, RecentEvents]
    emits: [LegitimacyModifier]
    behavior: "Calculate legitimacy changes from events"

  FactionDriftPlugin:
    reads: [InternalFactionsState, RecentDecisions, InfoEnvironment]
    emits: [FactionModifier]
    behavior: "Update faction satisfaction and radicalization"

  MissionGeneratorPlugin:
    reads: [FactionState, StrategicGoals, WorldView]
    emits: [ProposeMission]
    behavior: "Propose missions from faction goals and problems"

  MissionAssignmentPlugin:
    reads: [FactionState, Missions, AvailableAssets]
    emits: [AssignMission]
    behavior: "Propose mission assignments to fleets or players"
```

### Mission Resolver

The `MissionResolver` handles mission lifecycle:

- Validates `ProposeMission` outputs and instantiates `Mission` entities
- Validates `AssignMission` outputs and updates mission assignments
- Tracks mission progress and completion
- Emits mission outcome events for other systems to react to

Missions are world objects with identity, persistence, and visibility rules—they need deterministic adjudication and causal trace IDs.

### Governance Resolver

The `GovernanceResolver` handles:

- Advancing decision queues per government type rules
- Applying legitimacy modifiers
- Processing government transitions
- Resolving conflicting faction demands

## MVP Staging

### P2 (Core Governance)

- [ ] 3 government types (Autocracy, Corporate, Representative)
- [ ] Basic legitimacy system
- [ ] Decision queue with type-appropriate latency
- [ ] 2-3 internal faction types
- [ ] Basic mission generation and assignment

### P3 (Full Governance)

- [ ] All 5 government types
- [ ] Political capital system
- [ ] Full internal faction mechanics (radicalization, coups)
- [ ] Government transitions (reform, revolution, conquest)
- [ ] Complex mission chains and consequences
- [ ] Faction relationship dynamics

## Related Documents

- [People System](people.md) — Named individuals, faces, roles, and leadership
- [Entity Framework](entity-framework.md) — How governance fits the plugin/resolver pattern
- [Damage and Boarding](damage-and-boarding.md) — How capture affects governance
- [World Requirements](../requirements/world.md) — Governance requirements list
- [Architecture](architecture.md) — Strategic layer overview
- [System Interactions](system-interactions.md) — Mechanic interactions and feedback loops
- [Glossary](../vision/glossary.md) — Canonical terminology
