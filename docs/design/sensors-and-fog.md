# Sensors and Fog of War Design

Combat operates on uncertain information. Ships maintain evolving **track tables** of contacts, share tracks over a contested **Tactical Data Mesh**, and make decisions with imperfect knowledge.

This design prioritizes *interesting decisions* (emissions control, jamming, deception) over simulation fidelity.

## Canonical Types Used

- **Track**: Fused, time-evolving estimate with uncertainty
- **TrackQuality**: Q0 (cue) through Q3 (fire-control shared)
- **Signature**: Modality-specific detectability (radar/sonar/RF/visual)
- **STP**: Shared Tactical Picture (ship's belief state)
- **TDM**: Tactical Data Mesh (networking layer)

## Goals

- Sensor-limited combat (no perfect information)
- Multiple sensor modalities with distinct tradeoffs
- Contested communications with track sharing and isolation
- Distinct radar hardware mechanisms vs. combat system capabilities
- Deterministic and seedable for DRL

## Non-Goals

- High-fidelity RF propagation or antenna patterns
- Accurate sonar oceanography
- Real cryptography or datalink protocols
- Full strategic ISR modeling

## Core Concepts

### Terminology

| Term | Definition |
|------|------------|
| **Contact** | Single sensor detection (e.g., bearing-only blip) |
| **Measurement** | Contact expressed as data (bearing, range, doppler) |
| **Track** | Fused, time-evolving estimate of an entity with uncertainty |
| **STP** | Shared Tactical Picture—what a ship believes is happening |
| **TDM** | Tactical Data Mesh—networking layer for track sharing |
| **Emitter** | Anything that radiates (radar, jammer, comms) |

Player-facing names: "Tactical Picture" and "Data Link"

### System Flow

```
┌──────────────┐   measurements   ┌─────────────────┐
│ Sensor Suite │ ───────────────► │ Track Manager   │
└──────────────┘                  │ (association +  │
                                  │  filtering)     │
                                  └────────┬────────┘
                                           │ updates
                                           ▼
                                  ┌─────────────────┐
                                  │ Local Tracks    │
                                  └────────┬────────┘
                                           │ publish/receive
                                           ▼
                                  ┌─────────────────┐
                                  │ TDM (link tier  │
                                  │ + ECM effects)  │
                                  └────────┬────────┘
                                           │
                                           ▼
                                  ┌─────────────────┐
                                  │ STP View        │
                                  │ (ship's belief) │
                                  └─────────────────┘
```

## Track Model

### Track Fields

**Identity**:
- `track_id`: Unique within battle
- `track_class`: Surface / sub / air / missile / unknown

**Kinematics**:
- `pos_xy`, `vel_xy`, `heading` (as available)
- `depth_state` (if known)

**Uncertainty**:
- `quality`: 0.0–1.0 scalar
- `age_s`: Seconds since last update
- `source_tags`: Which sensors/nodes contributed

**Identification**:
- `iff_state`: Authentication level
- `id_confidence`: 0.0–1.0

**Engagement**:
- `engageable`: Boolean per weapon type

### Track Quality Levels

| Level | Name | Meaning | Unlocks |
|-------|------|---------|---------|
| Q0 | Cue | "Something exists" (bearing-only) | Investigation |
| Q1 | Coarse | Usable for maneuvering | Navigation, patrol |
| Q2 | Fire Control (local) | Engageable by own weapons | Local firing |
| Q3 | Fire Control (shared) | Engageable via shared data | Remote engagement |

Quality bands create clean gameplay rules without physics simulation.

## Sensor Modalities

Sensors interact with a target's **modality-specific signature** (radar/sonar/RF/visual). Many mechanics (EMCON, jamming, pop-up, layer transitions, damage) operate primarily by modifying signature and therefore track quality.

### Radar

**Mechanism Types**:

| Type | Strengths | Limits |
|------|-----------|--------|
| Mechanical | Cheap, simple | Slow scan, low track capacity |
| Phased-array | Beam agility, track-while-scan | Power/heat cost, high-value target |

**Role Configurations**:
- **Search/Volume**: Wide area, lower precision, initial detection
- **Surface Search**: Better clutter handling, short range
- **Fire Control**: Narrow beam, high precision, terminal guidance

### Sonar

Primary sensor when submerged. Modeled similarly with active/passive modes and layer-dependent performance.

### Passive RF (ESM)

Detects enemy emissions (radar, communications). Bearing-only without triangulation.

### Visual/EO/IR

Weather and visibility limited. Short range but hard to deceive.

### Emissions Control (EMCON)

Player choice with tradeoffs:
- EMCON: Reduced detectability, reduced own picture
- LPI modes: Reduced ESM signature, reduced effective range

## Combat System Suites

"Aegis-like" is a **suite**, not just good radar. Separating hardware from systems enables:
- Specialist hull roles (sensor pickets vs. shooters vs. fusion nodes)
- Saturation modeling
- Cooperative engagement

### Suite Capabilities

| Capability | Description |
|------------|-------------|
| Track Management | Higher max tracks, better association under clutter |
| Remote Cueing | Ingest Q0/Q1 cues to point sensors |
| Engage-on-Remote | Fire weapons on shared Q3 tracks |
| Cooperative Engagement | One ship maintains track while another fires |

## Tactical Data Mesh (TDM)

### Core Behavior

Ships share **track updates**, not ground truth. Sharing is limited by:
- Link tier (capability)
- Bandwidth and latency
- EW environment (jamming, dead zones)
- Doctrine ("share everything" vs. "need-to-know")

### Link Tiers

| Tier | Name | Shares | Supports |
|------|------|--------|----------|
| T0 | Minimal | Coarse cues (Q0/Q1) at low rate | Voice/flags equivalent |
| T1 | Basic | Track kinematics + classification | Standard operations |
| T2 | Rich | Uncertainty, quality, full metadata | Remote cueing |
| T3 | Cooperative | High-rate Q3, engagement data | Engage-on-remote |

### Special Nodes

- **Fusion Node**: Merges many sources, republishes curated STP
- **Relay Node**: Extends mesh coverage (tall mast, drone, buoy)
- **Fixed Site**: Stationary platforms with high-power sensors

## Electronic Warfare

### Effects on Sensing

- **Jamming**: Reduces detection quality, increases false alarms
- **Deception/Decoys**: Injects ghost tracks, corrupts classification

### Effects on Networking

- **Comms Jamming**: Reduces TDM throughput, increases latency
- **ECM Dead Zones**: Map regions with severe degradation

### MVP Approach

Model EW as modifiers on:
- Detection probability
- Track quality
- Ghost track injection rate

## IFF and Identification

IFF is a **track property**, not ground truth. States:

| State | Meaning |
|-------|---------|
| `friendly_authenticated` | Crypto-verified, consistent behavior |
| `friendly_assumed` | Formation/proximity/doctrine-based |
| `unknown` | No identification |
| `suspect` | Inconsistent emissions/motion |
| `hostile_declared` | ROE or post-engagement |

Deception and uncertainty affect IFF without requiring perfect modeling.

## DRL Observation Guidance

Use track-table observations, not perfect state:

**Own Ship**:
- Kinematics, system status, emissions mode

**Tracks** (up to N):
- Relative position estimate
- Quality and age
- Classification and IFF probabilities
- Uncertainty scalars

Partial observability suggests recurrent policies (GRU/LSTM) or explicit belief-state features.

## Integration with Other Systems

**Full Simulation provides**:
- Sensor suite modules per ship
- Link tier and mesh membership
- Doctrine toggles (EMCON policy)
- Crew quality modifiers

**Battle Map provides**:
- Weather and visibility
- ECM/interference zones
- Terrain occlusion (optional)

**Stationary Platforms contribute**:
- Weather monitoring (forecast improvements)
- Sensor coverage
- Mesh relay/fusion capacity
