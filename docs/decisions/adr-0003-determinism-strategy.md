# ADR 0003: Determinism Strategy

## Status
Accepted

## Context

Determinism is required for:

1. **DRL Training**: Reproducible episodes for debugging training regressions.
2. **Replay System**: Recreate any battle from seed + action log.
3. **Multiplayer** (future): Lockstep or validation requires identical simulation.
4. **Debugging**: "Why did this happen?" requires reproducible state.

Determinism has multiple levels:
- **Same-run**: Identical within a single execution.
- **Same-platform**: Identical across runs on the same OS/hardware/build.
- **Cross-platform**: Identical across different OSes, CPUs, compilers.

Cross-platform determinism is significantly harder due to floating-point variance.

## Decision

### Guarantee: Same-Platform Determinism

**Definition**: Same build + same platform + same seed + same inputs = **bit-identical** output.

This is the hard guarantee. Every design and implementation choice must preserve this.

### Stretch Goal: Cross-Platform Determinism

Cross-platform determinism is a **non-goal for MVP** but the architecture should not preclude it. If needed later, options include:
- Fixed-point math for all game logic
- Soft-float libraries
- Constrained floating-point (no transcendentals, careful operation order)

### Implementation Requirements

#### Fixed Timestep

All simulation uses fixed timestep, never variable `dt`:
- **Substep**: 0.1 seconds (configurable per BattlePackage)
- **Tactical tick**: 1.0 seconds (fixed)
- **Strategic tick**: 1 day (fixed)

No frame-rate-dependent physics. No "catch-up" variable steps.

#### Seeded RNG

All randomness uses explicit seeded RNG streams:

```
battle_rng = SeededRNG(battle_package.seed)
```

**RNG discipline**:
- RNG state advances only in the APPLY phase, after all resolution.
- RNG calls are ordered deterministically (by entity_id, then by usage type).
- Each system *may* have its own sub-stream (derived from master seed) to isolate consumption patterns.

#### Iteration Order

All iterations over collections use **stable, deterministic order**:
- Entities: sorted by `entity_id` (ascending)
- Plugins: sorted by `plugin_id` (ascending)
- Outputs: sorted by `(type, tick, source_id, output_id)`
- Map keys: sorted by key (never hash order)

**Never** use Python `dict` iteration without explicit sorting. **Never** use `set` iteration without sorting.

#### Floating-Point Handling

For same-platform determinism:
- Use IEEE 754 double precision (`f64`).
- Avoid transcendental functions where possible (sin, cos, sqrt have platform variance).
- Where transcendentals are needed, document and isolate.
- Do not rely on floating-point associativity: `(a + b) + c` may differ from `a + (b + c)`.

For cross-platform determinism (if pursued later):
- Replace with fixed-point arithmetic, OR
- Use soft-float library, OR
- Constrain to add/sub/mul/div only with explicit operation order.

#### State Serialization

All state must be serializable and round-trip identical:
- No transient/cached values that affect behavior.
- Private plugin state stored in `PrivateComponent`, fully serialized.
- Replay = deserialize initial state + replay action log.

## Consequences

### Enables

- DRL training with reproducible episodes.
- Replay system for debugging and spectating.
- Deterministic tests (run 1000 battles, expect diff=0).
- Future multiplayer via lockstep or validation.

### Costs

- **Performance constraints**: Fixed timestep may require more steps than adaptive. Can't skip frames.
- **Implementation discipline**: Every iteration, every RNG call, every float operation must be careful.
- **No easy transcendentals**: Heading math, distance calculations need care.
- **Testing overhead**: Determinism tests must run on CI.

### Follow-ups Required

- Implement `SeededRNG` wrapper with stream derivation.
- Establish coding guidelines for iteration order.
- Add determinism regression tests to CI.
- Document which operations are safe vs require review.
- If cross-platform needed: spike fixed-point math library.

## Alternatives Considered

### Cross-Platform Determinism as Requirement

Require bit-identical results across Windows/Linux/Mac/ARM/x86.

**Rejected because**:
- Requires fixed-point math or soft-float, adding significant complexity.
- Limits language/library choices (no numpy with platform-native BLAS).
- MVP doesn't need multiplayer; DRL training is same-platform.
- Can be added later without architectural change (swap math layer).

### Non-Deterministic with Statistical Validation

Accept non-determinism, validate via statistical properties.

**Rejected because**:
- Debugging becomes "run 1000 times and check distribution."
- Replays are impossible.
- DRL training regressions are harder to diagnose.
- The cost of determinism is paid once; the benefit compounds.

### Variable Timestep with Interpolation

Use variable dt, interpolate for display.

**Rejected because**:
- Accumulation errors cause divergence over time.
- "Close enough" isn't good enough for replay.
- Fixed timestep is simpler and sufficient.
