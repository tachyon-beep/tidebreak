//! Physics resolver for movement commands and physics integration.
//!
//! The `PhysicsResolver` handles:
//! - `SetVelocity` commands: Update entity velocity
//! - `SetHeading` commands: Update entity heading
//! - Physics integration: Apply `position += velocity * dt` each tick
//!
//! # Fixed Timestep
//!
//! The physics resolver uses a fixed timestep of 1/60 seconds (60 FPS).
//! This ensures deterministic physics regardless of actual frame time.

use glam::Vec2;

use crate::arena::Arena;
use crate::entity::EntityId;
use crate::output::{Command, OutputEnvelope, OutputKind};

use super::Resolver;

/// Fixed timestep for physics integration (1/60 second = ~16.67ms).
pub const FIXED_DT: f32 = 1.0 / 60.0;

/// Resolver for physics-related commands and integration.
///
/// Handles movement commands (`SetVelocity`, `SetHeading`) and performs
/// physics integration each tick.
///
/// # Processing Order
///
/// 1. Apply all velocity changes from `SetVelocity` commands
/// 2. Apply all heading changes from `SetHeading` commands
/// 3. Integrate physics: `position += velocity * dt` for all entities
///
/// # Example
///
/// ```
/// use tidebreak_core::resolver::PhysicsResolver;
/// use tidebreak_core::resolver::Resolver;
/// use tidebreak_core::output::OutputKind;
///
/// let resolver = PhysicsResolver::new();
/// assert!(resolver.handles().contains(&OutputKind::Command));
/// ```
#[derive(Debug, Clone, Default)]
pub struct PhysicsResolver {
    /// Fixed timestep for physics integration
    dt: f32,
}

impl PhysicsResolver {
    /// Creates a new physics resolver with the default fixed timestep.
    #[must_use]
    pub fn new() -> Self {
        Self { dt: FIXED_DT }
    }

    /// Creates a physics resolver with a custom timestep.
    ///
    /// Useful for testing or non-standard tick rates.
    #[must_use]
    pub fn with_dt(dt: f32) -> Self {
        Self { dt }
    }

    /// Returns the timestep used for physics integration.
    #[must_use]
    pub fn dt(&self) -> f32 {
        self.dt
    }

    /// Applies a velocity change to an entity.
    fn apply_set_velocity(next: &mut Arena, target: EntityId, velocity: Vec2) {
        if let Some(entity) = next.get_mut(target) {
            // Try each entity type that has physics
            if let Some(ship) = entity.as_ship_mut() {
                ship.physics.velocity = velocity;
            } else if let Some(projectile) = entity.as_projectile_mut() {
                projectile.physics.velocity = velocity;
            } else if let Some(squadron) = entity.as_squadron_mut() {
                squadron.physics.velocity = velocity;
            }
            // Platforms don't have physics - ignore
        }
    }

    /// Applies a heading change to an entity.
    fn apply_set_heading(next: &mut Arena, target: EntityId, heading: f32) {
        if let Some(entity) = next.get_mut(target) {
            // Try each entity type that has transform
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

    /// Integrates physics for all entities: position += velocity * dt.
    ///
    /// After updating positions, syncs the spatial index for all entities
    /// that moved (those with non-zero velocity).
    fn integrate_physics(&self, next: &mut Arena) {
        let dt = self.dt;

        // First pass: collect IDs of entities that will move (non-zero velocity)
        let moved_entities: Vec<EntityId> = next
            .entities_sorted()
            .filter_map(|entity| {
                let has_velocity = if let Some(ship) = entity.as_ship() {
                    ship.physics.velocity != Vec2::ZERO
                } else if let Some(projectile) = entity.as_projectile() {
                    projectile.physics.velocity != Vec2::ZERO
                } else if let Some(squadron) = entity.as_squadron() {
                    squadron.physics.velocity != Vec2::ZERO
                } else {
                    false // Platforms don't have physics
                };
                if has_velocity {
                    Some(entity.id())
                } else {
                    None
                }
            })
            .collect();

        // Second pass: apply physics integration
        for entity in next.entities_sorted_mut() {
            // Try each entity type that has physics
            if let Some(ship) = entity.as_ship_mut() {
                ship.transform.position += ship.physics.velocity * dt;
            } else if let Some(projectile) = entity.as_projectile_mut() {
                projectile.transform.position += projectile.physics.velocity * dt;
            } else if let Some(squadron) = entity.as_squadron_mut() {
                squadron.transform.position += squadron.physics.velocity * dt;
            }
            // Platforms don't have physics - no integration
        }

        // Third pass: update spatial index for entities that moved
        for entity_id in moved_entities {
            next.update_spatial(entity_id);
        }
    }
}

impl Resolver for PhysicsResolver {
    fn handles(&self) -> &[OutputKind] {
        &[OutputKind::Command]
    }

    fn resolve(&self, outputs: &[&OutputEnvelope], _current: &Arena, next: &mut Arena) {
        // Process commands in order (deterministic)
        for envelope in outputs {
            if let Some(command) = envelope.output().as_command() {
                match command {
                    Command::SetVelocity { target, velocity } => {
                        Self::apply_set_velocity(next, *target, *velocity);
                    }
                    Command::SetHeading { target, heading } => {
                        Self::apply_set_heading(next, *target, *heading);
                    }
                    // Other commands are not handled by physics resolver
                    Command::FireWeapon { .. } | Command::SpawnProjectile { .. } => {}
                }
            }
        }

        // Integrate physics after all commands are processed
        self.integrate_physics(next);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EntityInner, EntityTag, ShipComponents};
    use crate::output::{Output, PluginId, PluginInstanceId, TraceId};

    fn make_envelope(output: Output, target: EntityId) -> OutputEnvelope {
        OutputEnvelope::new(
            output,
            PluginInstanceId::new(target, PluginId::new("test")),
            TraceId::new(0),
            0,
            0,
        )
    }

    mod resolver_trait_tests {
        use super::*;

        #[test]
        fn handles_command_kind() {
            let resolver = PhysicsResolver::new();
            assert!(resolver.handles().contains(&OutputKind::Command));
            assert!(!resolver.handles().contains(&OutputKind::Modifier));
            assert!(!resolver.handles().contains(&OutputKind::Event));
        }

        #[test]
        fn default_dt() {
            let resolver = PhysicsResolver::new();
            assert!((resolver.dt() - FIXED_DT).abs() < 0.0001);
        }

        #[test]
        fn custom_dt() {
            let resolver = PhysicsResolver::with_dt(0.1);
            assert!((resolver.dt() - 0.1).abs() < 0.0001);
        }
    }

    mod set_velocity_tests {
        use super::*;

        #[test]
        fn set_velocity_updates_ship() {
            let mut arena = Arena::new();
            let ship_id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            let envelope = make_envelope(
                Output::Command(Command::SetVelocity {
                    target: ship_id,
                    velocity: Vec2::new(10.0, 5.0),
                }),
                ship_id,
            );

            let resolver = PhysicsResolver::with_dt(0.0); // No integration
            let current = arena.clone();
            resolver.resolve(&[&envelope], &current, &mut arena);

            let ship = arena.get(ship_id).unwrap().as_ship().unwrap();
            assert_eq!(ship.physics.velocity, Vec2::new(10.0, 5.0));
        }

        #[test]
        fn set_velocity_nonexistent_entity_ignored() {
            let mut arena = Arena::new();
            let fake_id = EntityId::new(999);

            let envelope = make_envelope(
                Output::Command(Command::SetVelocity {
                    target: fake_id,
                    velocity: Vec2::new(10.0, 5.0),
                }),
                fake_id,
            );

            let resolver = PhysicsResolver::with_dt(0.0);
            let current = arena.clone();
            // Should not panic
            resolver.resolve(&[&envelope], &current, &mut arena);
        }

        #[test]
        fn set_velocity_multiple_commands() {
            let mut arena = Arena::new();
            let ship1 = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );
            let ship2 = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            let envelope1 = make_envelope(
                Output::Command(Command::SetVelocity {
                    target: ship1,
                    velocity: Vec2::new(10.0, 0.0),
                }),
                ship1,
            );
            let envelope2 = make_envelope(
                Output::Command(Command::SetVelocity {
                    target: ship2,
                    velocity: Vec2::new(0.0, 20.0),
                }),
                ship2,
            );

            let resolver = PhysicsResolver::with_dt(0.0);
            let current = arena.clone();
            resolver.resolve(&[&envelope1, &envelope2], &current, &mut arena);

            assert_eq!(
                arena.get(ship1).unwrap().as_ship().unwrap().physics.velocity,
                Vec2::new(10.0, 0.0)
            );
            assert_eq!(
                arena.get(ship2).unwrap().as_ship().unwrap().physics.velocity,
                Vec2::new(0.0, 20.0)
            );
        }

        #[test]
        fn set_velocity_last_write_wins() {
            let mut arena = Arena::new();
            let ship_id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            let envelope1 = make_envelope(
                Output::Command(Command::SetVelocity {
                    target: ship_id,
                    velocity: Vec2::new(10.0, 0.0),
                }),
                ship_id,
            );
            let envelope2 = make_envelope(
                Output::Command(Command::SetVelocity {
                    target: ship_id,
                    velocity: Vec2::new(0.0, 20.0),
                }),
                ship_id,
            );

            let resolver = PhysicsResolver::with_dt(0.0);
            let current = arena.clone();
            resolver.resolve(&[&envelope1, &envelope2], &current, &mut arena);

            // Last write wins
            let ship = arena.get(ship_id).unwrap().as_ship().unwrap();
            assert_eq!(ship.physics.velocity, Vec2::new(0.0, 20.0));
        }
    }

    mod set_heading_tests {
        use super::*;

        #[test]
        fn set_heading_updates_ship() {
            let mut arena = Arena::new();
            let ship_id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            let envelope = make_envelope(
                Output::Command(Command::SetHeading {
                    target: ship_id,
                    heading: 1.5,
                }),
                ship_id,
            );

            let resolver = PhysicsResolver::with_dt(0.0);
            let current = arena.clone();
            resolver.resolve(&[&envelope], &current, &mut arena);

            let ship = arena.get(ship_id).unwrap().as_ship().unwrap();
            assert!((ship.transform.heading - 1.5).abs() < 0.0001);
        }

        #[test]
        fn set_heading_nonexistent_entity_ignored() {
            let mut arena = Arena::new();
            let fake_id = EntityId::new(999);

            let envelope = make_envelope(
                Output::Command(Command::SetHeading {
                    target: fake_id,
                    heading: 1.5,
                }),
                fake_id,
            );

            let resolver = PhysicsResolver::with_dt(0.0);
            let current = arena.clone();
            // Should not panic
            resolver.resolve(&[&envelope], &current, &mut arena);
        }
    }

    mod physics_integration_tests {
        use super::*;

        #[test]
        fn integration_updates_position() {
            let mut arena = Arena::new();
            let ship_id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            // Set initial velocity
            if let Some(ship) = arena.get_mut(ship_id).unwrap().as_ship_mut() {
                ship.physics.velocity = Vec2::new(60.0, 30.0);
            }

            let resolver = PhysicsResolver::with_dt(1.0); // 1 second for easy math
            let current = arena.clone();
            resolver.resolve(&[], &current, &mut arena);

            let ship = arena.get(ship_id).unwrap().as_ship().unwrap();
            // position += velocity * dt = (0,0) + (60,30) * 1 = (60, 30)
            assert!((ship.transform.position.x - 60.0).abs() < 0.0001);
            assert!((ship.transform.position.y - 30.0).abs() < 0.0001);
        }

        #[test]
        fn integration_with_default_dt() {
            let mut arena = Arena::new();
            let ship_id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            // Set velocity
            if let Some(ship) = arena.get_mut(ship_id).unwrap().as_ship_mut() {
                ship.physics.velocity = Vec2::new(600.0, 0.0); // 600 m/s
            }

            let resolver = PhysicsResolver::new();
            let current = arena.clone();
            resolver.resolve(&[], &current, &mut arena);

            let ship = arena.get(ship_id).unwrap().as_ship().unwrap();
            // position += velocity * dt = (0,0) + (600,0) * (1/60) = (10, 0)
            assert!((ship.transform.position.x - 10.0).abs() < 0.0001);
        }

        #[test]
        fn integration_multiple_entities() {
            let mut arena = Arena::new();
            let ship1 = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );
            let ship2 = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            if let Some(ship) = arena.get_mut(ship1).unwrap().as_ship_mut() {
                ship.physics.velocity = Vec2::new(10.0, 0.0);
            }
            if let Some(ship) = arena.get_mut(ship2).unwrap().as_ship_mut() {
                ship.physics.velocity = Vec2::new(0.0, 20.0);
            }

            let resolver = PhysicsResolver::with_dt(1.0);
            let current = arena.clone();
            resolver.resolve(&[], &current, &mut arena);

            let s1 = arena.get(ship1).unwrap().as_ship().unwrap();
            let s2 = arena.get(ship2).unwrap().as_ship().unwrap();
            assert!((s1.transform.position.x - 10.0).abs() < 0.0001);
            assert!((s2.transform.position.y - 20.0).abs() < 0.0001);
        }

        #[test]
        fn velocity_command_then_integration() {
            let mut arena = Arena::new();
            let ship_id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            let envelope = make_envelope(
                Output::Command(Command::SetVelocity {
                    target: ship_id,
                    velocity: Vec2::new(100.0, 50.0),
                }),
                ship_id,
            );

            let resolver = PhysicsResolver::with_dt(1.0);
            let current = arena.clone();
            resolver.resolve(&[&envelope], &current, &mut arena);

            let ship = arena.get(ship_id).unwrap().as_ship().unwrap();
            // Velocity was set, then integration applied
            assert_eq!(ship.physics.velocity, Vec2::new(100.0, 50.0));
            assert!((ship.transform.position.x - 100.0).abs() < 0.0001);
            assert!((ship.transform.position.y - 50.0).abs() < 0.0001);
        }

        #[test]
        fn integration_updates_spatial_index() {
            let mut arena = Arena::new();

            // Spawn ship at origin
            let ship_id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            // Set velocity on the ship
            if let Some(ship) = arena.get_mut(ship_id).unwrap().as_ship_mut() {
                ship.physics.velocity = Vec2::new(100.0, 0.0);
            }

            // Verify initial spatial position is at origin
            let initial_pos = arena.spatial().get(ship_id).unwrap();
            assert_eq!(initial_pos, Vec2::ZERO);

            let resolver = PhysicsResolver::with_dt(1.0);
            let current = arena.clone();
            resolver.resolve(&[], &current, &mut arena);

            // After integration, position should be (100, 0)
            let ship = arena.get(ship_id).unwrap().as_ship().unwrap();
            assert!((ship.transform.position.x - 100.0).abs() < 0.0001);

            // Verify spatial index was also updated
            let spatial_pos = arena.spatial().get(ship_id).unwrap();
            assert!(
                (spatial_pos.x - 100.0).abs() < 0.0001,
                "Spatial index was not updated after physics integration. Expected x=100, got x={}",
                spatial_pos.x
            );
        }

        #[test]
        fn spatial_queries_work_after_physics() {
            let mut arena = Arena::new();

            // Spawn two ships at different positions
            let ship1 = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::at_position(Vec2::new(0.0, 0.0), 0.0)),
            );
            let ship2 = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::at_position(Vec2::new(500.0, 0.0), 0.0)),
            );

            // Give ship1 velocity to move toward ship2
            if let Some(ship) = arena.get_mut(ship1).unwrap().as_ship_mut() {
                ship.physics.velocity = Vec2::new(400.0, 0.0);
            }

            // Initial query: only ship1 should be within 100 units of origin
            let near_origin = arena.spatial().query_radius(Vec2::ZERO, 100.0);
            assert_eq!(near_origin, vec![ship1]);

            // Run physics with dt=1.0 - ship1 moves to (400, 0)
            let resolver = PhysicsResolver::with_dt(1.0);
            let current = arena.clone();
            resolver.resolve(&[], &current, &mut arena);

            // Now ship1 should be closer to ship2
            // Query near ship2 (500, 0) with radius 150 should find both ships
            let near_ship2 = arena.spatial().query_radius(Vec2::new(500.0, 0.0), 150.0);
            assert!(
                near_ship2.contains(&ship1) && near_ship2.contains(&ship2),
                "After physics, spatial query should find both ships near (500,0). Found: {:?}",
                near_ship2
            );
        }
    }

    mod output_filtering_tests {
        use super::*;
        use crate::entity::components::StatusFlags;
        use crate::output::{Event, Modifier};

        #[test]
        fn ignores_modifier_outputs() {
            let mut arena = Arena::new();
            let ship_id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            let envelope = make_envelope(
                Output::Modifier(Modifier::ApplyDamage {
                    target: ship_id,
                    amount: 50.0,
                }),
                ship_id,
            );

            let resolver = PhysicsResolver::with_dt(0.0);
            let current = arena.clone();
            resolver.resolve(&[&envelope], &current, &mut arena);

            // HP should be unchanged (physics resolver ignores modifiers)
            let ship = arena.get(ship_id).unwrap().as_ship().unwrap();
            assert_eq!(ship.combat.hp, 100.0);
        }

        #[test]
        fn ignores_event_outputs() {
            let mut arena = Arena::new();
            let ship_id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            let envelope = make_envelope(
                Output::Event(Event::WeaponFired {
                    source: ship_id,
                    weapon_slot: 0,
                }),
                ship_id,
            );

            let resolver = PhysicsResolver::with_dt(0.0);
            let current = arena.clone();
            // Should not panic and should not change state
            resolver.resolve(&[&envelope], &current, &mut arena);
        }

        #[test]
        fn ignores_fire_weapon_command() {
            let mut arena = Arena::new();
            let ship1 = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );
            let ship2 = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            let envelope = make_envelope(
                Output::Command(Command::FireWeapon {
                    source: ship1,
                    target: ship2,
                    slot: 0,
                }),
                ship1,
            );

            let resolver = PhysicsResolver::with_dt(0.0);
            let current = arena.clone();
            // Should not panic - fire weapon is not handled by physics
            resolver.resolve(&[&envelope], &current, &mut arena);
        }

        #[test]
        fn ignores_set_status_flag_modifier() {
            let mut arena = Arena::new();
            let ship_id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            let envelope = make_envelope(
                Output::Modifier(Modifier::SetStatusFlag {
                    target: ship_id,
                    flag: StatusFlags::MOBILITY_DISABLED,
                    value: true,
                }),
                ship_id,
            );

            let resolver = PhysicsResolver::with_dt(0.0);
            let current = arena.clone();
            resolver.resolve(&[&envelope], &current, &mut arena);

            // Status flag should be unchanged
            let ship = arena.get(ship_id).unwrap().as_ship().unwrap();
            assert!(!ship.combat.status_flags.contains(StatusFlags::MOBILITY_DISABLED));
        }
    }
}
