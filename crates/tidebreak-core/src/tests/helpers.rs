//! Test helper functions for setting up simulations and entities.
//!
//! This module provides factory functions and setup utilities that make
//! writing tests more ergonomic and consistent.

use glam::Vec2;

use crate::arena::Arena;
use crate::entity::{
    AmmoType, CombatState, EntityId, EntityInner, EntityTag, ShipComponents, Track, TrackQuality,
    WeaponState,
};
use crate::simulation::Simulation;

// =============================================================================
// Test Scenario Setup
// =============================================================================

/// Sets up a standard test scenario with 3 ships in a triangle formation.
///
/// Ships are placed at:
/// - Ship 0: (0, 0)
/// - Ship 1: (100, 0)
/// - Ship 2: (50, 86.6) - approximately equilateral triangle
///
/// # Arguments
///
/// * `sim` - The simulation to set up
///
/// # Returns
///
/// A vector of the spawned ship entity IDs.
pub fn setup_test_scenario(sim: &mut Simulation) -> Vec<EntityId> {
    let mut ids = Vec::new();
    ids.push(spawn_test_ship(sim.arena_mut(), Vec2::new(0.0, 0.0)));
    ids.push(spawn_test_ship(sim.arena_mut(), Vec2::new(100.0, 0.0)));
    ids.push(spawn_test_ship(
        sim.arena_mut(),
        Vec2::new(50.0, 86.6), // Equilateral triangle height
    ));
    ids
}

/// Sets up a combat scenario with an attacker and target.
///
/// The attacker is at the origin with a weapon ready.
/// The target is at (10, 0) within typical weapon range.
///
/// # Arguments
///
/// * `sim` - The simulation to set up
///
/// # Returns
///
/// A tuple of (attacker_id, target_id).
pub fn setup_combat_scenario(sim: &mut Simulation) -> (EntityId, EntityId) {
    let attacker = spawn_armed_ship(sim.arena_mut(), Vec2::new(0.0, 0.0));
    let target = spawn_test_ship(sim.arena_mut(), Vec2::new(10.0, 0.0));

    // Add target to attacker's track table
    add_track(sim.arena_mut(), attacker, target, Vec2::new(10.0, 0.0));

    (attacker, target)
}

/// Sets up a sensor detection scenario.
///
/// Creates an observer ship and a target ship within sensor range.
///
/// # Arguments
///
/// * `sim` - The simulation to set up
///
/// # Returns
///
/// A tuple of (observer_id, target_id).
pub fn setup_sensor_scenario(sim: &mut Simulation) -> (EntityId, EntityId) {
    let observer = spawn_test_ship(sim.arena_mut(), Vec2::new(0.0, 0.0));
    let target = spawn_test_ship(sim.arena_mut(), Vec2::new(20.0, 0.0)); // Within default radar range
    (observer, target)
}

// =============================================================================
// Entity Factory Functions
// =============================================================================

/// Spawns a test ship at the given position.
///
/// Creates a ship with default components and 100 HP.
///
/// # Arguments
///
/// * `arena` - The arena to spawn in
/// * `position` - World position for the ship
///
/// # Returns
///
/// The entity ID of the spawned ship.
pub fn spawn_test_ship(arena: &mut Arena, position: Vec2) -> EntityId {
    let inner = EntityInner::Ship(ShipComponents::at_position(position, 0.0));
    arena.spawn(EntityTag::Ship, inner)
}

/// Spawns a ship with a weapon at the given position.
///
/// Creates a ship with one weapon in slot 0, ready to fire.
///
/// # Arguments
///
/// * `arena` - The arena to spawn in
/// * `position` - World position for the ship
///
/// # Returns
///
/// The entity ID of the spawned ship.
pub fn spawn_armed_ship(arena: &mut Arena, position: Vec2) -> EntityId {
    let weapons = vec![WeaponState::new(0, 1.0, AmmoType::Bullet)];
    let inner = EntityInner::Ship(ShipComponents {
        transform: crate::entity::TransformState::new(position, 0.0),
        physics: crate::entity::PhysicsState::default(),
        combat: CombatState::with_weapons(100.0, weapons),
        sensor: crate::entity::SensorState::default(),
        inventory: crate::entity::InventoryState::default(),
    });
    arena.spawn(EntityTag::Ship, inner)
}

/// Spawns a ship with custom HP at the given position.
///
/// # Arguments
///
/// * `arena` - The arena to spawn in
/// * `position` - World position for the ship
/// * `hp` - Current HP for the ship
/// * `max_hp` - Maximum HP for the ship
///
/// # Returns
///
/// The entity ID of the spawned ship.
pub fn spawn_ship_with_hp(arena: &mut Arena, position: Vec2, hp: f32, max_hp: f32) -> EntityId {
    let inner = EntityInner::Ship(ShipComponents {
        transform: crate::entity::TransformState::new(position, 0.0),
        physics: crate::entity::PhysicsState::default(),
        combat: CombatState {
            hp,
            max_hp,
            weapons: Vec::new(),
            status_flags: crate::entity::StatusFlags::empty(),
        },
        sensor: crate::entity::SensorState::default(),
        inventory: crate::entity::InventoryState::default(),
    });
    arena.spawn(EntityTag::Ship, inner)
}

// =============================================================================
// State Manipulation Functions
// =============================================================================

/// Sets the velocity of an entity.
///
/// Works for ships, projectiles, and squadrons. Does nothing for platforms.
///
/// # Arguments
///
/// * `arena` - The arena containing the entity
/// * `id` - The entity to modify
/// * `velocity` - The new velocity vector
pub fn set_velocity(arena: &mut Arena, id: EntityId, velocity: Vec2) {
    if let Some(entity) = arena.get_mut(id) {
        if let Some(ship) = entity.as_ship_mut() {
            ship.physics.velocity = velocity;
        } else if let Some(projectile) = entity.as_projectile_mut() {
            projectile.physics.velocity = velocity;
        } else if let Some(squadron) = entity.as_squadron_mut() {
            squadron.physics.velocity = velocity;
        }
    }
}

/// Sets the heading of an entity.
///
/// # Arguments
///
/// * `arena` - The arena containing the entity
/// * `id` - The entity to modify
/// * `heading` - The new heading in radians
pub fn set_heading(arena: &mut Arena, id: EntityId, heading: f32) {
    if let Some(entity) = arena.get_mut(id) {
        if let Some(ship) = entity.as_ship_mut() {
            ship.transform.heading = heading;
        } else if let Some(platform) = entity.as_platform_mut() {
            platform.transform.heading = heading;
        } else if let Some(projectile) = entity.as_projectile_mut() {
            projectile.transform.heading = heading;
        } else if let Some(squadron) = entity.as_squadron_mut() {
            squadron.transform.heading = heading;
        }
    }
}

/// Adds a track entry to an entity's sensor track table.
///
/// Only works for ships and platforms (entities with sensors).
///
/// # Arguments
///
/// * `arena` - The arena containing the entity
/// * `observer` - The entity adding the track
/// * `target` - The target entity being tracked
/// * `target_position` - The observed position of the target
pub fn add_track(arena: &mut Arena, observer: EntityId, target: EntityId, target_position: Vec2) {
    if let Some(entity) = arena.get_mut(observer) {
        if let Some(ship) = entity.as_ship_mut() {
            ship.sensor.track_table.push(Track::new(
                target,
                target_position,
                TrackQuality::FireControl,
            ));
        } else if let Some(platform) = entity.as_platform_mut() {
            platform.sensor.track_table.push(Track::new(
                target,
                target_position,
                TrackQuality::FireControl,
            ));
        }
    }
}

/// Makes a weapon in the specified slot ready to fire.
///
/// Sets the weapon's cooldown to 0 so it can fire immediately.
///
/// # Arguments
///
/// * `arena` - The arena containing the entity
/// * `id` - The entity owning the weapon
/// * `slot` - The weapon slot index
pub fn make_weapon_ready(arena: &mut Arena, id: EntityId, slot: usize) {
    if let Some(entity) = arena.get_mut(id) {
        if let Some(ship) = entity.as_ship_mut() {
            if let Some(weapon) = ship.combat.get_weapon_mut(slot) {
                weapon.cooldown = 0.0;
            }
        } else if let Some(squadron) = entity.as_squadron_mut() {
            if let Some(weapon) = squadron.combat.get_weapon_mut(slot) {
                weapon.cooldown = 0.0;
            }
        }
    }
}

/// Sets the HP of an entity.
///
/// Only works for ships and squadrons (entities with combat).
///
/// # Arguments
///
/// * `arena` - The arena containing the entity
/// * `id` - The entity to modify
/// * `hp` - The new HP value
pub fn set_hp(arena: &mut Arena, id: EntityId, hp: f32) {
    if let Some(entity) = arena.get_mut(id) {
        if let Some(ship) = entity.as_ship_mut() {
            ship.combat.hp = hp;
        } else if let Some(squadron) = entity.as_squadron_mut() {
            squadron.combat.hp = hp;
        }
    }
}

// =============================================================================
// State Query Functions
// =============================================================================

/// Gets the HP of an entity.
///
/// Returns 0.0 if the entity doesn't exist or doesn't have combat state.
///
/// # Arguments
///
/// * `arena` - The arena containing the entity
/// * `id` - The entity to query
///
/// # Returns
///
/// The current HP of the entity.
pub fn get_hp(arena: &Arena, id: EntityId) -> f32 {
    arena
        .get(id)
        .and_then(|e| e.as_ship().map(|s| s.combat.hp))
        .or_else(|| {
            arena
                .get(id)
                .and_then(|e| e.as_squadron().map(|s| s.combat.hp))
        })
        .unwrap_or(0.0)
}

/// Gets the position of an entity.
///
/// Returns `None` if the entity doesn't exist.
///
/// # Arguments
///
/// * `arena` - The arena containing the entity
/// * `id` - The entity to query
///
/// # Returns
///
/// The position of the entity, if it exists.
pub fn get_position(arena: &Arena, id: EntityId) -> Option<Vec2> {
    arena.get(id).map(|e| {
        if let Some(ship) = e.as_ship() {
            ship.transform.position
        } else if let Some(platform) = e.as_platform() {
            platform.transform.position
        } else if let Some(projectile) = e.as_projectile() {
            projectile.transform.position
        } else if let Some(squadron) = e.as_squadron() {
            squadron.transform.position
        } else {
            Vec2::ZERO
        }
    })
}

/// Gets the velocity of an entity.
///
/// Returns `None` if the entity doesn't exist or doesn't have physics.
///
/// # Arguments
///
/// * `arena` - The arena containing the entity
/// * `id` - The entity to query
///
/// # Returns
///
/// The velocity of the entity, if applicable.
pub fn get_velocity(arena: &Arena, id: EntityId) -> Option<Vec2> {
    arena.get(id).and_then(|e| {
        if let Some(ship) = e.as_ship() {
            Some(ship.physics.velocity)
        } else if let Some(projectile) = e.as_projectile() {
            Some(projectile.physics.velocity)
        } else if let Some(squadron) = e.as_squadron() {
            Some(squadron.physics.velocity)
        } else {
            None // Platforms don't have velocity
        }
    })
}

/// Gets the heading of an entity.
///
/// Returns `None` if the entity doesn't exist.
///
/// # Arguments
///
/// * `arena` - The arena containing the entity
/// * `id` - The entity to query
///
/// # Returns
///
/// The heading of the entity in radians.
pub fn get_heading(arena: &Arena, id: EntityId) -> Option<f32> {
    arena.get(id).map(|e| {
        if let Some(ship) = e.as_ship() {
            ship.transform.heading
        } else if let Some(platform) = e.as_platform() {
            platform.transform.heading
        } else if let Some(projectile) = e.as_projectile() {
            projectile.transform.heading
        } else if let Some(squadron) = e.as_squadron() {
            squadron.transform.heading
        } else {
            0.0
        }
    })
}

/// Checks if an entity is destroyed.
///
/// Returns `false` if the entity doesn't exist or doesn't have combat state.
///
/// # Arguments
///
/// * `arena` - The arena containing the entity
/// * `id` - The entity to query
///
/// # Returns
///
/// `true` if the entity has the DESTROYED flag or HP <= 0.
pub fn is_destroyed(arena: &Arena, id: EntityId) -> bool {
    arena
        .get(id)
        .and_then(|e| e.as_ship().map(|s| s.combat.is_destroyed()))
        .or_else(|| {
            arena
                .get(id)
                .and_then(|e| e.as_squadron().map(|s| s.combat.is_destroyed()))
        })
        .unwrap_or(false)
}

/// Gets the track table for an entity.
///
/// Returns an empty slice if the entity doesn't exist or doesn't have sensors.
///
/// # Arguments
///
/// * `arena` - The arena containing the entity
/// * `id` - The entity to query
///
/// # Returns
///
/// A reference to the track table.
pub fn get_track_count(arena: &Arena, id: EntityId) -> usize {
    arena
        .get(id)
        .and_then(|e| e.as_ship().map(|s| s.sensor.track_table.len()))
        .or_else(|| {
            arena
                .get(id)
                .and_then(|e| e.as_platform().map(|p| p.sensor.track_table.len()))
        })
        .unwrap_or(0)
}

// =============================================================================
// Tests for helpers
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_test_ship_at_position() {
        let mut arena = Arena::new();
        let id = spawn_test_ship(&mut arena, Vec2::new(100.0, 200.0));

        let pos = get_position(&arena, id).unwrap();
        assert_eq!(pos, Vec2::new(100.0, 200.0));
    }

    #[test]
    fn spawn_armed_ship_has_weapon() {
        let mut arena = Arena::new();
        let id = spawn_armed_ship(&mut arena, Vec2::new(0.0, 0.0));

        let ship = arena.get(id).unwrap().as_ship().unwrap();
        assert_eq!(ship.combat.weapons.len(), 1);
        assert!(ship.combat.weapons[0].is_ready());
    }

    #[test]
    fn set_and_get_velocity() {
        let mut arena = Arena::new();
        let id = spawn_test_ship(&mut arena, Vec2::ZERO);

        set_velocity(&mut arena, id, Vec2::new(10.0, 20.0));

        let vel = get_velocity(&arena, id).unwrap();
        assert_eq!(vel, Vec2::new(10.0, 20.0));
    }

    #[test]
    fn set_and_get_hp() {
        let mut arena = Arena::new();
        let id = spawn_test_ship(&mut arena, Vec2::ZERO);

        set_hp(&mut arena, id, 50.0);

        let hp = get_hp(&arena, id);
        assert!((hp - 50.0).abs() < 0.001);
    }

    #[test]
    fn add_track_to_ship() {
        let mut arena = Arena::new();
        let observer = spawn_test_ship(&mut arena, Vec2::ZERO);
        let target = spawn_test_ship(&mut arena, Vec2::new(100.0, 0.0));

        add_track(&mut arena, observer, target, Vec2::new(100.0, 0.0));

        let count = get_track_count(&arena, observer);
        assert_eq!(count, 1);
    }

    #[test]
    fn setup_test_scenario_spawns_three_ships() {
        let mut sim = Simulation::new(42);
        let ids = setup_test_scenario(&mut sim);

        assert_eq!(ids.len(), 3);
        assert_eq!(sim.arena().entity_count(), 3);
    }

    #[test]
    fn setup_combat_scenario_creates_armed_attacker() {
        let mut sim = Simulation::new(42);
        let (attacker, target) = setup_combat_scenario(&mut sim);

        // Attacker should have a weapon
        let ship = sim.arena().get(attacker).unwrap().as_ship().unwrap();
        assert!(!ship.combat.weapons.is_empty());

        // Attacker should have target in track table
        let track_count = get_track_count(sim.arena(), attacker);
        assert_eq!(track_count, 1);

        // Both entities should exist
        assert!(sim.arena().get(attacker).is_some());
        assert!(sim.arena().get(target).is_some());
    }
}
