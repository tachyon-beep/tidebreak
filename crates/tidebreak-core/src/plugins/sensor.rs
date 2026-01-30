//! Sensor plugin for entity detection.
//!
//! The `SensorPlugin` detects nearby entities using radar and emits
//! `ContactDetected` events for each detection.
//!
//! # Supported Entity Types
//!
//! - Ships
//! - Platforms
//!
//! # Outputs
//!
//! - `Event::ContactDetected`: Emitted for each entity within radar range

use crate::entity::components::TrackQuality;
use crate::entity::EntityTag;
use crate::output::{Event, Output, OutputKind, PluginId};
use crate::plugin::{ComponentKind, Plugin, PluginContext, PluginDeclaration};
use crate::world_view::WorldView;

/// Plugin that detects nearby entities using sensors.
///
/// The sensor plugin queries for entities within radar range and emits
/// `ContactDetected` events for each detection.
///
/// # Example
///
/// ```
/// use tidebreak_core::plugins::SensorPlugin;
/// use tidebreak_core::plugin::Plugin;
///
/// let plugin = SensorPlugin::new();
/// assert_eq!(plugin.declaration().id.as_str(), "sensor");
/// ```
pub struct SensorPlugin {
    declaration: PluginDeclaration,
}

impl SensorPlugin {
    /// Creates a new `SensorPlugin`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            declaration: PluginDeclaration {
                id: PluginId::from_static("sensor"),
                required_tags: vec![EntityTag::Ship, EntityTag::Platform],
                reads: vec![ComponentKind::Transform, ComponentKind::Sensor],
                emits: vec![OutputKind::Event],
            },
        }
    }
}

impl Default for SensorPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for SensorPlugin {
    fn declaration(&self) -> &PluginDeclaration {
        &self.declaration
    }

    fn run(&self, ctx: &PluginContext, view: &WorldView) -> Vec<Output> {
        let mut outputs = vec![];

        // Get our position and sensor range
        let Some(transform) = view.get_transform(ctx.entity_id) else {
            return outputs;
        };
        let Some(sensor) = view.get_sensor(ctx.entity_id) else {
            return outputs;
        };

        // Query nearby entities using radar range
        let nearby = view.query_in_radius(transform.position, sensor.radar_range);

        for target_id in nearby {
            // Skip self
            if target_id == ctx.entity_id {
                continue;
            }

            // Emit ContactDetected event
            // Use Coarse quality for initial radar detection
            outputs.push(Output::Event(Event::ContactDetected {
                observer: ctx.entity_id,
                target: target_id,
                quality: TrackQuality::Coarse,
            }));
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
    use crate::entity::{
        EntityId, EntityInner, PlatformComponents, ProjectileComponents, ShipComponents,
    };
    use crate::output::TraceId;
    use glam::Vec2;

    #[test]
    fn new_creates_plugin() {
        let plugin = SensorPlugin::new();
        assert_eq!(plugin.declaration().id.as_str(), "sensor");
    }

    #[test]
    fn default_creates_plugin() {
        let plugin = SensorPlugin::default();
        assert_eq!(plugin.declaration().id.as_str(), "sensor");
    }

    #[test]
    fn declaration_has_correct_tags() {
        let plugin = SensorPlugin::new();
        let decl = plugin.declaration();

        assert!(decl.required_tags.contains(&EntityTag::Ship));
        assert!(decl.required_tags.contains(&EntityTag::Platform));
        assert!(!decl.required_tags.contains(&EntityTag::Squadron));
        assert!(!decl.required_tags.contains(&EntityTag::Projectile));
    }

    #[test]
    fn declaration_reads_transform_and_sensor() {
        let plugin = SensorPlugin::new();
        let decl = plugin.declaration();

        assert!(decl.reads.contains(&ComponentKind::Transform));
        assert!(decl.reads.contains(&ComponentKind::Sensor));
    }

    #[test]
    fn declaration_emits_events() {
        let plugin = SensorPlugin::new();
        let decl = plugin.declaration();

        assert!(decl.emits.contains(&OutputKind::Event));
    }

    #[test]
    fn run_emits_contact_detected_for_nearby_entities() {
        let plugin = SensorPlugin::new();
        let mut arena = Arena::new();

        // Ship at origin with default sensor range (10000m)
        let ship_id = arena.spawn(
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::at_position(Vec2::new(0.0, 0.0), 0.0)),
        );

        // Another ship within range
        let target_id = arena.spawn(
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::at_position(Vec2::new(5000.0, 0.0), 0.0)),
        );

        let view = WorldView::for_plugin(&arena, plugin.declaration(), arena.current_tick());
        let ctx = PluginContext {
            entity_id: ship_id,
            tick: arena.current_tick(),
            trace_id: TraceId::new(0),
        };

        let outputs = plugin.run(&ctx, &view);

        // Should have one contact detected event
        assert_eq!(outputs.len(), 1);

        match &outputs[0] {
            Output::Event(Event::ContactDetected {
                observer,
                target,
                quality,
            }) => {
                assert_eq!(*observer, ship_id);
                assert_eq!(*target, target_id);
                assert_eq!(*quality, TrackQuality::Coarse);
            }
            _ => panic!("Expected ContactDetected event"),
        }
    }

    #[test]
    fn run_skips_self() {
        let plugin = SensorPlugin::new();
        let mut arena = Arena::new();

        // Single ship - should not detect itself
        let ship_id = arena.spawn(
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::at_position(Vec2::new(0.0, 0.0), 0.0)),
        );

        let view = WorldView::for_plugin(&arena, plugin.declaration(), arena.current_tick());
        let ctx = PluginContext {
            entity_id: ship_id,
            tick: arena.current_tick(),
            trace_id: TraceId::new(0),
        };

        let outputs = plugin.run(&ctx, &view);

        // Should have no outputs (only self in range)
        assert!(outputs.is_empty());
    }

    #[test]
    fn run_detects_multiple_entities() {
        let plugin = SensorPlugin::new();
        let mut arena = Arena::new();

        // Ship at origin
        let ship_id = arena.spawn(
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::at_position(Vec2::new(0.0, 0.0), 0.0)),
        );

        // Multiple targets within range
        let _target1 = arena.spawn(
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::at_position(Vec2::new(1000.0, 0.0), 0.0)),
        );
        let _target2 = arena.spawn(
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::at_position(Vec2::new(-1000.0, 0.0), 0.0)),
        );
        let _target3 = arena.spawn(
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::at_position(Vec2::new(0.0, 1000.0), 0.0)),
        );

        let view = WorldView::for_plugin(&arena, plugin.declaration(), arena.current_tick());
        let ctx = PluginContext {
            entity_id: ship_id,
            tick: arena.current_tick(),
            trace_id: TraceId::new(0),
        };

        let outputs = plugin.run(&ctx, &view);

        // Should detect all 3 targets
        assert_eq!(outputs.len(), 3);

        // All should be ContactDetected events
        for output in &outputs {
            assert!(matches!(output, Output::Event(Event::ContactDetected { .. })));
        }
    }

    #[test]
    fn run_ignores_entities_outside_range() {
        let plugin = SensorPlugin::new();
        let mut arena = Arena::new();

        // Ship at origin with default radar range 10000m
        let ship_id = arena.spawn(
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::at_position(Vec2::new(0.0, 0.0), 0.0)),
        );

        // Target beyond radar range
        let _far_target = arena.spawn(
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::at_position(Vec2::new(20000.0, 0.0), 0.0)),
        );

        let view = WorldView::for_plugin(&arena, plugin.declaration(), arena.current_tick());
        let ctx = PluginContext {
            entity_id: ship_id,
            tick: arena.current_tick(),
            trace_id: TraceId::new(0),
        };

        let outputs = plugin.run(&ctx, &view);

        // Should have no outputs (target out of range)
        assert!(outputs.is_empty());
    }

    #[test]
    fn run_detects_different_entity_types() {
        let plugin = SensorPlugin::new();
        let mut arena = Arena::new();

        // Ship at origin
        let ship_id = arena.spawn(
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::at_position(Vec2::new(0.0, 0.0), 0.0)),
        );

        // Different entity types within range
        let _platform = arena.spawn(
            EntityTag::Platform,
            EntityInner::Platform(PlatformComponents::at_position(Vec2::new(1000.0, 0.0))),
        );
        let _projectile = arena.spawn(
            EntityTag::Projectile,
            EntityInner::Projectile(ProjectileComponents::at_position_with_velocity(
                Vec2::new(2000.0, 0.0),
                0.0,
                Vec2::new(100.0, 0.0),
            )),
        );

        let view = WorldView::for_plugin(&arena, plugin.declaration(), arena.current_tick());
        let ctx = PluginContext {
            entity_id: ship_id,
            tick: arena.current_tick(),
            trace_id: TraceId::new(0),
        };

        let outputs = plugin.run(&ctx, &view);

        // Should detect both platform and projectile
        assert_eq!(outputs.len(), 2);
    }

    #[test]
    fn run_for_platform() {
        let plugin = SensorPlugin::new();
        let mut arena = Arena::new();

        // Platform at origin
        let platform_id = arena.spawn(
            EntityTag::Platform,
            EntityInner::Platform(PlatformComponents::at_position(Vec2::new(0.0, 0.0))),
        );

        // Ship within range
        let _ship = arena.spawn(
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::at_position(Vec2::new(5000.0, 0.0), 0.0)),
        );

        let view = WorldView::for_plugin(&arena, plugin.declaration(), arena.current_tick());
        let ctx = PluginContext {
            entity_id: platform_id,
            tick: arena.current_tick(),
            trace_id: TraceId::new(0),
        };

        let outputs = plugin.run(&ctx, &view);

        // Platform should detect the ship
        assert_eq!(outputs.len(), 1);
    }

    #[test]
    fn run_with_nonexistent_entity() {
        let plugin = SensorPlugin::new();
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
        assert_send_sync::<SensorPlugin>();
    }
}
