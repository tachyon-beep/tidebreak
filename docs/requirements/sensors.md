# Sensor Requirements

Requirements for detection, tracking, fog of war, and electronic warfare.

See: [design/sensors-and-fog.md](../design/sensors-and-fog.md)

## Fog of War (P0)

- Support sensor-limited combat (no perfect information by default)
- Support per-ship track tables with uncertain contacts
- Support tracks aging and decaying when not updated
- Support deterministic sensor behavior given seed

## Track Model (P0)

- Support tracks with position estimate, quality, and age
- Support track classification (surface/sub/air/missile/unknown)
- Support track quality levels: Q0 (cue), Q1 (coarse), Q2 (fire control local), Q3 (fire control shared)
- Support quality determining what actions are possible (engagement, sharing)

## Track Model (P1)

- Support IFF states (authenticated, assumed, unknown, suspect, hostile)
- Support misidentification and uncertain IFF
- Support track source tagging (which sensors contributed)

## Sensor Modalities (P0)

- Support radar for surface/air detection
- Support sonar for underwater detection
- Support layer-dependent sensor availability

## Sensor Modalities (P1)

- Support passive RF (ESM) for emitter detection
- Support visual/EO/IR with weather limitations
- Support sensor range and noise parameters

## Radar Types (P1)

- Support mechanically-scanned radar (slower scan, lower track capacity)
- Support phased-array radar (beam agility, track-while-scan, power cost)
- Support radar role configurations (search, surface, fire control)

## Emissions Control (P1)

- Support EMCON modes reducing detectability but degrading own picture
- Support emissions choices affecting both detection and counter-detection

## Tactical Data Mesh (P1)

- Support sharing tracks between friendly units
- Support link tier determining what gets shared (cue vs. full kinematics)
- Support bandwidth and latency constraints

## Tactical Data Mesh (P2)

- Support fusion nodes merging and republishing tracks
- Support relay nodes extending mesh coverage
- Support stationary platforms as mesh contributors

## Electronic Warfare (P1)

- Support jamming reducing detection quality and increasing false tracks
- Support jamming affecting mesh throughput and latency

## Electronic Warfare (P2)

- Support decoys creating ghost tracks
- Support ECM dead zones as map features
- Support deception corrupting classification and IFF

## Combat System Suites (P2)

- Support suite capabilities distinct from raw radar quality
- Support higher track capacity and better association
- Support remote cueing (ingesting external tracks to point sensors)
- Support engage-on-remote for some weapons on shared Q3 tracks
