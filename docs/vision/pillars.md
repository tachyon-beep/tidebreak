# Design Pillars

These principles guide design decisions across all Tidebreak systems. When evaluating a feature or mechanic, ask: does it reinforce these pillars?

## 1. Depth Creates Tactical Space

The ocean's vertical dimension is the game's defining feature. Depth layers aren't cosmetic—they fundamentally change combat, detection, and movement.

**What this means:**
- Every layer has distinct sensor, weapon, and hazard profiles
- Transitioning layers is a tactical decision with costs and risks
- Controlling layers means controlling the battlespace
- Ships should be specialized for different layers, forcing fleet composition choices

**What to avoid:**
- Depth as purely defensive (dive to escape)
- Layers that feel like reskins of each other
- Universal ships that work equally well everywhere

## 2. Imperfect Information Drives Decisions

Combat operates on uncertain data. The fog of war isn't an annoyance—it's the core decision space.

**What this means:**
- Ships track contacts with varying quality, not perfect positions
- Sensors have modality-specific strengths and weaknesses
- Electronic warfare degrades both sensing and communication
- Misidentification and friendly fire are possible
- Sharing tactical data is valuable and contestable

**What to avoid:**
- Perfect information as the default, fog as optional
- Sensors as simple range circles
- Communication that "just works"

## 3. Scale Matters

A jetski and an arcology-ship are fundamentally different entities. Game systems should reflect this through different modeling fidelity, not just stat scaling.

**What this means:**
- Small ships: abstracted damage, quick boarding resolution
- Capital ships: component damage, subsystem targeting, repair management
- Arcology-ships: compartmentalized damage, population concerns, political consequences
- Fleet hierarchy creates distinct tactical roles

**What to avoid:**
- Everything using the same damage model at different HP values
- Arcologies as "big ships with more guns"
- Scaling purely through numbers

## 4. Nations, Not Just Ships

Arcology-ships are floating societies with politics, economies, and populations. Their survival is a strategic crisis, not a tactical setback.

**What this means:**
- Arcologies have government types that affect decision-making
- Internal factions create political pressure
- Economic systems produce and consume resources
- Capturing an arcology is a complex, multi-phase operation
- Losing an arcology has catastrophic faction-level consequences

**What to avoid:**
- Governments as flavor text
- Populations as passive HP pools
- Boarding as quick dice rolls on big ships

## 5. Determinism Enables Learning

The simulation must be deterministic for DRL training, replay debugging, and fair multiplayer. Randomness comes from seeded RNG, not hidden state.

**What this means:**
- Same seed + same inputs = same outputs on same platform/build (strict)
- Cross-platform determinism via fixed-point math or deterministic modes (goal)
- All random events use explicit RNG streams
- State is fully serializable and replayable
- Headless simulation runs faster than real-time

**What to avoid:**
- Hidden state that affects outcomes
- Frame-rate-dependent physics
- Non-reproducible bugs
- Promising cross-platform determinism without the engineering to back it up

## 6. Systems Interlock

Weather affects sensors. Sensors affect targeting. Damage affects crew. Crew affects repairs. Repairs affect survival. Survival affects morale. Morale affects surrender.

**What this means:**
- Systems influence each other through explicit mechanics
- Cascading effects create emergent situations
- Players can trace cause-and-effect chains
- No system exists in isolation

**What to avoid:**
- Orthogonal systems that don't interact
- Effects that appear from nowhere
- Mechanics that only matter in their own subsystem

## 7. Explainable Causality

Players can trace outcomes to causes, even through uncertainty. The world happens *around* the player with visible reasons, not *to* the player through opaque systems.

**What this means:**
- Major changes (government flips, faction defections, surrenders) generate attributable event chains
- Debug tooling produces the same causal chains developers see
- Track quality and sensor uncertainty are *visible* uncertainty, not hidden dice
- "Why did this happen?" has an answer in the game state

**What to avoid:**
- Outcomes that appear random when they're actually systemic
- Hidden modifiers the player can't discover
- "The AI decided to do X" without traceable reasoning
- Fog of war that hides causality, not just information

## 8. Grounded Values, Not Magic Numbers

Every value in the game should trace back to its origins. No number exists in isolation—it either comes from authored content, accumulated state, or a formula combining them. If you can't trace a value back through the chain, it's a magic number.

### The Value Hierarchy

```
Content (authored)
    ↓ interpreted by
Systems (grounded logic)
    ↓ produces
State (accumulated over time)
    ↓ computed into
Derived Values (what the game uses)
```

**Content** is authored data in libraries—plugins, archetypes, definitions:
- Government plugins (Autocracy: decision_latency=1, coup_risk=high)
- Weapon definitions (Mk3 Torpedo: damage=150, range=8km)
- Faction archetypes (philosophy, tech_focus, starting_holdings)
- Trait definitions (Reckless: risk_tolerance +0.3)

**Systems** interpret content through grounded logic:
- Economy system processes production chains and trade routes
- Combat system resolves weapon fire and damage
- Governance system applies government plugins to decisions
- Selection logic chooses which content applies based on game state

**State** accumulates from systems processing content over time:
- `days_of_supply = 12` (economy system tracked consumption)
- `recent_battles = [victory, defeat]` (combat system recorded outcomes)
- `faction_morale = 0.7` (morale system integrated battle results)
- `population_makeup = {Engineering: 0.15, Populist: 0.35}` (demographics shifted)

**Derived Values** are computed from state when needed:
- `ship_morale = faction_morale × supply_modifier × condition × leadership`
- `faction_influence = population_share × structural_multiplier`
- `legitimacy = base_legitimacy × policy_alignment × economic_health`

### The Traceability Test

For any value, ask: **"Can I trace this back through state to content?"**

✅ **Ship morale = 0.59**
```
0.59 = faction_morale(0.7) × supply_modifier(0.9) × condition(0.85) × leadership(1.1)
       ↑                      ↑                      ↑                  ↑
       state: from battles    state: from economy    state: from damage  content: captain traits
       ↑                      ↑                      ↑
       content: faction       content: ship loadout  content: weapon damage
       archetype              + trade routes         that hit us
```

❌ **morale = 0.6** — Where does this come from? Magic number. Reject it.

### Four Types of Numbers

| Type | Example | Rule |
|------|---------|------|
| **Content properties** | Torpedo damage = 150 | Authored in libraries. Selection must be grounded. |
| **Accumulated state** | days_of_supply = 12 | Produced by systems processing content over time. |
| **Derived values** | ship_morale = 0.59 | Computed from state via grounded formulas. |
| **Tunable coefficients** | LEVERAGE_WEIGHT = 1.5 | Explicit knobs on formulas. Documented and intentional. |

### Tunable Coefficients

Values derive from state, but *coefficients* weight the formulas:

```
influence = population_share × (structural_multiplier × LEVERAGE_WEIGHT)
morale = faction_morale × (supply_factor × SUPPLY_WEIGHT) × (condition × CONDITION_WEIGHT)
```

- ✅ `LEVERAGE_WEIGHT = 1.5` — Tunable coefficient, documented purpose
- ✅ `SUPPLY_WEIGHT = 0.8` — Tunable coefficient, affects balance
- ❌ `influence = 0.65` — Magic number with no derivation

If everyone dies in 5 minutes, we adjust coefficients—we don't invent new magic numbers. Coefficients are the tuning knobs; the formulas stay grounded.

### Content vs Systems

**Content defines what things are.** Government plugins, weapon stats, faction archetypes—authored libraries of properties.

**Systems decide what applies and how it combines.** Grounded logic selects from content based on game state.

```
// Content (authored)
Autocracy plugin: { decision_latency: 1, coup_risk: 0.3 }
Mk3 Torpedo: { damage: 150, range: 8000 }

// Systems (grounded logic)
government = arcology.government_type  // Autocracy
apply(government.plugin)               // System interprets content

available_weapons = ship.weapons.filter(w => w.ammo > 0)
selected = tactics.choose(available_weapons, target)  // Grounded selection
damage = selected.damage               // Content property, but selection was grounded
```

The principle: **Systems interpret content; they don't invent behavior.**

### What This Enables

- **Traceability**: Any value can be explained as "X because Y because Z... because content"
- **Predictable tuning**: Change a coefficient, understand the impact
- **Emergent behavior**: Systems compose content in interesting ways
- **Moddable content**: Add new plugins without changing system logic
- **Honest simulation**: No hidden modifiers, no designer fiat

### What to Avoid

- Abstract 0-1 values tuned by feel with no derivation
- "Influence" or "power" disconnected from what produces them
- Modifiers that exist to balance rather than simulate
- Values that can't be explained as "X because Y"
- Systems that invent behavior instead of interpreting content

## Using These Pillars

When designing or evaluating a feature:

1. **Does it reinforce at least one pillar?** Features should strengthen core identity.
2. **Does it contradict any pillar?** Contradictions need strong justification.
3. **Does it create interesting decisions?** Pillars exist to generate gameplay, not restrict it.

When pillars conflict (e.g., detailed arcology simulation vs. determinism performance), document the tradeoff and make an explicit decision.
