# Cross-Cutting Requirements

Requirements for determinism, replay, configuration, and performance.

See: [design/architecture.md](../design/architecture.md), [design/entity-framework.md](../design/entity-framework.md)

## Determinism (P0)

- Support seedable RNG for all simulation and world generation
- Support deterministic simulation given same seed, inputs, and initial state
- Support no hidden state affecting outcomes
- Support no floating-point nondeterminism in critical paths

## Serialization (P0)

- Support full state serialization for save/load
- Support state snapshots for replay
- Support versioned data contracts with forward compatibility

## Replay (P0)

- Support replaying episodes from seed and action log
- Support debugging via step-by-step replay inspection
- Support regression testing via replay comparison

## Telemetry (P1)

- Support structured event logging with timestamps
- Support causal chain IDs (source, cause, trace)
- Support event counts and summaries in battle results

## Telemetry (P2)

- Support causal graph visualization
- Support determinism diff tool (pinpoint divergence tick)

## Configuration (P1)

- Support data-driven configuration for:
  - Ship classes and loadouts
  - Weapons and defenses
  - Layer definitions and transition timings
  - Weather and hazard parameters
  - Factions and starting conditions

- Support repeatable scenario definitions (file-based)
- Support curriculum configuration for DRL

## Performance (P0)

- Support headless simulation faster than real-time
- Support fleet-scale battles (dozens of ships) without stability degradation

## Performance (P1)

- Support configurable simulation fidelity (detail scaling)
- Support profiling hooks for optimization

## Debug Tools (P1)

- Support developer-facing debug overlays for:
  - Detection/sensor state
  - Layer and transition state
  - Weather intensity and hazard zones
  - Track tables and contact quality

## Code Quality (P0)

- Support type checking (mypy strict mode)
- Support linting (ruff)
- Support automated testing (pytest)
- Support pre-commit hooks for code quality
