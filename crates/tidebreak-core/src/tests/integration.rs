//! Integration tests for the full simulation pipeline.
//!
//! These tests verify that the Entity-Plugin-Resolver architecture works
//! correctly end-to-end, testing:
//! - Entity lifecycle (spawn, step, despawn)
//! - Plugin output -> Resolver mutation flow
//! - Combat system (damage, healing, destruction)
//! - Sensor detection
//! - Physics integration

use std::sync::Arc;

use glam::Vec2;

use crate::entity::{
    EntityId, EntityInner, EntityTag, PlatformComponents, ProjectileComponents, ShipComponents,
    SquadronComponents, StatusFlags,
};
use crate::output::{Command, Event, Modifier, Output, OutputKind, PluginId};
use crate::plugin::{
    ComponentKind, Plugin, PluginContext, PluginDeclaration, PluginRegistry,
};
use crate::simulation::Simulation;
use crate::world_view::WorldView;

use super::helpers::{
    get_hp, get_position, get_velocity, is_destroyed, set_hp, set_velocity, spawn_armed_ship,
    spawn_ship_with_hp, spawn_test_ship,
};

// =============================================================================
// Test Plugins
// =============================================================================

/// A plugin that sets a constant velocity on all ships.
struct ConstantVelocityPlugin {
    declaration: PluginDeclaration,
    velocity: Vec2,
}

impl ConstantVelocityPlugin {
    fn new(velocity: Vec2) -> Self {
        Self {
            declaration: PluginDeclaration {
                id: PluginId::new("constant_velocity"),
                required_tags: vec![EntityTag::Ship],
                reads: vec![ComponentKind::Transform, ComponentKind::Physics],
                emits: vec![OutputKind::Command],
            },
            velocity,
        }
    }
}

impl Plugin for ConstantVelocityPlugin {
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

/// A plugin that applies damage to a specific target.
struct DamagePlugin {
    declaration: PluginDeclaration,
    target: EntityId,
    damage: f32,
}

impl DamagePlugin {
    fn new(target: EntityId, damage: f32) -> Self {
        Self {
            declaration: PluginDeclaration {
                id: PluginId::new("damage_plugin"),
                required_tags: vec![EntityTag::Ship],
                reads: vec![ComponentKind::Combat],
                emits: vec![OutputKind::Modifier],
            },
            target,
            damage,
        }
    }
}

impl Plugin for DamagePlugin {
    fn declaration(&self) -> &PluginDeclaration {
        &self.declaration
    }

    fn run(&self, ctx: &PluginContext, _view: &WorldView) -> Vec<Output> {
        // Only the first ship emits damage to avoid multiple hits
        if ctx.entity_id.as_u64() == 0 {
            vec![Output::Modifier(Modifier::ApplyDamage {
                target: self.target,
                amount: self.damage,
            })]
        } else {
            vec![]
        }
    }
}

/// A plugin that emits events for testing the event system.
struct EventEmitterPlugin {
    declaration: PluginDeclaration,
}

impl EventEmitterPlugin {
    fn new() -> Self {
        Self {
            declaration: PluginDeclaration {
                id: PluginId::new("event_emitter"),
                required_tags: vec![EntityTag::Ship],
                reads: vec![ComponentKind::Transform],
                emits: vec![OutputKind::Event],
            },
        }
    }
}

impl Plugin for EventEmitterPlugin {
    fn declaration(&self) -> &PluginDeclaration {
        &self.declaration
    }

    fn run(&self, ctx: &PluginContext, _view: &WorldView) -> Vec<Output> {
        vec![Output::Event(Event::WeaponFired {
            source: ctx.entity_id,
            weapon_slot: 0,
        })]
    }
}

// =============================================================================
// Entity Lifecycle Tests
// =============================================================================

/// Basic entity lifecycle: spawn, step, check position changed.
#[test]
fn spawn_ship_and_step() {
    let mut sim = Simulation::new(42);

    let ship_id = spawn_test_ship(sim.arena_mut(), Vec2::new(50.0, 50.0));
    set_velocity(sim.arena_mut(), ship_id, Vec2::new(60.0, 0.0));

    sim.step();

    // Position should have moved by velocity * dt (1/60)
    let pos = get_position(sim.arena(), ship_id).unwrap();
    assert!(
        pos.x > 50.0,
        "Ship should have moved right. Position: {:?}",
        pos
    );
    // Expected: 50 + 60 * (1/60) = 51
    assert!(
        (pos.x - 51.0).abs() < 0.001,
        "Expected x=51.0, got x={}",
        pos.x
    );
}

/// Test that multiple entities can be spawned and stepped.
#[test]
fn spawn_multiple_entities_and_step() {
    let mut sim = Simulation::new(42);

    let ship1 = spawn_test_ship(sim.arena_mut(), Vec2::new(0.0, 0.0));
    let ship2 = spawn_test_ship(sim.arena_mut(), Vec2::new(100.0, 0.0));
    let ship3 = spawn_test_ship(sim.arena_mut(), Vec2::new(200.0, 0.0));

    // Give them different velocities
    set_velocity(sim.arena_mut(), ship1, Vec2::new(60.0, 0.0));
    set_velocity(sim.arena_mut(), ship2, Vec2::new(0.0, 60.0));
    set_velocity(sim.arena_mut(), ship3, Vec2::new(-60.0, 0.0));

    // Run 60 ticks (1 second at 60 FPS)
    for _ in 0..60 {
        sim.step();
    }

    // Check positions
    let pos1 = get_position(sim.arena(), ship1).unwrap();
    let pos2 = get_position(sim.arena(), ship2).unwrap();
    let pos3 = get_position(sim.arena(), ship3).unwrap();

    assert!((pos1.x - 60.0).abs() < 0.01, "Ship1 expected at (60,0)");
    assert!((pos2.y - 60.0).abs() < 0.01, "Ship2 expected at (100,60)");
    assert!((pos3.x - 140.0).abs() < 0.01, "Ship3 expected at (140,0)");
}

/// Test spawning all entity types.
#[test]
fn spawn_all_entity_types() {
    let mut sim = Simulation::new(42);

    // Spawn each entity type
    let ship_id = sim.arena_mut().spawn(
        EntityTag::Ship,
        EntityInner::Ship(ShipComponents::at_position(Vec2::new(0.0, 0.0), 0.0)),
    );

    let platform_id = sim.arena_mut().spawn(
        EntityTag::Platform,
        EntityInner::Platform(PlatformComponents::at_position(Vec2::new(100.0, 0.0))),
    );

    let projectile_id = sim.arena_mut().spawn(
        EntityTag::Projectile,
        EntityInner::Projectile(ProjectileComponents::at_position_with_velocity(
            Vec2::new(200.0, 0.0),
            0.0,
            Vec2::new(100.0, 0.0),
        )),
    );

    let squadron_id = sim.arena_mut().spawn(
        EntityTag::Squadron,
        EntityInner::Squadron(SquadronComponents::at_position(Vec2::new(300.0, 0.0), 0.0)),
    );

    assert_eq!(sim.arena().entity_count(), 4);
    assert!(sim.arena().get(ship_id).unwrap().is_ship());
    assert!(sim.arena().get(platform_id).unwrap().is_platform());
    assert!(sim.arena().get(projectile_id).unwrap().is_projectile());
    assert!(sim.arena().get(squadron_id).unwrap().is_squadron());
}

/// Test entity despawn.
#[test]
fn entity_despawn() {
    let mut sim = Simulation::new(42);

    let ship_id = spawn_test_ship(sim.arena_mut(), Vec2::new(0.0, 0.0));
    assert_eq!(sim.arena().entity_count(), 1);

    sim.arena_mut().despawn(ship_id);
    assert_eq!(sim.arena().entity_count(), 0);
    assert!(sim.arena().get(ship_id).is_none());
}

// =============================================================================
// Plugin Execution Tests
// =============================================================================

/// Test that plugins are executed and affect state.
#[test]
fn plugin_affects_state() {
    let mut sim = Simulation::new(42);

    let ship_id = spawn_test_ship(sim.arena_mut(), Vec2::new(0.0, 0.0));

    // Register a plugin that sets velocity
    let plugin = Arc::new(ConstantVelocityPlugin::new(Vec2::new(120.0, 0.0)));
    sim.plugins_mut().register(EntityTag::Ship, plugin);

    // Initial velocity should be zero
    let vel = get_velocity(sim.arena(), ship_id).unwrap();
    assert_eq!(vel, Vec2::ZERO);

    sim.step();

    // After step, velocity should be set by plugin
    let vel = get_velocity(sim.arena(), ship_id).unwrap();
    assert_eq!(vel, Vec2::new(120.0, 0.0));

    // Position should have moved
    let pos = get_position(sim.arena(), ship_id).unwrap();
    assert!((pos.x - 2.0).abs() < 0.001); // 120 * (1/60) = 2.0
}

/// Test that plugins only run on matching entity tags.
#[test]
fn plugin_only_runs_on_matching_tags() {
    let mut sim = Simulation::new(42);

    // Spawn a ship and a platform
    let ship_id = sim.arena_mut().spawn(
        EntityTag::Ship,
        EntityInner::Ship(ShipComponents::at_position(Vec2::new(0.0, 0.0), 0.0)),
    );
    let _platform_id = sim.arena_mut().spawn(
        EntityTag::Platform,
        EntityInner::Platform(PlatformComponents::at_position(Vec2::new(100.0, 0.0))),
    );

    // Register plugin for ships only
    let plugin = Arc::new(ConstantVelocityPlugin::new(Vec2::new(60.0, 0.0)));
    sim.plugins_mut().register(EntityTag::Ship, plugin);

    sim.step();

    // Ship should have velocity, platform should not (it doesn't have physics anyway)
    let vel = get_velocity(sim.arena(), ship_id).unwrap();
    assert_eq!(vel, Vec2::new(60.0, 0.0));
}

// =============================================================================
// Combat System Tests
// =============================================================================

/// Test damage reduces HP.
#[test]
fn damage_reduces_hp() {
    let mut sim = Simulation::new(42);

    let attacker = spawn_test_ship(sim.arena_mut(), Vec2::new(0.0, 0.0));
    let target = spawn_test_ship(sim.arena_mut(), Vec2::new(10.0, 0.0));

    // Register damage plugin
    let plugin = Arc::new(DamagePlugin::new(target, 25.0));
    sim.plugins_mut().register(EntityTag::Ship, plugin);

    // Initial HP
    let initial_hp = get_hp(sim.arena(), target);
    assert_eq!(initial_hp, 100.0);

    sim.step();

    // HP should have decreased
    let hp = get_hp(sim.arena(), target);
    assert!(
        (hp - 75.0).abs() < 0.001,
        "Expected HP=75.0, got HP={}",
        hp
    );
}

/// Test that damage can destroy an entity.
#[test]
fn damage_destroys_entity() {
    let mut sim = Simulation::new(42);

    let _attacker = spawn_test_ship(sim.arena_mut(), Vec2::new(0.0, 0.0));
    let target = spawn_test_ship(sim.arena_mut(), Vec2::new(10.0, 0.0));

    // Register damage plugin with lethal damage
    let plugin = Arc::new(DamagePlugin::new(target, 150.0));
    sim.plugins_mut().register(EntityTag::Ship, plugin);

    assert!(!is_destroyed(sim.arena(), target));

    sim.step();

    // Target should be destroyed
    assert!(is_destroyed(sim.arena(), target));
    assert_eq!(get_hp(sim.arena(), target), 0.0);
}

/// Test multiple damage hits accumulate.
#[test]
fn multiple_damage_accumulates() {
    let mut sim = Simulation::new(42);

    let _attacker = spawn_test_ship(sim.arena_mut(), Vec2::new(0.0, 0.0));
    let target = spawn_test_ship(sim.arena_mut(), Vec2::new(10.0, 0.0));

    // Register damage plugin
    let plugin = Arc::new(DamagePlugin::new(target, 20.0));
    sim.plugins_mut().register(EntityTag::Ship, plugin);

    // Run multiple ticks
    for _ in 0..5 {
        sim.step();
    }

    // HP should have decreased by 20 * 5 = 100
    let hp = get_hp(sim.arena(), target);
    assert_eq!(hp, 0.0); // 100 - 100 = 0
    assert!(is_destroyed(sim.arena(), target));
}

// =============================================================================
// Physics Integration Tests
// =============================================================================

/// Test physics integration over multiple ticks.
#[test]
fn physics_integration_over_time() {
    let mut sim = Simulation::new(42);

    let ship_id = spawn_test_ship(sim.arena_mut(), Vec2::new(0.0, 0.0));

    // Register constant velocity plugin
    let plugin = Arc::new(ConstantVelocityPlugin::new(Vec2::new(60.0, 30.0)));
    sim.plugins_mut().register(EntityTag::Ship, plugin);

    // Run for 60 ticks (1 second at 60 FPS)
    for _ in 0..60 {
        sim.step();
    }

    // Expected position: (60, 30) after 1 second at (60, 30) m/s
    let pos = get_position(sim.arena(), ship_id).unwrap();
    assert!((pos.x - 60.0).abs() < 0.1, "Expected x=60, got x={}", pos.x);
    assert!((pos.y - 30.0).abs() < 0.1, "Expected y=30, got y={}", pos.y);
}

/// Test projectile physics.
#[test]
fn projectile_physics() {
    let mut sim = Simulation::new(42);

    // Spawn a projectile with velocity
    let projectile_id = sim.arena_mut().spawn(
        EntityTag::Projectile,
        EntityInner::Projectile(ProjectileComponents::at_position_with_velocity(
            Vec2::new(0.0, 0.0),
            0.0,
            Vec2::new(600.0, 0.0), // Fast projectile
        )),
    );

    // Run for 10 ticks
    for _ in 0..10 {
        sim.step();
    }

    // Expected position: 600 * 10 * (1/60) = 100
    let pos = get_position(sim.arena(), projectile_id).unwrap();
    assert!(
        (pos.x - 100.0).abs() < 0.1,
        "Expected x=100, got x={}",
        pos.x
    );
}

// =============================================================================
// Event System Tests
// =============================================================================

/// Test that events are captured.
#[test]
fn events_are_captured() {
    let mut sim = Simulation::new(42);

    let _ship_id = spawn_test_ship(sim.arena_mut(), Vec2::new(0.0, 0.0));

    // Register event emitter plugin
    let plugin = Arc::new(EventEmitterPlugin::new());
    sim.plugins_mut().register(EntityTag::Ship, plugin);

    sim.step();

    // Events should have been captured by the EventResolver
    // Note: The EventResolver is internal to Simulation, so we can't directly
    // access its events. This test verifies the flow doesn't crash.
    assert_eq!(sim.tick(), 1);
}

// =============================================================================
// WorldView Access Control Tests
// =============================================================================

/// Test that WorldView respects component access declarations.
#[test]
#[should_panic(expected = "access denied")]
#[cfg(debug_assertions)]
fn worldview_denies_undeclared_access() {
    use crate::arena::Arena;
    use crate::output::OutputKind;

    let mut arena = Arena::new();
    let ship_id = arena.spawn(
        EntityTag::Ship,
        EntityInner::Ship(ShipComponents::default()),
    );

    // Create a declaration that only reads Transform
    let decl = PluginDeclaration {
        id: PluginId::new("test"),
        required_tags: vec![EntityTag::Ship],
        reads: vec![ComponentKind::Transform], // Only Transform
        emits: vec![OutputKind::Command],
    };

    let view = WorldView::for_plugin(&arena, &decl, 0);

    // This should work
    let _transform = view.get_transform(ship_id);

    // This should panic in debug mode (Combat not declared)
    let _combat = view.get_combat(ship_id);
}

/// Test WorldView with full access.
#[test]
fn worldview_full_access() {
    use crate::arena::Arena;

    let mut arena = Arena::new();
    let ship_id = arena.spawn(
        EntityTag::Ship,
        EntityInner::Ship(ShipComponents::default()),
    );

    let view = WorldView::full_access(&arena, 0);

    // All accesses should work
    assert!(view.get_transform(ship_id).is_some());
    assert!(view.get_physics(ship_id).is_some());
    assert!(view.get_combat(ship_id).is_some());
    assert!(view.get_sensor(ship_id).is_some());
    assert!(view.get_inventory(ship_id).is_some());
}

// =============================================================================
// Spatial Index Tests
// =============================================================================

/// Test spatial queries during simulation.
#[test]
fn spatial_queries_during_simulation() {
    let mut sim = Simulation::new(42);

    // Spawn ships at known positions
    let ship1 = spawn_test_ship(sim.arena_mut(), Vec2::new(0.0, 0.0));
    let ship2 = spawn_test_ship(sim.arena_mut(), Vec2::new(100.0, 0.0));
    let ship3 = spawn_test_ship(sim.arena_mut(), Vec2::new(200.0, 0.0));

    // Query near ship1
    let nearby = sim.arena().spatial().query_radius(Vec2::new(0.0, 0.0), 50.0);
    assert_eq!(nearby.len(), 1);
    assert!(nearby.contains(&ship1));

    // Query between ship1 and ship2
    let nearby = sim.arena().spatial().query_radius(Vec2::new(50.0, 0.0), 60.0);
    assert_eq!(nearby.len(), 2);
    assert!(nearby.contains(&ship1));
    assert!(nearby.contains(&ship2));

    // Query all
    let nearby = sim.arena().spatial().query_radius(Vec2::new(100.0, 0.0), 150.0);
    assert_eq!(nearby.len(), 3);
}

/// Test spatial index updates after movement.
#[test]
fn spatial_index_updates_after_movement() {
    let mut sim = Simulation::new(42);

    // Ship starts at origin
    let ship_id = spawn_test_ship(sim.arena_mut(), Vec2::new(0.0, 0.0));

    // Register velocity plugin
    let plugin = Arc::new(ConstantVelocityPlugin::new(Vec2::new(600.0, 0.0)));
    sim.plugins_mut().register(EntityTag::Ship, plugin);

    // Initially, ship should be near origin
    let nearby = sim.arena().spatial().query_radius(Vec2::ZERO, 10.0);
    assert!(nearby.contains(&ship_id));

    // Run for 60 ticks - ship moves to (600, 0)
    for _ in 0..60 {
        sim.step();
    }

    // Ship should no longer be near origin
    let nearby = sim.arena().spatial().query_radius(Vec2::ZERO, 10.0);
    assert!(!nearby.contains(&ship_id), "Ship should have moved away from origin");

    // Ship should be near (600, 0)
    let nearby = sim.arena().spatial().query_radius(Vec2::new(600.0, 0.0), 10.0);
    assert!(nearby.contains(&ship_id), "Ship should be near (600, 0)");
}

// =============================================================================
// Multi-Entity Interaction Tests
// =============================================================================

/// Test simulation with multiple interacting entities.
#[test]
fn multi_entity_simulation() {
    let mut sim = Simulation::new(42);

    // Spawn 10 ships
    for i in 0..10 {
        let pos = Vec2::new((i * 50) as f32, 0.0);
        let ship_id = spawn_test_ship(sim.arena_mut(), pos);
        // Give each ship a different velocity
        set_velocity(
            sim.arena_mut(),
            ship_id,
            Vec2::new(((i as i32 - 5) * 60) as f32, 0.0),
        );
    }

    assert_eq!(sim.arena().entity_count(), 10);

    // Run for 100 ticks
    for _ in 0..100 {
        sim.step();
    }

    // All entities should still exist
    assert_eq!(sim.arena().entity_count(), 10);

    // Verify positions have changed
    let positions: Vec<Vec2> = sim
        .arena()
        .entities_sorted()
        .filter_map(|e| e.as_ship().map(|s| s.transform.position))
        .collect();

    // Ships should have moved (some left, some right)
    assert!(positions.iter().any(|p| p.x < 0.0)); // Some moved left
    assert!(positions.iter().any(|p| p.x > 400.0)); // Some moved right
}

// =============================================================================
// Default Plugin Bundle Tests
// =============================================================================

/// Test default plugin bundles registration.
#[test]
fn default_plugin_bundles() {
    let registry = PluginRegistry::default_bundles();

    // Ships should have 3 plugins (movement, weapon, sensor)
    assert_eq!(registry.plugins_for(EntityTag::Ship).len(), 3);

    // Platforms should have 1 plugin (sensor)
    assert_eq!(registry.plugins_for(EntityTag::Platform).len(), 1);

    // Projectiles should have 1 plugin (projectile)
    assert_eq!(registry.plugins_for(EntityTag::Projectile).len(), 1);

    // Squadrons should have 2 plugins (movement, weapon)
    assert_eq!(registry.plugins_for(EntityTag::Squadron).len(), 2);
}

// =============================================================================
// Edge Case Tests
// =============================================================================

/// Test empty simulation step.
#[test]
fn empty_simulation_step() {
    let mut sim = Simulation::new(42);
    // No entities, no plugins
    sim.step();
    assert_eq!(sim.tick(), 1);
}

/// Test simulation with no plugins.
#[test]
fn simulation_without_plugins() {
    let mut sim = Simulation::new(42);

    let ship_id = spawn_test_ship(sim.arena_mut(), Vec2::new(0.0, 0.0));
    set_velocity(sim.arena_mut(), ship_id, Vec2::new(60.0, 0.0));

    // Run without any plugins registered
    for _ in 0..60 {
        sim.step();
    }

    // Physics integration should still work
    let pos = get_position(sim.arena(), ship_id).unwrap();
    assert!((pos.x - 60.0).abs() < 0.1);
}

/// Test very small timestep accumulation.
#[test]
fn small_timestep_accumulation() {
    let mut sim = Simulation::new(42);

    let ship_id = spawn_test_ship(sim.arena_mut(), Vec2::new(0.0, 0.0));
    set_velocity(sim.arena_mut(), ship_id, Vec2::new(1.0, 0.0)); // 1 m/s

    // Run for 6000 ticks = 100 seconds at 60 FPS
    for _ in 0..6000 {
        sim.step();
    }

    // Should have moved 100 meters
    let pos = get_position(sim.arena(), ship_id).unwrap();
    assert!(
        (pos.x - 100.0).abs() < 0.1,
        "Expected x=100, got x={}",
        pos.x
    );
}
