//! MVP plugins for the Entity-Plugin-Resolver architecture.
//!
//! This module provides the core plugins for the Tidebreak combat simulation:
//!
//! - [`MovementPlugin`]: Handles entity movement (placeholder for AI/player input)
//! - [`SensorPlugin`]: Detects nearby entities and emits contact events
//! - [`WeaponPlugin`]: Fires weapons at tracked targets
//! - [`ProjectilePlugin`]: Handles projectile behavior
//!
//! # Architecture
//!
//! Plugins follow the Entity-Plugin-Resolver pattern:
//! - Plugins read from an immutable [`WorldView`](crate::world_view::WorldView)
//! - Plugins emit [`Output`](crate::output::Output)s as proposals for state changes
//! - Resolvers collect and process outputs to mutate state
//!
//! # Registration
//!
//! Use [`PluginRegistry::default_bundles()`](crate::plugin::PluginRegistry::default_bundles)
//! to create a registry with all MVP plugins pre-registered for their appropriate
//! entity types.

mod movement;
mod projectile;
mod sensor;
mod weapon;

pub use movement::MovementPlugin;
pub use projectile::ProjectilePlugin;
pub use sensor::SensorPlugin;
pub use weapon::WeaponPlugin;
