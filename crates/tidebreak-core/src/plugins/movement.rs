//! Movement plugin for entity locomotion.
//!
//! The `MovementPlugin` is responsible for handling entity movement. In MVP,
//! it acts as a placeholder that maintains current velocity. In future versions,
//! it will receive input from DRL policies or player commands.
//!
//! # Supported Entity Types
//!
//! - Ships
//! - Squadrons
//!
//! # Outputs
//!
//! Currently emits no outputs (entities maintain their current velocity).
//! Future versions will emit `SetVelocity` and `SetHeading` commands.

use crate::entity::EntityTag;
use crate::output::{Output, OutputKind, PluginId};
use crate::plugin::{ComponentKind, Plugin, PluginContext, PluginDeclaration};
use crate::world_view::WorldView;

/// Plugin that handles entity movement.
///
/// For MVP, this plugin maintains the current velocity (no active control).
/// Later versions will integrate with DRL policies or player input.
///
/// # Example
///
/// ```
/// use tidebreak_core::plugins::MovementPlugin;
/// use tidebreak_core::plugin::Plugin;
///
/// let plugin = MovementPlugin::new();
/// assert_eq!(plugin.declaration().id.as_str(), "movement");
/// ```
pub struct MovementPlugin {
    declaration: PluginDeclaration,
}

impl MovementPlugin {
    /// Creates a new `MovementPlugin`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            declaration: PluginDeclaration {
                id: PluginId::from_static("movement"),
                required_tags: vec![EntityTag::Ship, EntityTag::Squadron],
                reads: vec![ComponentKind::Transform, ComponentKind::Physics],
                emits: vec![OutputKind::Command],
            },
        }
    }
}

impl Default for MovementPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for MovementPlugin {
    fn declaration(&self) -> &PluginDeclaration {
        &self.declaration
    }

    fn run(&self, _ctx: &PluginContext, _view: &WorldView) -> Vec<Output> {
        // For MVP: maintain current velocity (placeholder for AI/player input)
        // Later: will receive input from DRL policy
        vec![] // No outputs for now - just let physics continue
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::Arena;
    use crate::entity::{EntityId, EntityInner, ShipComponents, SquadronComponents};
    use crate::output::TraceId;
    use glam::Vec2;

    #[test]
    fn new_creates_plugin() {
        let plugin = MovementPlugin::new();
        assert_eq!(plugin.declaration().id.as_str(), "movement");
    }

    #[test]
    fn default_creates_plugin() {
        let plugin = MovementPlugin::default();
        assert_eq!(plugin.declaration().id.as_str(), "movement");
    }

    #[test]
    fn declaration_has_correct_tags() {
        let plugin = MovementPlugin::new();
        let decl = plugin.declaration();

        assert!(decl.required_tags.contains(&EntityTag::Ship));
        assert!(decl.required_tags.contains(&EntityTag::Squadron));
        assert!(!decl.required_tags.contains(&EntityTag::Platform));
        assert!(!decl.required_tags.contains(&EntityTag::Projectile));
    }

    #[test]
    fn declaration_reads_transform_and_physics() {
        let plugin = MovementPlugin::new();
        let decl = plugin.declaration();

        assert!(decl.reads.contains(&ComponentKind::Transform));
        assert!(decl.reads.contains(&ComponentKind::Physics));
    }

    #[test]
    fn declaration_emits_commands() {
        let plugin = MovementPlugin::new();
        let decl = plugin.declaration();

        assert!(decl.emits.contains(&OutputKind::Command));
    }

    #[test]
    fn run_returns_empty_for_ship() {
        let plugin = MovementPlugin::new();
        let mut arena = Arena::new();

        let ship_id = arena.spawn(
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::at_position(Vec2::new(100.0, 200.0), 0.0)),
        );

        let view = WorldView::for_plugin(&arena, plugin.declaration(), arena.current_tick());
        let ctx = PluginContext {
            entity_id: ship_id,
            tick: arena.current_tick(),
            trace_id: TraceId::new(0),
        };

        let outputs = plugin.run(&ctx, &view);
        assert!(outputs.is_empty());
    }

    #[test]
    fn run_returns_empty_for_squadron() {
        let plugin = MovementPlugin::new();
        let mut arena = Arena::new();

        let squadron_id = arena.spawn(
            EntityTag::Squadron,
            EntityInner::Squadron(SquadronComponents::at_position(Vec2::new(100.0, 200.0), 0.0)),
        );

        let view = WorldView::for_plugin(&arena, plugin.declaration(), arena.current_tick());
        let ctx = PluginContext {
            entity_id: squadron_id,
            tick: arena.current_tick(),
            trace_id: TraceId::new(0),
        };

        let outputs = plugin.run(&ctx, &view);
        assert!(outputs.is_empty());
    }

    #[test]
    fn plugin_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MovementPlugin>();
    }

    #[test]
    fn run_with_nonexistent_entity() {
        let plugin = MovementPlugin::new();
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
}
