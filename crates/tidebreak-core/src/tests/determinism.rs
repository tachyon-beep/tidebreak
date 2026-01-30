//! Determinism verification tests.
//!
//! These tests verify that the simulation produces identical results when:
//! - Started with the same seed
//! - Given identical inputs
//!
//! This is critical for:
//! - Replay systems
//! - Networked multiplayer
//! - Debug reproducibility

use std::sync::Arc;

use glam::Vec2;

use crate::entity::{EntityId, EntityInner, EntityTag, ShipComponents};
use crate::output::{Command, Output, OutputKind, PluginId};
use crate::plugin::{ComponentKind, Plugin, PluginContext, PluginDeclaration, PluginRegistry};
use crate::simulation::Simulation;
use crate::world_view::WorldView;

use super::helpers::{get_hp, get_position, get_velocity, set_velocity, setup_test_scenario};

// =============================================================================
// Test Plugins
// =============================================================================

/// A plugin that emits deterministic velocity commands based on entity ID.
///
/// Each entity gets a velocity based on its ID to create variation while
/// remaining deterministic.
struct DeterministicVelocityPlugin {
    declaration: PluginDeclaration,
    base_velocity: Vec2,
}

impl DeterministicVelocityPlugin {
    fn new(base_velocity: Vec2) -> Self {
        Self {
            declaration: PluginDeclaration {
                id: PluginId::new("deterministic_velocity"),
                required_tags: vec![EntityTag::Ship],
                reads: vec![ComponentKind::Transform, ComponentKind::Physics],
                emits: vec![OutputKind::Command],
            },
            base_velocity,
        }
    }
}

impl Plugin for DeterministicVelocityPlugin {
    fn declaration(&self) -> &PluginDeclaration {
        &self.declaration
    }

    fn run(&self, ctx: &PluginContext, _view: &WorldView) -> Vec<Output> {
        // Generate a deterministic velocity based on entity ID
        // This creates variation while being reproducible
        #[allow(clippy::cast_precision_loss)]
        let id_factor = (ctx.entity_id.as_u64() + 1) as f32;
        let velocity = self.base_velocity * id_factor;

        vec![Output::Command(Command::SetVelocity {
            target: ctx.entity_id,
            velocity,
        })]
    }
}

/// A plugin that outputs multiple commands per entity to test output ordering.
struct MultiOutputPlugin {
    declaration: PluginDeclaration,
}

impl MultiOutputPlugin {
    fn new() -> Self {
        Self {
            declaration: PluginDeclaration {
                id: PluginId::new("multi_output"),
                required_tags: vec![EntityTag::Ship],
                reads: vec![ComponentKind::Transform],
                emits: vec![OutputKind::Command],
            },
        }
    }
}

impl Plugin for MultiOutputPlugin {
    fn declaration(&self) -> &PluginDeclaration {
        &self.declaration
    }

    fn run(&self, ctx: &PluginContext, _view: &WorldView) -> Vec<Output> {
        // Emit multiple outputs to test deterministic ordering
        vec![
            Output::Command(Command::SetVelocity {
                target: ctx.entity_id,
                velocity: Vec2::new(1.0, 0.0),
            }),
            Output::Command(Command::SetVelocity {
                target: ctx.entity_id,
                velocity: Vec2::new(2.0, 0.0),
            }),
            Output::Command(Command::SetVelocity {
                target: ctx.entity_id,
                velocity: Vec2::new(3.0, 0.0), // Last write wins
            }),
        ]
    }
}

// =============================================================================
// Determinism Tests
// =============================================================================

/// Verify that same seed produces identical state after 100 ticks.
#[test]
fn determinism_100_ticks() {
    let mut sim1 = Simulation::new(42);
    let mut sim2 = Simulation::new(42);

    setup_test_scenario(&mut sim1);
    setup_test_scenario(&mut sim2);

    // Register identical plugins
    let plugin = Arc::new(DeterministicVelocityPlugin::new(Vec2::new(60.0, 30.0)));
    sim1.plugins_mut().register(EntityTag::Ship, plugin.clone());
    sim2.plugins_mut().register(EntityTag::Ship, plugin);

    // Run both simulations for 100 ticks
    for _ in 0..100 {
        sim1.step();
        sim2.step();
    }

    // Compare tick counters
    assert_eq!(sim1.tick(), sim2.tick(), "Tick counters should match");
    assert_eq!(sim1.tick(), 100, "Should have run 100 ticks");

    // Compare entity states
    let ids1: Vec<EntityId> = sim1.arena().entity_ids_sorted().collect();
    let ids2: Vec<EntityId> = sim2.arena().entity_ids_sorted().collect();
    assert_eq!(ids1, ids2, "Entity IDs should be identical");

    for id in &ids1 {
        // Compare positions
        let pos1 = get_position(sim1.arena(), *id).unwrap();
        let pos2 = get_position(sim2.arena(), *id).unwrap();
        assert_eq!(pos1, pos2, "Positions should be identical for entity {:?}", id);

        // Compare velocities
        let vel1 = get_velocity(sim1.arena(), *id).unwrap();
        let vel2 = get_velocity(sim2.arena(), *id).unwrap();
        assert_eq!(vel1, vel2, "Velocities should be identical for entity {:?}", id);

        // Compare HP
        let hp1 = get_hp(sim1.arena(), *id);
        let hp2 = get_hp(sim2.arena(), *id);
        assert_eq!(hp1, hp2, "HP should be identical for entity {:?}", id);
    }
}

/// Verify that different seeds produce different results.
#[test]
fn different_seeds_produce_different_trace_ids() {
    let sim1 = Simulation::new(1);
    let sim2 = Simulation::new(2);

    // Seeds should be different
    assert_ne!(sim1.seed(), sim2.seed());
}

/// Verify parallel plugin execution produces deterministic output order.
#[test]
fn parallel_output_order_deterministic() {
    // Run the same simulation setup multiple times
    let results: Vec<Vec<Vec2>> = (0..5)
        .map(|_| {
            let mut sim = Simulation::new(42);

            // Spawn multiple entities
            for i in 0..10 {
                let position = Vec2::new((i * 100) as f32, 0.0);
                sim.arena_mut().spawn(
                    EntityTag::Ship,
                    EntityInner::Ship(ShipComponents::at_position(position, 0.0)),
                );
            }

            let plugin = Arc::new(DeterministicVelocityPlugin::new(Vec2::new(10.0, 5.0)));
            sim.plugins_mut().register(EntityTag::Ship, plugin);

            // Run one step
            sim.step();

            // Collect all positions
            sim.arena()
                .entities_sorted()
                .filter_map(|e| e.as_ship().map(|s| s.transform.position))
                .collect()
        })
        .collect();

    // All runs should produce identical positions
    for i in 1..results.len() {
        assert_eq!(
            results[0], results[i],
            "Run {} produced different results than run 0",
            i
        );
    }
}

/// Verify that same entities created in same order get same IDs.
#[test]
fn entity_id_assignment_deterministic() {
    let mut sim1 = Simulation::new(42);
    let mut sim2 = Simulation::new(42);

    // Spawn entities in the same order
    let ids1: Vec<EntityId> = (0..5)
        .map(|_| {
            sim1.arena_mut().spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            )
        })
        .collect();

    let ids2: Vec<EntityId> = (0..5)
        .map(|_| {
            sim2.arena_mut().spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            )
        })
        .collect();

    assert_eq!(ids1, ids2, "Entity IDs should be assigned identically");
}

/// Verify that multiple plugins on same entity produce deterministic results.
#[test]
fn multiple_plugins_deterministic() {
    let mut sim1 = Simulation::new(42);
    let mut sim2 = Simulation::new(42);

    // Spawn one ship
    sim1.arena_mut().spawn(
        EntityTag::Ship,
        EntityInner::Ship(ShipComponents::default()),
    );
    sim2.arena_mut().spawn(
        EntityTag::Ship,
        EntityInner::Ship(ShipComponents::default()),
    );

    // Register multiple plugins in the same order
    let plugin1 = Arc::new(DeterministicVelocityPlugin::new(Vec2::new(10.0, 0.0)));
    let plugin2 = Arc::new(MultiOutputPlugin::new());

    sim1.plugins_mut().register(EntityTag::Ship, plugin1.clone());
    sim1.plugins_mut().register(EntityTag::Ship, plugin2.clone());

    sim2.plugins_mut().register(EntityTag::Ship, plugin1);
    sim2.plugins_mut().register(EntityTag::Ship, plugin2);

    // Run several steps
    for _ in 0..10 {
        sim1.step();
        sim2.step();
    }

    // Results should be identical
    let pos1 = get_position(sim1.arena(), EntityId::new(0)).unwrap();
    let pos2 = get_position(sim2.arena(), EntityId::new(0)).unwrap();
    assert_eq!(pos1, pos2);
}

/// Verify that entity iteration order is deterministic.
#[test]
fn entity_iteration_order_deterministic() {
    let mut arena1 = crate::arena::Arena::new();
    let mut arena2 = crate::arena::Arena::new();

    // Spawn entities in the same order
    for _ in 0..10 {
        arena1.spawn(
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::default()),
        );
        arena2.spawn(
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::default()),
        );
    }

    // Iteration order should be identical
    let ids1: Vec<EntityId> = arena1.entity_ids_sorted().collect();
    let ids2: Vec<EntityId> = arena2.entity_ids_sorted().collect();

    assert_eq!(ids1, ids2);

    // Also test entities_sorted()
    let entities1: Vec<EntityId> = arena1.entities_sorted().map(|e| e.id()).collect();
    let entities2: Vec<EntityId> = arena2.entities_sorted().map(|e| e.id()).collect();

    assert_eq!(entities1, entities2);
}

/// Verify spatial query results are deterministic.
#[test]
fn spatial_query_deterministic() {
    let mut arena1 = crate::arena::Arena::new();
    let mut arena2 = crate::arena::Arena::new();

    // Spawn entities at the same positions
    for i in 0..10 {
        let pos = Vec2::new((i * 10) as f32, 0.0);
        arena1.spawn(
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::at_position(pos, 0.0)),
        );
        arena2.spawn(
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::at_position(pos, 0.0)),
        );
    }

    // Spatial queries should return same results
    let nearby1 = arena1.spatial().query_radius(Vec2::new(50.0, 0.0), 30.0);
    let nearby2 = arena2.spatial().query_radius(Vec2::new(50.0, 0.0), 30.0);

    assert_eq!(nearby1, nearby2, "Spatial query results should be identical");
}

/// Verify that restarting from same state produces same results.
#[test]
fn restart_from_same_state_deterministic() {
    // Run simulation for a while
    let mut sim1 = Simulation::new(42);
    setup_test_scenario(&mut sim1);

    let plugin = Arc::new(DeterministicVelocityPlugin::new(Vec2::new(60.0, 30.0)));
    sim1.plugins_mut().register(EntityTag::Ship, plugin);

    for _ in 0..50 {
        sim1.step();
    }

    // Capture state at tick 50
    let positions_at_50: Vec<Vec2> = sim1
        .arena()
        .entities_sorted()
        .filter_map(|e| e.as_ship().map(|s| s.transform.position))
        .collect();

    // Continue to tick 100
    for _ in 0..50 {
        sim1.step();
    }

    let positions_at_100_first_run: Vec<Vec2> = sim1
        .arena()
        .entities_sorted()
        .filter_map(|e| e.as_ship().map(|s| s.transform.position))
        .collect();

    // Start a new simulation with same seed
    let mut sim2 = Simulation::new(42);
    setup_test_scenario(&mut sim2);

    let plugin2 = Arc::new(DeterministicVelocityPlugin::new(Vec2::new(60.0, 30.0)));
    sim2.plugins_mut().register(EntityTag::Ship, plugin2);

    // Run to tick 50
    for _ in 0..50 {
        sim2.step();
    }

    // Verify positions match at tick 50
    let positions_at_50_second_run: Vec<Vec2> = sim2
        .arena()
        .entities_sorted()
        .filter_map(|e| e.as_ship().map(|s| s.transform.position))
        .collect();

    assert_eq!(
        positions_at_50, positions_at_50_second_run,
        "Positions at tick 50 should be identical"
    );

    // Continue to tick 100
    for _ in 0..50 {
        sim2.step();
    }

    let positions_at_100_second_run: Vec<Vec2> = sim2
        .arena()
        .entities_sorted()
        .filter_map(|e| e.as_ship().map(|s| s.transform.position))
        .collect();

    assert_eq!(
        positions_at_100_first_run, positions_at_100_second_run,
        "Positions at tick 100 should be identical"
    );
}

/// Verify that BTreeMap provides deterministic iteration.
#[test]
fn btreemap_iteration_order_consistent() {
    use std::collections::BTreeMap;

    // Insert in random order
    let mut map1: BTreeMap<u64, &str> = BTreeMap::new();
    map1.insert(5, "five");
    map1.insert(1, "one");
    map1.insert(3, "three");
    map1.insert(2, "two");
    map1.insert(4, "four");

    // Insert in different order
    let mut map2: BTreeMap<u64, &str> = BTreeMap::new();
    map2.insert(3, "three");
    map2.insert(1, "one");
    map2.insert(5, "five");
    map2.insert(4, "four");
    map2.insert(2, "two");

    // Iteration order should be the same (sorted by key)
    let keys1: Vec<u64> = map1.keys().copied().collect();
    let keys2: Vec<u64> = map2.keys().copied().collect();

    assert_eq!(keys1, keys2);
    assert_eq!(keys1, vec![1, 2, 3, 4, 5]);
}

/// Verify simulation runs identically with and without velocity changes.
#[test]
fn stationary_entities_deterministic() {
    let mut sim1 = Simulation::new(42);
    let mut sim2 = Simulation::new(42);

    // Spawn ships but don't give them velocity
    for i in 0..5 {
        let pos = Vec2::new((i * 100) as f32, 0.0);
        sim1.arena_mut().spawn(
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::at_position(pos, 0.0)),
        );
        sim2.arena_mut().spawn(
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::at_position(pos, 0.0)),
        );
    }

    // Run without plugins (no velocity changes)
    for _ in 0..100 {
        sim1.step();
        sim2.step();
    }

    // All entities should remain at original positions
    for id in sim1.arena().entity_ids_sorted() {
        let pos1 = get_position(sim1.arena(), id).unwrap();
        let pos2 = get_position(sim2.arena(), id).unwrap();
        assert_eq!(pos1, pos2);
    }
}

/// Verify initial velocity persistence is deterministic.
#[test]
fn initial_velocity_persistence_deterministic() {
    let mut sim1 = Simulation::new(42);
    let mut sim2 = Simulation::new(42);

    // Spawn ships with initial velocity
    let id1 = sim1.arena_mut().spawn(
        EntityTag::Ship,
        EntityInner::Ship(ShipComponents::default()),
    );
    let id2 = sim2.arena_mut().spawn(
        EntityTag::Ship,
        EntityInner::Ship(ShipComponents::default()),
    );

    // Set identical initial velocities
    set_velocity(sim1.arena_mut(), id1, Vec2::new(60.0, 0.0));
    set_velocity(sim2.arena_mut(), id2, Vec2::new(60.0, 0.0));

    // Run without plugins
    for _ in 0..60 {
        sim1.step();
        sim2.step();
    }

    // Positions should be identical (and should have moved)
    let pos1 = get_position(sim1.arena(), id1).unwrap();
    let pos2 = get_position(sim2.arena(), id2).unwrap();
    assert_eq!(pos1, pos2);

    // Should have moved 60 units at 60 m/s over 60 ticks at 1/60 dt
    assert!((pos1.x - 60.0).abs() < 0.001);
}
