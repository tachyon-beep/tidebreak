# Sandbox Design: Naval Combat Arena

This document applies sandbox design principles to Tidebreak's core combat experience—identifying the constraints that create meaningful choices, the progressive revelation that teaches through play, and the expression dimensions that let players develop personal mastery.

## Context

**Player Fantasy**: Command a fleet across depth layers, making life-or-death decisions with imperfect information. Feel the weight of commanding vessels where mistakes cost lives and tactical brilliance can turn hopeless odds.

**Core Experience**: Moment-to-moment tension between offense and stealth, commitment and flexibility, individual ship survival and fleet objectives. The permanent uncertainty of fog-of-war combat.

**Scope**: Players affect tactical outcomes within a battle—ship positioning, weapon timing, layer transitions, sensor management, fleet coordination. Strategic context (why we're fighting, what we're protecting) comes from the campaign layer.

## Constraint Architecture

### Primary Constraints

| Constraint | Type | Purpose | Interesting Choices Created |
| ---------- | ---- | ------- | --------------------------- |
| **Depth Layers** | Spatial | Forces commitment decisions | Dive to evade or stay and fight? Which layer gives tactical advantage? |
| **Track Quality** | Knowledge | Rewards sensor investment | Fire on uncertain track or wait for better data? Reveal position with active sensors? |
| **Transition Vulnerability** | Temporal | Punishes reactive diving | Commit early or accept you're trapped? Cover a transitioning ally? |
| **Weapon-Layer Targeting** | Physical | Prevents universal solutions | Bring ASW capability or accept sub vulnerability? Specialize or generalize fleet? |
| **TDM Link Tiers** | Social | Creates information asymmetry | Tight formation for better data link or spread for coverage? |
| **Hull Specialization** | Resource | Forces fleet composition choices | Sensor pickets vs. shooters vs. fusion nodes? |

### Constraint Interactions

The magic of Tidebreak's sandbox emerges from constraint interactions:

1. **Layer + Sensors**: Diving removes you from radar but puts you in sonar-only mode. You trade one uncertainty for another.

2. **Track Quality + Weapons**: Fire-control quality (Q2/Q3) unlocks engagement, but getting there requires sensor exposure. Patience vs. opportunity.

3. **Transition + Weather**: Storm conditions degrade surface combat but also degrade the sensors that would catch you diving. Weather creates windows.

4. **TDM + EW**: Sharing tracks requires active data links. Jamming degrades the mesh. Tight coordination makes you vulnerable to disruption.

5. **Specialization + Attrition**: Losing your sensor picket blinds the fleet. Losing your fusion node fragments the tactical picture. Fleet composition creates vulnerability profiles.

## Progressive Revelation

### Layer 1: First 30 Minutes

**Player learns**:

- Ships move with momentum (can't stop or turn instantly)
- Weapons have range limits and fire arcs
- Enemy positions are uncertain (track quality visible)
- Diving takes time and makes you vulnerable
- Getting hit hurts—damage cascades through systems

**Through**:

- Tutorial scenario: 1v1 surface combat in calm conditions
- Enemy starts visible, player discovers tracking as enemy maneuvers
- First dive attempt shows transition vulnerability when enemy punishes it
- Component damage from grazing hit shows cascade (weapons offline → can't return fire → evasion only)

**Feels like**: Learning to sail, not reading a manual. "Oh, I can't just dive whenever I want."

### Layer 2: First 5 Hours

**Player learns**:

- EMCON tradeoffs (silence vs. blindness)
- Cross-layer detection asymmetries (subs see surface better than surface sees subs)
- Weather creates tactical windows
- Track sharing requires link investment
- Fleet composition determines capability gaps

**Through**:

- Scenario: Convoy escort against submarine threat
- Player discovers EMCON when active radar pings attract missiles
- Cross-layer play when sub-hunting becomes main challenge
- Weather scenario where storm provides cover for risky maneuver
- Fleet scenario where losing sensor picket cripples situational awareness

**Feels like**: Developing intuition. "I should probably go quiet before entering that zone."

### Layer 3: Ongoing

**Player discovers**:

- Pop-up timing windows (when to break stealth for maximum impact)
- EW as offensive tool (jamming enemy mesh before strike)
- Transition-covering tactics (fleet maneuvers to protect diving ships)
- Bait-and-ambush patterns (surface decoy with submerged striker)
- Arcology-protection doctrine (screening, layered defense, sacrifice plays)

**Through**:

- Self-directed experimentation
- Watching replays of losses to understand what happened
- Community-shared tactics
- DRL opponent behaviors that demonstrate emergent strategies

**Feels like**: Personal style emerging. "My signature move is the weather-masked dive."

## Onboarding Design

### First Scenario: "First Contact"

```yaml
visible_goal: "Enemy corvette approaching—survive and destroy it"
blocking_element: "Enemy will reach weapon range before you can escape"
affordance: "Your ship has weapons, theirs is visible on sensors"
discovery: "Combat is about maneuvering and timing, not HP pools"
```

### Second Scenario: "The Dive"

```yaml
visible_goal: "Escape superior enemy force"
blocking_element: "Can't outrun them on surface"
affordance: "Dive button exists, depth layers shown on UI"
discovery: "Diving takes time, you're vulnerable during transition, but underwater changes the rules"
```

### Third Scenario: "Fog of War"

```yaml
visible_goal: "Find and destroy enemy before they find you"
blocking_element: "Enemy position unknown"
affordance: "Sensors show tracks with quality indicators, EMCON toggle available"
discovery: "Active sensors reveal enemy but also reveal you—information is a two-way street"
```

### Teaching Sequence

1. **Movement & Combat** (scenario 1): Core loop—maneuver, fire, damage
2. **Depth Layers** (scenario 2): Strategic states, transition commitment
3. **Information Warfare** (scenario 3): Sensors, EMCON, track quality
4. **Fleet Command** (scenario 4): Multi-ship coordination, roles, TDM
5. **Weather & Terrain** (scenario 5): Environmental factors, chokepoints
6. **Full Complexity** (scenario 6): All systems, asymmetric engagement

Each scenario introduces ONE major system while reinforcing previous learning.

## Expression Architecture

### Dimensions Available

| Dimension | Range of Expression | Examples |
| --------- | ------------------- | -------- |
| **Tactical** | Aggressive ↔ Patient | Rush with overwhelming force vs. methodical track-building |
| **Posture** | Loud ↔ Silent | Active sensor flood vs. EMCON discipline |
| **Layer** | Surface-dominant ↔ Depth-exploiting | Surface brawling vs. sub-based ambush |
| **Fleet** | Specialist ↔ Generalist | Dedicated picket/shooter roles vs. multi-role flexibility |
| **Risk** | Conservative ↔ Committing | Preserve forces vs. decisive action |

### Player Types Supported

**Builders** (fleet composition):

- Design fleet loadouts for specific doctrines
- Optimize ship configurations for roles
- Create asymmetric team compositions

**Explorers** (tactical discovery):

- Find terrain exploits and tactical positions
- Discover layer transition timing windows
- Map sensor coverage gaps

**Optimizers** (efficiency mastery):

- Minimize losses while achieving objectives
- Perfect timing on weapon salvos and transitions
- Maximize track quality with minimum emissions

**Socializers** (coordination play):

- Develop multi-ship tactics requiring tight coordination
- Create fleet doctrines for team play
- Lead tactical mesh networks

## Failure Safety

**Experimentation cost**: Medium-low in combat arena (single battle), higher in campaign (persistent consequences)

**Recovery options**:

- Battle arena: Restart scenario, try different approach
- Campaign: Tactical retreat, accept losses, rebuild
- Replay system: Watch what happened, understand failure

**Learning from failure**:

- Death replays show causal chain (what detected you, what hit you, why)
- Track quality at moment of engagement visible (did you fire blind?)
- Transition timeline shows vulnerability windows
- Damage cascade shows which hit started the spiral

The **Explainable Causality** pillar ensures failure teaches rather than frustrates.

## Scope Boundaries

**Player can affect**:

- Ship positioning and maneuvering within battle
- Weapon timing and target selection
- Layer transition decisions
- EMCON and sensor mode choices
- Fleet coordination and formation
- Tactical objective prioritization

**Player cannot affect** (and why this is good):

- Ship stats mid-battle (prevents save-scumming optimal loadouts)
- Weather patterns (forces adaptation, not optimization)
- Enemy reinforcements (maintains tension of incomplete information)
- Track ground truth (information asymmetry is the game)
- Ally AI decisions in single-player (you command your fleet, not the battle)

## Anti-Patterns Avoided

| Anti-Pattern | How Tidebreak Avoids It |
| ------------ | ----------------------- |
| **Unlimited freedom** | Depth layers, sensor constraints, hull specialization force choices |
| **Tutorial walls** | Scenarios teach through environmental challenge, not text boxes |
| **Permanent failure** | Arena mode allows experimentation; campaign has retreat options |
| **Hidden mechanics** | Track quality visible, damage cascades shown, causality traceable |
| **Single solution** | Multiple viable approaches (stealth, aggression, mixed doctrine) |

## Implementation Notes

### Combat Arena MVP Priorities

1. **Movement feel**: Get Starsector-like momentum right before anything else
2. **Layer transitions**: Core tension loop must feel weighty
3. **Track quality visualization**: Players must see uncertainty, not just experience it
4. **Damage feedback**: Cascades need clear cause-effect presentation

### DRL Training Implications

The sandbox constraints create learnable patterns:

- Layer transitions have clear timing windows → agents can learn exploitation
- Track quality thresholds unlock actions → agents can learn sensor management
- Weapon-layer targeting creates rock-paper-scissors → agents can learn fleet composition

Constraint-rich sandboxes produce more interesting emergent behaviors than open systems.

### Testing Priorities

1. **Constraint bypass**: Ensure no dominant strategy that ignores layer system
2. **Learning curve**: Players should reach competence in ~2 hours, mastery in ~20
3. **Expression validation**: Different playstyles should be viable, not just tolerated
4. **Failure clarity**: When players lose, they should know why

## Related Documents

- [Design Pillars](../vision/pillars.md) — Guiding principles this sandbox supports
- [Layers and Terrain](layers-and-terrain.md) — Depth layer mechanics
- [Sensors and Fog](sensors-and-fog.md) — Information warfare systems
- [Architecture](architecture.md) — Combat Arena technical design
