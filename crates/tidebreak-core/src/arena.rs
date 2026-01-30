//! Arena module for the combat simulation.
//!
//! The Arena is the container for all entities in a combat simulation. It provides:
//! - Entity storage with deterministic iteration order (`BTreeMap`)
//! - Spatial indexing for proximity queries
//! - Entity lifecycle management (spawn/despawn)
//! - Trace ID generation for causal chain tracking
//!
//! # Architecture
//!
//! The Arena uses a `BTreeMap` for entity storage to ensure deterministic iteration
//! order (required by ADR-0003). Entity IDs are monotonically increasing, and the
//! `BTreeMap`'s natural ordering guarantees consistent iteration across platforms.
//!
//! # Spatial Index Synchronization
//!
//! **Important**: The spatial index is NOT automatically synchronized when entity
//! positions change. This is an intentional design choice for performance:
//!
//! - When modifying entity position via `get_mut()`, you **must** call
//!   `update_spatial(id)` afterward to sync the index.
//! - This allows batch updates: modify many entities first, then sync all
//!   spatial indices at once before queries.
//! - Spawning and despawning automatically update the spatial index.
//!
//! ```
//! # use tidebreak_core::arena::Arena;
//! # use tidebreak_core::entity::{EntityTag, EntityInner, ShipComponents};
//! # use glam::Vec2;
//! # let mut arena = Arena::new();
//! # let ship_id = arena.spawn(EntityTag::Ship, EntityInner::Ship(ShipComponents::default()));
//! // After modifying position:
//! if let Some(entity) = arena.get_mut(ship_id) {
//!     if let Some(ship) = entity.as_ship_mut() {
//!         ship.transform.position = Vec2::new(500.0, 500.0);
//!     }
//! }
//! // REQUIRED: sync spatial index after position change
//! arena.update_spatial(ship_id);
//! ```
//!
//! # Example
//!
//! ```
//! use tidebreak_core::arena::Arena;
//! use tidebreak_core::entity::{EntityTag, EntityInner, ShipComponents};
//! use glam::Vec2;
//!
//! let mut arena = Arena::new();
//!
//! // Spawn a ship at position (100, 200)
//! let components = ShipComponents::at_position(Vec2::new(100.0, 200.0), 0.0);
//! let ship_id = arena.spawn(EntityTag::Ship, EntityInner::Ship(components));
//!
//! // Query entities near the ship
//! let nearby = arena.spatial().query_radius(Vec2::new(100.0, 200.0), 50.0);
//! assert!(nearby.contains(&ship_id));
//! ```

use std::collections::{BTreeMap, HashMap};

use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::entity::{Entity, EntityId, EntityInner, EntityTag};
use crate::output::TraceId;

// =============================================================================
// Spatial Index
// =============================================================================

/// Simple spatial index for proximity queries.
///
/// This MVP implementation uses a `HashMap` for position storage. While not
/// optimal for large numbers of entities, it provides correct behavior for
/// early development.
///
/// # Note on `HashMap` Usage
///
/// `HashMap` is acceptable here because we only query by known entity IDs or
/// perform full scans for radius queries. The non-deterministic iteration
/// order of `HashMap` doesn't affect correctness since we're not iterating
/// over it in a way that affects simulation state.
///
/// # Future Improvements
///
/// For production, consider:
/// - Spatial hashing (grid-based)
/// - Quadtree/octree for dynamic entities
/// - R-tree for complex spatial queries
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpatialIndex {
    /// Entity positions indexed by ID.
    positions: HashMap<EntityId, Vec2>,
}

impl SpatialIndex {
    /// Creates a new empty spatial index.
    #[must_use]
    pub fn new() -> Self {
        Self {
            positions: HashMap::new(),
        }
    }

    /// Inserts or updates an entity's position in the index.
    ///
    /// # Arguments
    ///
    /// * `id` - The entity ID
    /// * `pos` - The entity's position
    pub fn insert(&mut self, id: EntityId, pos: Vec2) {
        self.positions.insert(id, pos);
    }

    /// Removes an entity from the spatial index.
    ///
    /// # Arguments
    ///
    /// * `id` - The entity ID to remove
    pub fn remove(&mut self, id: EntityId) {
        self.positions.remove(&id);
    }

    /// Returns the position of an entity, if known.
    ///
    /// # Arguments
    ///
    /// * `id` - The entity ID to look up
    #[must_use]
    pub fn get(&self, id: EntityId) -> Option<Vec2> {
        self.positions.get(&id).copied()
    }

    /// Queries for entities within a radius of a center point.
    ///
    /// Returns entity IDs in a deterministic order (sorted by ID) for
    /// consistent simulation behavior.
    ///
    /// # Arguments
    ///
    /// * `center` - The center point of the query
    /// * `radius` - The search radius
    ///
    /// # Returns
    ///
    /// A vector of entity IDs within the radius, sorted by ID.
    #[must_use]
    pub fn query_radius(&self, center: Vec2, radius: f32) -> Vec<EntityId> {
        let radius_sq = radius * radius;
        let mut results: Vec<EntityId> = self
            .positions
            .iter()
            .filter(|(_, pos)| center.distance_squared(**pos) <= radius_sq)
            .map(|(id, _)| *id)
            .collect();

        // Sort for deterministic order
        results.sort();
        results
    }

    /// Returns the number of entities in the spatial index.
    #[must_use]
    pub fn len(&self) -> usize {
        self.positions.len()
    }

    /// Returns true if the spatial index is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    /// Updates the position of an entity if it exists in the index.
    ///
    /// Returns true if the entity was found and updated.
    pub fn update(&mut self, id: EntityId, pos: Vec2) -> bool {
        use std::collections::hash_map::Entry;
        match self.positions.entry(id) {
            Entry::Occupied(mut entry) => {
                entry.insert(pos);
                true
            }
            Entry::Vacant(_) => false,
        }
    }
}

// =============================================================================
// Arena
// =============================================================================

/// Combat arena containing all simulation entities.
///
/// The Arena is the central container for a combat simulation. It manages:
/// - Entity storage with deterministic iteration order
/// - Spatial indexing for proximity queries
/// - Entity lifecycle (spawn/despawn)
/// - Simulation tick tracking
/// - Trace ID generation for causal chains
///
/// # Determinism
///
/// The Arena uses `BTreeMap` for entity storage to ensure deterministic
/// iteration order across platforms (see ADR-0003). Entity IDs are assigned
/// monotonically, and the `BTreeMap`'s natural ordering guarantees that
/// iterating over entities always produces the same sequence.
///
/// # Example
///
/// ```
/// use tidebreak_core::arena::Arena;
/// use tidebreak_core::entity::{EntityTag, EntityInner, ShipComponents};
/// use glam::Vec2;
///
/// let mut arena = Arena::new();
///
/// // Spawn entities
/// let ship1 = arena.spawn(
///     EntityTag::Ship,
///     EntityInner::Ship(ShipComponents::at_position(Vec2::new(0.0, 0.0), 0.0))
/// );
/// let ship2 = arena.spawn(
///     EntityTag::Ship,
///     EntityInner::Ship(ShipComponents::at_position(Vec2::new(100.0, 0.0), 0.0))
/// );
///
/// // Iterate in deterministic order
/// let ids: Vec<_> = arena.entity_ids_sorted().collect();
/// assert_eq!(ids, vec![ship1, ship2]);
///
/// // Get entity by ID
/// let entity = arena.get(ship1).unwrap();
/// assert!(entity.is_ship());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arena {
    /// Monotonically increasing entity ID counter.
    next_id: u64,
    /// Entity storage with deterministic iteration order.
    ///
    /// Use `entity_ids_sorted()`, `entities_sorted()`, or `entities_sorted_mut()`
    /// for iteration. Use `get()` or `get_mut()` for single entity access.
    entities: BTreeMap<EntityId, Entity>,
    /// Spatial index for proximity queries.
    ///
    /// Use `spatial()` or `spatial_mut()` to access the index.
    spatial: SpatialIndex,
    /// Current simulation tick.
    ///
    /// Use `current_tick()` to read and `advance_tick()` to increment.
    tick: u64,
    /// Monotonically increasing trace ID counter.
    next_trace_id: u64,
}

impl Arena {
    /// Creates a new empty arena.
    ///
    /// The arena starts at tick 0 with no entities.
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_id: 0,
            entities: BTreeMap::new(),
            spatial: SpatialIndex::new(),
            tick: 0,
            next_trace_id: 0,
        }
    }

    /// Spawns a new entity in the arena.
    ///
    /// The entity is assigned a unique ID and added to both the entity map
    /// and the spatial index (if it has a transform component).
    ///
    /// # Arguments
    ///
    /// * `tag` - The entity type tag
    /// * `inner` - The entity's component storage
    ///
    /// # Returns
    ///
    /// The unique ID assigned to the new entity.
    ///
    /// # Example
    ///
    /// ```
    /// use tidebreak_core::arena::Arena;
    /// use tidebreak_core::entity::{EntityTag, EntityInner, ShipComponents};
    /// use glam::Vec2;
    ///
    /// let mut arena = Arena::new();
    /// let id = arena.spawn(
    ///     EntityTag::Ship,
    ///     EntityInner::Ship(ShipComponents::at_position(Vec2::new(100.0, 200.0), 0.0))
    /// );
    ///
    /// assert!(arena.get(id).is_some());
    /// ```
    pub fn spawn(&mut self, tag: EntityTag, inner: EntityInner) -> EntityId {
        let id = EntityId::new(self.next_id);
        self.next_id += 1;

        let entity = Entity::new(id, tag, inner);

        // Update spatial index with entity position
        if let Some(pos) = Self::get_entity_position(&entity) {
            self.spatial.insert(id, pos);
        }

        self.entities.insert(id, entity);
        id
    }

    /// Despawns an entity from the arena.
    ///
    /// The entity is removed from both the entity map and the spatial index.
    ///
    /// # Arguments
    ///
    /// * `id` - The entity ID to remove
    ///
    /// # Returns
    ///
    /// The removed entity, if it existed.
    pub fn despawn(&mut self, id: EntityId) -> Option<Entity> {
        self.spatial.remove(id);
        self.entities.remove(&id)
    }

    /// Returns a reference to an entity by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The entity ID to look up
    #[must_use]
    pub fn get(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    /// Returns a mutable reference to an entity by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The entity ID to look up
    #[must_use]
    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    /// Returns an iterator over entity IDs in deterministic (sorted) order.
    ///
    /// This is the primary way to iterate over entities in simulation code.
    /// The order is guaranteed to be consistent across platforms.
    pub fn entity_ids_sorted(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.entities.keys().copied()
    }

    /// Returns an iterator over entities in deterministic (sorted by ID) order.
    pub fn entities_sorted(&self) -> impl Iterator<Item = &Entity> + '_ {
        self.entities.values()
    }

    /// Returns an iterator over mutable entities in deterministic order.
    pub fn entities_sorted_mut(&mut self) -> impl Iterator<Item = &mut Entity> + '_ {
        self.entities.values_mut()
    }

    /// Generates a new unique trace ID.
    ///
    /// Trace IDs are used to track causal chains across outputs and events.
    /// They are monotonically increasing within an arena.
    pub fn new_trace_id(&mut self) -> TraceId {
        let id = TraceId::new(self.next_trace_id);
        self.next_trace_id += 1;
        id
    }

    /// Returns the number of entities in the arena.
    #[must_use]
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Returns true if the arena has no entities.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Returns a reference to the spatial index.
    #[must_use]
    pub fn spatial(&self) -> &SpatialIndex {
        &self.spatial
    }

    /// Returns a mutable reference to the spatial index.
    #[must_use]
    pub fn spatial_mut(&mut self) -> &mut SpatialIndex {
        &mut self.spatial
    }

    /// Returns the current simulation tick.
    #[must_use]
    pub const fn current_tick(&self) -> u64 {
        self.tick
    }

    /// Advances the simulation tick counter.
    pub fn advance_tick(&mut self) {
        self.tick += 1;
    }

    /// Updates the spatial index for an entity.
    ///
    /// Call this after modifying an entity's position to keep the spatial
    /// index in sync.
    pub fn update_spatial(&mut self, id: EntityId) {
        if let Some(entity) = self.entities.get(&id) {
            if let Some(pos) = Self::get_entity_position(entity) {
                self.spatial.insert(id, pos);
            }
        }
    }

    /// Helper to extract position from an entity's inner components.
    ///
    /// # Returns
    ///
    /// Currently all entity types have a position, so this always returns `Some`.
    /// However, we return `Option<Vec2>` for future extensibility:
    ///
    /// - Abstract entities (e.g., fleet command, faction state) may lack spatial presence
    /// - Entities being transferred between layers may temporarily have no position
    /// - This allows callers to handle the None case gracefully without breaking changes
    ///
    /// The `#[allow(clippy::unnecessary_wraps)]` acknowledges that today this always
    /// returns `Some`, but the API contract explicitly supports `None` for future use.
    #[allow(clippy::unnecessary_wraps)]
    fn get_entity_position(entity: &Entity) -> Option<Vec2> {
        match entity.inner() {
            EntityInner::Ship(c) => Some(c.transform.position),
            EntityInner::Platform(c) => Some(c.transform.position),
            EntityInner::Projectile(c) => Some(c.transform.position),
            EntityInner::Squadron(c) => Some(c.transform.position),
        }
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
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

    mod spatial_index_tests {
        use super::*;

        #[test]
        fn new_creates_empty_index() {
            let index = SpatialIndex::new();
            assert!(index.is_empty());
            assert_eq!(index.len(), 0);
        }

        #[test]
        fn insert_and_get() {
            let mut index = SpatialIndex::new();
            let id = EntityId::new(1);
            let pos = Vec2::new(100.0, 200.0);

            index.insert(id, pos);

            assert_eq!(index.get(id), Some(pos));
            assert_eq!(index.len(), 1);
        }

        #[test]
        fn insert_updates_existing() {
            let mut index = SpatialIndex::new();
            let id = EntityId::new(1);

            index.insert(id, Vec2::new(100.0, 200.0));
            index.insert(id, Vec2::new(300.0, 400.0));

            assert_eq!(index.get(id), Some(Vec2::new(300.0, 400.0)));
            assert_eq!(index.len(), 1);
        }

        #[test]
        fn remove_deletes_entry() {
            let mut index = SpatialIndex::new();
            let id = EntityId::new(1);

            index.insert(id, Vec2::new(100.0, 200.0));
            index.remove(id);

            assert!(index.get(id).is_none());
            assert!(index.is_empty());
        }

        #[test]
        fn remove_nonexistent_is_noop() {
            let mut index = SpatialIndex::new();
            index.remove(EntityId::new(999));
            assert!(index.is_empty());
        }

        #[test]
        fn query_radius_finds_entities() {
            let mut index = SpatialIndex::new();

            // Place entities at known positions
            index.insert(EntityId::new(1), Vec2::new(0.0, 0.0));
            index.insert(EntityId::new(2), Vec2::new(50.0, 0.0));
            index.insert(EntityId::new(3), Vec2::new(150.0, 0.0));

            // Query radius 100 around origin
            let results = index.query_radius(Vec2::ZERO, 100.0);

            // Should find entities 1 and 2
            assert_eq!(results.len(), 2);
            assert!(results.contains(&EntityId::new(1)));
            assert!(results.contains(&EntityId::new(2)));
            assert!(!results.contains(&EntityId::new(3)));
        }

        #[test]
        fn query_radius_returns_sorted_results() {
            let mut index = SpatialIndex::new();

            // Insert in non-sorted order
            index.insert(EntityId::new(5), Vec2::new(10.0, 0.0));
            index.insert(EntityId::new(2), Vec2::new(20.0, 0.0));
            index.insert(EntityId::new(8), Vec2::new(30.0, 0.0));

            let results = index.query_radius(Vec2::ZERO, 100.0);

            // Results should be sorted by ID
            assert_eq!(
                results,
                vec![EntityId::new(2), EntityId::new(5), EntityId::new(8)]
            );
        }

        #[test]
        fn query_radius_empty_index() {
            let index = SpatialIndex::new();
            let results = index.query_radius(Vec2::ZERO, 100.0);
            assert!(results.is_empty());
        }

        #[test]
        fn query_radius_zero_radius() {
            let mut index = SpatialIndex::new();
            index.insert(EntityId::new(1), Vec2::new(0.0, 0.0));

            // Zero radius should still find entity at exact position
            let results = index.query_radius(Vec2::ZERO, 0.0);
            assert_eq!(results, vec![EntityId::new(1)]);
        }

        #[test]
        fn query_radius_boundary_case() {
            let mut index = SpatialIndex::new();
            index.insert(EntityId::new(1), Vec2::new(100.0, 0.0));

            // Entity at exactly the radius boundary should be included
            let results = index.query_radius(Vec2::ZERO, 100.0);
            assert!(results.contains(&EntityId::new(1)));
        }

        #[test]
        fn update_existing_position() {
            let mut index = SpatialIndex::new();
            let id = EntityId::new(1);

            index.insert(id, Vec2::new(0.0, 0.0));
            assert!(index.update(id, Vec2::new(100.0, 100.0)));
            assert_eq!(index.get(id), Some(Vec2::new(100.0, 100.0)));
        }

        #[test]
        fn update_nonexistent_returns_false() {
            let mut index = SpatialIndex::new();
            assert!(!index.update(EntityId::new(999), Vec2::new(0.0, 0.0)));
        }

        #[test]
        fn serialization_roundtrip() {
            let mut index = SpatialIndex::new();
            index.insert(EntityId::new(1), Vec2::new(100.0, 200.0));
            index.insert(EntityId::new(2), Vec2::new(300.0, 400.0));

            let json = serde_json::to_string(&index).unwrap();
            let deserialized: SpatialIndex = serde_json::from_str(&json).unwrap();

            assert_eq!(
                deserialized.get(EntityId::new(1)),
                Some(Vec2::new(100.0, 200.0))
            );
            assert_eq!(
                deserialized.get(EntityId::new(2)),
                Some(Vec2::new(300.0, 400.0))
            );
        }
    }

    mod arena_tests {
        use super::*;

        #[test]
        fn new_creates_empty_arena() {
            let arena = Arena::new();

            assert!(arena.is_empty());
            assert_eq!(arena.entity_count(), 0);
            assert_eq!(arena.current_tick(), 0);
        }

        #[test]
        fn default_creates_empty_arena() {
            let arena = Arena::default();
            assert!(arena.is_empty());
        }

        #[test]
        fn spawn_creates_entity_with_sequential_ids() {
            let mut arena = Arena::new();

            let id1 = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );
            let id2 = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );
            let id3 = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            assert_eq!(id1, EntityId::new(0));
            assert_eq!(id2, EntityId::new(1));
            assert_eq!(id3, EntityId::new(2));
            assert_eq!(arena.entity_count(), 3);
        }

        #[test]
        fn spawn_adds_to_spatial_index() {
            let mut arena = Arena::new();

            let components = ShipComponents::at_position(Vec2::new(100.0, 200.0), 0.0);
            let id = arena.spawn(EntityTag::Ship, EntityInner::Ship(components));

            assert_eq!(arena.spatial().get(id), Some(Vec2::new(100.0, 200.0)));
        }

        #[test]
        fn spawn_all_entity_types() {
            let mut arena = Arena::new();

            let ship_id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::at_position(Vec2::new(0.0, 0.0), 0.0)),
            );
            let platform_id = arena.spawn(
                EntityTag::Platform,
                EntityInner::Platform(PlatformComponents::at_position(Vec2::new(100.0, 0.0))),
            );
            let projectile_id = arena.spawn(
                EntityTag::Projectile,
                EntityInner::Projectile(ProjectileComponents::at_position_with_velocity(
                    Vec2::new(200.0, 0.0),
                    0.0,
                    Vec2::new(100.0, 0.0),
                )),
            );
            let squadron_id = arena.spawn(
                EntityTag::Squadron,
                EntityInner::Squadron(SquadronComponents::at_position(Vec2::new(300.0, 0.0), 0.0)),
            );

            // All should be in spatial index
            assert!(arena.spatial().get(ship_id).is_some());
            assert!(arena.spatial().get(platform_id).is_some());
            assert!(arena.spatial().get(projectile_id).is_some());
            assert!(arena.spatial().get(squadron_id).is_some());

            // All should be retrievable
            assert!(arena.get(ship_id).unwrap().is_ship());
            assert!(arena.get(platform_id).unwrap().is_platform());
            assert!(arena.get(projectile_id).unwrap().is_projectile());
            assert!(arena.get(squadron_id).unwrap().is_squadron());
        }

        #[test]
        fn despawn_removes_entity() {
            let mut arena = Arena::new();
            let id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            let removed = arena.despawn(id);

            assert!(removed.is_some());
            assert!(arena.get(id).is_none());
            assert!(arena.is_empty());
        }

        #[test]
        fn despawn_removes_from_spatial() {
            let mut arena = Arena::new();
            let components = ShipComponents::at_position(Vec2::new(100.0, 200.0), 0.0);
            let id = arena.spawn(EntityTag::Ship, EntityInner::Ship(components));

            arena.despawn(id);

            assert!(arena.spatial().get(id).is_none());
        }

        #[test]
        fn despawn_nonexistent_returns_none() {
            let mut arena = Arena::new();
            let removed = arena.despawn(EntityId::new(999));
            assert!(removed.is_none());
        }

        #[test]
        fn get_returns_entity() {
            let mut arena = Arena::new();
            let id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            let entity = arena.get(id);
            assert!(entity.is_some());
            assert_eq!(entity.unwrap().id(), id);
            assert!(entity.unwrap().is_ship());
        }

        #[test]
        fn get_nonexistent_returns_none() {
            let arena = Arena::new();
            assert!(arena.get(EntityId::new(999)).is_none());
        }

        #[test]
        fn get_mut_returns_mutable_entity() {
            let mut arena = Arena::new();
            let id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            let entity = arena.get_mut(id).unwrap();
            // Modify the entity
            if let Some(ship) = entity.as_ship_mut() {
                ship.combat.hp = 50.0;
            }

            // Verify modification persists
            let entity = arena.get(id).unwrap();
            assert_eq!(entity.as_ship().unwrap().combat.hp, 50.0);
        }

        #[test]
        fn entity_ids_sorted_returns_deterministic_order() {
            let mut arena = Arena::new();

            // Spawn entities (IDs will be 0, 1, 2)
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

            let ids: Vec<_> = arena.entity_ids_sorted().collect();

            assert_eq!(
                ids,
                vec![EntityId::new(0), EntityId::new(1), EntityId::new(2)]
            );
        }

        #[test]
        fn entity_ids_sorted_after_despawn() {
            let mut arena = Arena::new();

            let id0 = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );
            let id1 = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );
            let id2 = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            // Remove the middle entity
            arena.despawn(id1);

            let ids: Vec<_> = arena.entity_ids_sorted().collect();
            assert_eq!(ids, vec![id0, id2]);
        }

        #[test]
        fn entities_sorted_iterator() {
            let mut arena = Arena::new();

            arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );
            arena.spawn(
                EntityTag::Platform,
                EntityInner::Platform(PlatformComponents::default()),
            );

            let entities: Vec<_> = arena.entities_sorted().collect();
            assert_eq!(entities.len(), 2);
            assert!(entities[0].is_ship());
            assert!(entities[1].is_platform());
        }

        #[test]
        fn entities_sorted_mut_iterator() {
            let mut arena = Arena::new();

            arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );
            arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            // Modify all ships
            for entity in arena.entities_sorted_mut() {
                if let Some(ship) = entity.as_ship_mut() {
                    ship.combat.hp = 50.0;
                }
            }

            // Verify modifications
            for entity in arena.entities_sorted() {
                assert_eq!(entity.as_ship().unwrap().combat.hp, 50.0);
            }
        }

        #[test]
        fn new_trace_id_generates_sequential_ids() {
            let mut arena = Arena::new();

            let trace1 = arena.new_trace_id();
            let trace2 = arena.new_trace_id();
            let trace3 = arena.new_trace_id();

            assert_eq!(trace1.as_u64(), 0);
            assert_eq!(trace2.as_u64(), 1);
            assert_eq!(trace3.as_u64(), 2);
        }

        #[test]
        fn advance_tick_increments() {
            let mut arena = Arena::new();
            assert_eq!(arena.current_tick(), 0);

            arena.advance_tick();
            assert_eq!(arena.current_tick(), 1);

            arena.advance_tick();
            arena.advance_tick();
            assert_eq!(arena.current_tick(), 3);
        }

        #[test]
        fn update_spatial_syncs_position() {
            let mut arena = Arena::new();

            // Spawn ship at origin
            let id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::at_position(Vec2::ZERO, 0.0)),
            );

            // Move the ship
            if let Some(entity) = arena.get_mut(id) {
                if let Some(ship) = entity.as_ship_mut() {
                    ship.transform.position = Vec2::new(500.0, 500.0);
                }
            }

            // Spatial index is now out of sync
            assert_eq!(arena.spatial().get(id), Some(Vec2::ZERO));

            // Update spatial index
            arena.update_spatial(id);

            // Now it should be synced
            assert_eq!(arena.spatial().get(id), Some(Vec2::new(500.0, 500.0)));
        }

        #[test]
        fn spatial_queries_work_through_arena() {
            let mut arena = Arena::new();

            // Spawn ships at different positions
            let near_id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::at_position(Vec2::new(10.0, 10.0), 0.0)),
            );
            let _far_id = arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::at_position(Vec2::new(1000.0, 1000.0), 0.0)),
            );

            // Query near origin
            let nearby = arena.spatial().query_radius(Vec2::ZERO, 50.0);
            assert_eq!(nearby.len(), 1);
            assert!(nearby.contains(&near_id));
        }

        #[test]
        fn serialization_roundtrip() {
            let mut arena = Arena::new();

            // Spawn some entities
            arena.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::at_position(Vec2::new(100.0, 200.0), 1.0)),
            );
            arena.spawn(
                EntityTag::Platform,
                EntityInner::Platform(PlatformComponents::at_position(Vec2::new(300.0, 400.0))),
            );

            // Advance tick and generate trace IDs
            arena.advance_tick();
            arena.advance_tick();
            let _ = arena.new_trace_id();
            let _ = arena.new_trace_id();

            // Serialize and deserialize
            let json = serde_json::to_string(&arena).unwrap();
            let deserialized: Arena = serde_json::from_str(&json).unwrap();

            // Verify state is preserved
            assert_eq!(deserialized.entity_count(), 2);
            assert_eq!(deserialized.current_tick(), 2);

            // Next spawned entity should continue sequence
            let mut deserialized = deserialized;
            let new_id = deserialized.spawn(
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );
            assert_eq!(new_id, EntityId::new(2));

            // Next trace ID should continue sequence
            let new_trace = deserialized.new_trace_id();
            assert_eq!(new_trace.as_u64(), 2);
        }

        #[test]
        fn determinism_test_iteration_order() {
            // Create two arenas and populate them the same way
            fn populate(arena: &mut Arena) -> Vec<EntityId> {
                vec![
                    arena.spawn(
                        EntityTag::Ship,
                        EntityInner::Ship(ShipComponents::default()),
                    ),
                    arena.spawn(
                        EntityTag::Platform,
                        EntityInner::Platform(PlatformComponents::default()),
                    ),
                    arena.spawn(
                        EntityTag::Projectile,
                        EntityInner::Projectile(ProjectileComponents::default()),
                    ),
                    arena.spawn(
                        EntityTag::Squadron,
                        EntityInner::Squadron(SquadronComponents::default()),
                    ),
                ]
            }

            let mut arena1 = Arena::new();
            let mut arena2 = Arena::new();

            let ids1 = populate(&mut arena1);
            let ids2 = populate(&mut arena2);

            // IDs should be identical
            assert_eq!(ids1, ids2);

            // Iteration order should be identical
            let iter1: Vec<_> = arena1.entity_ids_sorted().collect();
            let iter2: Vec<_> = arena2.entity_ids_sorted().collect();
            assert_eq!(iter1, iter2);
        }

        #[test]
        fn determinism_test_spatial_queries() {
            // Create two arenas with same entities
            fn create_arena() -> Arena {
                let mut arena = Arena::new();
                arena.spawn(
                    EntityTag::Ship,
                    EntityInner::Ship(ShipComponents::at_position(Vec2::new(10.0, 0.0), 0.0)),
                );
                arena.spawn(
                    EntityTag::Ship,
                    EntityInner::Ship(ShipComponents::at_position(Vec2::new(20.0, 0.0), 0.0)),
                );
                arena.spawn(
                    EntityTag::Ship,
                    EntityInner::Ship(ShipComponents::at_position(Vec2::new(30.0, 0.0), 0.0)),
                );
                arena
            }

            let arena1 = create_arena();
            let arena2 = create_arena();

            // Spatial queries should return same results
            let nearby1 = arena1.spatial().query_radius(Vec2::ZERO, 50.0);
            let nearby2 = arena2.spatial().query_radius(Vec2::ZERO, 50.0);
            assert_eq!(nearby1, nearby2);
        }
    }
}
