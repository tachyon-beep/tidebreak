//! # Tidebreak Core
//!
//! Combat Arena core simulation for Tidebreak.
//!
//! This crate provides the deterministic combat simulation engine, implementing
//! the Entity-Plugin-Resolver architecture for tactical naval battles.
//!
//! ## Architecture
//!
//! See ADR-0001 for the Entity-Plugin-Resolver pattern.
//!
//! - **Entities**: Ships, weapons, projectiles, platforms
//! - **Plugins**: Sensors, weapons, movement, damage control
//! - **Resolvers**: Physics, combat, detection, damage
//!
//! ## Usage
//!
//! ```rust,ignore
//! use tidebreak_core::{Arena, BattlePackage};
//!
//! let arena = Arena::new();
//! let package = BattlePackage::load("battle.json")?;
//! let result = arena.simulate(package)?;
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]

// Re-export murk for spatial queries
pub use murk;

// Core modules
pub mod arena;
pub mod entity;
pub mod output;
pub mod plugin;
pub mod plugins;
pub mod resolver;
pub mod simulation;
pub mod world_view;

// Placeholder modules - to be implemented
// pub mod contracts;

// Re-exports for convenience
pub use arena::{Arena, SpatialIndex};
pub use output::PluginId;
pub use plugin::{ComponentKind, Plugin, PluginContext, PluginDeclaration, PluginRegistry};
pub use plugins::{MovementPlugin, ProjectilePlugin, SensorPlugin, WeaponPlugin};
pub use resolver::{CombatResolver, EventResolver, PhysicsResolver, Resolver};
pub use simulation::Simulation;
pub use world_view::WorldView;

// Test modules
#[cfg(test)]
mod tests;

#[cfg(test)]
mod basic_tests {
    use super::*;
    use crate::entity::{EntityInner, EntityTag, ShipComponents};

    #[test]
    fn test_arena_creation() {
        let _arena = Arena::new();
    }

    #[test]
    fn test_arena_spawn_and_get() {
        let mut arena = Arena::new();
        let id = arena.spawn(
            EntityTag::Ship,
            EntityInner::Ship(ShipComponents::default()),
        );
        assert!(arena.get(id).is_some());
    }
}
