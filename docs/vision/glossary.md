# Glossary

Canonical terminology for Tidebreak. Use these terms consistently across all documentation and code.

## World & Setting

**Arcology-Ship**
: A supercarrier-scale vessel that functions as a mobile nation. Contains a population of 10,000+, internal economy, government, and factions. Also called "Nomad-City" or "Mobile Nation."

**The Surge**
: The cataclysm that drowned most of the old world. Referenced in lore but not directly simulated.

**Remaining Land**
: The limited livable land that survived. Heavily contested. Land factions control the mega shipyards.

**Mega Shipyard**
: Land-based facilities capable of building carriers, supertankers, and purpose-built arcology-ships. The only source of new megaships. Controlled by land factions, creating a critical power asymmetry.

**Purpose-Built Arcology**
: An arcology-ship designed and constructed as a mobile city from the start. Superior to converted vessels. Only available from mega shipyards.

**Converted Arcology**
: An arcology-ship created by refactoring a supertanker or other large vessel. More common than purpose-built. Often jury-rigged and less efficient.

**Faction**
: A political entity in the world. May control multiple ships, platforms, and arcologies. Land factions control shipyards; ocean factions control food production and trade routes.

## Depth & Layers

Depth is modeled as **discrete strategic states** (rule sets), not continuous 3D position or physical depth ranges. A unit is always in exactly one layer. Maps define where each layer is available via State Blockers.

**Layer**
: A strategic state with its own detection, weapon, and hazard rules. The three layers are Surface, Submerged, and Abyssal. Layers are rule sets, not depth ranges—"Surface" means "surface interaction rules apply," not "0-100m."

**Surface Layer** ("The Arena")
: The primary combat ruleset. Full sensor access (radar, visual, sonar). Full weapon access. Affected by weather. Vulnerable to all weapon types.

**Submerged Layer** ("The Stealth Layer")
: The concealment and ambush ruleset. Sonar-primary detection. Torpedoes, mines, and pop-up missiles. **Contract: Ballistic and line-of-sight weapons cannot target Submerged units; energy weapons are surface-limited unless explicitly tagged as sub-capable.**

**Abyssal Layer** ("The Flank")
: The strategic transit ruleset. Minimal combat capability. Near-total weapon immunity (only specialized deep-pressure ordnance applies). Requires specialized hulls. Used for bypass, retreat, and deep resource access.

**Layer Transition**
: The process of changing layers. Takes 30–60+ seconds. During transition: (1) unit can be **detected and engaged** by systems targeting either origin or destination layer, (2) unit **cannot fire** weapons, (3) unit generates a **massive signature spike** (breaks stealth). Can be interrupted by heavy damage, potentially botching the maneuver.

**Pop-Up Maneuver**
: Submerged units briefly entering a vulnerable state to fire surface-grade weapons (missiles) before retreating. Creates temporary detection spike and engagement window.

**Crush Depth**
: Maximum operating depth before hull failure. Limits which ships can operate in Abyssal layer.

**State Blocker**
: Terrain property that prevents certain layers. Deep Ocean: all three layers valid. Shelf/Coastal: Abyssal blocked. Shallows/Reef: Submerged and Abyssal blocked. Land: all layers blocked.

## Ships & Platforms

**Wave Skimmer**
: Smallest combat unit (jetskis, small boats). Swarm tactics, point defense.

**Cutter**
: Fast patrol/attack craft. Escort, scouting, fast attack.

**Corvette**
: Main line combatant. Balance of speed and firepower.

**Frigate**
: Heavy gunboat. Small fleet flagship.

**Dreadnought**
: Capital ship. Fleet anchor with massive firepower.

**Carrier**
: Capital ship that launches and recovers smaller craft.

**Stationary Platform**
: Fixed infrastructure (farms, rigs, fortresses, sea cities). Cannot maneuver but can mount sensors and defenses.

## Sensors & Information

**Track**
: A fused, time-evolving estimate of an entity's state. Contains position, velocity, uncertainty, age, and identification.

**Contact**
: A single sensor detection before fusion into a track.

**Track Quality**
: Confidence level of a track. Ranges from Q0 (bearing-only cue) to Q3 (fire-control quality, shareable for remote engagement).

**STP (Shared Tactical Picture)**
: The combined track table a ship believes is true, merging local sensor data with shared tracks from allies.

**TDM (Tactical Data Mesh)**
: The networking layer that shares track updates between friendly units. Subject to bandwidth, latency, and jamming.

**Radar**
: Active sensor for surface/air detection. Types include mechanically-scanned and phased-array.

**Sonar**
: Acoustic sensor for underwater detection. Primary sensor modality when submerged.

**ESM (Electronic Support Measures)**
: Passive detection of enemy emissions (radar, communications).

**EMCON (Emissions Control)**
: Operating with reduced or no active emissions to avoid detection. Improves stealth but degrades own sensor picture.

**Signature**
: A scalar (or small vector) representing how detectable an entity is to each sensor modality (radar/sonar/RF/visual). Signature increases during layer transitions, pop-up maneuvers, active emissions (radar/jamming/comms), and with damage effects (fires, flooding, cavitation). Many mechanics (EMCON, jamming, transitions, damage) operate primarily by modifying signature.

## Electronic Warfare

**EW (Electronic Warfare)**
: Umbrella term for jamming, deception, and countermeasures.

**ECM (Electronic Countermeasures)**
: Active measures to degrade enemy sensors or communications.

**Jamming**
: Degrading enemy sensors by flooding frequencies with noise. Reduces detection quality and increases false tracks.

**Decoy**
: Entity or emission designed to create false contacts.

**ECM Dead Zone**
: Map region where communications and sensors are severely degraded.

## Combat

**Damage Tier**
: Modeling fidelity for damage, based on ship size:
- Tier 0: Single HP + status flags (small craft)
- Tier 1: Component health without topology (warships)
- Tier 2: Compartments with dependencies (capitals, arcologies)

**Boarding Tier**
: Resolution complexity for boarding actions:
- Tier 0: Quick contested resolution (small ships)
- Tier 1+: Multi-phase objective control (large ships)

**Component**
: A damageable subsystem (propulsion, sensors, weapons, power).

**Compartment**
: A spatial section of a large ship with its own health, state, and occupants.

**Damage Control**
: Crew activity to contain cascading damage (fire, flooding) and repair components.

**Ship Fate**
: Final state of a ship after battle. Values: OPERATIONAL (fighting-capable), DISABLED (mission-killed but afloat), DESTROYED (sunk/vaporized), SCUTTLED (self-destroyed), CAPTURED (enemy control).

**Capture Method**
: How a ship was captured (only present when fate=CAPTURED). Values: BOARDED (taken by force via boarding action), SURRENDERED (crew surrendered due to morale collapse).

**Beachhead Established**
: Tactical victory state for arcology boarding. Attackers have successfully injected troops but do not control the ship. Siege resolution happens in the strategic layer over days/weeks. Not "captured"—the city is under siege.

**Boarding Status**
: Progress state for boarding attempts. Values: NONE, ATTEMPTED (in progress), ESTABLISHED (beachhead secured), REPULSED (attackers driven off).

**Breach Quality**
: Measure of how hard it is for defenders to purge an established beachhead (0.0–1.0). Depends on whether attackers secured dock nodes, disabled security systems, inserted supplies, or just bodies.

**Transfer Rate**
: Speed of troop insertion during docked boarding. Function of dock integrity, local suppression, sea state, and command link quality—not a static number.

## Governance & Politics

**Government Type**
: The political system of an arcology-ship. Affects decision speed, legitimacy, crisis response. Examples: Autocracy, Corporate Meritocracy, Direct Democracy.

**Legitimacy**
: Measure of government authority. Affects compliance and resistance.

**Political Capital**
: Ability to push through unpopular decisions without losing legitimacy.

**Internal Faction**
: A group within an arcology's population with distinct goals and leaders. Examples: Militarists, Traders, Technocrats.

**Crisis Event**
: A stress event (plague, mutiny, shortage) that tests government response.

## People & Leadership

**Person**
: A named individual modeled as an entity with `PersonState`. Only key individuals are modeled (governors, captains, faction leaders)—populations are aggregate.

**Face**
: A named individual used for player interaction. Faces are instantiated as Person entities when the player is present; otherwise abstracted into aggregate politics.

**FaceId**
: Stable handle for a named position (governor, faction leader) that persists whether or not the Person entity is currently instantiated.

**Role**
: A position a Person occupies (XO, Captain, Fleet Commander, Governor, Security Chief). Roles connect People to entities and determine what plugins read their stats.

**RoleAssignment**
: The link between a Person and the entity they serve. Contains role type, authority level, and tenure.

**Trait**
: A behavioural or competence modifier attached to a Person. Affects how they perform their role. Examples: Cautious, Ruthless, Charismatic, Paranoid.

**Competence**
: A Person's skill in a domain (command, operations, engineering, intelligence, politics). 0.0–1.0 per domain.

**Loyalty**
: A Person's alignment to their current employer/leader (0.0–1.0). Affects defection risk and coup participation.

**Ambition**
: A Person's drive for power and status (0.0–1.0). High ambition + low satisfaction = coup/defection risk.

**Grudge**
: A recorded grievance against another entity. Decays over time unless reinforced. Affects revenge missions and defection targets.

**Dismissal**
: Removing a Person from their role. Creates grudge, reputation shift, and makes them available to rival factions.

## AI & Simulation

**DRL (Deep Reinforcement Learning)**
: Machine learning approach used to train AI controllers.

**Combat Arena**
: Self-contained battle simulator used for player combat and DRL training. Accepts a BattlePackage, returns a BattleResult.

**BattlePackage**
: Input data contract for the Combat Arena (ships, terrain, weather, teams).

**BattleResult**
: Output data contract from the Combat Arena (winner, outcomes, events).

**Determinism**
: Property that same seed + same inputs = same outputs **on the same platform/build** (strict requirement). Cross-platform determinism is an engineering goal requiring fixed-point math or deterministic physics modes. Required for DRL training, replay debugging, and multiplayer integrity.

**Headless Mode**
: Running simulation without rendering for fast training and CI.

## Time & Ticks

**Tick**
: The discrete time step of a simulation layer. Different layers run at different tick rates. All state changes occur at tick boundaries for determinism.

**Strategic Tick**
: The time step for the campaign/strategic layer. **Canonical rate: 1 day.** Governance decisions, faction AI, economic production, and disposition drift operate on strategic ticks. Players can accelerate strategic time in the UX.

**Tactical Tick**
: The time step for combat simulation. **Canonical rate: 1 second.** Weapon cooldowns, movement, and sensor updates occur on tactical ticks. The underlying physics may use finer substeps (e.g., 0.1s) for accuracy, but game mechanics reference tactical ticks. Players can accelerate/pause tactical time in the UX.

**Substep**
: A physics integration step smaller than the tactical tick (e.g., 0.1s). Used internally for numerical stability. Not exposed to game mechanics—only tactical ticks are meaningful for cooldowns, durations, etc.

## Architecture & Code

**Entity**
: The fundamental runtime container for any game object (Ship, Faction, Arcology, Platform). Has no behavior itself—behavior comes from attached Plugins. Entities have identity, state components, and optional children.

**Plugin**
: A modular capability attached to an Entity (e.g., `RadarPlugin`, `AutocracyPlugin`, `TorpedoPlugin`). Contains logic and configuration but acts only via the Shared Contract. Plugins read from WorldView and emit Outputs; they never mutate state directly.

**Shared Contract**
: The strict data interface (state schemas, event schemas, output types) that Plugins use to interact. Ensures determinism, prevents code coupling, and defines what mutations are legal. Nothing important exists outside the contract.

**Resolver**
: A deterministic system that collects Plugin outputs (commands, effects, votes, attacks) and applies them to produce state changes (damage, laws passed, position updates). Resolvers enforce the rules; plugins only propose.

**WorldView**
: An immutable snapshot of the current simulation state. Plugins read from WorldView to make decisions. Corresponds to "Observation" in DRL terms.

**Output**
: A typed proposal emitted by a Plugin (e.g., `FireWeapon`, `ApplyModifier`, `QueueDecision`). Outputs are collected and resolved; they don't take effect immediately.

**Causal Chain**
: Metadata on events and outputs (source_id, cause_id, trace_id) that enables tracing cause-and-effect through the simulation. Required for debugging and player-facing explanations.
