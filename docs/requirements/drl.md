# DRL Requirements

Requirements for deep reinforcement learning integration and AI training.

See: [design/combat-arena.md](../design/combat-arena.md), [design/architecture.md](../design/architecture.md)

## Environment API (P0)

- Support Gymnasium/PettingZoo-style interface (reset, step)
- Support multi-agent mapping (agent_id to observation/action/reward)
- Support headless mode for fast rollouts
- Support deterministic episodes from seed

## Observation Space (P0)

- Support per-agent observations including:
  - Own ship state (position, velocity, heading, layer, hp, cooldowns, ammo)
  - Sensed contacts (not ground truth): relative position, quality, classification
  - Local environment (current, hazard severity, visibility)
  - Context (team ID, time remaining)

- Support track-table observations reflecting fog of war
- Support fixed-shape observation encoding for neural networks

## Observation Space (P1)

- Support uncertainty features in observations (track age, quality)
- Support action masking (cannot fire during cooldown, cannot dive if incapable)

## Action Space (P0)

- Support hybrid action space:
  - Continuous: throttle, turn rate
  - Discrete: fire_primary, fire_torpedo, surface, submerge

## Reward Structure (P0)

- Support terminal rewards (win/loss/draw)
- Support shaped rewards:
  - Damage dealt minus damage taken
  - Survival bonus
  - Time penalty (avoid stalling)
- Support penalties (hazard zones, wasted ammo, collisions)

## Training Curriculum (P1)

- Support curriculum with progressive complexity:
  1. 1v1 in calm conditions
  2. Add currents and obstacles
  3. Add weather hazards
  4. Add sensor occlusion
  5. Multi-ship coordination
  6. Asymmetric fleet compositions

- Support scripted controllers for curriculum bootstrapping
- Support scenario configuration for curriculum stages

## Multi-Scale Control (P2)

- Support hierarchical control concept:
  - Ship-level tactics (micro)
  - Fleet-level coordination (meso)
  - Strategic/economic decisions (macro)

- Support learnable state representation at each scale

## Robustness (P2)

- Support training with environment variation (weather, hazards)
- Support adversarial conditions configuration (worst-case testing)

## Reproducibility (P0)

- Support episode logging (seed, scenario, actions)
- Support deterministic replay from logs
- Support regression testing against baseline policies

## Evaluation (P1)

- Support evaluation metrics (win rate, survival, efficiency)
- Support baseline comparison (scripted controllers)
- Support TensorBoard or equivalent logging
