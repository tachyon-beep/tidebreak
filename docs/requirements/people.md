# People Requirements

Requirements for named individuals, roles, and leadership systems.

See: [design/people.md](../design/people.md), [design/governance.md](../design/governance.md)

## Person Entities (P2)

- Support Person entities with PersonState component
- Support stable PersonId for save/replay continuity
- Support limited named individuals per context:
  - Player fleet: XO + captains
  - Factions: Leader + 2-5 cabinet roles
  - Arcologies: Governor + key positions
- Support person status tracking (employed, dismissed, exiled, dead)

## Competence (P2)

- Support competence profile with 5 domains:
  - Command (fleet tactics, morale)
  - Operations (navigation, logistics)
  - Engineering (repairs, reliability)
  - Intelligence (sensors, counter-intel)
  - Politics (legitimacy, negotiation)
- Support competence affecting role performance via modifiers
- Support competence range 0.0–1.0 per domain

## Traits (P2)

- Support trait system with curated list (8-12 traits initially)
- Support trait categories:
  - Temperament (cautious, reckless, ruthless, compassionate)
  - Governance style (proceduralist, populist, technocratic)
  - Reliability (meticulous, improviser)
  - Social (charismatic, intimidating, divisive)
- Support traits affecting gameplay mechanics, not just flavor

## Traits (P3)

- Support expanded trait list (up to 20 traits)
- Support pathology traits (paranoid, vengeful, ambitious)
- Support trait interactions and combinations

## Roles (P2)

- Support role assignment linking Person to Entity
- Support role types:
  - Fleet: Commander, Captain, XO
  - Arcology: Governor, Security Chief
  - Faction: Leader, Military Chief
- Support authority level per role assignment
- Support role tenure tracking

## Roles (P3)

- Support full role hierarchy (ChiefEngineer, Diplomat, Spymaster)
- Support role succession rules by government type
- Support role competition and appointment mechanics

## Loyalty and Ambition (P2)

- Support loyalty (0.0–1.0) to current employer
- Support ambition (0.0–1.0) affecting behavior
- Support loyalty/ambition drift based on events

## Loyalty and Ambition (P3)

- Support loyalty affecting defection probability
- Support ambition affecting coup participation
- Support complex loyalty networks (to person, faction, ideology)

## Grudges (P2)

- Support grudge records tracking grievances
- Support grudge intensity (0.0–1.0)
- Support grudge decay over time
- Support grudge causes (dismissed, betrayed, defeated)

## Grudges (P3)

- Support grudge-driven revenge missions
- Support grudge affecting recruitment by rivals
- Support grudge escalation and de-escalation

## Reputation (P2)

- Support per-faction reputation for persons
- Support reputation affecting recruitment and opportunities
- Support reputation events (dismissal, victory, scandal)

## Reputation (P3)

- Support global notoriety
- Support reputation affecting mission availability
- Support reputation transfer (your XO's reputation reflects on you)

## Faces and Fidelity (P2)

- Support FaceId as stable handle for named positions
- Support FaceRecord persisting identity when Person not instantiated
- Support FaceRoster linking positions to FaceIds per arcology
- Support presence-gated instantiation:
  - Player present: Person entities exist
  - Player absent: Aggregate politics only

## Faces and Fidelity (P3)

- Support dormant Person entities (persist but don't run plugins)
- Support offscreen leadership changes via FaceRoster updates
- Support news/event summary when player arrives at changed location

## Leadership Effects (P2)

- Support governor competence affecting decision quality
- Support captain competence affecting ship performance
- Support leader traits affecting crisis response
- Support effects via modifier system (ApplyModifier outputs)

## Leadership Effects (P3)

- Support faction leader affecting faction-wide modifiers
- Support security chief affecting coup detection
- Support detailed trait-to-mechanic mappings

## Dismissal and Recruitment (P2)

- Support dismissing persons from roles
- Support dismissal creating grudge
- Support dismissed persons entering "available pool"
- Support factions recruiting from available pool

## Dismissal and Recruitment (P3)

- Support "exile market" where factions compete for talent
- Support recruitment based on competence + ideology match
- Support poaching between factions

## Career Transitions (P3)

- Support transitions: dismissed → recruited → promoted → leader
- Support fired XO becoming rival faction leader
- Support persons forming splinter factions
- Support persons staging coups

## Succession (P3)

- Support succession rules per government type:
  - Autocracy: Designated heir or power struggle
  - Junta: Senior officer or factional contest
  - Corporate: Board appointment
  - Democracy: Election
- Support succession crisis on leader death/incapacity
- Support succession affecting legitimacy

## Elections and Appointments (P3)

- Support democratic elections with candidate pool
- Support candidate competence and traits affecting votes
- Support board appointments in corporate governments
- Support ideology affinity affecting election outcomes

## Coups (P3)

- Support coup attempts by ambitious persons
- Support security chief detection of coup plots
- Support coup success/failure based on:
  - Plotter's network
  - Internal faction support
  - Security effectiveness
  - Legitimacy level
- Support coup consequences (purges, instability)

## Attribution (P2)

- Support causal chain metadata on person events
- Support "why did this happen" traceability
- Support events: PersonDefected, PersonDismissed, PersonRecruited

## Attribution (P3)

- Support detailed cause chains for complex events
- Support player feedback ("Your attack caused X to radicalize")
- Support replay analysis of leadership changes
