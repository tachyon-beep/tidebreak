//! Weapon plugin for combat actions.
//!
//! The `WeaponPlugin` handles weapon firing based on available targets
//! from the track table.
//!
//! # Supported Entity Types
//!
//! - Ships
//! - Squadrons
//!
//! # Outputs
//!
//! - `Command::FireWeapon`: Emitted when firing at a tracked target

use crate::entity::EntityTag;
use crate::output::{Command, Output, OutputKind, PluginId};
use crate::plugin::{ComponentKind, Plugin, PluginContext, PluginDeclaration};
use crate::world_view::WorldView;

/// Plugin that handles weapon firing.
///
/// The weapon plugin checks available weapons and fires at tracked targets.
/// For MVP, it fires each ready weapon at the first available track.
///
/// # Example
///
/// ```
/// use tidebreak_core::plugins::WeaponPlugin;
/// use tidebreak_core::plugin::Plugin;
///
/// let plugin = WeaponPlugin::new();
/// assert_eq!(plugin.declaration().id.as_str(), "weapon");
/// ```
pub struct WeaponPlugin {
    declaration: PluginDeclaration,
}

impl WeaponPlugin {
    /// Creates a new `WeaponPlugin`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            declaration: PluginDeclaration {
                id: PluginId::from_static("weapon"),
                required_tags: vec![EntityTag::Ship, EntityTag::Squadron],
                reads: vec![
                    ComponentKind::Transform,
                    ComponentKind::Combat,
                    ComponentKind::Sensor,
                ],
                emits: vec![OutputKind::Command, OutputKind::Event],
            },
        }
    }
}

impl Default for WeaponPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for WeaponPlugin {
    fn declaration(&self) -> &PluginDeclaration {
        &self.declaration
    }

    fn run(&self, ctx: &PluginContext, view: &WorldView) -> Vec<Output> {
        let mut outputs = vec![];

        // Get our combat state (weapons)
        let Some(combat) = view.get_combat(ctx.entity_id) else {
            return outputs;
        };

        // Get our sensor state (track table)
        let Some(sensor) = view.get_sensor(ctx.entity_id) else {
            return outputs;
        };

        // Check if we have any tracks to fire at
        if sensor.track_table.is_empty() {
            return outputs;
        }

        // Check each weapon
        for weapon in &combat.weapons {
            if !weapon.is_ready() {
                continue;
            }

            // Fire at first available target from tracks
            if let Some(track) = sensor.track_table.first() {
                outputs.push(Output::Command(Command::FireWeapon {
                    source: ctx.entity_id,
                    target: track.target_id,
                    slot: weapon.slot,
                }));
            }
        }

        outputs
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::Arena;
    use crate::entity::components::{AmmoType, Track, TrackQuality, WeaponState};
    use crate::entity::{EntityId, EntityInner, ShipComponents, SquadronComponents};
    use crate::output::TraceId;
    use glam::Vec2;

    fn create_ship_with_weapon_and_track(arena: &mut Arena) -> (EntityId, EntityId) {
        // Create a ship with a weapon and a track
        let mut ship_components = ShipComponents::at_position(Vec2::new(0.0, 0.0), 0.0);

        // Add a ready weapon
        ship_components
            .combat
            .weapons
            .push(WeaponState::new(0, 1.0, AmmoType::Missile));

        // First spawn the target so we have an ID
        let target_id = arena.spawn(
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::at_position(Vec2::new(5000.0, 0.0), 0.0)),
        );

        // Add a track for the target
        ship_components.sensor.track_table.push(Track::new(
            target_id,
            Vec2::new(5000.0, 0.0),
            TrackQuality::FireControl,
        ));

        let ship_id = arena.spawn(EntityTag::Ship, EntityInner::Ship(ship_components));

        (ship_id, target_id)
    }

    #[test]
    fn new_creates_plugin() {
        let plugin = WeaponPlugin::new();
        assert_eq!(plugin.declaration().id.as_str(), "weapon");
    }

    #[test]
    fn default_creates_plugin() {
        let plugin = WeaponPlugin::default();
        assert_eq!(plugin.declaration().id.as_str(), "weapon");
    }

    #[test]
    fn declaration_has_correct_tags() {
        let plugin = WeaponPlugin::new();
        let decl = plugin.declaration();

        assert!(decl.required_tags.contains(&EntityTag::Ship));
        assert!(decl.required_tags.contains(&EntityTag::Squadron));
        assert!(!decl.required_tags.contains(&EntityTag::Platform));
        assert!(!decl.required_tags.contains(&EntityTag::Projectile));
    }

    #[test]
    fn declaration_reads_correct_components() {
        let plugin = WeaponPlugin::new();
        let decl = plugin.declaration();

        assert!(decl.reads.contains(&ComponentKind::Transform));
        assert!(decl.reads.contains(&ComponentKind::Combat));
        assert!(decl.reads.contains(&ComponentKind::Sensor));
    }

    #[test]
    fn declaration_emits_commands_and_events() {
        let plugin = WeaponPlugin::new();
        let decl = plugin.declaration();

        assert!(decl.emits.contains(&OutputKind::Command));
        assert!(decl.emits.contains(&OutputKind::Event));
    }

    #[test]
    fn run_fires_at_tracked_target() {
        let plugin = WeaponPlugin::new();
        let mut arena = Arena::new();

        let (ship_id, target_id) = create_ship_with_weapon_and_track(&mut arena);

        let view = WorldView::for_plugin(&arena, plugin.declaration(), arena.current_tick());
        let ctx = PluginContext {
            entity_id: ship_id,
            tick: arena.current_tick(),
            trace_id: TraceId::new(0),
        };

        let outputs = plugin.run(&ctx, &view);

        // Should have one FireWeapon command
        assert_eq!(outputs.len(), 1);

        match &outputs[0] {
            Output::Command(Command::FireWeapon {
                source,
                target,
                slot,
            }) => {
                assert_eq!(*source, ship_id);
                assert_eq!(*target, target_id);
                assert_eq!(*slot, 0);
            }
            _ => panic!("Expected FireWeapon command"),
        }
    }

    #[test]
    fn run_returns_empty_without_tracks() {
        let plugin = WeaponPlugin::new();
        let mut arena = Arena::new();

        // Ship with weapon but no tracks
        let mut ship_components = ShipComponents::at_position(Vec2::new(0.0, 0.0), 0.0);
        ship_components
            .combat
            .weapons
            .push(WeaponState::new(0, 1.0, AmmoType::Missile));

        let ship_id = arena.spawn(EntityTag::Ship, EntityInner::Ship(ship_components));

        let view = WorldView::for_plugin(&arena, plugin.declaration(), arena.current_tick());
        let ctx = PluginContext {
            entity_id: ship_id,
            tick: arena.current_tick(),
            trace_id: TraceId::new(0),
        };

        let outputs = plugin.run(&ctx, &view);

        // Should have no outputs (no tracks)
        assert!(outputs.is_empty());
    }

    #[test]
    fn run_returns_empty_without_weapons() {
        let plugin = WeaponPlugin::new();
        let mut arena = Arena::new();

        // Create target first
        let target_id = arena.spawn(
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::at_position(Vec2::new(5000.0, 0.0), 0.0)),
        );

        // Ship with track but no weapons
        let mut ship_components = ShipComponents::at_position(Vec2::new(0.0, 0.0), 0.0);
        ship_components.sensor.track_table.push(Track::new(
            target_id,
            Vec2::new(5000.0, 0.0),
            TrackQuality::FireControl,
        ));

        let ship_id = arena.spawn(EntityTag::Ship, EntityInner::Ship(ship_components));

        let view = WorldView::for_plugin(&arena, plugin.declaration(), arena.current_tick());
        let ctx = PluginContext {
            entity_id: ship_id,
            tick: arena.current_tick(),
            trace_id: TraceId::new(0),
        };

        let outputs = plugin.run(&ctx, &view);

        // Should have no outputs (no weapons)
        assert!(outputs.is_empty());
    }

    #[test]
    fn run_skips_weapons_on_cooldown() {
        let plugin = WeaponPlugin::new();
        let mut arena = Arena::new();

        // Create target first
        let target_id = arena.spawn(
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::at_position(Vec2::new(5000.0, 0.0), 0.0)),
        );

        // Ship with weapon on cooldown
        let mut ship_components = ShipComponents::at_position(Vec2::new(0.0, 0.0), 0.0);
        let mut weapon = WeaponState::new(0, 1.0, AmmoType::Missile);
        weapon.cooldown = 0.5; // On cooldown
        ship_components.combat.weapons.push(weapon);
        ship_components.sensor.track_table.push(Track::new(
            target_id,
            Vec2::new(5000.0, 0.0),
            TrackQuality::FireControl,
        ));

        let ship_id = arena.spawn(EntityTag::Ship, EntityInner::Ship(ship_components));

        let view = WorldView::for_plugin(&arena, plugin.declaration(), arena.current_tick());
        let ctx = PluginContext {
            entity_id: ship_id,
            tick: arena.current_tick(),
            trace_id: TraceId::new(0),
        };

        let outputs = plugin.run(&ctx, &view);

        // Should have no outputs (weapon on cooldown)
        assert!(outputs.is_empty());
    }

    #[test]
    fn run_fires_multiple_weapons() {
        let plugin = WeaponPlugin::new();
        let mut arena = Arena::new();

        // Create target first
        let target_id = arena.spawn(
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::at_position(Vec2::new(5000.0, 0.0), 0.0)),
        );

        // Ship with multiple weapons
        let mut ship_components = ShipComponents::at_position(Vec2::new(0.0, 0.0), 0.0);
        ship_components
            .combat
            .weapons
            .push(WeaponState::new(0, 1.0, AmmoType::Missile));
        ship_components
            .combat
            .weapons
            .push(WeaponState::new(1, 1.0, AmmoType::Torpedo));
        ship_components.sensor.track_table.push(Track::new(
            target_id,
            Vec2::new(5000.0, 0.0),
            TrackQuality::FireControl,
        ));

        let ship_id = arena.spawn(EntityTag::Ship, EntityInner::Ship(ship_components));

        let view = WorldView::for_plugin(&arena, plugin.declaration(), arena.current_tick());
        let ctx = PluginContext {
            entity_id: ship_id,
            tick: arena.current_tick(),
            trace_id: TraceId::new(0),
        };

        let outputs = plugin.run(&ctx, &view);

        // Should fire both weapons
        assert_eq!(outputs.len(), 2);

        // Verify different slots
        let slots: Vec<usize> = outputs
            .iter()
            .filter_map(|o| match o {
                Output::Command(Command::FireWeapon { slot, .. }) => Some(*slot),
                _ => None,
            })
            .collect();
        assert!(slots.contains(&0));
        assert!(slots.contains(&1));
    }

    #[test]
    fn run_for_squadron() {
        let plugin = WeaponPlugin::new();
        let mut arena = Arena::new();

        // Create target first
        let target_id = arena.spawn(
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::at_position(Vec2::new(5000.0, 0.0), 0.0)),
        );

        // Squadron with weapon and track
        // Note: Squadrons don't have sensors by default in our component model,
        // so this test verifies the plugin handles missing sensor gracefully
        let squadron_id = arena.spawn(
            EntityTag::Squadron,
            EntityInner::Squadron(SquadronComponents::at_position(Vec2::new(0.0, 0.0), 0.0)),
        );

        let view = WorldView::for_plugin(&arena, plugin.declaration(), arena.current_tick());
        let ctx = PluginContext {
            entity_id: squadron_id,
            tick: arena.current_tick(),
            trace_id: TraceId::new(0),
        };

        // Squadrons don't have sensors, so should return empty
        let outputs = plugin.run(&ctx, &view);
        assert!(outputs.is_empty());

        // Silence unused variable warning
        let _ = target_id;
    }

    #[test]
    fn run_with_nonexistent_entity() {
        let plugin = WeaponPlugin::new();
        let arena = Arena::new();

        let view = WorldView::for_plugin(&arena, plugin.declaration(), arena.current_tick());
        let ctx = PluginContext {
            entity_id: EntityId::new(999),
            tick: arena.current_tick(),
            trace_id: TraceId::new(0),
        };

        // Should not panic, just return empty outputs
        let outputs = plugin.run(&ctx, &view);
        assert!(outputs.is_empty());
    }

    #[test]
    fn plugin_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WeaponPlugin>();
    }
}
