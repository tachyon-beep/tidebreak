//! Combat resolver for damage, healing, and status effects.
//!
//! The `CombatResolver` handles:
//! - `ApplyDamage` modifiers: Reduce entity HP
//! - `ApplyHealing` modifiers: Increase entity HP (capped at max)
//! - `SetStatusFlag` modifiers: Enable or disable status flags
//!
//! # Destruction Handling
//!
//! When an entity's HP reaches 0 or below, the `DESTROYED` flag is set.
//! The entity is not immediately removed - that's handled by a cleanup phase.

use crate::arena::Arena;
use crate::entity::components::StatusFlags;
use crate::entity::EntityId;
use crate::output::{Modifier, OutputEnvelope, OutputKind};

use super::Resolver;

/// Resolver for combat-related modifiers.
///
/// Handles damage, healing, and status flag changes.
///
/// # Processing Order
///
/// 1. Apply all damage modifiers (summed for each target)
/// 2. Apply all healing modifiers (summed for each target, capped at max HP)
/// 3. Apply all status flag changes
/// 4. Set `DESTROYED` flag for any entities with HP <= 0
///
/// Note: The current implementation processes in output order, not the
/// batched order described above. This matches the "last-write-wins" for
/// status flags and allows damage/healing to be processed incrementally.
///
/// # Example
///
/// ```
/// use tidebreak_core::resolver::CombatResolver;
/// use tidebreak_core::resolver::Resolver;
/// use tidebreak_core::output::OutputKind;
///
/// let resolver = CombatResolver::new();
/// assert!(resolver.handles().contains(&OutputKind::Modifier));
/// ```
#[derive(Debug, Clone, Default)]
pub struct CombatResolver;

impl CombatResolver {
    /// Creates a new combat resolver.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Applies damage to an entity, setting DESTROYED flag if HP <= 0.
    fn apply_damage(next: &mut Arena, target: EntityId, amount: f32) {
        if let Some(entity) = next.get_mut(target) {
            // Try each entity type that has combat
            if let Some(ship) = entity.as_ship_mut() {
                ship.combat.hp -= amount;
                if ship.combat.hp <= 0.0 {
                    ship.combat.hp = 0.0;
                    ship.combat.status_flags.insert(StatusFlags::DESTROYED);
                }
            } else if let Some(squadron) = entity.as_squadron_mut() {
                squadron.combat.hp -= amount;
                if squadron.combat.hp <= 0.0 {
                    squadron.combat.hp = 0.0;
                    squadron.combat.status_flags.insert(StatusFlags::DESTROYED);
                }
            }
            // Platforms and projectiles don't have combat state
        }
    }

    /// Applies healing to an entity, capped at max HP.
    fn apply_healing(next: &mut Arena, target: EntityId, amount: f32) {
        if let Some(entity) = next.get_mut(target) {
            if let Some(ship) = entity.as_ship_mut() {
                ship.combat.hp = (ship.combat.hp + amount).min(ship.combat.max_hp);
            } else if let Some(squadron) = entity.as_squadron_mut() {
                squadron.combat.hp = (squadron.combat.hp + amount).min(squadron.combat.max_hp);
            }
        }
    }

    /// Sets or clears a status flag on an entity.
    fn set_status_flag(next: &mut Arena, target: EntityId, flag: StatusFlags, value: bool) {
        if let Some(entity) = next.get_mut(target) {
            if let Some(ship) = entity.as_ship_mut() {
                if value {
                    ship.combat.status_flags.insert(flag);
                } else {
                    ship.combat.status_flags.remove(flag);
                }
            } else if let Some(squadron) = entity.as_squadron_mut() {
                if value {
                    squadron.combat.status_flags.insert(flag);
                } else {
                    squadron.combat.status_flags.remove(flag);
                }
            }
        }
    }
}

impl Resolver for CombatResolver {
    fn handles(&self) -> &[OutputKind] {
        &[OutputKind::Modifier]
    }

    fn resolve(&self, outputs: &[&OutputEnvelope], _current: &Arena, next: &mut Arena) {
        for envelope in outputs {
            if let Some(modifier) = envelope.output().as_modifier() {
                match modifier {
                    Modifier::ApplyDamage { target, amount } => {
                        Self::apply_damage(next, *target, *amount);
                    }
                    Modifier::ApplyHealing { target, amount } => {
                        Self::apply_healing(next, *target, *amount);
                    }
                    Modifier::SetStatusFlag { target, flag, value } => {
                        Self::set_status_flag(next, *target, *flag, *value);
                    }
                    // ModifyStat is more complex and not MVP
                    Modifier::ModifyStat { .. } => {}
                }
            }
        }
        // Commands like FireWeapon are not yet implemented.
        // When command support is added, they should be processed separately
        // and registered in the `handles()` method.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EntityInner, EntityTag, ShipComponents, SquadronComponents};
    use crate::output::{Command, Event, Output, PluginId, PluginInstanceId, TraceId};
    use glam::Vec2;

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
        fn handles_modifier_kind() {
            let resolver = CombatResolver::new();
            assert!(resolver.handles().contains(&OutputKind::Modifier));
            assert!(!resolver.handles().contains(&OutputKind::Command));
            assert!(!resolver.handles().contains(&OutputKind::Event));
        }
    }

    mod apply_damage_tests {
        use super::*;

        #[test]
        fn damage_reduces_hp() {
            let mut arena = Arena::new();
            let ship_id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            let envelope = make_envelope(
                Output::Modifier(Modifier::ApplyDamage {
                    target: ship_id,
                    amount: 30.0,
                }),
                ship_id,
            );

            let resolver = CombatResolver::new();
            let current = arena.clone();
            resolver.resolve(&[&envelope], &current, &mut arena);

            let ship = arena.get(ship_id).unwrap().as_ship().unwrap();
            assert!((ship.combat.hp - 70.0).abs() < 0.0001);
        }

        #[test]
        fn damage_kills_entity_at_zero_hp() {
            let mut arena = Arena::new();
            let ship_id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            let envelope = make_envelope(
                Output::Modifier(Modifier::ApplyDamage {
                    target: ship_id,
                    amount: 100.0,
                }),
                ship_id,
            );

            let resolver = CombatResolver::new();
            let current = arena.clone();
            resolver.resolve(&[&envelope], &current, &mut arena);

            let ship = arena.get(ship_id).unwrap().as_ship().unwrap();
            assert_eq!(ship.combat.hp, 0.0);
            assert!(ship.combat.status_flags.contains(StatusFlags::DESTROYED));
        }

        #[test]
        fn damage_kills_entity_below_zero_hp() {
            let mut arena = Arena::new();
            let ship_id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            let envelope = make_envelope(
                Output::Modifier(Modifier::ApplyDamage {
                    target: ship_id,
                    amount: 150.0, // More than max HP
                }),
                ship_id,
            );

            let resolver = CombatResolver::new();
            let current = arena.clone();
            resolver.resolve(&[&envelope], &current, &mut arena);

            let ship = arena.get(ship_id).unwrap().as_ship().unwrap();
            assert_eq!(ship.combat.hp, 0.0); // Clamped to 0
            assert!(ship.combat.status_flags.contains(StatusFlags::DESTROYED));
        }

        #[test]
        fn damage_nonexistent_entity_ignored() {
            let mut arena = Arena::new();
            let fake_id = EntityId::new(999);

            let envelope = make_envelope(
                Output::Modifier(Modifier::ApplyDamage {
                    target: fake_id,
                    amount: 50.0,
                }),
                fake_id,
            );

            let resolver = CombatResolver::new();
            let current = arena.clone();
            // Should not panic
            resolver.resolve(&[&envelope], &current, &mut arena);
        }

        #[test]
        fn damage_multiple_hits_accumulate() {
            let mut arena = Arena::new();
            let ship_id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            let envelope1 = make_envelope(
                Output::Modifier(Modifier::ApplyDamage {
                    target: ship_id,
                    amount: 20.0,
                }),
                ship_id,
            );
            let envelope2 = make_envelope(
                Output::Modifier(Modifier::ApplyDamage {
                    target: ship_id,
                    amount: 30.0,
                }),
                ship_id,
            );

            let resolver = CombatResolver::new();
            let current = arena.clone();
            resolver.resolve(&[&envelope1, &envelope2], &current, &mut arena);

            let ship = arena.get(ship_id).unwrap().as_ship().unwrap();
            // 100 - 20 - 30 = 50
            assert!((ship.combat.hp - 50.0).abs() < 0.0001);
        }

        #[test]
        fn damage_squadron() {
            let mut arena = Arena::new();
            let squadron_id = arena.spawn(
                EntityTag::Squadron,
                EntityInner::Squadron(SquadronComponents::at_position(Vec2::ZERO, 0.0)),
            );

            let envelope = make_envelope(
                Output::Modifier(Modifier::ApplyDamage {
                    target: squadron_id,
                    amount: 30.0,
                }),
                squadron_id,
            );

            let resolver = CombatResolver::new();
            let current = arena.clone();
            resolver.resolve(&[&envelope], &current, &mut arena);

            let squadron = arena.get(squadron_id).unwrap().as_squadron().unwrap();
            assert!((squadron.combat.hp - 70.0).abs() < 0.0001);
        }
    }

    mod apply_healing_tests {
        use super::*;

        #[test]
        fn healing_increases_hp() {
            let mut arena = Arena::new();
            let ship_id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            // First damage the ship
            if let Some(ship) = arena.get_mut(ship_id).unwrap().as_ship_mut() {
                ship.combat.hp = 50.0;
            }

            let envelope = make_envelope(
                Output::Modifier(Modifier::ApplyHealing {
                    target: ship_id,
                    amount: 20.0,
                }),
                ship_id,
            );

            let resolver = CombatResolver::new();
            let current = arena.clone();
            resolver.resolve(&[&envelope], &current, &mut arena);

            let ship = arena.get(ship_id).unwrap().as_ship().unwrap();
            assert!((ship.combat.hp - 70.0).abs() < 0.0001);
        }

        #[test]
        fn healing_capped_at_max_hp() {
            let mut arena = Arena::new();
            let ship_id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            // Damage the ship slightly
            if let Some(ship) = arena.get_mut(ship_id).unwrap().as_ship_mut() {
                ship.combat.hp = 90.0;
            }

            let envelope = make_envelope(
                Output::Modifier(Modifier::ApplyHealing {
                    target: ship_id,
                    amount: 50.0, // Would go over max
                }),
                ship_id,
            );

            let resolver = CombatResolver::new();
            let current = arena.clone();
            resolver.resolve(&[&envelope], &current, &mut arena);

            let ship = arena.get(ship_id).unwrap().as_ship().unwrap();
            assert_eq!(ship.combat.hp, 100.0); // Capped at max
        }

        #[test]
        fn healing_nonexistent_entity_ignored() {
            let mut arena = Arena::new();
            let fake_id = EntityId::new(999);

            let envelope = make_envelope(
                Output::Modifier(Modifier::ApplyHealing {
                    target: fake_id,
                    amount: 50.0,
                }),
                fake_id,
            );

            let resolver = CombatResolver::new();
            let current = arena.clone();
            // Should not panic
            resolver.resolve(&[&envelope], &current, &mut arena);
        }
    }

    mod set_status_flag_tests {
        use super::*;

        #[test]
        fn set_status_flag_enables_flag() {
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

            let resolver = CombatResolver::new();
            let current = arena.clone();
            resolver.resolve(&[&envelope], &current, &mut arena);

            let ship = arena.get(ship_id).unwrap().as_ship().unwrap();
            assert!(ship.combat.status_flags.contains(StatusFlags::MOBILITY_DISABLED));
        }

        #[test]
        fn set_status_flag_disables_flag() {
            let mut arena = Arena::new();
            let ship_id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            // First enable the flag
            if let Some(ship) = arena.get_mut(ship_id).unwrap().as_ship_mut() {
                ship.combat.status_flags.insert(StatusFlags::WEAPONS_DISABLED);
            }

            let envelope = make_envelope(
                Output::Modifier(Modifier::SetStatusFlag {
                    target: ship_id,
                    flag: StatusFlags::WEAPONS_DISABLED,
                    value: false,
                }),
                ship_id,
            );

            let resolver = CombatResolver::new();
            let current = arena.clone();
            resolver.resolve(&[&envelope], &current, &mut arena);

            let ship = arena.get(ship_id).unwrap().as_ship().unwrap();
            assert!(!ship.combat.status_flags.contains(StatusFlags::WEAPONS_DISABLED));
        }

        #[test]
        fn set_multiple_status_flags() {
            let mut arena = Arena::new();
            let ship_id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            let envelope1 = make_envelope(
                Output::Modifier(Modifier::SetStatusFlag {
                    target: ship_id,
                    flag: StatusFlags::MOBILITY_DISABLED,
                    value: true,
                }),
                ship_id,
            );
            let envelope2 = make_envelope(
                Output::Modifier(Modifier::SetStatusFlag {
                    target: ship_id,
                    flag: StatusFlags::SENSORS_DISABLED,
                    value: true,
                }),
                ship_id,
            );

            let resolver = CombatResolver::new();
            let current = arena.clone();
            resolver.resolve(&[&envelope1, &envelope2], &current, &mut arena);

            let ship = arena.get(ship_id).unwrap().as_ship().unwrap();
            assert!(ship.combat.status_flags.contains(StatusFlags::MOBILITY_DISABLED));
            assert!(ship.combat.status_flags.contains(StatusFlags::SENSORS_DISABLED));
        }

        #[test]
        fn set_status_flag_nonexistent_entity_ignored() {
            let mut arena = Arena::new();
            let fake_id = EntityId::new(999);

            let envelope = make_envelope(
                Output::Modifier(Modifier::SetStatusFlag {
                    target: fake_id,
                    flag: StatusFlags::DESTROYED,
                    value: true,
                }),
                fake_id,
            );

            let resolver = CombatResolver::new();
            let current = arena.clone();
            // Should not panic
            resolver.resolve(&[&envelope], &current, &mut arena);
        }

        #[test]
        fn set_status_flag_squadron() {
            let mut arena = Arena::new();
            let squadron_id = arena.spawn(
                EntityTag::Squadron,
                EntityInner::Squadron(SquadronComponents::default()),
            );

            let envelope = make_envelope(
                Output::Modifier(Modifier::SetStatusFlag {
                    target: squadron_id,
                    flag: StatusFlags::WEAPONS_DISABLED,
                    value: true,
                }),
                squadron_id,
            );

            let resolver = CombatResolver::new();
            let current = arena.clone();
            resolver.resolve(&[&envelope], &current, &mut arena);

            let squadron = arena.get(squadron_id).unwrap().as_squadron().unwrap();
            assert!(squadron.combat.status_flags.contains(StatusFlags::WEAPONS_DISABLED));
        }
    }

    mod output_filtering_tests {
        use super::*;

        #[test]
        fn ignores_command_outputs() {
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

            let resolver = CombatResolver::new();
            let current = arena.clone();
            resolver.resolve(&[&envelope], &current, &mut arena);

            // Velocity should be unchanged (combat resolver ignores SetVelocity)
            let ship = arena.get(ship_id).unwrap().as_ship().unwrap();
            assert_eq!(ship.physics.velocity, Vec2::ZERO);
        }

        #[test]
        fn ignores_event_outputs() {
            let mut arena = Arena::new();
            let ship_id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            let envelope = make_envelope(
                Output::Event(Event::EntityDestroyed {
                    entity: ship_id,
                    destroyer: None,
                }),
                ship_id,
            );

            let resolver = CombatResolver::new();
            let current = arena.clone();
            // Should not panic and should not change state
            resolver.resolve(&[&envelope], &current, &mut arena);

            // Entity should still exist and be undamaged
            let ship = arena.get(ship_id).unwrap().as_ship().unwrap();
            assert_eq!(ship.combat.hp, 100.0);
            assert!(!ship.combat.status_flags.contains(StatusFlags::DESTROYED));
        }
    }
}
