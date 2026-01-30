//! Entity module for the Entity-Plugin-Resolver architecture.
//!
//! This module provides the core entity types for Tidebreak's combat simulation:
//! - [`EntityId`]: Unique identifier for entities
//! - [`EntityTag`]: Type classification for plugin bundle selection
//! - [`EntityInner`]: Type-safe storage for entity-specific components
//! - [`Entity`]: The complete entity container
//!
//! # Architecture
//!
//! The entity system uses a hybrid approach (see ADR-0007):
//! - `EntityTag` determines which plugins run on an entity
//! - `EntityInner` provides type-safe component storage
//! - Concrete component structs avoid runtime type checking overhead
//!
//! # Example
//!
//! ```
//! use tidebreak_core::entity::{Entity, EntityId, EntityTag, EntityInner};
//! use tidebreak_core::entity::components::ShipComponents;
//!
//! let ship = Entity::new(
//!     EntityId::new(42),
//!     EntityTag::Ship,
//!     EntityInner::Ship(ShipComponents::default()),
//! );
//!
//! assert_eq!(ship.id().as_u64(), 42);
//! assert_eq!(ship.tag(), EntityTag::Ship);
//! ```

pub mod components;

use serde::{Deserialize, Serialize};
use std::fmt;

pub use components::{
    PlatformComponents, ProjectileComponents, ShipComponents, SquadronComponents,
};

/// Unique identifier for an entity.
///
/// `EntityId` is a newtype wrapper around `u64` that provides type safety and
/// a clear semantic meaning. Entity IDs are immutable once assigned and must
/// be unique within an arena.
///
/// # Ordering
///
/// Entity IDs are ordered by their numeric value, which is used to ensure
/// deterministic iteration order across all entities.
///
/// # Example
///
/// ```
/// use tidebreak_core::entity::EntityId;
///
/// let id1 = EntityId::new(1);
/// let id2 = EntityId::new(2);
///
/// assert!(id1 < id2);
/// assert_eq!(id1.as_u64(), 1);
/// ```
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EntityId(u64);

impl EntityId {
    /// Creates a new `EntityId` from a raw `u64` value.
    ///
    /// # Arguments
    ///
    /// * `id` - The raw identifier value
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Returns the raw `u64` value of this identifier.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl fmt::Debug for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EntityId({})", self.0)
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u64> for EntityId {
    fn from(id: u64) -> Self {
        Self::new(id)
    }
}

impl From<EntityId> for u64 {
    fn from(id: EntityId) -> Self {
        id.0
    }
}

/// Entity type tag for plugin bundle selection.
///
/// `EntityTag` determines which plugins are eligible to run on an entity.
/// This decouples plugin selection from the storage representation in
/// [`EntityInner`], allowing flexibility in how plugins are organized.
///
/// # Variants
///
/// - `Ship`: Naval vessels from jetskis to city-ships
/// - `Platform`: Static or semi-static installations (buoys, bases)
/// - `Projectile`: In-flight weapons (missiles, torpedoes)
/// - `Squadron`: Groups of aircraft or small craft
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityTag {
    /// Naval vessel (jetski, frigate, carrier, city-ship, etc.)
    Ship,
    /// Static or semi-static installation (buoy, oil rig, base)
    Platform,
    /// In-flight weapon (missile, torpedo, shell)
    Projectile,
    /// Group of aircraft or small craft operating as a unit
    Squadron,
}

impl fmt::Display for EntityTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ship => write!(f, "Ship"),
            Self::Platform => write!(f, "Platform"),
            Self::Projectile => write!(f, "Projectile"),
            Self::Squadron => write!(f, "Squadron"),
        }
    }
}

/// Type-safe storage for entity-specific components.
///
/// `EntityInner` uses an enum to provide zero-cost, type-safe access to
/// entity components. Each variant contains the component struct for that
/// entity type.
///
/// # Consistency with EntityTag
///
/// The `EntityInner` variant should always match the entity's `EntityTag`.
/// For example, `EntityTag::Ship` must pair with `EntityInner::Ship(_)`.
/// The [`Entity::new`] constructor does not enforce this at compile time,
/// but inconsistent pairings will cause logic errors.
///
/// # Future Migration
///
/// If entity types exceed 10 or component combinations become unwieldy,
/// this enum may be migrated to archetype-based storage while keeping
/// the `Entity` wrapper stable (see ADR-0007).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EntityInner {
    /// Ship components (physics, combat, sensors, inventory)
    Ship(ShipComponents),
    /// Platform components (position, optional combat/sensors)
    Platform(PlatformComponents),
    /// Projectile components (physics, guidance, warhead)
    Projectile(ProjectileComponents),
    /// Squadron components (formation, mission, aggregate state)
    Squadron(SquadronComponents),
}

impl EntityInner {
    /// Returns the corresponding `EntityTag` for this inner storage.
    ///
    /// This can be used to verify tag-inner consistency or to derive
    /// a tag from existing inner storage.
    #[must_use]
    pub const fn tag(&self) -> EntityTag {
        match self {
            Self::Ship(_) => EntityTag::Ship,
            Self::Platform(_) => EntityTag::Platform,
            Self::Projectile(_) => EntityTag::Projectile,
            Self::Squadron(_) => EntityTag::Squadron,
        }
    }

    /// Returns a reference to the ship components, if this is a ship.
    #[must_use]
    pub const fn as_ship(&self) -> Option<&ShipComponents> {
        match self {
            Self::Ship(components) => Some(components),
            _ => None,
        }
    }

    /// Returns a mutable reference to the ship components, if this is a ship.
    #[must_use]
    pub fn as_ship_mut(&mut self) -> Option<&mut ShipComponents> {
        match self {
            Self::Ship(components) => Some(components),
            _ => None,
        }
    }

    /// Returns a reference to the platform components, if this is a platform.
    #[must_use]
    pub const fn as_platform(&self) -> Option<&PlatformComponents> {
        match self {
            Self::Platform(components) => Some(components),
            _ => None,
        }
    }

    /// Returns a mutable reference to the platform components, if this is a platform.
    #[must_use]
    pub fn as_platform_mut(&mut self) -> Option<&mut PlatformComponents> {
        match self {
            Self::Platform(components) => Some(components),
            _ => None,
        }
    }

    /// Returns a reference to the projectile components, if this is a projectile.
    #[must_use]
    pub const fn as_projectile(&self) -> Option<&ProjectileComponents> {
        match self {
            Self::Projectile(components) => Some(components),
            _ => None,
        }
    }

    /// Returns a mutable reference to the projectile components, if this is a projectile.
    #[must_use]
    pub fn as_projectile_mut(&mut self) -> Option<&mut ProjectileComponents> {
        match self {
            Self::Projectile(components) => Some(components),
            _ => None,
        }
    }

    /// Returns a reference to the squadron components, if this is a squadron.
    #[must_use]
    pub const fn as_squadron(&self) -> Option<&SquadronComponents> {
        match self {
            Self::Squadron(components) => Some(components),
            _ => None,
        }
    }

    /// Returns a mutable reference to the squadron components, if this is a squadron.
    #[must_use]
    pub fn as_squadron_mut(&mut self) -> Option<&mut SquadronComponents> {
        match self {
            Self::Squadron(components) => Some(components),
            _ => None,
        }
    }
}

/// A complete entity in the combat simulation.
///
/// An `Entity` combines:
/// - A unique [`EntityId`] for identification and ordering
/// - An [`EntityTag`] that determines which plugins operate on it
/// - An [`EntityInner`] containing type-specific components
///
/// # Invariants
///
/// - The `EntityId` must be unique within an arena
/// - The `EntityTag` should match the `EntityInner` variant
///
/// # Example
///
/// ```
/// use tidebreak_core::entity::{Entity, EntityId, EntityTag, EntityInner};
/// use tidebreak_core::entity::components::ShipComponents;
///
/// let ship = Entity::new(
///     EntityId::new(1),
///     EntityTag::Ship,
///     EntityInner::Ship(ShipComponents::default()),
/// );
///
/// assert!(ship.is_ship());
/// assert!(!ship.is_projectile());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entity {
    id: EntityId,
    tag: EntityTag,
    inner: EntityInner,
}

impl Entity {
    /// Creates a new entity with the given ID, tag, and inner storage.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for this entity
    /// * `tag` - Type classification for plugin selection
    /// * `inner` - Type-specific component storage
    ///
    /// # Note
    ///
    /// The caller is responsible for ensuring `tag` and `inner` are consistent
    /// (e.g., `EntityTag::Ship` with `EntityInner::Ship(_)`).
    #[must_use]
    pub const fn new(id: EntityId, tag: EntityTag, inner: EntityInner) -> Self {
        Self { id, tag, inner }
    }

    /// Creates a new ship entity with default components.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for this ship
    #[must_use]
    pub fn new_ship(id: EntityId) -> Self {
        Self::new(
            id,
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::default()),
        )
    }

    /// Creates a new platform entity with default components.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for this platform
    #[must_use]
    pub fn new_platform(id: EntityId) -> Self {
        Self::new(
            id,
            EntityTag::Platform,
            EntityInner::Platform(PlatformComponents::default()),
        )
    }

    /// Creates a new projectile entity with default components.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for this projectile
    #[must_use]
    pub fn new_projectile(id: EntityId) -> Self {
        Self::new(
            id,
            EntityTag::Projectile,
            EntityInner::Projectile(ProjectileComponents::default()),
        )
    }

    /// Creates a new squadron entity with default components.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for this squadron
    #[must_use]
    pub fn new_squadron(id: EntityId) -> Self {
        Self::new(
            id,
            EntityTag::Squadron,
            EntityInner::Squadron(SquadronComponents::default()),
        )
    }

    /// Returns the entity's unique identifier.
    #[must_use]
    pub const fn id(&self) -> EntityId {
        self.id
    }

    /// Returns the entity's type tag.
    #[must_use]
    pub const fn tag(&self) -> EntityTag {
        self.tag
    }

    /// Returns a reference to the entity's inner component storage.
    #[must_use]
    pub const fn inner(&self) -> &EntityInner {
        &self.inner
    }

    /// Returns a mutable reference to the entity's inner component storage.
    #[must_use]
    pub fn inner_mut(&mut self) -> &mut EntityInner {
        &mut self.inner
    }

    /// Returns `true` if this entity is a ship.
    #[must_use]
    pub const fn is_ship(&self) -> bool {
        matches!(self.tag, EntityTag::Ship)
    }

    /// Returns `true` if this entity is a platform.
    #[must_use]
    pub const fn is_platform(&self) -> bool {
        matches!(self.tag, EntityTag::Platform)
    }

    /// Returns `true` if this entity is a projectile.
    #[must_use]
    pub const fn is_projectile(&self) -> bool {
        matches!(self.tag, EntityTag::Projectile)
    }

    /// Returns `true` if this entity is a squadron.
    #[must_use]
    pub const fn is_squadron(&self) -> bool {
        matches!(self.tag, EntityTag::Squadron)
    }

    /// Returns the ship components if this is a ship, `None` otherwise.
    #[must_use]
    pub const fn as_ship(&self) -> Option<&ShipComponents> {
        self.inner.as_ship()
    }

    /// Returns mutable ship components if this is a ship, `None` otherwise.
    #[must_use]
    pub fn as_ship_mut(&mut self) -> Option<&mut ShipComponents> {
        self.inner.as_ship_mut()
    }

    /// Returns the platform components if this is a platform, `None` otherwise.
    #[must_use]
    pub const fn as_platform(&self) -> Option<&PlatformComponents> {
        self.inner.as_platform()
    }

    /// Returns mutable platform components if this is a platform, `None` otherwise.
    #[must_use]
    pub fn as_platform_mut(&mut self) -> Option<&mut PlatformComponents> {
        self.inner.as_platform_mut()
    }

    /// Returns the projectile components if this is a projectile, `None` otherwise.
    #[must_use]
    pub const fn as_projectile(&self) -> Option<&ProjectileComponents> {
        self.inner.as_projectile()
    }

    /// Returns mutable projectile components if this is a projectile, `None` otherwise.
    #[must_use]
    pub fn as_projectile_mut(&mut self) -> Option<&mut ProjectileComponents> {
        self.inner.as_projectile_mut()
    }

    /// Returns the squadron components if this is a squadron, `None` otherwise.
    #[must_use]
    pub const fn as_squadron(&self) -> Option<&SquadronComponents> {
        self.inner.as_squadron()
    }

    /// Returns mutable squadron components if this is a squadron, `None` otherwise.
    #[must_use]
    pub fn as_squadron_mut(&mut self) -> Option<&mut SquadronComponents> {
        self.inner.as_squadron_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod entity_id_tests {
        use super::*;

        #[test]
        fn new_creates_id_with_value() {
            let id = EntityId::new(42);
            assert_eq!(id.as_u64(), 42);
        }

        #[test]
        fn copy_semantics() {
            let id1 = EntityId::new(1);
            let id2 = id1; // Copy
            assert_eq!(id1, id2);
        }

        #[test]
        fn clone_semantics() {
            let id1 = EntityId::new(1);
            let id2 = id1.clone();
            assert_eq!(id1, id2);
        }

        #[test]
        fn equality() {
            let id1 = EntityId::new(1);
            let id2 = EntityId::new(1);
            let id3 = EntityId::new(2);

            assert_eq!(id1, id2);
            assert_ne!(id1, id3);
        }

        #[test]
        fn ordering() {
            let id1 = EntityId::new(1);
            let id2 = EntityId::new(2);
            let id3 = EntityId::new(3);

            assert!(id1 < id2);
            assert!(id2 < id3);
            assert!(id1 < id3);

            let mut ids = vec![id3, id1, id2];
            ids.sort();
            assert_eq!(ids, vec![id1, id2, id3]);
        }

        #[test]
        fn hashing() {
            use std::collections::HashSet;

            let mut set = HashSet::new();
            set.insert(EntityId::new(1));
            set.insert(EntityId::new(2));
            set.insert(EntityId::new(1)); // Duplicate

            assert_eq!(set.len(), 2);
            assert!(set.contains(&EntityId::new(1)));
            assert!(set.contains(&EntityId::new(2)));
        }

        #[test]
        fn debug_format() {
            let id = EntityId::new(42);
            assert_eq!(format!("{:?}", id), "EntityId(42)");
        }

        #[test]
        fn display_format() {
            let id = EntityId::new(42);
            assert_eq!(format!("{}", id), "42");
        }

        #[test]
        fn from_u64() {
            let id: EntityId = 42u64.into();
            assert_eq!(id.as_u64(), 42);
        }

        #[test]
        fn into_u64() {
            let id = EntityId::new(42);
            let value: u64 = id.into();
            assert_eq!(value, 42);
        }

        #[test]
        fn serialization_roundtrip() {
            let id = EntityId::new(12345);
            let json = serde_json::to_string(&id).unwrap();
            let deserialized: EntityId = serde_json::from_str(&json).unwrap();
            assert_eq!(id, deserialized);
        }
    }

    mod entity_tag_tests {
        use super::*;

        #[test]
        fn all_variants_exist() {
            let _ship = EntityTag::Ship;
            let _platform = EntityTag::Platform;
            let _projectile = EntityTag::Projectile;
            let _squadron = EntityTag::Squadron;
        }

        #[test]
        fn copy_semantics() {
            let tag1 = EntityTag::Ship;
            let tag2 = tag1; // Copy
            assert_eq!(tag1, tag2);
        }

        #[test]
        fn equality() {
            assert_eq!(EntityTag::Ship, EntityTag::Ship);
            assert_ne!(EntityTag::Ship, EntityTag::Platform);
        }

        #[test]
        fn hashing() {
            use std::collections::HashSet;

            let mut set = HashSet::new();
            set.insert(EntityTag::Ship);
            set.insert(EntityTag::Platform);
            set.insert(EntityTag::Ship); // Duplicate

            assert_eq!(set.len(), 2);
        }

        #[test]
        fn display_format() {
            assert_eq!(format!("{}", EntityTag::Ship), "Ship");
            assert_eq!(format!("{}", EntityTag::Platform), "Platform");
            assert_eq!(format!("{}", EntityTag::Projectile), "Projectile");
            assert_eq!(format!("{}", EntityTag::Squadron), "Squadron");
        }

        #[test]
        fn serialization_roundtrip() {
            let tag = EntityTag::Ship;
            let json = serde_json::to_string(&tag).unwrap();
            let deserialized: EntityTag = serde_json::from_str(&json).unwrap();
            assert_eq!(tag, deserialized);
        }
    }

    mod entity_inner_tests {
        use super::*;

        #[test]
        fn tag_matches_variant() {
            let ship = EntityInner::Ship(ShipComponents::default());
            assert_eq!(ship.tag(), EntityTag::Ship);

            let platform = EntityInner::Platform(PlatformComponents::default());
            assert_eq!(platform.tag(), EntityTag::Platform);

            let projectile = EntityInner::Projectile(ProjectileComponents::default());
            assert_eq!(projectile.tag(), EntityTag::Projectile);

            let squadron = EntityInner::Squadron(SquadronComponents::default());
            assert_eq!(squadron.tag(), EntityTag::Squadron);
        }

        #[test]
        fn as_ship_accessors() {
            let mut ship = EntityInner::Ship(ShipComponents::default());
            assert!(ship.as_ship().is_some());
            assert!(ship.as_ship_mut().is_some());
            assert!(ship.as_platform().is_none());
        }

        #[test]
        fn as_platform_accessors() {
            let mut platform = EntityInner::Platform(PlatformComponents::default());
            assert!(platform.as_platform().is_some());
            assert!(platform.as_platform_mut().is_some());
            assert!(platform.as_ship().is_none());
        }

        #[test]
        fn as_projectile_accessors() {
            let mut projectile = EntityInner::Projectile(ProjectileComponents::default());
            assert!(projectile.as_projectile().is_some());
            assert!(projectile.as_projectile_mut().is_some());
            assert!(projectile.as_ship().is_none());
        }

        #[test]
        fn as_squadron_accessors() {
            let mut squadron = EntityInner::Squadron(SquadronComponents::default());
            assert!(squadron.as_squadron().is_some());
            assert!(squadron.as_squadron_mut().is_some());
            assert!(squadron.as_ship().is_none());
        }

        #[test]
        fn serialization_roundtrip() {
            let inner = EntityInner::Ship(ShipComponents::default());
            let json = serde_json::to_string(&inner).unwrap();
            let deserialized: EntityInner = serde_json::from_str(&json).unwrap();
            assert_eq!(inner, deserialized);
        }
    }

    mod entity_tests {
        use super::*;

        #[test]
        fn new_creates_entity() {
            let entity = Entity::new(
                EntityId::new(1),
                EntityTag::Ship,
                EntityInner::Ship(ShipComponents::default()),
            );

            assert_eq!(entity.id(), EntityId::new(1));
            assert_eq!(entity.tag(), EntityTag::Ship);
        }

        #[test]
        fn convenience_constructors() {
            let ship = Entity::new_ship(EntityId::new(1));
            assert!(ship.is_ship());
            assert_eq!(ship.tag(), EntityTag::Ship);

            let platform = Entity::new_platform(EntityId::new(2));
            assert!(platform.is_platform());
            assert_eq!(platform.tag(), EntityTag::Platform);

            let projectile = Entity::new_projectile(EntityId::new(3));
            assert!(projectile.is_projectile());
            assert_eq!(projectile.tag(), EntityTag::Projectile);

            let squadron = Entity::new_squadron(EntityId::new(4));
            assert!(squadron.is_squadron());
            assert_eq!(squadron.tag(), EntityTag::Squadron);
        }

        #[test]
        fn is_type_predicates() {
            let ship = Entity::new_ship(EntityId::new(1));
            assert!(ship.is_ship());
            assert!(!ship.is_platform());
            assert!(!ship.is_projectile());
            assert!(!ship.is_squadron());
        }

        #[test]
        fn as_type_accessors() {
            let mut ship = Entity::new_ship(EntityId::new(1));
            assert!(ship.as_ship().is_some());
            assert!(ship.as_ship_mut().is_some());
            assert!(ship.as_platform().is_none());
            assert!(ship.as_projectile().is_none());
            assert!(ship.as_squadron().is_none());
        }

        #[test]
        fn inner_access() {
            let mut entity = Entity::new_ship(EntityId::new(1));

            // Immutable access
            let _inner = entity.inner();

            // Mutable access
            let _inner_mut = entity.inner_mut();
        }

        #[test]
        fn serialization_roundtrip() {
            let entity = Entity::new_ship(EntityId::new(42));
            let json = serde_json::to_string(&entity).unwrap();
            let deserialized: Entity = serde_json::from_str(&json).unwrap();

            assert_eq!(entity.id(), deserialized.id());
            assert_eq!(entity.tag(), deserialized.tag());
        }

        #[test]
        fn clone_semantics() {
            let entity1 = Entity::new_ship(EntityId::new(1));
            let entity2 = entity1.clone();

            assert_eq!(entity1.id(), entity2.id());
            assert_eq!(entity1.tag(), entity2.tag());
        }
    }
}
