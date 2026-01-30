//! Simulation module with the 4-phase execution loop.
//!
//! The `Simulation` struct orchestrates the Entity-Plugin-Resolver architecture
//! through a deterministic execution loop:
//!
//! 1. **SNAPSHOT**: Freeze current state (implicit - `current` is immutable during plugins)
//! 2. **PLUGIN**: Execute all plugins in parallel, collecting outputs
//! 3. **RESOLUTION**: Clone current to next, run resolvers with outputs
//! 4. **APPLY**: Swap buffers, advance tick
//!
//! # Determinism
//!
//! The simulation guarantees deterministic execution:
//! - Plugins are executed in parallel but their outputs are sorted deterministically
//! - Entities are iterated in ID order (via `BTreeMap`)
//! - Trace IDs are generated deterministically from the master seed
//!
//! # Example
//!
//! ```
//! use tidebreak_core::simulation::Simulation;
//! use tidebreak_core::entity::{EntityTag, EntityInner, ShipComponents};
//! use glam::Vec2;
//!
//! let mut sim = Simulation::new(42);
//!
//! // Spawn entities
//! let ship_id = sim.arena_mut().spawn(
//!     EntityTag::Ship,
//!     EntityInner::Ship(ShipComponents::at_position(Vec2::new(0.0, 0.0), 0.0)),
//! );
//!
//! // Run simulation steps
//! for _ in 0..10 {
//!     sim.step();
//! }
//!
//! assert_eq!(sim.tick(), 10);
//! ```

use rayon::prelude::*;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::arena::Arena;
use crate::output::{OutputEnvelope, PluginInstanceId, TraceId};
use crate::plugin::{PluginContext, PluginRegistry};
use crate::resolver::{CombatResolver, EventResolver, PhysicsResolver, Resolver};
use crate::world_view::WorldView;

// =============================================================================
// Simulation
// =============================================================================

/// The main simulation orchestrator implementing the 4-phase execution loop.
///
/// `Simulation` manages:
/// - Current and next arena state (double-buffered)
/// - Plugin registry for entity-to-plugin mapping
/// - Resolvers for output processing
/// - Master seed for deterministic trace ID generation
///
/// # Double Buffering
///
/// The simulation uses two arenas:
/// - `current`: Read-only snapshot for plugin execution
/// - `next`: Mutable state that resolvers write to
///
/// After each tick, the buffers are swapped to avoid copying.
///
/// # Determinism
///
/// Given the same seed and same inputs, the simulation produces identical
/// results across runs and platforms. This is achieved by:
/// - Sorting all plugin outputs before resolution
/// - Using `BTreeMap` for entity storage (deterministic iteration)
/// - Generating trace IDs from a hash of (seed, tick, entity, plugin)
pub struct Simulation {
    /// Current arena state (read-only during plugin phase).
    current: Arena,
    /// Next arena state (written to by resolvers).
    next: Arena,
    /// Registry of plugins organized by entity tag.
    plugins: PluginRegistry,
    /// Resolvers that process plugin outputs.
    resolvers: Vec<Box<dyn Resolver>>,
    /// Master seed for deterministic trace ID generation.
    master_seed: u64,
}

impl fmt::Debug for Simulation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Simulation")
            .field("current", &self.current)
            .field("next", &self.next)
            .field("plugins", &self.plugins)
            .field("resolvers", &format!("[{} resolvers]", self.resolvers.len()))
            .field("master_seed", &self.master_seed)
            .finish()
    }
}

impl Simulation {
    /// Creates a new simulation with the given master seed.
    ///
    /// The simulation starts at tick 0 with empty arenas and the default
    /// set of resolvers (Physics, Combat, Event).
    ///
    /// # Arguments
    ///
    /// * `seed` - Master seed for deterministic trace ID generation
    ///
    /// # Example
    ///
    /// ```
    /// use tidebreak_core::simulation::Simulation;
    ///
    /// let sim = Simulation::new(12345);
    /// assert_eq!(sim.tick(), 0);
    /// assert_eq!(sim.seed(), 12345);
    /// ```
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            current: Arena::default(),
            next: Arena::default(),
            plugins: PluginRegistry::new(),
            resolvers: vec![
                Box::new(PhysicsResolver::new()),
                Box::new(CombatResolver::new()),
                Box::new(EventResolver::new()),
            ],
            master_seed: seed,
        }
    }

    /// Executes one simulation tick using the 4-phase execution loop.
    ///
    /// # Execution Phases
    ///
    /// 1. **SNAPSHOT**: The current arena is treated as immutable during this tick.
    ///    Plugins read from a frozen snapshot of the world state.
    ///
    /// 2. **PLUGIN**: All plugins for all entities are executed in parallel.
    ///    Each plugin reads from a `WorldView` scoped to its declared components
    ///    and emits `Output`s wrapped in `OutputEnvelope`s.
    ///
    /// 3. **RESOLUTION**: The next arena is cloned from current. Each resolver
    ///    processes its relevant outputs and mutates the next arena.
    ///
    /// 4. **APPLY**: The current and next arenas are swapped (O(1) pointer swap),
    ///    and the tick counter is advanced.
    ///
    /// # Determinism
    ///
    /// Plugin outputs are sorted by (`entity_id`, `plugin_id`, sequence) before
    /// resolution to ensure deterministic processing regardless of parallel
    /// execution order.
    pub fn step(&mut self) {
        let tick = self.current.current_tick();

        // PHASE 1: SNAPSHOT (implicit - current is immutable during plugin phase)

        // PHASE 2: PLUGIN - execute all plugins in parallel
        let outputs = self.execute_plugins_parallel(tick);

        // PHASE 3: RESOLUTION - clone current to next, run resolvers
        self.next.clone_from(&self.current);
        for resolver in &self.resolvers {
            let relevant: Vec<_> = outputs
                .iter()
                .filter(|o| resolver.handles().contains(&o.output().kind()))
                .collect();
            resolver.resolve(&relevant, &self.current, &mut self.next);
        }

        // PHASE 4: APPLY - swap buffers, advance tick
        std::mem::swap(&mut self.current, &mut self.next);
        self.current.advance_tick();
    }

    /// Executes all plugins in parallel and collects their outputs.
    ///
    /// This method:
    /// 1. Collects all (`entity_id`, `plugin_index`, plugin) tuples
    /// 2. Executes plugins in parallel using rayon
    /// 3. Wraps outputs in envelopes with causal chain metadata
    /// 4. Sorts outputs for deterministic resolution order
    ///
    /// # Arguments
    ///
    /// * `tick` - The current simulation tick
    ///
    /// # Returns
    ///
    /// A vector of `OutputEnvelope`s sorted by (`entity_id`, `plugin_id`, sequence).
    fn execute_plugins_parallel(&self, tick: u64) -> Vec<OutputEnvelope> {
        // Collect (entity_id, plugin_idx, plugin) tuples
        let plugin_instances: Vec<_> = self
            .current
            .entities_sorted()
            .flat_map(|entity| {
                self.plugins
                    .plugins_for(entity.tag())
                    .iter()
                    .enumerate()
                    .map(move |(idx, plugin)| (entity.id(), idx, Arc::clone(plugin)))
            })
            .collect();

        // Execute in parallel with rayon
        let mut all_outputs: Vec<OutputEnvelope> = plugin_instances
            .par_iter()
            .flat_map(|(entity_id, plugin_idx, plugin)| {
                let decl = plugin.declaration();
                let view = WorldView::for_plugin(&self.current, decl, tick);
                let trace_id =
                    self.generate_trace_id(tick, entity_id.as_u64(), *plugin_idx as u64);

                let ctx = PluginContext {
                    entity_id: *entity_id,
                    tick,
                    trace_id,
                };

                let outputs = plugin.run(&ctx, &view);

                // Wrap in envelopes
                // The sequence number is u32, which can hold up to ~4B outputs per plugin per tick.
                // In practice, plugins emit at most a handful of outputs per tick.
                #[allow(clippy::cast_possible_truncation)]
                outputs
                    .into_iter()
                    .enumerate()
                    .map(|(seq, output)| {
                        OutputEnvelope::new(
                            output,
                            PluginInstanceId::new(*entity_id, decl.id.clone()),
                            trace_id,
                            tick,
                            seq as u32,
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        // CRITICAL: Sort for determinism
        all_outputs.sort_by(|a, b| {
            let entity_cmp = a.source().entity_id().cmp(&b.source().entity_id());
            if entity_cmp != std::cmp::Ordering::Equal {
                return entity_cmp;
            }
            let plugin_cmp = a.source().plugin_id().as_str().cmp(b.source().plugin_id().as_str());
            if plugin_cmp != std::cmp::Ordering::Equal {
                return plugin_cmp;
            }
            a.sequence().cmp(&b.sequence())
        });

        all_outputs
    }

    /// Generates a deterministic trace ID from the simulation state.
    ///
    /// The trace ID is derived by hashing:
    /// - Master seed
    /// - Current tick
    /// - Entity ID
    /// - Plugin index
    ///
    /// This ensures reproducible trace IDs across runs with the same seed.
    fn generate_trace_id(&self, tick: u64, entity: u64, plugin: u64) -> TraceId {
        let mut hasher = DefaultHasher::new();
        self.master_seed.hash(&mut hasher);
        tick.hash(&mut hasher);
        entity.hash(&mut hasher);
        plugin.hash(&mut hasher);
        TraceId::new(hasher.finish())
    }

    /// Returns a read-only reference to the current arena state.
    ///
    /// Use this to inspect entities and their components after simulation steps.
    #[must_use]
    pub fn arena(&self) -> &Arena {
        &self.current
    }

    /// Returns a mutable reference to the current arena.
    ///
    /// Use this for initial setup (spawning entities) before running steps.
    /// Avoid mutating the arena during a step - use plugins and resolvers instead.
    #[must_use]
    pub fn arena_mut(&mut self) -> &mut Arena {
        &mut self.current
    }

    /// Returns the current simulation tick.
    ///
    /// The tick counter starts at 0 and increments by 1 after each `step()`.
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.current.current_tick()
    }

    /// Returns a mutable reference to the plugin registry.
    ///
    /// Use this to register plugins before running simulation steps.
    ///
    /// # Example
    ///
    /// ```
    /// use tidebreak_core::simulation::Simulation;
    /// use tidebreak_core::entity::EntityTag;
    /// use tidebreak_core::plugin::{Plugin, PluginContext, PluginDeclaration, PluginId, ComponentKind};
    /// use tidebreak_core::output::{Output, OutputKind};
    /// use tidebreak_core::world_view::WorldView;
    /// use std::sync::Arc;
    ///
    /// struct DummyPlugin {
    ///     declaration: PluginDeclaration,
    /// }
    ///
    /// impl Plugin for DummyPlugin {
    ///     fn declaration(&self) -> &PluginDeclaration {
    ///         &self.declaration
    ///     }
    ///     fn run(&self, _ctx: &PluginContext, _view: &WorldView) -> Vec<Output> {
    ///         vec![]
    ///     }
    /// }
    ///
    /// let mut sim = Simulation::new(42);
    /// let plugin = Arc::new(DummyPlugin {
    ///     declaration: PluginDeclaration {
    ///         id: PluginId::new("dummy"),
    ///         required_tags: vec![EntityTag::Ship],
    ///         reads: vec![ComponentKind::Transform],
    ///         emits: vec![OutputKind::Command],
    ///     },
    /// });
    /// sim.plugins_mut().register(EntityTag::Ship, plugin);
    /// ```
    #[must_use]
    pub fn plugins_mut(&mut self) -> &mut PluginRegistry {
        &mut self.plugins
    }

    /// Returns the master seed used for deterministic trace ID generation.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.master_seed
    }

    /// Adds a custom resolver to the simulation.
    ///
    /// Resolvers are executed in the order they are added. The default resolvers
    /// (Physics, Combat, Event) are added in `new()`.
    ///
    /// # Arguments
    ///
    /// * `resolver` - The resolver to add
    pub fn add_resolver(&mut self, resolver: Box<dyn Resolver>) {
        self.resolvers.push(resolver);
    }

    /// Returns the number of resolvers in the simulation.
    #[must_use]
    pub fn resolver_count(&self) -> usize {
        self.resolvers.len()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EntityInner, EntityTag, ShipComponents};
    use crate::output::{Command, Output, OutputKind, PluginId};
    use crate::plugin::{ComponentKind, Plugin, PluginContext, PluginDeclaration};
    use glam::Vec2;

    // Test plugin that emits a velocity command
    struct VelocityPlugin {
        declaration: PluginDeclaration,
        velocity: Vec2,
    }

    impl VelocityPlugin {
        fn new(velocity: Vec2) -> Self {
            Self {
                declaration: PluginDeclaration {
                    id: PluginId::new("velocity_test"),
                    required_tags: vec![EntityTag::Ship],
                    reads: vec![ComponentKind::Transform, ComponentKind::Physics],
                    emits: vec![OutputKind::Command],
                },
                velocity,
            }
        }
    }

    impl Plugin for VelocityPlugin {
        fn declaration(&self) -> &PluginDeclaration {
            &self.declaration
        }

        fn run(&self, ctx: &PluginContext, _view: &WorldView) -> Vec<Output> {
            vec![Output::Command(Command::SetVelocity {
                target: ctx.entity_id,
                velocity: self.velocity,
            })]
        }
    }

    // No-op plugin for testing - used in tests where plugin output is not needed
    #[allow(dead_code)]
    struct NoOpPlugin {
        declaration: PluginDeclaration,
    }

    #[allow(dead_code)]
    impl NoOpPlugin {
        fn new(id: &str) -> Self {
            Self {
                declaration: PluginDeclaration {
                    id: PluginId::new(id),
                    required_tags: vec![EntityTag::Ship],
                    reads: vec![ComponentKind::Transform],
                    emits: vec![OutputKind::Command],
                },
            }
        }
    }

    impl Plugin for NoOpPlugin {
        fn declaration(&self) -> &PluginDeclaration {
            &self.declaration
        }

        fn run(&self, _ctx: &PluginContext, _view: &WorldView) -> Vec<Output> {
            vec![]
        }
    }

    mod creation_tests {
        use super::*;

        #[test]
        fn new_creates_simulation() {
            let sim = Simulation::new(42);
            assert_eq!(sim.tick(), 0);
            assert_eq!(sim.seed(), 42);
            assert!(sim.arena().is_empty());
        }

        #[test]
        fn arena_mut_allows_setup() {
            let mut sim = Simulation::new(42);
            let ship_id = sim.arena_mut().spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );
            assert!(sim.arena().get(ship_id).is_some());
        }

        #[test]
        fn different_seeds_produce_different_trace_ids() {
            let sim1 = Simulation::new(1);
            let sim2 = Simulation::new(2);

            let trace1 = sim1.generate_trace_id(0, 0, 0);
            let trace2 = sim2.generate_trace_id(0, 0, 0);

            assert_ne!(trace1, trace2);
        }
    }

    mod step_tests {
        use super::*;

        #[test]
        fn step_advances_tick() {
            let mut sim = Simulation::new(42);
            assert_eq!(sim.tick(), 0);

            sim.step();
            assert_eq!(sim.tick(), 1);

            sim.step();
            sim.step();
            assert_eq!(sim.tick(), 3);
        }

        #[test]
        fn step_with_no_plugins_no_entities() {
            let mut sim = Simulation::new(42);
            // Should not panic
            sim.step();
            assert_eq!(sim.tick(), 1);
        }

        #[test]
        fn step_with_entities_no_plugins() {
            let mut sim = Simulation::new(42);
            sim.arena_mut().spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            // Should not panic
            sim.step();
            assert_eq!(sim.tick(), 1);
        }

        #[test]
        fn step_with_plugins() {
            let mut sim = Simulation::new(42);
            let ship_id = sim.arena_mut().spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            let plugin = Arc::new(VelocityPlugin::new(Vec2::new(60.0, 0.0)));
            sim.plugins_mut().register(EntityTag::Ship, plugin);

            // Check plugins registered
            let plugins = sim.plugins_mut().plugins_for(EntityTag::Ship);
            assert_eq!(plugins.len(), 1, "Plugin not registered");

            sim.step();

            // The physics resolver uses FIXED_DT = 1/60
            // velocity = (60, 0), so position += (60, 0) * (1/60) = (1, 0)
            let ship = sim.arena().get(ship_id).unwrap().as_ship().unwrap();

            // First check velocity was set
            assert_eq!(
                ship.physics.velocity,
                Vec2::new(60.0, 0.0),
                "Velocity was not set"
            );

            // After step: plugin emitted SetVelocity, physics resolver set velocity to (60,0)
            // and then integrated: position += velocity * dt = (0,0) + (60,0) * (1/60) = (1, 0)
            let expected_x = 60.0 / 60.0; // 1.0
            assert!(
                (ship.transform.position.x - expected_x).abs() < 0.0001,
                "Expected x={}, got x={}, velocity={:?}",
                expected_x,
                ship.transform.position.x,
                ship.physics.velocity
            );
        }

        #[test]
        fn multiple_steps_accumulate() {
            let mut sim = Simulation::new(42);
            let ship_id = sim.arena_mut().spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            let plugin = Arc::new(VelocityPlugin::new(Vec2::new(60.0, 0.0)));
            sim.plugins_mut().register(EntityTag::Ship, plugin);

            for _ in 0..10 {
                sim.step();
            }

            // After 10 steps at velocity 60, with dt=1/60, position = 10
            let ship = sim.arena().get(ship_id).unwrap().as_ship().unwrap();
            assert!((ship.transform.position.x - 10.0).abs() < 0.0001);
        }
    }

    mod resolver_filtering_tests {
        use super::*;
        use crate::output::Modifier;

        struct DamagePlugin {
            declaration: PluginDeclaration,
            amount: f32,
        }

        impl DamagePlugin {
            fn new(amount: f32) -> Self {
                Self {
                    declaration: PluginDeclaration {
                        id: PluginId::new("damage_test"),
                        required_tags: vec![EntityTag::Ship],
                        reads: vec![ComponentKind::Combat],
                        emits: vec![OutputKind::Modifier],
                    },
                    amount,
                }
            }
        }

        impl Plugin for DamagePlugin {
            fn declaration(&self) -> &PluginDeclaration {
                &self.declaration
            }

            fn run(&self, ctx: &PluginContext, _view: &WorldView) -> Vec<Output> {
                vec![Output::Modifier(Modifier::ApplyDamage {
                    target: ctx.entity_id,
                    amount: self.amount,
                })]
            }
        }

        #[test]
        fn resolver_receives_only_relevant_outputs() {
            let mut sim = Simulation::new(42);
            let ship_id = sim.arena_mut().spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            // Register both a velocity plugin (Command) and damage plugin (Modifier)
            let velocity_plugin = Arc::new(VelocityPlugin::new(Vec2::new(60.0, 0.0)));
            let damage_plugin = Arc::new(DamagePlugin::new(25.0));

            sim.plugins_mut().register(EntityTag::Ship, velocity_plugin);
            sim.plugins_mut().register(EntityTag::Ship, damage_plugin);

            sim.step();

            let ship = sim.arena().get(ship_id).unwrap().as_ship().unwrap();

            // Physics resolver handled velocity
            assert_eq!(ship.physics.velocity, Vec2::new(60.0, 0.0));

            // Combat resolver handled damage
            assert!((ship.combat.hp - 75.0).abs() < 0.0001);
        }
    }

    mod determinism_tests {
        use super::*;

        #[test]
        fn same_seed_same_results() {
            fn run_simulation(seed: u64) -> Vec2 {
                let mut sim = Simulation::new(seed);
                let ship_id = sim.arena_mut().spawn(
                    EntityTag::Ship,
                    EntityInner::Ship(ShipComponents::at_position(Vec2::ZERO, 0.0)),
                );

                let plugin = Arc::new(VelocityPlugin::new(Vec2::new(60.0, 30.0)));
                sim.plugins_mut().register(EntityTag::Ship, plugin);

                for _ in 0..10 {
                    sim.step();
                }

                sim.arena()
                    .get(ship_id)
                    .unwrap()
                    .as_ship()
                    .unwrap()
                    .transform
                    .position
            }

            let pos1 = run_simulation(42);
            let pos2 = run_simulation(42);

            assert_eq!(pos1, pos2);
        }

        #[test]
        fn trace_ids_are_deterministic() {
            let sim = Simulation::new(12345);

            // Same inputs should produce same trace ID
            let trace1 = sim.generate_trace_id(10, 5, 2);
            let trace2 = sim.generate_trace_id(10, 5, 2);
            assert_eq!(trace1, trace2);

            // Different inputs should produce different trace IDs
            let trace3 = sim.generate_trace_id(10, 5, 3);
            assert_ne!(trace1, trace3);
        }

        #[test]
        fn output_order_is_deterministic() {
            // Create multiple entities and plugins to test parallel execution order
            let mut sim = Simulation::new(42);

            for i in 0..5 {
                sim.arena_mut().spawn(
                    EntityTag::Ship,
                    EntityInner::Ship(ShipComponents::at_position(
                        Vec2::new(i as f32 * 100.0, 0.0),
                        0.0,
                    )),
                );
            }

            let plugin = Arc::new(VelocityPlugin::new(Vec2::new(60.0, 0.0)));
            sim.plugins_mut().register(EntityTag::Ship, plugin);

            // Run multiple times to verify determinism despite parallel execution
            let mut results = Vec::new();
            for _ in 0..5 {
                let mut sim_copy = Simulation::new(42);
                for i in 0..5 {
                    sim_copy.arena_mut().spawn(
                        EntityTag::Ship,
                        EntityInner::Ship(ShipComponents::at_position(
                            Vec2::new(i as f32 * 100.0, 0.0),
                            0.0,
                        )),
                    );
                }
                let plugin = Arc::new(VelocityPlugin::new(Vec2::new(60.0, 0.0)));
                sim_copy.plugins_mut().register(EntityTag::Ship, plugin);

                sim_copy.step();

                let positions: Vec<_> = sim_copy
                    .arena()
                    .entities_sorted()
                    .map(|e| e.as_ship().unwrap().transform.position)
                    .collect();
                results.push(positions);
            }

            // All runs should produce identical results
            for i in 1..results.len() {
                assert_eq!(results[0], results[i]);
            }
        }
    }

    mod plugin_execution_tests {
        use super::*;
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct CountingPlugin {
            declaration: PluginDeclaration,
            counter: Arc<AtomicUsize>,
        }

        impl CountingPlugin {
            fn new(counter: Arc<AtomicUsize>) -> Self {
                Self {
                    declaration: PluginDeclaration {
                        id: PluginId::new("counting"),
                        required_tags: vec![EntityTag::Ship],
                        reads: vec![ComponentKind::Transform],
                        emits: vec![OutputKind::Command],
                    },
                    counter,
                }
            }
        }

        impl Plugin for CountingPlugin {
            fn declaration(&self) -> &PluginDeclaration {
                &self.declaration
            }

            fn run(&self, _ctx: &PluginContext, _view: &WorldView) -> Vec<Output> {
                self.counter.fetch_add(1, Ordering::SeqCst);
                vec![]
            }
        }

        #[test]
        fn plugins_execute_for_all_matching_entities() {
            let counter = Arc::new(AtomicUsize::new(0));

            let mut sim = Simulation::new(42);

            // Spawn 5 ships
            for _ in 0..5 {
                sim.arena_mut().spawn(
                    EntityTag::Ship,
                    EntityInner::Ship(ShipComponents::default()),
                );
            }

            let plugin = Arc::new(CountingPlugin::new(Arc::clone(&counter)));
            sim.plugins_mut().register(EntityTag::Ship, plugin);

            sim.step();

            // Plugin should have executed once per ship
            assert_eq!(counter.load(Ordering::SeqCst), 5);
        }

        #[test]
        fn plugins_only_execute_for_matching_tags() {
            let counter = Arc::new(AtomicUsize::new(0));

            let mut sim = Simulation::new(42);

            // Spawn ships and platforms
            sim.arena_mut().spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );
            sim.arena_mut().spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );
            sim.arena_mut().spawn(
                EntityTag::Platform,
                EntityInner::Platform(crate::entity::PlatformComponents::default()),
            );

            let plugin = Arc::new(CountingPlugin::new(Arc::clone(&counter)));
            sim.plugins_mut().register(EntityTag::Ship, plugin);

            sim.step();

            // Plugin should only execute for ships (2), not platforms
            assert_eq!(counter.load(Ordering::SeqCst), 2);
        }

        #[test]
        fn multiple_plugins_per_entity() {
            let counter1 = Arc::new(AtomicUsize::new(0));
            let counter2 = Arc::new(AtomicUsize::new(0));

            let mut sim = Simulation::new(42);
            sim.arena_mut().spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            let plugin1 = Arc::new(CountingPlugin::new(Arc::clone(&counter1)));
            let plugin2 = Arc::new(CountingPlugin::new(Arc::clone(&counter2)));

            sim.plugins_mut().register(EntityTag::Ship, plugin1);
            sim.plugins_mut().register(EntityTag::Ship, plugin2);

            sim.step();

            // Both plugins should execute for the ship
            assert_eq!(counter1.load(Ordering::SeqCst), 1);
            assert_eq!(counter2.load(Ordering::SeqCst), 1);
        }
    }

    mod parallel_vs_sequential_tests {
        use super::*;

        /// Run the simulation with a single thread to compare against parallel execution
        fn run_sequential(seed: u64, entity_count: usize) -> Vec<Vec2> {
            // Note: rayon will use the global thread pool by default
            // For a true sequential test we'd need rayon::ThreadPoolBuilder
            // But for determinism testing, we just need to verify same results
            let mut sim = Simulation::new(seed);

            for i in 0..entity_count {
                sim.arena_mut().spawn(
                    EntityTag::Ship,
                    EntityInner::Ship(ShipComponents::at_position(
                        Vec2::new(i as f32 * 10.0, 0.0),
                        0.0,
                    )),
                );
            }

            let plugin = Arc::new(VelocityPlugin::new(Vec2::new(60.0, 30.0)));
            sim.plugins_mut().register(EntityTag::Ship, plugin);

            for _ in 0..5 {
                sim.step();
            }

            sim.arena()
                .entities_sorted()
                .map(|e| e.as_ship().unwrap().transform.position)
                .collect()
        }

        #[test]
        fn parallel_execution_produces_deterministic_results() {
            // Run the same simulation multiple times
            let results: Vec<_> = (0..3).map(|_| run_sequential(42, 10)).collect();

            // All runs should produce identical positions
            for i in 1..results.len() {
                assert_eq!(
                    results[0], results[i],
                    "Run {} produced different results",
                    i
                );
            }
        }
    }
}
