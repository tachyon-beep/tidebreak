//! Projectile plugin for in-flight weapon behavior.
//!
//! The `ProjectilePlugin` handles projectile movement and guidance.
//! In MVP, projectiles maintain their current velocity. Future versions
//! will implement homing and target tracking.
//!
//! # Supported Entity Types
//!
//! - Projectiles
//!
//! # Outputs
//!
//! Currently emits no outputs (projectiles maintain their velocity).
//! Future versions will emit `SetVelocity` commands for homing.

use crate::entity::EntityTag;
use crate::output::{Output, OutputKind, PluginId};
use crate::plugin::{ComponentKind, Plugin, PluginContext, PluginDeclaration};
use crate::world_view::WorldView;

/// Plugin that handles projectile behavior.
///
/// For MVP, projectiles maintain their current velocity.
/// Later versions will implement homing and target tracking.
///
/// # Example
///
/// ```
/// use tidebreak_core::plugins::ProjectilePlugin;
/// use tidebreak_core::plugin::Plugin;
///
/// let plugin = ProjectilePlugin::new();
/// assert_eq!(plugin.declaration().id.as_str(), "projectile");
/// ```
pub struct ProjectilePlugin {
    declaration: PluginDeclaration,
}

impl ProjectilePlugin {
    /// Creates a new `ProjectilePlugin`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            declaration: PluginDeclaration {
                id: PluginId::from_static("projectile"),
                required_tags: vec![EntityTag::Projectile],
                reads: vec![ComponentKind::Transform, ComponentKind::Physics],
                emits: vec![OutputKind::Command],
            },
        }
    }
}

impl Default for ProjectilePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ProjectilePlugin {
    fn declaration(&self) -> &PluginDeclaration {
        &self.declaration
    }

    fn run(&self, _ctx: &PluginContext, _view: &WorldView) -> Vec<Output> {
        // For MVP: projectiles just maintain current velocity
        // Later: implement homing toward target
        vec![]
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::Arena;
    use crate::entity::{EntityId, EntityInner, ProjectileComponents};
    use crate::output::TraceId;
    use glam::Vec2;

    #[test]
    fn new_creates_plugin() {
        let plugin = ProjectilePlugin::new();
        assert_eq!(plugin.declaration().id.as_str(), "projectile");
    }

    #[test]
    fn default_creates_plugin() {
        let plugin = ProjectilePlugin::default();
        assert_eq!(plugin.declaration().id.as_str(), "projectile");
    }

    #[test]
    fn declaration_has_correct_tags() {
        let plugin = ProjectilePlugin::new();
        let decl = plugin.declaration();

        assert!(decl.required_tags.contains(&EntityTag::Projectile));
        assert!(!decl.required_tags.contains(&EntityTag::Ship));
        assert!(!decl.required_tags.contains(&EntityTag::Platform));
        assert!(!decl.required_tags.contains(&EntityTag::Squadron));
    }

    #[test]
    fn declaration_reads_transform_and_physics() {
        let plugin = ProjectilePlugin::new();
        let decl = plugin.declaration();

        assert!(decl.reads.contains(&ComponentKind::Transform));
        assert!(decl.reads.contains(&ComponentKind::Physics));
    }

    #[test]
    fn declaration_emits_commands() {
        let plugin = ProjectilePlugin::new();
        let decl = plugin.declaration();

        assert!(decl.emits.contains(&OutputKind::Command));
    }

    #[test]
    fn run_returns_empty() {
        let plugin = ProjectilePlugin::new();
        let mut arena = Arena::new();

        let projectile_id = arena.spawn(
            EntityTag::Projectile,
            EntityInner::Projectile(ProjectileComponents::at_position_with_velocity(
                Vec2::new(100.0, 200.0),
                0.0,
                Vec2::new(500.0, 0.0),
            )),
        );

        let view = WorldView::for_plugin(&arena, plugin.declaration(), arena.current_tick());
        let ctx = PluginContext {
            entity_id: projectile_id,
            tick: arena.current_tick(),
            trace_id: TraceId::new(0),
        };

        let outputs = plugin.run(&ctx, &view);
        assert!(outputs.is_empty());
    }

    #[test]
    fn run_with_nonexistent_entity() {
        let plugin = ProjectilePlugin::new();
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
        assert_send_sync::<ProjectilePlugin>();
    }
}
