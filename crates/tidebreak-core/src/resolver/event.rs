//! Event resolver for telemetry and event logging.
//!
//! The `EventResolver` captures event outputs for telemetry and debugging.
//! Unlike other resolvers, it does not mutate game state - it only records
//! events that occurred during the tick.
//!
//! # Usage
//!
//! The event resolver maintains an internal log that can be drained with
//! `take_events()`. This is typically done at the end of each tick to
//! emit telemetry or trigger follow-up processing.

use std::sync::Mutex;

use crate::arena::Arena;
use crate::output::{OutputEnvelope, OutputKind};

use super::Resolver;

/// Resolver that records event outputs for telemetry.
///
/// This resolver captures all event outputs without modifying game state.
/// Events can be retrieved with `take_events()` and used for:
/// - Telemetry and analytics
/// - Replay systems
/// - Triggering audio/visual effects
/// - Debug logging
///
/// # Thread Safety
///
/// The internal event log is protected by a `Mutex` to satisfy the
/// `Send + Sync` requirements of the `Resolver` trait, even though
/// the resolver is typically used single-threaded.
///
/// # Example
///
/// ```
/// use tidebreak_core::resolver::EventResolver;
/// use tidebreak_core::resolver::Resolver;
/// use tidebreak_core::output::OutputKind;
///
/// let resolver = EventResolver::new();
/// assert!(resolver.handles().contains(&OutputKind::Event));
///
/// // After resolve(), drain the events
/// let events = resolver.take_events();
/// ```
#[derive(Debug, Default)]
pub struct EventResolver {
    /// Internal event log, protected by a mutex for thread safety.
    event_log: Mutex<Vec<OutputEnvelope>>,
}

impl EventResolver {
    /// Creates a new event resolver with an empty log.
    #[must_use]
    pub fn new() -> Self {
        Self {
            event_log: Mutex::new(Vec::new()),
        }
    }

    /// Drains and returns all recorded events.
    ///
    /// This clears the internal log. Typically called at the end of each
    /// tick to process events.
    ///
    /// # Returns
    ///
    /// A vector of event envelopes in the order they were recorded.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned (should not happen under
    /// normal circumstances).
    pub fn take_events(&self) -> Vec<OutputEnvelope> {
        let mut log = self.event_log.lock().unwrap();
        std::mem::take(&mut *log)
    }

    /// Returns the number of events currently in the log.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.event_log.lock().unwrap().len()
    }

    /// Returns true if the event log is empty.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.event_log.lock().unwrap().is_empty()
    }

    /// Clears all events from the log without returning them.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    pub fn clear(&self) {
        self.event_log.lock().unwrap().clear();
    }
}

impl Resolver for EventResolver {
    fn handles(&self) -> &[OutputKind] {
        &[OutputKind::Event]
    }

    fn resolve(&self, outputs: &[&OutputEnvelope], _current: &Arena, _next: &mut Arena) {
        let mut log = self.event_log.lock().unwrap();
        for envelope in outputs {
            if envelope.output().is_event() {
                log.push((*envelope).clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EntityId, EntityInner, EntityTag, ShipComponents};
    use crate::output::{Command, Event, Modifier, Output, PluginId, PluginInstanceId, TraceId};
    use glam::Vec2;

    fn make_envelope(output: Output, entity: EntityId) -> OutputEnvelope {
        OutputEnvelope::new(
            output,
            PluginInstanceId::new(entity, PluginId::new("test")),
            TraceId::new(0),
            0,
            0,
        )
    }

    mod resolver_trait_tests {
        use super::*;

        #[test]
        fn handles_event_kind() {
            let resolver = EventResolver::new();
            assert!(resolver.handles().contains(&OutputKind::Event));
            assert!(!resolver.handles().contains(&OutputKind::Command));
            assert!(!resolver.handles().contains(&OutputKind::Modifier));
        }
    }

    mod event_capture_tests {
        use super::*;

        #[test]
        fn captures_weapon_fired_event() {
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

            let resolver = EventResolver::new();
            let current = arena.clone();
            resolver.resolve(&[&envelope], &current, &mut arena);

            assert_eq!(resolver.event_count(), 1);
            let events = resolver.take_events();
            assert_eq!(events.len(), 1);
            assert!(events[0].output().is_event());
        }

        #[test]
        fn captures_damage_dealt_event() {
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
                Output::Event(Event::DamageDealt {
                    source: ship1,
                    target: ship2,
                    amount: 50.0,
                }),
                ship1,
            );

            let resolver = EventResolver::new();
            let current = arena.clone();
            resolver.resolve(&[&envelope], &current, &mut arena);

            let events = resolver.take_events();
            assert_eq!(events.len(), 1);
            if let Some(Event::DamageDealt { amount, .. }) = events[0].output().as_event() {
                assert!((amount - 50.0).abs() < 0.0001);
            } else {
                panic!("Expected DamageDealt event");
            }
        }

        #[test]
        fn captures_entity_destroyed_event() {
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

            let resolver = EventResolver::new();
            let current = arena.clone();
            resolver.resolve(&[&envelope], &current, &mut arena);

            let events = resolver.take_events();
            assert_eq!(events.len(), 1);
            assert!(matches!(
                events[0].output().as_event(),
                Some(Event::EntityDestroyed { .. })
            ));
        }

        #[test]
        fn captures_multiple_events() {
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
                Output::Event(Event::WeaponFired {
                    source: ship1,
                    weapon_slot: 0,
                }),
                ship1,
            );
            let envelope2 = make_envelope(
                Output::Event(Event::DamageDealt {
                    source: ship1,
                    target: ship2,
                    amount: 25.0,
                }),
                ship1,
            );
            let envelope3 = make_envelope(
                Output::Event(Event::WeaponFired {
                    source: ship2,
                    weapon_slot: 1,
                }),
                ship2,
            );

            let resolver = EventResolver::new();
            let current = arena.clone();
            resolver.resolve(&[&envelope1, &envelope2, &envelope3], &current, &mut arena);

            assert_eq!(resolver.event_count(), 3);
            let events = resolver.take_events();
            assert_eq!(events.len(), 3);
        }

        #[test]
        fn events_in_order() {
            let mut arena = Arena::new();
            let ship_id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            let envelope1 = make_envelope(
                Output::Event(Event::WeaponFired {
                    source: ship_id,
                    weapon_slot: 0,
                }),
                ship_id,
            );
            let envelope2 = make_envelope(
                Output::Event(Event::WeaponFired {
                    source: ship_id,
                    weapon_slot: 1,
                }),
                ship_id,
            );

            let resolver = EventResolver::new();
            let current = arena.clone();
            resolver.resolve(&[&envelope1, &envelope2], &current, &mut arena);

            let events = resolver.take_events();
            if let Some(Event::WeaponFired { weapon_slot, .. }) = events[0].output().as_event() {
                assert_eq!(*weapon_slot, 0);
            }
            if let Some(Event::WeaponFired { weapon_slot, .. }) = events[1].output().as_event() {
                assert_eq!(*weapon_slot, 1);
            }
        }
    }

    mod take_events_tests {
        use super::*;

        #[test]
        fn take_events_drains_log() {
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

            let resolver = EventResolver::new();
            let current = arena.clone();
            resolver.resolve(&[&envelope], &current, &mut arena);

            let events = resolver.take_events();
            assert_eq!(events.len(), 1);

            // Log should now be empty
            assert!(resolver.is_empty());
            let events2 = resolver.take_events();
            assert!(events2.is_empty());
        }

        #[test]
        fn take_events_empty_log() {
            let resolver = EventResolver::new();
            let events = resolver.take_events();
            assert!(events.is_empty());
        }

        #[test]
        fn clear_empties_log() {
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

            let resolver = EventResolver::new();
            let current = arena.clone();
            resolver.resolve(&[&envelope], &current, &mut arena);

            assert!(!resolver.is_empty());
            resolver.clear();
            assert!(resolver.is_empty());
        }
    }

    mod output_filtering_tests {
        use super::*;
        use crate::entity::components::StatusFlags;

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

            let resolver = EventResolver::new();
            let current = arena.clone();
            resolver.resolve(&[&envelope], &current, &mut arena);

            // No events should be recorded
            assert!(resolver.is_empty());
        }

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

            let resolver = EventResolver::new();
            let current = arena.clone();
            resolver.resolve(&[&envelope], &current, &mut arena);

            // No events should be recorded
            assert!(resolver.is_empty());
        }

        #[test]
        fn filters_mixed_outputs() {
            let mut arena = Arena::new();
            let ship_id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            let cmd = make_envelope(
                Output::Command(Command::SetVelocity {
                    target: ship_id,
                    velocity: Vec2::new(10.0, 5.0),
                }),
                ship_id,
            );
            let modifier = make_envelope(
                Output::Modifier(Modifier::SetStatusFlag {
                    target: ship_id,
                    flag: StatusFlags::ON_FIRE,
                    value: true,
                }),
                ship_id,
            );
            let event = make_envelope(
                Output::Event(Event::WeaponFired {
                    source: ship_id,
                    weapon_slot: 0,
                }),
                ship_id,
            );

            let resolver = EventResolver::new();
            let current = arena.clone();
            resolver.resolve(&[&cmd, &modifier, &event], &current, &mut arena);

            // Only the event should be recorded
            assert_eq!(resolver.event_count(), 1);
            let events = resolver.take_events();
            assert!(events[0].output().is_event());
        }
    }

    mod no_state_mutation_tests {
        use super::*;

        #[test]
        fn does_not_mutate_arena() {
            let mut arena = Arena::new();
            let ship_id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            // Get initial state
            let initial_hp = arena.get(ship_id).unwrap().as_ship().unwrap().combat.hp;
            let initial_pos = arena
                .get(ship_id)
                .unwrap()
                .as_ship()
                .unwrap()
                .transform
                .position;

            let envelope = make_envelope(
                Output::Event(Event::EntityDestroyed {
                    entity: ship_id,
                    destroyer: None,
                }),
                ship_id,
            );

            let resolver = EventResolver::new();
            let current = arena.clone();
            resolver.resolve(&[&envelope], &current, &mut arena);

            // State should be unchanged
            let ship = arena.get(ship_id).unwrap().as_ship().unwrap();
            assert_eq!(ship.combat.hp, initial_hp);
            assert_eq!(ship.transform.position, initial_pos);
        }
    }
}
