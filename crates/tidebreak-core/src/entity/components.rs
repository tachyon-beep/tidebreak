//! Forward-declared component structs for entity types.
//!
//! These are placeholder structs that will be filled in by Task 21 with actual
//! state components (TransformState, PhysicsState, etc.).
//!
//! The component structs hold all state for a particular entity type.

use serde::{Deserialize, Serialize};

/// Components for Ship entities.
///
/// Ships are the primary naval units in combat, ranging from jetskis to mobile
/// city-ships. They have full physics, combat, sensor, and inventory systems.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ShipComponents {
    // Will contain: transform, physics, combat, sensor, inventory, track_table
    // Placeholder field to prevent "never constructed" warning
    _placeholder: (),
}

/// Components for Platform entities.
///
/// Platforms are static or semi-static installations (buoys, oil rigs, bases).
/// They have position and potentially sensors/weapons but limited mobility.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PlatformComponents {
    // Will contain: transform, combat (optional), sensor (optional)
    _placeholder: (),
}

/// Components for Projectile entities.
///
/// Projectiles are in-flight weapons (missiles, torpedoes, shells). They have
/// physics and targeting but no inventory or complex systems.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProjectileComponents {
    // Will contain: transform, physics, guidance, warhead
    _placeholder: (),
}

/// Components for Squadron entities.
///
/// Squadrons are groups of aircraft or small craft that operate as a unit.
/// They have aggregate state rather than individual member tracking.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SquadronComponents {
    // Will contain: transform, formation, mission, aggregate_state
    _placeholder: (),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_structs_are_default() {
        let _ship = ShipComponents::default();
        let _platform = PlatformComponents::default();
        let _projectile = ProjectileComponents::default();
        let _squadron = SquadronComponents::default();
    }

    #[test]
    fn component_structs_are_serializable() {
        let ship = ShipComponents::default();
        let json = serde_json::to_string(&ship).unwrap();
        let _deserialized: ShipComponents = serde_json::from_str(&json).unwrap();
    }
}
