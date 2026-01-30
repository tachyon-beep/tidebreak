//! Plugin system for the Entity-Plugin-Resolver architecture.
//!
//! This module provides the Plugin trait and supporting types for implementing
//! game logic in a modular, composable way. Plugins read from an immutable
//! [`WorldView`](crate::world_view::WorldView) and emit [`Output`]s that are
//! collected and resolved by the resolution phase.
//!
//! # Architecture
//!
//! Plugins follow a strict read-only paradigm:
//! - Plugins receive a [`WorldView`] scoped to only the components they declared
//! - Plugins emit [`Output`]s as proposals for state changes
//! - Plugins cannot directly mutate state
//! - Plugins can run in parallel (since they only read)
//!
//! # Plugin Declaration
//!
//! Each plugin declares:
//! - Its unique identifier ([`PluginId`])
//! - Required entity tags (which entity types it operates on)
//! - Components it reads (for `WorldView` scoping)
//! - Output kinds it emits (for resolver routing)
//!
//! # Plugin Registry
//!
//! The [`PluginRegistry`] bundles plugins by entity tag, allowing efficient
//! lookup of which plugins should run on each entity type.
//!
//! # Example
//!
//! ```
//! use tidebreak_core::plugin::{
//!     Plugin, PluginContext, PluginDeclaration, PluginId, PluginRegistry,
//!     ComponentKind,
//! };
//! use tidebreak_core::world_view::WorldView;
//! use tidebreak_core::output::{Output, Command, OutputKind};
//! use tidebreak_core::entity::{EntityTag, EntityId};
//! use glam::Vec2;
//! use std::sync::Arc;
//!
//! struct MovementPlugin {
//!     declaration: PluginDeclaration,
//! }
//!
//! impl MovementPlugin {
//!     fn new() -> Self {
//!         Self {
//!             declaration: PluginDeclaration {
//!                 id: PluginId::new("movement"),
//!                 required_tags: vec![EntityTag::Ship, EntityTag::Squadron],
//!                 reads: vec![ComponentKind::Transform, ComponentKind::Physics],
//!                 emits: vec![OutputKind::Command],
//!             },
//!         }
//!     }
//! }
//!
//! impl Plugin for MovementPlugin {
//!     fn declaration(&self) -> &PluginDeclaration {
//!         &self.declaration
//!     }
//!
//!     fn run(&self, ctx: &PluginContext, view: &WorldView) -> Vec<Output> {
//!         // Plugin logic here - read from view, emit outputs
//!         vec![]
//!     }
//! }
//!
//! // Register plugins in a registry
//! let mut registry = PluginRegistry::new();
//! let plugin = Arc::new(MovementPlugin::new());
//! registry.register(EntityTag::Ship, plugin.clone());
//! registry.register(EntityTag::Squadron, plugin);
//!
//! // Get plugins for an entity type
//! let ship_plugins = registry.plugins_for(EntityTag::Ship);
//! assert_eq!(ship_plugins.len(), 1);
//! ```

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::entity::{EntityId, EntityTag};
use crate::output::{Output, OutputKind, TraceId};
use crate::world_view::WorldView;

// Re-export PluginId from output so users can use `plugin::PluginId`
pub use crate::output::PluginId;

// =============================================================================
// Component Kind
// =============================================================================

/// Component type identifiers for plugin declarations.
///
/// Used in [`PluginDeclaration`] to specify which component types a plugin
/// needs to read. The [`WorldView`] enforces that plugins can only access
/// components they declared.
///
/// # Variants
///
/// - `Transform`: Position and heading ([`TransformState`](crate::entity::TransformState))
/// - `Physics`: Velocity and movement limits ([`PhysicsState`](crate::entity::PhysicsState))
/// - `Combat`: Health, weapons, status ([`CombatState`](crate::entity::CombatState))
/// - `Sensor`: Detection capabilities ([`SensorState`](crate::entity::SensorState))
/// - `Inventory`: Fuel and ammunition ([`InventoryState`](crate::entity::InventoryState))
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComponentKind {
    /// Transform component (position, heading)
    Transform,
    /// Physics component (velocity, movement limits)
    Physics,
    /// Combat component (health, weapons, status)
    Combat,
    /// Sensor component (detection, track table)
    Sensor,
    /// Inventory component (fuel, ammunition)
    Inventory,
}

impl fmt::Display for ComponentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transform => write!(f, "Transform"),
            Self::Physics => write!(f, "Physics"),
            Self::Combat => write!(f, "Combat"),
            Self::Sensor => write!(f, "Sensor"),
            Self::Inventory => write!(f, "Inventory"),
        }
    }
}

// =============================================================================
// Plugin Declaration
// =============================================================================

/// Declaration of a plugin's capabilities and requirements.
///
/// The declaration specifies:
/// - `id`: Unique identifier for the plugin
/// - `required_tags`: Which entity types this plugin operates on
/// - `reads`: Which component types the plugin needs to read
/// - `emits`: Which output kinds the plugin may emit
///
/// This information is used to:
/// - Route plugins to appropriate entities
/// - Scope the [`WorldView`] to only allowed components
/// - Validate output types at debug time
///
/// # Example
///
/// ```
/// use tidebreak_core::plugin::{PluginDeclaration, PluginId, ComponentKind};
/// use tidebreak_core::output::OutputKind;
/// use tidebreak_core::entity::EntityTag;
///
/// let decl = PluginDeclaration {
///     id: PluginId::new("sensor"),
///     required_tags: vec![EntityTag::Ship, EntityTag::Platform],
///     reads: vec![ComponentKind::Transform, ComponentKind::Sensor],
///     emits: vec![OutputKind::Event],
/// };
///
/// assert!(decl.reads.contains(&ComponentKind::Sensor));
/// ```
#[derive(Debug, Clone)]
pub struct PluginDeclaration {
    /// Unique identifier for this plugin.
    pub id: PluginId,
    /// Entity tags this plugin operates on.
    /// The plugin will only run on entities with matching tags.
    pub required_tags: Vec<EntityTag>,
    /// Component types this plugin reads.
    /// The `WorldView` will only allow access to these components.
    pub reads: Vec<ComponentKind>,
    /// Output kinds this plugin may emit.
    /// Used for validation and resolver routing.
    pub emits: Vec<OutputKind>,
}

impl PluginDeclaration {
    /// Checks if this plugin operates on the given entity tag.
    #[must_use]
    pub fn supports_tag(&self, tag: EntityTag) -> bool {
        self.required_tags.contains(&tag)
    }

    /// Checks if this plugin reads the given component kind.
    #[must_use]
    pub fn reads_component(&self, kind: ComponentKind) -> bool {
        self.reads.contains(&kind)
    }

    /// Checks if this plugin emits the given output kind.
    #[must_use]
    pub fn emits_output(&self, kind: OutputKind) -> bool {
        self.emits.contains(&kind)
    }
}

// =============================================================================
// Plugin Context
// =============================================================================

/// Contextual information passed to a plugin during execution.
///
/// The context provides:
/// - `entity_id`: The entity this plugin instance is operating on
/// - `tick`: The current simulation tick
/// - `trace_id`: A trace ID for causal chain tracking
///
/// # Example
///
/// ```
/// use tidebreak_core::plugin::PluginContext;
/// use tidebreak_core::entity::EntityId;
/// use tidebreak_core::output::TraceId;
///
/// let ctx = PluginContext {
///     entity_id: EntityId::new(42),
///     tick: 100,
///     trace_id: TraceId::new(1),
/// };
///
/// assert_eq!(ctx.entity_id, EntityId::new(42));
/// assert_eq!(ctx.tick, 100);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct PluginContext {
    /// The entity this plugin is operating on.
    pub entity_id: EntityId,
    /// The current simulation tick.
    pub tick: u64,
    /// Trace ID for causal chain tracking.
    pub trace_id: TraceId,
}

// =============================================================================
// Plugin Trait
// =============================================================================

/// A plugin that implements game logic for entities.
///
/// Plugins are the primary mechanism for implementing game behavior. They:
/// - Declare their capabilities via [`PluginDeclaration`]
/// - Read from an immutable [`WorldView`]
/// - Emit [`Output`]s as proposals for state changes
///
/// # Thread Safety
///
/// Plugins must be `Send + Sync` to allow parallel execution. The plugin
/// execution loop may run plugins concurrently since they only read from
/// an immutable snapshot.
///
/// # Implementation Guidelines
///
/// 1. **No side effects**: Plugins should not have observable side effects
///    during `run()` - all effects should be expressed through outputs.
///
/// 2. **Determinism**: Given the same inputs, a plugin should always produce
///    the same outputs. Avoid using system time, random numbers (except
///    via arena RNG), or other non-deterministic sources.
///
/// 3. **Respect declarations**: Only access components declared in `reads`,
///    and only emit output kinds declared in `emits`.
///
/// # Example
///
/// See the module-level documentation for a complete example.
pub trait Plugin: Send + Sync {
    /// Returns the plugin's declaration.
    ///
    /// The declaration specifies what the plugin reads and emits.
    fn declaration(&self) -> &PluginDeclaration;

    /// Executes the plugin logic.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Context containing the entity ID, tick, and trace ID
    /// * `view` - Immutable view of the world state, scoped to declared components
    ///
    /// # Returns
    ///
    /// A vector of outputs representing proposed state changes or events.
    fn run(&self, ctx: &PluginContext, view: &WorldView) -> Vec<Output>;
}

// =============================================================================
// Plugin Registry
// =============================================================================

/// Registry of plugins organized by entity tag.
///
/// The registry allows efficient lookup of which plugins should run on
/// entities of a given type. Plugins can be registered for multiple
/// entity tags.
///
/// # Example
///
/// ```
/// use tidebreak_core::plugin::{
///     Plugin, PluginContext, PluginDeclaration, PluginId, PluginRegistry,
///     ComponentKind,
/// };
/// use tidebreak_core::world_view::WorldView;
/// use tidebreak_core::output::{Output, OutputKind};
/// use tidebreak_core::entity::EntityTag;
/// use std::sync::Arc;
///
/// // Create a simple test plugin
/// struct TestPlugin {
///     declaration: PluginDeclaration,
/// }
///
/// impl Plugin for TestPlugin {
///     fn declaration(&self) -> &PluginDeclaration {
///         &self.declaration
///     }
///     fn run(&self, _ctx: &PluginContext, _view: &WorldView) -> Vec<Output> {
///         vec![]
///     }
/// }
///
/// let mut registry = PluginRegistry::new();
///
/// let plugin = Arc::new(TestPlugin {
///     declaration: PluginDeclaration {
///         id: PluginId::new("test"),
///         required_tags: vec![EntityTag::Ship],
///         reads: vec![ComponentKind::Transform],
///         emits: vec![OutputKind::Command],
///     },
/// });
///
/// registry.register(EntityTag::Ship, plugin);
///
/// assert_eq!(registry.plugins_for(EntityTag::Ship).len(), 1);
/// assert_eq!(registry.plugins_for(EntityTag::Platform).len(), 0);
/// ```
#[derive(Default)]
pub struct PluginRegistry {
    /// Plugins bundled by entity tag.
    bundles: HashMap<EntityTag, Vec<Arc<dyn Plugin>>>,
}

impl PluginRegistry {
    /// Creates a new empty plugin registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bundles: HashMap::new(),
        }
    }

    /// Registers a plugin for the given entity tag.
    ///
    /// A plugin can be registered for multiple tags by calling this method
    /// multiple times with the same plugin Arc.
    ///
    /// # Arguments
    ///
    /// * `tag` - The entity tag to register the plugin for
    /// * `plugin` - The plugin to register (wrapped in Arc for shared ownership)
    pub fn register(&mut self, tag: EntityTag, plugin: Arc<dyn Plugin>) {
        self.bundles.entry(tag).or_default().push(plugin);
    }

    /// Returns the plugins registered for the given entity tag.
    ///
    /// # Arguments
    ///
    /// * `tag` - The entity tag to look up
    ///
    /// # Returns
    ///
    /// A slice of plugins registered for this tag, or an empty slice if none.
    #[must_use]
    pub fn plugins_for(&self, tag: EntityTag) -> &[Arc<dyn Plugin>] {
        self.bundles.get(&tag).map_or(&[], Vec::as_slice)
    }

    /// Returns the total number of plugin registrations.
    ///
    /// Note: A plugin registered for multiple tags is counted multiple times.
    #[must_use]
    pub fn registration_count(&self) -> usize {
        self.bundles.values().map(Vec::len).sum()
    }

    /// Returns true if the registry has no plugins.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bundles.is_empty() || self.bundles.values().all(Vec::is_empty)
    }

    /// Clears all plugins from the registry.
    pub fn clear(&mut self) {
        self.bundles.clear();
    }

    /// Returns an iterator over all (tag, plugins) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&EntityTag, &Vec<Arc<dyn Plugin>>)> {
        self.bundles.iter()
    }

    /// Creates a registry pre-populated with the default MVP plugin bundles.
    ///
    /// Registers the following plugins:
    /// - Ships: movement, weapons, sensors
    /// - Platforms: sensors only (stationary)
    /// - Projectiles: projectile behavior
    /// - Squadrons: movement, weapons
    ///
    /// # Example
    ///
    /// ```
    /// use tidebreak_core::plugin::PluginRegistry;
    /// use tidebreak_core::entity::EntityTag;
    ///
    /// let registry = PluginRegistry::default_bundles();
    ///
    /// // Ships have movement, weapon, and sensor plugins
    /// assert_eq!(registry.plugins_for(EntityTag::Ship).len(), 3);
    ///
    /// // Platforms have only sensor plugin
    /// assert_eq!(registry.plugins_for(EntityTag::Platform).len(), 1);
    ///
    /// // Projectiles have only projectile plugin
    /// assert_eq!(registry.plugins_for(EntityTag::Projectile).len(), 1);
    ///
    /// // Squadrons have movement and weapon plugins
    /// assert_eq!(registry.plugins_for(EntityTag::Squadron).len(), 2);
    /// ```
    #[must_use]
    pub fn default_bundles() -> Self {
        use crate::plugins::{MovementPlugin, ProjectilePlugin, SensorPlugin, WeaponPlugin};

        let mut registry = Self::new();

        // Ships: movement, weapons, sensors
        registry.register(EntityTag::Ship, Arc::new(MovementPlugin::new()));
        registry.register(EntityTag::Ship, Arc::new(WeaponPlugin::new()));
        registry.register(EntityTag::Ship, Arc::new(SensorPlugin::new()));

        // Platforms: sensors only (stationary)
        registry.register(EntityTag::Platform, Arc::new(SensorPlugin::new()));

        // Projectiles: projectile behavior
        registry.register(EntityTag::Projectile, Arc::new(ProjectilePlugin::new()));

        // Squadrons: movement, weapons
        registry.register(EntityTag::Squadron, Arc::new(MovementPlugin::new()));
        registry.register(EntityTag::Squadron, Arc::new(WeaponPlugin::new()));

        registry
    }
}

impl fmt::Debug for PluginRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PluginRegistry")
            .field("bundle_count", &self.bundles.len())
            .field("registration_count", &self.registration_count())
            .finish()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod plugin_id_tests {
        use super::*;

        #[test]
        fn new_creates_id() {
            let id = PluginId::new("movement");
            assert_eq!(id.as_str(), "movement");
        }

        #[test]
        fn const_creation() {
            const PLUGIN_ID: PluginId = PluginId::from_static("weapon");
            assert_eq!(PLUGIN_ID.as_str(), "weapon");
        }

        #[test]
        fn display_format() {
            let id = PluginId::new("sensor");
            assert_eq!(format!("{}", id), "sensor");
        }

        #[test]
        fn debug_format() {
            let id = PluginId::new("test");
            // Debug format shows the Cow wrapper
            let debug = format!("{:?}", id);
            assert!(debug.contains("PluginId"));
            assert!(debug.contains("test"));
        }

        #[test]
        fn equality() {
            let id1 = PluginId::new("test");
            let id2 = PluginId::new("test");
            let id3 = PluginId::new("other");

            assert_eq!(id1, id2);
            assert_ne!(id1, id3);
        }

        #[test]
        fn clone_semantics() {
            let id1 = PluginId::new("test");
            let id2 = id1.clone();
            assert_eq!(id1, id2);
        }

        #[test]
        fn static_and_owned_equality() {
            // Static and owned versions should be equal
            let static_id = PluginId::from_static("test");
            let owned_id = PluginId::new("test");
            assert_eq!(static_id, owned_id);
        }

        #[test]
        fn hashing() {
            use std::collections::HashSet;

            let mut set = HashSet::new();
            set.insert(PluginId::new("a"));
            set.insert(PluginId::new("b"));
            set.insert(PluginId::new("a")); // Duplicate

            assert_eq!(set.len(), 2);
            assert!(set.contains(&PluginId::new("a")));
            assert!(set.contains(&PluginId::new("b")));
        }

        #[test]
        fn from_static_str() {
            let id: PluginId = "movement".into();
            assert_eq!(id.as_str(), "movement");
        }

        #[test]
        fn serialization_roundtrip() {
            let id = PluginId::new("test_plugin");
            let json = serde_json::to_string(&id).unwrap();
            let deserialized: PluginId = serde_json::from_str(&json).unwrap();
            assert_eq!(id, deserialized);
        }
    }

    mod component_kind_tests {
        use super::*;

        #[test]
        fn all_variants_exist() {
            let _transform = ComponentKind::Transform;
            let _physics = ComponentKind::Physics;
            let _combat = ComponentKind::Combat;
            let _sensor = ComponentKind::Sensor;
            let _inventory = ComponentKind::Inventory;
        }

        #[test]
        fn display_format() {
            assert_eq!(format!("{}", ComponentKind::Transform), "Transform");
            assert_eq!(format!("{}", ComponentKind::Physics), "Physics");
            assert_eq!(format!("{}", ComponentKind::Combat), "Combat");
            assert_eq!(format!("{}", ComponentKind::Sensor), "Sensor");
            assert_eq!(format!("{}", ComponentKind::Inventory), "Inventory");
        }

        #[test]
        fn equality() {
            assert_eq!(ComponentKind::Transform, ComponentKind::Transform);
            assert_ne!(ComponentKind::Transform, ComponentKind::Physics);
        }

        #[test]
        fn hashing() {
            use std::collections::HashSet;

            let mut set = HashSet::new();
            set.insert(ComponentKind::Transform);
            set.insert(ComponentKind::Physics);
            set.insert(ComponentKind::Transform); // Duplicate

            assert_eq!(set.len(), 2);
        }

        #[test]
        fn serialization_roundtrip() {
            let kind = ComponentKind::Combat;
            let json = serde_json::to_string(&kind).unwrap();
            let deserialized: ComponentKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, deserialized);
        }
    }

    mod plugin_declaration_tests {
        use super::*;

        fn make_test_declaration() -> PluginDeclaration {
            PluginDeclaration {
                id: PluginId::new("test"),
                required_tags: vec![EntityTag::Ship, EntityTag::Squadron],
                reads: vec![ComponentKind::Transform, ComponentKind::Physics],
                emits: vec![OutputKind::Command, OutputKind::Event],
            }
        }

        #[test]
        fn supports_tag() {
            let decl = make_test_declaration();

            assert!(decl.supports_tag(EntityTag::Ship));
            assert!(decl.supports_tag(EntityTag::Squadron));
            assert!(!decl.supports_tag(EntityTag::Platform));
            assert!(!decl.supports_tag(EntityTag::Projectile));
        }

        #[test]
        fn reads_component() {
            let decl = make_test_declaration();

            assert!(decl.reads_component(ComponentKind::Transform));
            assert!(decl.reads_component(ComponentKind::Physics));
            assert!(!decl.reads_component(ComponentKind::Combat));
            assert!(!decl.reads_component(ComponentKind::Sensor));
            assert!(!decl.reads_component(ComponentKind::Inventory));
        }

        #[test]
        fn emits_output() {
            let decl = make_test_declaration();

            assert!(decl.emits_output(OutputKind::Command));
            assert!(decl.emits_output(OutputKind::Event));
            assert!(!decl.emits_output(OutputKind::Modifier));
        }

        #[test]
        fn empty_declaration() {
            let decl = PluginDeclaration {
                id: PluginId::new("empty"),
                required_tags: vec![],
                reads: vec![],
                emits: vec![],
            };

            assert!(!decl.supports_tag(EntityTag::Ship));
            assert!(!decl.reads_component(ComponentKind::Transform));
            assert!(!decl.emits_output(OutputKind::Command));
        }
    }

    mod plugin_context_tests {
        use super::*;

        #[test]
        fn creation() {
            let ctx = PluginContext {
                entity_id: EntityId::new(42),
                tick: 100,
                trace_id: TraceId::new(5),
            };

            assert_eq!(ctx.entity_id, EntityId::new(42));
            assert_eq!(ctx.tick, 100);
            assert_eq!(ctx.trace_id.as_u64(), 5);
        }

        #[test]
        fn copy_semantics() {
            let ctx1 = PluginContext {
                entity_id: EntityId::new(1),
                tick: 50,
                trace_id: TraceId::new(10),
            };

            let ctx2 = ctx1; // Copy
            assert_eq!(ctx1.entity_id, ctx2.entity_id);
            assert_eq!(ctx1.tick, ctx2.tick);
        }

        #[test]
        fn debug_format() {
            let ctx = PluginContext {
                entity_id: EntityId::new(1),
                tick: 0,
                trace_id: TraceId::new(0),
            };
            let debug = format!("{:?}", ctx);
            assert!(debug.contains("PluginContext"));
            assert!(debug.contains("entity_id"));
            assert!(debug.contains("tick"));
            assert!(debug.contains("trace_id"));
        }
    }

    mod plugin_registry_tests {
        use super::*;

        // Test plugin implementation
        struct TestPlugin {
            declaration: PluginDeclaration,
        }

        impl TestPlugin {
            fn new(id: &'static str, tags: Vec<EntityTag>) -> Self {
                Self {
                    declaration: PluginDeclaration {
                        id: PluginId::new(id),
                        required_tags: tags,
                        reads: vec![ComponentKind::Transform],
                        emits: vec![OutputKind::Command],
                    },
                }
            }
        }

        impl Plugin for TestPlugin {
            fn declaration(&self) -> &PluginDeclaration {
                &self.declaration
            }

            fn run(&self, _ctx: &PluginContext, _view: &WorldView) -> Vec<Output> {
                vec![]
            }
        }

        #[test]
        fn new_creates_empty_registry() {
            let registry = PluginRegistry::new();
            assert!(registry.is_empty());
            assert_eq!(registry.registration_count(), 0);
        }

        #[test]
        fn default_creates_empty_registry() {
            let registry = PluginRegistry::default();
            assert!(registry.is_empty());
        }

        #[test]
        fn register_single_plugin() {
            let mut registry = PluginRegistry::new();
            let plugin = Arc::new(TestPlugin::new("test", vec![EntityTag::Ship]));

            registry.register(EntityTag::Ship, plugin);

            assert!(!registry.is_empty());
            assert_eq!(registry.registration_count(), 1);
            assert_eq!(registry.plugins_for(EntityTag::Ship).len(), 1);
        }

        #[test]
        fn register_multiple_plugins_same_tag() {
            let mut registry = PluginRegistry::new();
            let plugin1 = Arc::new(TestPlugin::new("movement", vec![EntityTag::Ship]));
            let plugin2 = Arc::new(TestPlugin::new("weapon", vec![EntityTag::Ship]));

            registry.register(EntityTag::Ship, plugin1);
            registry.register(EntityTag::Ship, plugin2);

            assert_eq!(registry.plugins_for(EntityTag::Ship).len(), 2);
            assert_eq!(registry.registration_count(), 2);
        }

        #[test]
        fn register_plugin_multiple_tags() {
            let mut registry = PluginRegistry::new();
            let plugin = Arc::new(TestPlugin::new(
                "movement",
                vec![EntityTag::Ship, EntityTag::Squadron],
            ));

            registry.register(EntityTag::Ship, plugin.clone());
            registry.register(EntityTag::Squadron, plugin);

            assert_eq!(registry.plugins_for(EntityTag::Ship).len(), 1);
            assert_eq!(registry.plugins_for(EntityTag::Squadron).len(), 1);
            assert_eq!(registry.registration_count(), 2);
        }

        #[test]
        fn plugins_for_empty_tag() {
            let registry = PluginRegistry::new();
            let plugins = registry.plugins_for(EntityTag::Ship);
            assert!(plugins.is_empty());
        }

        #[test]
        fn plugins_for_unregistered_tag() {
            let mut registry = PluginRegistry::new();
            let plugin = Arc::new(TestPlugin::new("test", vec![EntityTag::Ship]));
            registry.register(EntityTag::Ship, plugin);

            assert!(registry.plugins_for(EntityTag::Platform).is_empty());
            assert!(registry.plugins_for(EntityTag::Projectile).is_empty());
            assert!(registry.plugins_for(EntityTag::Squadron).is_empty());
        }

        #[test]
        fn clear_removes_all() {
            let mut registry = PluginRegistry::new();
            registry.register(
                EntityTag::Ship,
                Arc::new(TestPlugin::new("a", vec![EntityTag::Ship])),
            );
            registry.register(
                EntityTag::Platform,
                Arc::new(TestPlugin::new("b", vec![EntityTag::Platform])),
            );

            registry.clear();

            assert!(registry.is_empty());
            assert_eq!(registry.registration_count(), 0);
        }

        #[test]
        fn iter_over_bundles() {
            let mut registry = PluginRegistry::new();
            registry.register(
                EntityTag::Ship,
                Arc::new(TestPlugin::new("ship_plugin", vec![EntityTag::Ship])),
            );
            registry.register(
                EntityTag::Platform,
                Arc::new(TestPlugin::new("platform_plugin", vec![EntityTag::Platform])),
            );

            let bundles: Vec<_> = registry.iter().collect();
            assert_eq!(bundles.len(), 2);
        }

        #[test]
        fn debug_format() {
            let mut registry = PluginRegistry::new();
            registry.register(
                EntityTag::Ship,
                Arc::new(TestPlugin::new("test", vec![EntityTag::Ship])),
            );

            let debug = format!("{:?}", registry);
            assert!(debug.contains("PluginRegistry"));
            assert!(debug.contains("bundle_count"));
            assert!(debug.contains("registration_count"));
        }

        #[test]
        fn plugin_declaration_accessible() {
            let mut registry = PluginRegistry::new();
            let plugin = Arc::new(TestPlugin::new("test_plugin", vec![EntityTag::Ship]));
            registry.register(EntityTag::Ship, plugin);

            let plugins = registry.plugins_for(EntityTag::Ship);
            assert_eq!(plugins[0].declaration().id.as_str(), "test_plugin");
        }
    }

    mod plugin_trait_tests {
        use super::*;
        use crate::arena::Arena;
        use crate::entity::{EntityInner, ShipComponents};
        use glam::Vec2;

        struct OutputProducingPlugin {
            declaration: PluginDeclaration,
        }

        impl Plugin for OutputProducingPlugin {
            fn declaration(&self) -> &PluginDeclaration {
                &self.declaration
            }

            fn run(&self, ctx: &PluginContext, view: &WorldView) -> Vec<Output> {
                // Access transform via world view
                if view.get_transform(ctx.entity_id).is_some() {
                    vec![Output::from(crate::output::Command::SetVelocity {
                        target: ctx.entity_id,
                        velocity: Vec2::new(10.0, 0.0),
                    })]
                } else {
                    vec![]
                }
            }
        }

        #[test]
        fn plugin_produces_output() {
            let plugin = OutputProducingPlugin {
                declaration: PluginDeclaration {
                    id: PluginId::new("output_test"),
                    required_tags: vec![EntityTag::Ship],
                    reads: vec![ComponentKind::Transform],
                    emits: vec![OutputKind::Command],
                },
            };

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
            assert_eq!(outputs.len(), 1);
            assert!(outputs[0].is_command());
        }

        #[test]
        fn plugin_is_send_sync() {
            fn assert_send_sync<T: Send + Sync>() {}

            struct DummyPlugin {
                declaration: PluginDeclaration,
            }

            impl Plugin for DummyPlugin {
                fn declaration(&self) -> &PluginDeclaration {
                    &self.declaration
                }
                fn run(&self, _: &PluginContext, _: &WorldView) -> Vec<Output> {
                    vec![]
                }
            }

            // This will fail at compile time if Plugin is not Send + Sync
            assert_send_sync::<DummyPlugin>();
        }
    }
}
