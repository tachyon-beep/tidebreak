# Entity-Plugin-Resolver Implementation Design

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement the core Entity-Plugin-Resolver architecture for Tidebreak's combat arena.

**Architecture:** Reactive ECS with 4-phase execution loop, typed entities, causal output tracking, deterministic parallel plugin execution.

**Tech Stack:** Rust (tidebreak-core), ChaCha8Rng for determinism, rayon for parallelization, BTreeMap for ordered storage.

---

## Design Overview

This design addresses all concerns from the architecture review:

| Concern | Resolution |
|---------|------------|
| Entity enum anti-pattern | Hybrid approach with EntityTag separation (ADR-0007) |
| Missing causal chains | OutputEnvelope wrapper with source_id, cause_id, trace_id |
| Parallel plugin execution | Explicit collection + sort strategy |
| RNG in resolution phase | Moved to APPLY phase per ADR-0003 |
| Output enum monolith | Nested enums by category |
| WorldView unbounded access | Scoped by plugin declarations |
| clone_from performance | Deferred - benchmark first, optimize if needed |

---

## 1. Core Types

### 1.1 Entity Storage (per ADR-0007)

```rust
use std::collections::BTreeMap;

/// Unique entity identifier, monotonically increasing
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(pub u64);

/// Tag determines plugin bundle, decoupled from storage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityTag {
    Ship,
    Platform,
    Projectile,
    Squadron,
}

/// Entity wrapper providing identity and dispatch
pub struct Entity {
    pub id: EntityId,
    pub tag: EntityTag,
    pub inner: EntityInner,
}

/// Storage enum - concrete types, no trait objects
pub enum EntityInner {
    Ship(ShipComponents),
    Platform(PlatformComponents),
    Projectile(ProjectileComponents),
    Squadron(SquadronComponents),
}

/// Ship components - all state a ship can have
pub struct ShipComponents {
    pub transform: TransformState,
    pub physics: PhysicsState,
    pub combat: CombatState,
    pub sensor: SensorState,
    pub inventory: InventoryState,
}

/// Shared access via traits
pub trait HasTransform {
    fn transform(&self) -> &TransformState;
    fn transform_mut(&mut self) -> &mut TransformState;
}

pub trait HasPhysics {
    fn physics(&self) -> &PhysicsState;
    fn physics_mut(&mut self) -> &mut PhysicsState;
}

// Implement for each entity type that has the component
impl HasTransform for ShipComponents { /* ... */ }
impl HasTransform for PlatformComponents { /* ... */ }
impl HasTransform for ProjectileComponents { /* ... */ }
```

### 1.2 State Components

```rust
use glam::Vec2;

/// Position and orientation
pub struct TransformState {
    pub position: Vec2,
    pub heading: f32,  // radians, CCW from +X
}

/// Velocity and movement
pub struct PhysicsState {
    pub velocity: Vec2,
    pub angular_velocity: f32,
    pub max_speed: f32,
    pub max_turn_rate: f32,
}

/// Combat state
pub struct CombatState {
    pub hp: f32,
    pub max_hp: f32,
    pub weapons: Vec<WeaponState>,
    pub status_flags: StatusFlags,
}

bitflags::bitflags! {
    pub struct StatusFlags: u32 {
        const MOBILITY_DISABLED = 0b0001;
        const WEAPONS_DISABLED  = 0b0010;
        const SENSORS_DISABLED  = 0b0100;
        const DESTROYED         = 0b1000;
    }
}

/// Sensor state
pub struct SensorState {
    pub radar_range: f32,
    pub sonar_range: f32,
    pub emissions_mode: EmissionsMode,
    pub track_table: Vec<Track>,
}

/// Inventory state
pub struct InventoryState {
    pub fuel: f32,
    pub ammo: HashMap<AmmoType, u32>,
}
```

---

## 2. Output System with Causal Chains

### 2.1 Output Categories (Nested Enums)

```rust
/// Top-level output categorization for resolver routing
pub enum Output {
    Command(Command),
    Modifier(Modifier),
    Event(Event),
}

/// Commands - actions to attempt
pub enum Command {
    SetVelocity { target: EntityId, velocity: Vec2 },
    SetHeading { target: EntityId, heading: f32 },
    FireWeapon { source: EntityId, target: EntityId, slot: usize },
    SpawnProjectile { source: EntityId, weapon_slot: usize, target_pos: Vec2 },
}

/// Modifiers - stat changes
pub enum Modifier {
    ApplyDamage { target: EntityId, amount: f32 },
    ApplyHealing { target: EntityId, amount: f32 },
    SetStatusFlag { target: EntityId, flag: StatusFlags, value: bool },
    ModifyStat { target: EntityId, stat: StatId, delta: f32 },
}

/// Events - facts for telemetry and reactions
pub enum Event {
    WeaponFired { source: EntityId, weapon_slot: usize },
    DamageDealt { source: EntityId, target: EntityId, amount: f32 },
    EntityDestroyed { entity: EntityId, destroyer: Option<EntityId> },
    ContactDetected { observer: EntityId, target: EntityId, quality: TrackQuality },
}

/// Output kind for resolver routing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutputKind {
    Command,
    Modifier,
    Event,
}

impl Output {
    pub fn kind(&self) -> OutputKind {
        match self {
            Output::Command(_) => OutputKind::Command,
            Output::Modifier(_) => OutputKind::Modifier,
            Output::Event(_) => OutputKind::Event,
        }
    }
}
```

### 2.2 Output Envelope with Causal Chains

```rust
/// Wrapper providing causal chain metadata for all outputs
#[derive(Debug, Clone)]
pub struct OutputEnvelope {
    /// The actual output
    pub output: Output,

    /// Which entity + plugin emitted this
    pub source: PluginInstanceId,

    /// Upstream event that triggered this (if reactive)
    pub cause: Option<EventId>,

    /// Root trace identifier for the causal chain
    pub trace_id: TraceId,

    /// Tick when emitted
    pub tick: u64,

    /// Index within this plugin's outputs (for deterministic ordering)
    pub sequence: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PluginInstanceId {
    pub entity_id: EntityId,
    pub plugin_id: PluginId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TraceId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventId(pub u64);
```

---

## 3. Plugin System

### 3.1 Plugin Declaration

```rust
/// Plugin identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PluginId(pub &'static str);

/// Declaration of what a plugin reads and emits
pub struct PluginDeclaration {
    pub id: PluginId,
    /// Entity tags this plugin applies to
    pub required_tags: Vec<EntityTag>,
    /// Components this plugin reads (for WorldView scoping)
    pub reads: Vec<ComponentKind>,
    /// Output kinds this plugin may emit
    pub emits: Vec<OutputKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentKind {
    Transform,
    Physics,
    Combat,
    Sensor,
    Inventory,
}
```

### 3.2 Plugin Trait

```rust
/// Plugin execution context
pub struct PluginContext<'a> {
    pub entity_id: EntityId,
    pub tick: u64,
    pub trace_id: TraceId,
}

/// Plugin trait - pure function from WorldView to Outputs
pub trait Plugin: Send + Sync {
    /// Static declaration of this plugin's requirements
    fn declaration(&self) -> &PluginDeclaration;

    /// Execute plugin, returning outputs
    ///
    /// INVARIANT: Must not access anything outside the provided WorldView
    /// INVARIANT: Must be deterministic given same inputs
    fn run(
        &self,
        ctx: &PluginContext,
        view: &WorldView,
    ) -> Vec<Output>;
}
```

### 3.3 Plugin Registry & Bundles

```rust
/// Registry mapping entity tags to plugin bundles
pub struct PluginRegistry {
    bundles: HashMap<EntityTag, Vec<Arc<dyn Plugin>>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        let mut registry = Self { bundles: HashMap::new() };

        // Ship bundle: movement, weapons, sensors
        registry.bundles.insert(EntityTag::Ship, vec![
            Arc::new(MovementPlugin),
            Arc::new(WeaponPlugin),
            Arc::new(SensorPlugin),
        ]);

        // Platform bundle: sensors only (stationary)
        registry.bundles.insert(EntityTag::Platform, vec![
            Arc::new(SensorPlugin),
        ]);

        // Projectile bundle: homing logic
        registry.bundles.insert(EntityTag::Projectile, vec![
            Arc::new(ProjectilePlugin),
        ]);

        registry
    }

    pub fn plugins_for(&self, tag: EntityTag) -> &[Arc<dyn Plugin>] {
        self.bundles.get(&tag).map(|v| v.as_slice()).unwrap_or(&[])
    }
}
```

---

## 4. WorldView (Scoped Access)

### 4.1 Scoped WorldView

```rust
/// Immutable view of world state, scoped by plugin declaration
pub struct WorldView<'a> {
    arena: &'a Arena,
    tick: u64,
    /// Components this view is allowed to access
    allowed_components: &'a [ComponentKind],
}

impl<'a> WorldView<'a> {
    /// Create a scoped WorldView for a specific plugin
    pub fn for_plugin(arena: &'a Arena, decl: &'a PluginDeclaration, tick: u64) -> Self {
        Self {
            arena,
            tick,
            allowed_components: &decl.reads,
        }
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Get entity by ID (always allowed)
    pub fn get_entity(&self, id: EntityId) -> Option<&Entity> {
        self.arena.entities.get(&id)
    }

    /// Get transform state (checks permission)
    pub fn get_transform(&self, id: EntityId) -> Option<&TransformState> {
        self.check_access(ComponentKind::Transform)?;
        self.arena.get_component::<TransformState>(id)
    }

    /// Get combat state (checks permission)
    pub fn get_combat(&self, id: EntityId) -> Option<&CombatState> {
        self.check_access(ComponentKind::Combat)?;
        self.arena.get_component::<CombatState>(id)
    }

    /// Spatial query - entities within radius
    pub fn query_in_radius(&self, center: Vec2, radius: f32) -> Vec<EntityId> {
        self.arena.spatial.query_radius(center, radius)
    }

    /// Query entities by tag
    pub fn query_by_tag(&self, tag: EntityTag) -> impl Iterator<Item = EntityId> + '_ {
        self.arena.entities.values()
            .filter(move |e| e.tag == tag)
            .map(|e| e.id)
    }

    fn check_access(&self, kind: ComponentKind) -> Option<()> {
        if self.allowed_components.contains(&kind) {
            Some(())
        } else {
            // In debug builds, panic to catch violations early
            #[cfg(debug_assertions)]
            panic!("Plugin accessed undeclared component: {:?}", kind);
            #[cfg(not(debug_assertions))]
            None
        }
    }
}
```

---

## 5. Resolver System

### 5.1 Resolver Trait

```rust
/// Resolver processes outputs and mutates NextState
pub trait Resolver: Send + Sync {
    /// Which output kinds this resolver handles
    fn handles(&self) -> &[OutputKind];

    /// Resolve outputs into state mutations
    ///
    /// INVARIANT: Only mutate `next`, never read from it (use `current`)
    /// INVARIANT: Must be deterministic given same inputs + output order
    fn resolve(
        &self,
        outputs: &[&OutputEnvelope],
        current: &Arena,
        next: &mut Arena,
    );
}
```

### 5.2 MVP Resolvers

```rust
/// Physics resolver - handles movement commands
pub struct PhysicsResolver;

impl Resolver for PhysicsResolver {
    fn handles(&self) -> &[OutputKind] {
        &[OutputKind::Command]
    }

    fn resolve(&self, outputs: &[&OutputEnvelope], current: &Arena, next: &mut Arena) {
        for envelope in outputs {
            match &envelope.output {
                Output::Command(Command::SetVelocity { target, velocity }) => {
                    if let Some(entity) = next.get_mut(*target) {
                        if let Some(physics) = entity.physics_mut() {
                            physics.velocity = *velocity;
                        }
                    }
                }
                Output::Command(Command::SetHeading { target, heading }) => {
                    if let Some(entity) = next.get_mut(*target) {
                        if let Some(transform) = entity.transform_mut() {
                            transform.heading = *heading;
                        }
                    }
                }
                _ => {} // Ignore non-physics commands
            }
        }

        // Apply physics integration
        let dt = 1.0 / 60.0; // Fixed timestep
        for entity in next.entities.values_mut() {
            if let Some(transform) = entity.transform_mut() {
                if let Some(physics) = entity.physics() {
                    transform.position += physics.velocity * dt;
                }
            }
        }
    }
}

/// Combat resolver - handles damage and weapons
pub struct CombatResolver;

impl Resolver for CombatResolver {
    fn handles(&self) -> &[OutputKind] {
        &[OutputKind::Command, OutputKind::Modifier]
    }

    fn resolve(&self, outputs: &[&OutputEnvelope], current: &Arena, next: &mut Arena) {
        for envelope in outputs {
            match &envelope.output {
                Output::Modifier(Modifier::ApplyDamage { target, amount }) => {
                    if let Some(entity) = next.get_mut(*target) {
                        if let Some(combat) = entity.combat_mut() {
                            combat.hp = (combat.hp - amount).max(0.0);
                            if combat.hp <= 0.0 {
                                combat.status_flags |= StatusFlags::DESTROYED;
                            }
                        }
                    }
                }
                Output::Modifier(Modifier::SetStatusFlag { target, flag, value }) => {
                    if let Some(entity) = next.get_mut(*target) {
                        if let Some(combat) = entity.combat_mut() {
                            if *value {
                                combat.status_flags |= *flag;
                            } else {
                                combat.status_flags -= *flag;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// Event resolver - records events for telemetry (no state mutation)
pub struct EventResolver {
    event_log: Vec<OutputEnvelope>,
}

impl Resolver for EventResolver {
    fn handles(&self) -> &[OutputKind] {
        &[OutputKind::Event]
    }

    fn resolve(&self, outputs: &[&OutputEnvelope], _current: &Arena, _next: &mut Arena) {
        // Events don't mutate state, just log for telemetry/replay
        // In real impl, append to event log
    }
}
```

---

## 6. Execution Loop (4-Phase with Parallel Plugins)

### 6.1 Arena Structure

```rust
pub struct Arena {
    /// Monotonic ID counter
    next_id: u64,

    /// All entities, BTreeMap for deterministic iteration
    pub entities: BTreeMap<EntityId, Entity>,

    /// Spatial index for proximity queries
    pub spatial: SpatialIndex,

    /// Current tick
    pub tick: u64,

    /// Trace ID counter for causal chains
    next_trace_id: u64,
}

impl Arena {
    pub fn spawn(&mut self, tag: EntityTag, inner: EntityInner) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;

        let entity = Entity { id, tag, inner };

        // Update spatial index
        if let Some(transform) = entity.transform() {
            self.spatial.insert(id, transform.position);
        }

        self.entities.insert(id, entity);
        id
    }

    pub fn despawn(&mut self, id: EntityId) {
        self.entities.remove(&id);
        self.spatial.remove(id);
    }

    pub fn get(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    pub fn new_trace_id(&mut self) -> TraceId {
        let id = TraceId(self.next_trace_id);
        self.next_trace_id += 1;
        id
    }
}
```

### 6.2 Simulation Core

```rust
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;

pub struct Simulation {
    /// Current state (read during PLUGIN phase)
    current: Arena,

    /// Next state (written during RESOLUTION phase)
    next: Arena,

    /// Plugin registry
    plugins: PluginRegistry,

    /// Resolvers in execution order
    resolvers: Vec<Box<dyn Resolver>>,

    /// Master RNG seed
    master_seed: u64,
}

impl Simulation {
    pub fn new(seed: u64) -> Self {
        Self {
            current: Arena::default(),
            next: Arena::default(),
            plugins: PluginRegistry::new(),
            resolvers: vec![
                Box::new(PhysicsResolver),
                Box::new(CombatResolver),
                Box::new(EventResolver::new()),
            ],
            master_seed: seed,
        }
    }

    /// Execute one simulation tick
    pub fn step(&mut self) {
        let tick = self.current.tick;

        // ============================================
        // PHASE 1: SNAPSHOT
        // ============================================
        // WorldView is created per-plugin in phase 2
        // Current state is immutable during plugin phase

        // ============================================
        // PHASE 2: PLUGIN (Parallel)
        // ============================================
        let outputs = self.execute_plugins_parallel(tick);

        // ============================================
        // PHASE 3: RESOLUTION (Sequential, Deterministic)
        // ============================================
        self.next.clone_from(&self.current);

        for resolver in &self.resolvers {
            let relevant: Vec<_> = outputs.iter()
                .filter(|o| resolver.handles().contains(&o.output.kind()))
                .collect();

            resolver.resolve(&relevant, &self.current, &mut self.next);
        }

        // ============================================
        // PHASE 4: APPLY
        // ============================================
        // Swap buffers
        std::mem::swap(&mut self.current, &mut self.next);

        // Advance tick
        self.current.tick += 1;

        // RNG advances here (per ADR-0003) if needed for next tick
        // Currently no tick-level randomness needed
    }

    /// Execute all plugins in parallel, return sorted outputs
    fn execute_plugins_parallel(&self, tick: u64) -> Vec<OutputEnvelope> {
        // Collect (entity_id, plugin_id) pairs for deterministic ordering
        let plugin_instances: Vec<_> = self.current.entities.iter()
            .flat_map(|(entity_id, entity)| {
                self.plugins.plugins_for(entity.tag)
                    .iter()
                    .enumerate()
                    .map(move |(idx, plugin)| (*entity_id, idx, plugin.clone()))
            })
            .collect();

        // Execute in parallel, collect outputs with ordering metadata
        let mut all_outputs: Vec<OutputEnvelope> = plugin_instances
            .par_iter()
            .flat_map(|(entity_id, plugin_idx, plugin)| {
                let decl = plugin.declaration();
                let view = WorldView::for_plugin(&self.current, decl, tick);

                let ctx = PluginContext {
                    entity_id: *entity_id,
                    tick,
                    trace_id: TraceId(hash(self.master_seed, tick, entity_id.0, *plugin_idx as u64)),
                };

                let outputs = plugin.run(&ctx, &view);

                // Wrap outputs with metadata
                outputs.into_iter().enumerate().map(|(seq, output)| {
                    OutputEnvelope {
                        output,
                        source: PluginInstanceId {
                            entity_id: *entity_id,
                            plugin_id: decl.id,
                        },
                        cause: None, // Set by reactive plugins
                        trace_id: ctx.trace_id,
                        tick,
                        sequence: seq as u32,
                    }
                }).collect::<Vec<_>>()
            })
            .collect();

        // CRITICAL: Sort for determinism after parallel collection
        all_outputs.sort_by_key(|o| (
            o.source.entity_id,
            o.source.plugin_id.0,
            o.sequence,
        ));

        all_outputs
    }
}

/// Deterministic hash for RNG seeding
fn hash(seed: u64, tick: u64, entity: u64, plugin: u64) -> u64 {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;

    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    tick.hash(&mut hasher);
    entity.hash(&mut hasher);
    plugin.hash(&mut hasher);
    hasher.finish()
}
```

---

## 7. Testing Strategy

### 7.1 Determinism Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn determinism_100_ticks() {
        let mut sim1 = Simulation::new(42);
        let mut sim2 = Simulation::new(42);

        // Setup identical initial state
        setup_test_scenario(&mut sim1);
        setup_test_scenario(&mut sim2);

        // Run both for 100 ticks
        for _ in 0..100 {
            sim1.step();
            sim2.step();
        }

        // States must be identical
        assert_eq!(sim1.current, sim2.current);
    }

    #[test]
    fn parallel_output_order_matches_sequential() {
        let mut sim = Simulation::new(42);
        setup_test_scenario(&mut sim);

        // Run parallel
        let parallel_outputs = sim.execute_plugins_parallel(0);

        // Run sequential
        let sequential_outputs = execute_plugins_sequential(&sim, 0);

        // Order must match after sorting
        assert_eq!(parallel_outputs, sequential_outputs);
    }

    #[test]
    fn plugin_cannot_access_undeclared_component() {
        // Plugin declares only Transform access
        // Attempting to access Combat should panic in debug
    }

    #[test]
    fn resolver_conflict_resolution() {
        // Two plugins emit ApplyDamage to same target
        // Both should apply, in deterministic order
    }
}
```

---

## 8. Implementation Phases

### Phase 1: Core Types (This PR)
- [ ] EntityId, EntityTag, Entity, EntityInner
- [ ] State components (Transform, Physics, Combat, Sensor, Inventory)
- [ ] HasTransform, HasPhysics traits
- [ ] Output enum hierarchy
- [ ] OutputEnvelope with causal chain fields

### Phase 2: Execution Loop
- [ ] Arena with BTreeMap storage
- [ ] WorldView with scoped access
- [ ] Plugin trait and PluginDeclaration
- [ ] PluginRegistry with bundles
- [ ] 4-phase step() implementation
- [ ] Parallel plugin execution with sorted output

### Phase 3: Resolvers
- [ ] Resolver trait
- [ ] PhysicsResolver
- [ ] CombatResolver
- [ ] EventResolver

### Phase 4: MVP Plugins
- [ ] MovementPlugin (velocity/heading commands)
- [ ] WeaponPlugin (fire commands, cooldowns)
- [ ] SensorPlugin (detection, track updates)
- [ ] ProjectilePlugin (homing logic)

### Phase 5: Testing & Validation
- [ ] Determinism test suite
- [ ] Plugin isolation tests
- [ ] Resolver conflict tests
- [ ] Benchmark parallel vs sequential

---

## 9. Open Questions (Deferred)

1. **clone_from performance**: Benchmark before optimizing. If problematic, consider copy-on-write or delta tracking.

2. **Spatial index integration with Murk**: Current design uses simple SpatialIndex. Murk octree can be integrated later for field queries.

3. **Event sourcing for replay**: Current design logs events but doesn't reconstruct from them. Full event sourcing is P1.

4. **Private plugin memory**: Design mentions PrivateComponent for plugin state. Defer until a plugin needs it.
