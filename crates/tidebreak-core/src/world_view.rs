//! `WorldView` provides scoped, read-only access to arena state for plugins.
//!
//! The [`WorldView`] is the primary mechanism for plugins to read game state.
//! It enforces component-level access control based on what the plugin declared
//! in its [`PluginDeclaration`](crate::plugin::PluginDeclaration).
//!
//! # Access Control
//!
//! Plugins must declare which components they read in their declaration. The
//! `WorldView` enforces this at runtime:
//! - In debug builds, accessing an undeclared component panics
//! - In release builds, it returns `None`
//!
//! This helps catch plugin bugs early and enforces the principle that plugins
//! should only access what they need.
//!
//! # Immutability
//!
//! `WorldView` provides only immutable access to the arena. This ensures that:
//! - Plugins cannot directly mutate state (must emit outputs instead)
//! - Multiple plugins can run in parallel safely
//! - The snapshot semantics of the execution loop are maintained
//!
//! # Example
//!
//! ```
//! use tidebreak_core::arena::Arena;
//! use tidebreak_core::entity::{EntityTag, EntityInner, ShipComponents};
//! use tidebreak_core::plugin::{PluginDeclaration, PluginId, ComponentKind};
//! use tidebreak_core::output::OutputKind;
//! use tidebreak_core::world_view::WorldView;
//! use glam::Vec2;
//!
//! // Create an arena with a ship
//! let mut arena = Arena::new();
//! let ship_id = arena.spawn(
//!     EntityTag::Ship,
//!     EntityInner::Ship(ShipComponents::at_position(Vec2::new(100.0, 200.0), 0.5)),
//! );
//!
//! // Create a plugin declaration that reads Transform
//! let decl = PluginDeclaration {
//!     id: PluginId::new("test"),
//!     required_tags: vec![EntityTag::Ship],
//!     reads: vec![ComponentKind::Transform],
//!     emits: vec![OutputKind::Command],
//! };
//!
//! // Create a scoped WorldView
//! let view = WorldView::for_plugin(&arena, &decl, arena.current_tick());
//!
//! // Can access transform (declared)
//! let transform = view.get_transform(ship_id);
//! assert!(transform.is_some());
//! assert_eq!(transform.unwrap().position, Vec2::new(100.0, 200.0));
//!
//! // Cannot access physics (not declared) - returns None
//! // In debug builds this would panic!
//! ```

use glam::Vec2;

use crate::arena::Arena;
use crate::entity::components::{
    CombatState, InventoryState, PhysicsState, SensorState, TransformState,
};
use crate::entity::{Entity, EntityId, EntityInner, EntityTag};
use crate::plugin::{ComponentKind, PluginDeclaration};

// =============================================================================
// WorldView
// =============================================================================

/// Scoped, read-only view of the arena for plugin access.
///
/// The `WorldView` wraps an immutable reference to the Arena and enforces
/// component-level access control based on the plugin's declaration.
///
/// # Lifetime
///
/// The `WorldView` borrows the Arena for the duration of plugin execution.
/// The `'a` lifetime parameter ties the view to the arena's lifetime.
///
/// # Component Access
///
/// Each `get_*` method checks permissions before returning the component:
/// - If the component kind is in `allowed_components`, access is granted
/// - Otherwise, in debug builds it panics; in release builds it returns `None`
///
/// # Entity Access
///
/// `get_entity()` is always allowed - plugins may need to inspect entity metadata
/// like ID and tag regardless of which components they read.
///
/// # Spatial Queries
///
/// `query_in_radius()` uses the arena's spatial index. This is always allowed
/// since it only returns entity IDs, not component data.
#[derive(Debug)]
pub struct WorldView<'a> {
    /// Reference to the arena being viewed.
    arena: &'a Arena,
    /// Current simulation tick.
    tick: u64,
    /// Component kinds this view is allowed to access.
    allowed_components: &'a [ComponentKind],
}

impl<'a> WorldView<'a> {
    /// Creates a `WorldView` scoped to a plugin's declared component access.
    ///
    /// # Arguments
    ///
    /// * `arena` - The arena to view
    /// * `decl` - The plugin declaration (determines allowed components)
    /// * `tick` - The current simulation tick
    ///
    /// # Returns
    ///
    /// A `WorldView` that only allows access to components declared in `decl.reads`.
    #[must_use]
    pub fn for_plugin(arena: &'a Arena, decl: &'a PluginDeclaration, tick: u64) -> Self {
        Self {
            arena,
            tick,
            allowed_components: &decl.reads,
        }
    }

    /// Creates a `WorldView` with full access to all components.
    ///
    /// This is primarily useful for testing or for system-level code that
    /// needs unrestricted access.
    ///
    /// # Arguments
    ///
    /// * `arena` - The arena to view
    /// * `tick` - The current simulation tick
    #[must_use]
    pub fn full_access(arena: &'a Arena, tick: u64) -> Self {
        // Use a static slice with all component kinds
        static ALL_COMPONENTS: &[ComponentKind] = &[
            ComponentKind::Transform,
            ComponentKind::Physics,
            ComponentKind::Combat,
            ComponentKind::Sensor,
            ComponentKind::Inventory,
        ];

        Self {
            arena,
            tick,
            allowed_components: ALL_COMPONENTS,
        }
    }

    /// Returns the current simulation tick.
    #[must_use]
    pub const fn tick(&self) -> u64 {
        self.tick
    }

    /// Returns a reference to an entity by ID.
    ///
    /// Entity access is always allowed - plugins may need to inspect entity
    /// metadata regardless of their component access declarations.
    ///
    /// # Arguments
    ///
    /// * `id` - The entity ID to look up
    ///
    /// # Returns
    ///
    /// The entity if it exists, `None` otherwise.
    #[must_use]
    pub fn get_entity(&self, id: EntityId) -> Option<&'a Entity> {
        self.arena.get(id)
    }

    /// Returns a reference to an entity's transform state.
    ///
    /// # Access Control
    ///
    /// Requires `ComponentKind::Transform` in the plugin declaration.
    /// Panics in debug builds if access is denied.
    ///
    /// # Arguments
    ///
    /// * `id` - The entity ID to look up
    ///
    /// # Returns
    ///
    /// The transform state if the entity exists and has this component.
    #[must_use]
    pub fn get_transform(&self, id: EntityId) -> Option<&'a TransformState> {
        self.check_access(ComponentKind::Transform)?;
        let entity = self.arena.get(id)?;
        Self::extract_transform(entity)
    }

    /// Returns a reference to an entity's physics state.
    ///
    /// # Access Control
    ///
    /// Requires `ComponentKind::Physics` in the plugin declaration.
    /// Panics in debug builds if access is denied.
    ///
    /// # Arguments
    ///
    /// * `id` - The entity ID to look up
    ///
    /// # Returns
    ///
    /// The physics state if the entity exists and has this component.
    #[must_use]
    pub fn get_physics(&self, id: EntityId) -> Option<&'a PhysicsState> {
        self.check_access(ComponentKind::Physics)?;
        let entity = self.arena.get(id)?;
        Self::extract_physics(entity)
    }

    /// Returns a reference to an entity's combat state.
    ///
    /// # Access Control
    ///
    /// Requires `ComponentKind::Combat` in the plugin declaration.
    /// Panics in debug builds if access is denied.
    ///
    /// # Arguments
    ///
    /// * `id` - The entity ID to look up
    ///
    /// # Returns
    ///
    /// The combat state if the entity exists and has this component.
    #[must_use]
    pub fn get_combat(&self, id: EntityId) -> Option<&'a CombatState> {
        self.check_access(ComponentKind::Combat)?;
        let entity = self.arena.get(id)?;
        Self::extract_combat(entity)
    }

    /// Returns a reference to an entity's sensor state.
    ///
    /// # Access Control
    ///
    /// Requires `ComponentKind::Sensor` in the plugin declaration.
    /// Panics in debug builds if access is denied.
    ///
    /// # Arguments
    ///
    /// * `id` - The entity ID to look up
    ///
    /// # Returns
    ///
    /// The sensor state if the entity exists and has this component.
    #[must_use]
    pub fn get_sensor(&self, id: EntityId) -> Option<&'a SensorState> {
        self.check_access(ComponentKind::Sensor)?;
        let entity = self.arena.get(id)?;
        Self::extract_sensor(entity)
    }

    /// Returns a reference to an entity's inventory state.
    ///
    /// # Access Control
    ///
    /// Requires `ComponentKind::Inventory` in the plugin declaration.
    /// Panics in debug builds if access is denied.
    ///
    /// # Arguments
    ///
    /// * `id` - The entity ID to look up
    ///
    /// # Returns
    ///
    /// The inventory state if the entity exists and has this component.
    #[must_use]
    pub fn get_inventory(&self, id: EntityId) -> Option<&'a InventoryState> {
        self.check_access(ComponentKind::Inventory)?;
        let entity = self.arena.get(id)?;
        Self::extract_inventory(entity)
    }

    /// Queries for entities within a radius of a center point.
    ///
    /// This is always allowed since it only returns entity IDs, not component data.
    /// The results are sorted by entity ID for deterministic ordering.
    ///
    /// # Arguments
    ///
    /// * `center` - The center point of the query
    /// * `radius` - The search radius in world units
    ///
    /// # Returns
    ///
    /// A vector of entity IDs within the radius, sorted by ID.
    #[must_use]
    pub fn query_in_radius(&self, center: Vec2, radius: f32) -> Vec<EntityId> {
        self.arena.spatial().query_radius(center, radius)
    }

    /// Queries for entities with a specific tag.
    ///
    /// This iterates through all entities and filters by tag. The results
    /// are in deterministic order (sorted by entity ID).
    ///
    /// # Arguments
    ///
    /// * `tag` - The entity tag to filter by
    ///
    /// # Returns
    ///
    /// An iterator over entity IDs matching the tag.
    pub fn query_by_tag(&self, tag: EntityTag) -> impl Iterator<Item = EntityId> + 'a {
        self.arena
            .entities_sorted()
            .filter(move |e| e.tag() == tag)
            .map(Entity::id)
    }

    /// Checks if access to a component kind is allowed.
    ///
    /// In debug builds, panics if access is denied.
    /// In release builds, returns `None` if access is denied.
    ///
    /// # Arguments
    ///
    /// * `kind` - The component kind to check
    ///
    /// # Returns
    ///
    /// `Some(())` if access is allowed, `None` if denied.
    #[allow(clippy::unnecessary_wraps)]
    fn check_access(&self, kind: ComponentKind) -> Option<()> {
        if self.allowed_components.contains(&kind) {
            Some(())
        } else {
            #[cfg(debug_assertions)]
            panic!(
                "WorldView access denied: plugin tried to access {:?} but only declared: {:?}",
                kind, self.allowed_components
            );

            #[cfg(not(debug_assertions))]
            None
        }
    }

    // =========================================================================
    // Component Extraction Helpers
    // =========================================================================

    /// Extracts transform from any entity type.
    ///
    /// Note: Currently all entity types have a transform, so this always returns `Some`.
    /// The `Option` return type maintains API consistency with other extract methods
    /// and allows for future entity types that might not have a transform.
    #[allow(clippy::unnecessary_wraps)]
    fn extract_transform(entity: &Entity) -> Option<&TransformState> {
        match entity.inner() {
            EntityInner::Ship(c) => Some(&c.transform),
            EntityInner::Platform(c) => Some(&c.transform),
            EntityInner::Projectile(c) => Some(&c.transform),
            EntityInner::Squadron(c) => Some(&c.transform),
        }
    }

    /// Extracts physics from entity types that have it.
    fn extract_physics(entity: &Entity) -> Option<&PhysicsState> {
        match entity.inner() {
            EntityInner::Ship(c) => Some(&c.physics),
            EntityInner::Projectile(c) => Some(&c.physics),
            EntityInner::Squadron(c) => Some(&c.physics),
            EntityInner::Platform(_) => None, // Platforms don't have physics
        }
    }

    /// Extracts combat from entity types that have it.
    fn extract_combat(entity: &Entity) -> Option<&CombatState> {
        match entity.inner() {
            EntityInner::Ship(c) => Some(&c.combat),
            EntityInner::Squadron(c) => Some(&c.combat),
            EntityInner::Platform(_) | EntityInner::Projectile(_) => None,
        }
    }

    /// Extracts sensor from entity types that have it.
    fn extract_sensor(entity: &Entity) -> Option<&SensorState> {
        match entity.inner() {
            EntityInner::Ship(c) => Some(&c.sensor),
            EntityInner::Platform(c) => Some(&c.sensor),
            EntityInner::Projectile(_) | EntityInner::Squadron(_) => None,
        }
    }

    /// Extracts inventory from entity types that have it.
    fn extract_inventory(entity: &Entity) -> Option<&InventoryState> {
        match entity.inner() {
            EntityInner::Ship(c) => Some(&c.inventory),
            EntityInner::Platform(_) | EntityInner::Projectile(_) | EntityInner::Squadron(_) => {
                None
            }
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{
        PlatformComponents, ProjectileComponents, ShipComponents, SquadronComponents,
    };
    use crate::output::{OutputKind, PluginId};

    // Helper to create a test arena with various entities
    fn create_test_arena() -> Arena {
        let mut arena = Arena::new();

        // Ship at origin
        arena.spawn(
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::at_position(Vec2::new(0.0, 0.0), 0.0)),
        );

        // Platform at (100, 0)
        arena.spawn(
            EntityTag::Platform,
            EntityInner::Platform(PlatformComponents::at_position(Vec2::new(100.0, 0.0))),
        );

        // Projectile at (200, 0)
        arena.spawn(
            EntityTag::Projectile,
            EntityInner::Projectile(ProjectileComponents::at_position_with_velocity(
                Vec2::new(200.0, 0.0),
                0.0,
                Vec2::new(100.0, 0.0),
            )),
        );

        // Squadron at (300, 0)
        arena.spawn(
            EntityTag::Squadron,
            EntityInner::Squadron(SquadronComponents::at_position(Vec2::new(300.0, 0.0), 0.0)),
        );

        arena
    }

    // Helper to create a declaration that reads only specific components
    fn make_declaration(reads: Vec<ComponentKind>) -> PluginDeclaration {
        PluginDeclaration {
            id: PluginId::new("test"),
            required_tags: vec![EntityTag::Ship],
            reads,
            emits: vec![OutputKind::Command],
        }
    }

    mod world_view_creation_tests {
        use super::*;

        #[test]
        fn for_plugin_creates_scoped_view() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![ComponentKind::Transform]);
            let view = WorldView::for_plugin(&arena, &decl, 42);

            assert_eq!(view.tick(), 42);
        }

        #[test]
        fn full_access_allows_all_components() {
            let arena = create_test_arena();
            let view = WorldView::full_access(&arena, 100);
            let ship_id = EntityId::new(0);

            // All components should be accessible
            assert!(view.get_transform(ship_id).is_some());
            assert!(view.get_physics(ship_id).is_some());
            assert!(view.get_combat(ship_id).is_some());
            assert!(view.get_sensor(ship_id).is_some());
            assert!(view.get_inventory(ship_id).is_some());
        }

        #[test]
        fn tick_returns_correct_value() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![]);
            let view = WorldView::for_plugin(&arena, &decl, 999);
            assert_eq!(view.tick(), 999);
        }
    }

    mod entity_access_tests {
        use super::*;

        #[test]
        fn get_entity_always_allowed() {
            let arena = create_test_arena();
            // Even with no reads declared, entity access works
            let decl = make_declaration(vec![]);
            let view = WorldView::for_plugin(&arena, &decl, 0);

            let entity = view.get_entity(EntityId::new(0));
            assert!(entity.is_some());
            assert!(entity.unwrap().is_ship());
        }

        #[test]
        fn get_entity_nonexistent_returns_none() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![]);
            let view = WorldView::for_plugin(&arena, &decl, 0);

            assert!(view.get_entity(EntityId::new(999)).is_none());
        }

        #[test]
        fn get_entity_all_types() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![]);
            let view = WorldView::for_plugin(&arena, &decl, 0);

            assert!(view.get_entity(EntityId::new(0)).unwrap().is_ship());
            assert!(view.get_entity(EntityId::new(1)).unwrap().is_platform());
            assert!(view.get_entity(EntityId::new(2)).unwrap().is_projectile());
            assert!(view.get_entity(EntityId::new(3)).unwrap().is_squadron());
        }
    }

    mod transform_access_tests {
        use super::*;

        #[test]
        fn get_transform_with_permission() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![ComponentKind::Transform]);
            let view = WorldView::for_plugin(&arena, &decl, 0);

            // Ship
            let transform = view.get_transform(EntityId::new(0));
            assert!(transform.is_some());
            assert_eq!(transform.unwrap().position, Vec2::new(0.0, 0.0));

            // Platform
            let transform = view.get_transform(EntityId::new(1));
            assert!(transform.is_some());
            assert_eq!(transform.unwrap().position, Vec2::new(100.0, 0.0));

            // Projectile
            let transform = view.get_transform(EntityId::new(2));
            assert!(transform.is_some());
            assert_eq!(transform.unwrap().position, Vec2::new(200.0, 0.0));

            // Squadron
            let transform = view.get_transform(EntityId::new(3));
            assert!(transform.is_some());
            assert_eq!(transform.unwrap().position, Vec2::new(300.0, 0.0));
        }

        #[test]
        fn get_transform_nonexistent_entity() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![ComponentKind::Transform]);
            let view = WorldView::for_plugin(&arena, &decl, 0);

            assert!(view.get_transform(EntityId::new(999)).is_none());
        }

        #[test]
        #[should_panic(expected = "access denied")]
        #[cfg(debug_assertions)]
        fn get_transform_without_permission_panics_debug() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![]); // No transform access
            let view = WorldView::for_plugin(&arena, &decl, 0);

            // This should panic in debug mode
            let _ = view.get_transform(EntityId::new(0));
        }
    }

    mod physics_access_tests {
        use super::*;

        #[test]
        fn get_physics_with_permission() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![ComponentKind::Physics]);
            let view = WorldView::for_plugin(&arena, &decl, 0);

            // Ship has physics
            assert!(view.get_physics(EntityId::new(0)).is_some());

            // Projectile has physics
            assert!(view.get_physics(EntityId::new(2)).is_some());

            // Squadron has physics
            assert!(view.get_physics(EntityId::new(3)).is_some());
        }

        #[test]
        fn get_physics_platform_returns_none() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![ComponentKind::Physics]);
            let view = WorldView::for_plugin(&arena, &decl, 0);

            // Platform doesn't have physics
            assert!(view.get_physics(EntityId::new(1)).is_none());
        }

        #[test]
        #[should_panic(expected = "access denied")]
        #[cfg(debug_assertions)]
        fn get_physics_without_permission_panics_debug() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![]); // No physics access
            let view = WorldView::for_plugin(&arena, &decl, 0);

            let _ = view.get_physics(EntityId::new(0));
        }
    }

    mod combat_access_tests {
        use super::*;

        #[test]
        fn get_combat_with_permission() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![ComponentKind::Combat]);
            let view = WorldView::for_plugin(&arena, &decl, 0);

            // Ship has combat
            assert!(view.get_combat(EntityId::new(0)).is_some());

            // Squadron has combat
            assert!(view.get_combat(EntityId::new(3)).is_some());
        }

        #[test]
        fn get_combat_platform_projectile_return_none() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![ComponentKind::Combat]);
            let view = WorldView::for_plugin(&arena, &decl, 0);

            // Platform doesn't have combat
            assert!(view.get_combat(EntityId::new(1)).is_none());

            // Projectile doesn't have combat
            assert!(view.get_combat(EntityId::new(2)).is_none());
        }

        #[test]
        #[should_panic(expected = "access denied")]
        #[cfg(debug_assertions)]
        fn get_combat_without_permission_panics_debug() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![]); // No combat access
            let view = WorldView::for_plugin(&arena, &decl, 0);

            let _ = view.get_combat(EntityId::new(0));
        }
    }

    mod sensor_access_tests {
        use super::*;

        #[test]
        fn get_sensor_with_permission() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![ComponentKind::Sensor]);
            let view = WorldView::for_plugin(&arena, &decl, 0);

            // Ship has sensor
            assert!(view.get_sensor(EntityId::new(0)).is_some());

            // Platform has sensor
            assert!(view.get_sensor(EntityId::new(1)).is_some());
        }

        #[test]
        fn get_sensor_projectile_squadron_return_none() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![ComponentKind::Sensor]);
            let view = WorldView::for_plugin(&arena, &decl, 0);

            // Projectile doesn't have sensor
            assert!(view.get_sensor(EntityId::new(2)).is_none());

            // Squadron doesn't have sensor
            assert!(view.get_sensor(EntityId::new(3)).is_none());
        }

        #[test]
        #[should_panic(expected = "access denied")]
        #[cfg(debug_assertions)]
        fn get_sensor_without_permission_panics_debug() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![]); // No sensor access
            let view = WorldView::for_plugin(&arena, &decl, 0);

            let _ = view.get_sensor(EntityId::new(0));
        }
    }

    mod inventory_access_tests {
        use super::*;

        #[test]
        fn get_inventory_with_permission() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![ComponentKind::Inventory]);
            let view = WorldView::for_plugin(&arena, &decl, 0);

            // Ship has inventory
            assert!(view.get_inventory(EntityId::new(0)).is_some());
        }

        #[test]
        fn get_inventory_other_types_return_none() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![ComponentKind::Inventory]);
            let view = WorldView::for_plugin(&arena, &decl, 0);

            // Platform doesn't have inventory
            assert!(view.get_inventory(EntityId::new(1)).is_none());

            // Projectile doesn't have inventory
            assert!(view.get_inventory(EntityId::new(2)).is_none());

            // Squadron doesn't have inventory
            assert!(view.get_inventory(EntityId::new(3)).is_none());
        }

        #[test]
        #[should_panic(expected = "access denied")]
        #[cfg(debug_assertions)]
        fn get_inventory_without_permission_panics_debug() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![]); // No inventory access
            let view = WorldView::for_plugin(&arena, &decl, 0);

            let _ = view.get_inventory(EntityId::new(0));
        }
    }

    mod spatial_query_tests {
        use super::*;

        #[test]
        fn query_in_radius_finds_nearby() {
            let arena = create_test_arena();
            // Spatial queries don't need any reads declared
            let decl = make_declaration(vec![]);
            let view = WorldView::for_plugin(&arena, &decl, 0);

            // Query near origin with radius 50 - should find ship only
            let nearby = view.query_in_radius(Vec2::ZERO, 50.0);
            assert_eq!(nearby.len(), 1);
            assert!(nearby.contains(&EntityId::new(0)));
        }

        #[test]
        fn query_in_radius_large_radius() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![]);
            let view = WorldView::for_plugin(&arena, &decl, 0);

            // Query with large radius - should find all entities
            let nearby = view.query_in_radius(Vec2::new(150.0, 0.0), 500.0);
            assert_eq!(nearby.len(), 4);
        }

        #[test]
        fn query_in_radius_returns_sorted() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![]);
            let view = WorldView::for_plugin(&arena, &decl, 0);

            let nearby = view.query_in_radius(Vec2::new(150.0, 0.0), 500.0);

            // Results should be sorted by ID
            assert_eq!(
                nearby,
                vec![
                    EntityId::new(0),
                    EntityId::new(1),
                    EntityId::new(2),
                    EntityId::new(3)
                ]
            );
        }

        #[test]
        fn query_in_radius_empty_result() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![]);
            let view = WorldView::for_plugin(&arena, &decl, 0);

            // Query far from all entities
            let nearby = view.query_in_radius(Vec2::new(10000.0, 10000.0), 10.0);
            assert!(nearby.is_empty());
        }
    }

    mod query_by_tag_tests {
        use super::*;

        #[test]
        fn query_by_tag_finds_matching() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![]);
            let view = WorldView::for_plugin(&arena, &decl, 0);

            let ships: Vec<_> = view.query_by_tag(EntityTag::Ship).collect();
            assert_eq!(ships.len(), 1);
            assert_eq!(ships[0], EntityId::new(0));

            let platforms: Vec<_> = view.query_by_tag(EntityTag::Platform).collect();
            assert_eq!(platforms.len(), 1);
            assert_eq!(platforms[0], EntityId::new(1));
        }

        #[test]
        fn query_by_tag_multiple_matches() {
            let mut arena = Arena::new();

            // Spawn multiple ships
            arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );
            arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );
            arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );
            arena.spawn(
                EntityTag::Platform,
                EntityInner::Platform(PlatformComponents::default()),
            );

            let decl = make_declaration(vec![]);
            let view = WorldView::for_plugin(&arena, &decl, 0);

            let ships: Vec<_> = view.query_by_tag(EntityTag::Ship).collect();
            assert_eq!(ships.len(), 3);
            // Should be sorted by ID
            assert_eq!(
                ships,
                vec![EntityId::new(0), EntityId::new(1), EntityId::new(2)]
            );
        }

        #[test]
        fn query_by_tag_no_matches() {
            let mut arena = Arena::new();
            arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            let decl = make_declaration(vec![]);
            let view = WorldView::for_plugin(&arena, &decl, 0);

            let projectiles: Vec<_> = view.query_by_tag(EntityTag::Projectile).collect();
            assert!(projectiles.is_empty());
        }
    }

    mod multiple_components_tests {
        use super::*;

        #[test]
        fn access_multiple_declared_components() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![
                ComponentKind::Transform,
                ComponentKind::Physics,
                ComponentKind::Combat,
            ]);
            let view = WorldView::for_plugin(&arena, &decl, 0);

            let ship_id = EntityId::new(0);

            // All declared components accessible
            assert!(view.get_transform(ship_id).is_some());
            assert!(view.get_physics(ship_id).is_some());
            assert!(view.get_combat(ship_id).is_some());
        }

        #[test]
        #[should_panic(expected = "access denied")]
        #[cfg(debug_assertions)]
        fn cannot_access_undeclared_with_multiple_declared() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![
                ComponentKind::Transform,
                ComponentKind::Physics,
                // Combat NOT declared
            ]);
            let view = WorldView::for_plugin(&arena, &decl, 0);

            // Should panic - Combat not in reads
            let _ = view.get_combat(EntityId::new(0));
        }
    }

    mod component_availability_matrix_tests {
        use super::*;

        // Test the component availability matrix across entity types
        //
        // Component   | Ship | Platform | Projectile | Squadron
        // ------------|------|----------|------------|----------
        // Transform   |  Y   |    Y     |     Y      |    Y
        // Physics     |  Y   |    N     |     Y      |    Y
        // Combat      |  Y   |    N     |     N      |    Y
        // Sensor      |  Y   |    Y     |     N      |    N
        // Inventory   |  Y   |    N     |     N      |    N

        #[test]
        fn ship_has_all_components() {
            let arena = create_test_arena();
            let view = WorldView::full_access(&arena, 0);
            let id = EntityId::new(0); // Ship

            assert!(view.get_transform(id).is_some());
            assert!(view.get_physics(id).is_some());
            assert!(view.get_combat(id).is_some());
            assert!(view.get_sensor(id).is_some());
            assert!(view.get_inventory(id).is_some());
        }

        #[test]
        fn platform_components() {
            let arena = create_test_arena();
            let view = WorldView::full_access(&arena, 0);
            let id = EntityId::new(1); // Platform

            assert!(view.get_transform(id).is_some());
            assert!(view.get_physics(id).is_none());
            assert!(view.get_combat(id).is_none());
            assert!(view.get_sensor(id).is_some());
            assert!(view.get_inventory(id).is_none());
        }

        #[test]
        fn projectile_components() {
            let arena = create_test_arena();
            let view = WorldView::full_access(&arena, 0);
            let id = EntityId::new(2); // Projectile

            assert!(view.get_transform(id).is_some());
            assert!(view.get_physics(id).is_some());
            assert!(view.get_combat(id).is_none());
            assert!(view.get_sensor(id).is_none());
            assert!(view.get_inventory(id).is_none());
        }

        #[test]
        fn squadron_components() {
            let arena = create_test_arena();
            let view = WorldView::full_access(&arena, 0);
            let id = EntityId::new(3); // Squadron

            assert!(view.get_transform(id).is_some());
            assert!(view.get_physics(id).is_some());
            assert!(view.get_combat(id).is_some());
            assert!(view.get_sensor(id).is_none());
            assert!(view.get_inventory(id).is_none());
        }
    }

    mod debug_format_tests {
        use super::*;

        #[test]
        fn world_view_debug_format() {
            let arena = create_test_arena();
            let decl = make_declaration(vec![ComponentKind::Transform]);
            let view = WorldView::for_plugin(&arena, &decl, 42);

            let debug = format!("{:?}", view);
            assert!(debug.contains("WorldView"));
            assert!(debug.contains("tick"));
            assert!(debug.contains("42"));
        }
    }
}
