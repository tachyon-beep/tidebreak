# ADR 0005: Time Model and Tick Hierarchy

## Status
Accepted

## Context

Tidebreak simulates at multiple timescales:

- **Physics**: Projectile motion, collision detection — needs high frequency.
- **Tactics**: Weapon cooldowns, layer transitions, DRL actions — needs human-scale frequency.
- **Strategy**: Governance decisions, economic cycles, faction relations — needs day-scale frequency.

A single tick rate cannot serve all needs:
- Too fast (0.01s) wastes CPU on strategy.
- Too slow (1 day) can't simulate combat.

The time model also affects:
- DRL observation/action frequency
- Cooldown and duration specifications throughout the codebase
- Determinism (fixed vs variable timestep)

## Decision

Use a **three-tier time hierarchy**:

| Tier | Name | Duration | Context | Fixed? |
|------|------|----------|---------|--------|
| **Substep** | Physics tick | 0.1s (default) | Physics integration, collision, projectile motion | Configurable per battle |
| **Tactical Tick** | Game tick | 1.0s | Combat mechanics, cooldowns, DRL actions, observations | Fixed |
| **Strategic Tick** | Campaign tick | 1 day | Economy, governance, faction AI, world events | Fixed |

### Relationships

```
1 Strategic Tick = 86,400 Tactical Ticks (1 day of combat time, if combat ran continuously)
1 Tactical Tick  = 10 Substeps (at default 0.1s substep)
```

In practice, combat is "zoomed in" — a 10-minute battle is 600 tactical ticks, while strategic time pauses.

### Usage Rules

**Substep** (`time_step_s` in BattlePackage):
- Used for: Physics integration, movement, projectile updates, collision detection.
- Configurable: Yes, per battle (e.g., 0.05s for high-precision scenarios).
- All physics uses fixed substep, never variable dt.

**Tactical Tick** (1.0 second, fixed):
- Used for: Weapon cooldowns, layer transition durations, sensor update rates, DRL observation/action cycle.
- Specified as: `TacticalTick` (tick count) or `TacticalDuration` (tick count as duration).
- Example: `weapon.cooldown = 3` means 3 tactical ticks = 3 seconds.

**Strategic Tick** (1 day, fixed):
- Used for: Governance decisions, economic production, faction morale decay, treaty timers.
- Specified as: `StrategicTick` (tick count) or `StrategicDuration` (tick count as duration).
- Example: `decision_latency = 2` means 2 strategic ticks = 2 days.

### DRL Implications

- **Observation frequency**: Once per tactical tick (1 Hz).
- **Action frequency**: Once per tactical tick (1 Hz).
- **Substeps between actions**: 10 (at default rate).

This means DRL agents make decisions once per second of game time, which aligns with human decision-making tempo.

### Duration Types in Code

```
# Tactical context (combat arena)
cooldown: TacticalDuration      # e.g., 3 ticks = 3 seconds
transition_time: TacticalDuration

# Strategic context (campaign)
decision_latency: StrategicDuration  # e.g., 2 ticks = 2 days
treaty_duration: StrategicDuration
```

Components inherit tick type from their context. Combat components use tactical ticks; governance components use strategic ticks.

## Consequences

### Enables

- **Appropriate precision**: Physics gets high frequency; strategy doesn't waste cycles.
- **Clear specifications**: "Cooldown: 3" unambiguously means 3 seconds in combat context.
- **DRL-friendly**: 1 Hz action rate is standard for game-playing RL.
- **Determinism**: Fixed timesteps at all levels.

### Costs

- **Cognitive overhead**: Contributors must know which tick type a component uses.
- **Conversion risk**: Mixing tick types (e.g., using strategic duration in combat) causes bugs.
- **Inflexibility**: Changing tactical tick from 1.0s would break all tuned values.

### Mitigations

- Type system enforces `TacticalDuration` vs `StrategicDuration` — can't accidentally mix.
- Documentation in `contracts.md` specifies tick type for each component.
- Substep is the only configurable rate; tactical and strategic are fixed.

### Follow-ups Required

- Define `TacticalTick`, `TacticalDuration`, `StrategicTick`, `StrategicDuration` types.
- Audit all duration/cooldown fields in contracts for correct tick type.
- Add glossary entry for time model (done: see glossary.md).
- Ensure BattlePackage documents that `time_step_s` is substep, not tactical tick.

## Alternatives Considered

### Single Tick Rate

One tick rate for everything (e.g., 0.1s everywhere).

**Rejected because**:
- Strategic simulation would run 864,000 ticks per day — wasteful.
- Governance decisions happening at 10 Hz is nonsensical.
- Different systems have fundamentally different update frequencies.

### Variable Timestep

Let dt vary based on frame rate or computation time.

**Rejected because**:
- Violates determinism (see ADR-0003).
- Accumulation errors cause divergence.
- Fixed timestep is simpler and sufficient.

### Different Tactical Tick Rate

Use 0.5s or 2.0s instead of 1.0s.

**Rejected because**:
- 1.0s is intuitive ("cooldown: 3" = 3 seconds).
- 1 Hz is standard DRL action frequency for games.
- Faster (0.5s) doubles observation/action data without clear benefit.
- Slower (2.0s) feels sluggish for combat.

### Continuous Time with Event Scheduling

Discrete-event simulation without fixed ticks.

**Rejected because**:
- Harder to make deterministic (event ordering edge cases).
- Less natural fit for DRL (wants regular observation intervals).
- Fixed timestep is proven and simple.
