//! Field propagation: diffusion and decay.
//!
//! This module provides functions for evolving field values over time through
//! physical processes like heat diffusion and signal decay.

use glam::Vec3;

use crate::field::{Field, FieldValues, Propagation};
use crate::octree::Direction;
use crate::universe::Universe;

/// Propagate all fields for one timestep.
///
/// This is the core propagation step that applies diffusion and decay to all
/// fields based on their configuration. It operates in three phases:
///
/// 1. **Collect**: Gather all leaf nodes from the octree (deterministic order)
/// 2. **Compute**: Calculate new values for each leaf based on propagation rules
/// 3. **Apply**: Write updated values back to the octree
///
/// This separation ensures determinism by reading from a frozen snapshot before
/// any writes occur.
pub fn propagate_all(universe: &mut Universe, dt: f64) {
    let dt_f32 = dt as f32;

    // Phase 1: Collect all leaves
    let leaves = universe.octree().collect_leaves();

    if leaves.is_empty() {
        return;
    }

    // Phase 2: Compute updates for each leaf
    let updates: Vec<(Vec3, FieldValues)> = leaves
        .iter()
        .map(|(pos, old_values)| {
            let mut new_values = *old_values;

            for field in Field::all() {
                let config = universe.field_config(*field);
                let old_val = old_values.get(*field);

                let new_val = match config.propagation {
                    Propagation::None => old_val,
                    Propagation::Diffusion { rate } => {
                        let neighbors = get_xy_neighbor_values(universe, *pos, *field);
                        apply_diffusion(old_val, &neighbors, rate, dt_f32)
                    }
                    Propagation::Decay { rate } => {
                        apply_decay(old_val, config.default_value, rate, dt_f32)
                    }
                    Propagation::DiffusionDecay {
                        diffusion_rate,
                        decay_rate,
                    } => {
                        let neighbors = get_xy_neighbor_values(universe, *pos, *field);
                        let diffused = apply_diffusion(old_val, &neighbors, diffusion_rate, dt_f32);
                        apply_decay(diffused, config.default_value, decay_rate, dt_f32)
                    }
                };

                new_values.set(*field, config.clamp(new_val));
            }

            (*pos, new_values)
        })
        .collect();

    // Phase 3: Apply updates
    for (pos, values) in updates {
        universe.set_point(pos, values);
    }
}

/// Get neighbor field values in the XY plane (4 neighbors).
///
/// Returns the field values from up to 4 neighbors (PosX, NegX, PosY, NegY).
/// For neighbors outside world bounds, the field's configured default value is used.
/// For empty cells within bounds, the queried value (which may be interpolated) is used.
fn get_xy_neighbor_values(universe: &Universe, pos: Vec3, field: Field) -> Vec<f32> {
    let default_value = universe.field_config(field).default_value;

    Direction::xy_directions()
        .iter()
        .map(|dir| {
            universe
                .octree()
                .find_neighbor(pos, *dir)
                .map(|values| values.get(field))
                .unwrap_or(default_value) // Out-of-bounds uses default
        })
        .collect()
}

/// Apply exponential decay toward a default value.
///
/// Models exponential decay where values approach `default` over time.
///
/// # Formula
/// `default + (old_value - default) * exp(-rate * dt)`
///
/// # Arguments
/// * `old_value` - Current field value
/// * `default` - Equilibrium value to decay toward
/// * `rate` - Decay rate (higher = faster decay)
/// * `dt` - Time step in seconds
///
/// # Returns
/// New field value after decay
#[must_use]
pub fn apply_decay(old_value: f32, default: f32, rate: f32, dt: f32) -> f32 {
    default + (old_value - default) * (-rate * dt).exp()
}

/// Apply discrete Laplacian diffusion.
///
/// Models diffusion where values spread to equalize with neighbors.
///
/// # Formula
/// `old_value + rate * dt * (sum(neighbors) - n * old_value)`
///
/// where `n` is the number of neighbors.
///
/// # Arguments
/// * `old_value` - Current field value at this cell
/// * `neighbor_values` - Field values of neighboring cells
/// * `rate` - Diffusion rate (higher = faster spreading)
/// * `dt` - Time step in seconds
///
/// # Returns
/// New field value after diffusion
#[must_use]
#[allow(clippy::cast_precision_loss)] // Neighbor count is small (typically 4-6)
pub fn apply_diffusion(old_value: f32, neighbor_values: &[f32], rate: f32, dt: f32) -> f32 {
    let n = neighbor_values.len() as f32;
    let neighbor_sum: f32 = neighbor_values.iter().sum();
    old_value + rate * dt * (neighbor_sum - n * old_value)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-6;

    #[test]
    fn test_decay_reduces_value() {
        // A hot value (500K) should decay toward ambient (293K)
        let old_value = 500.0;
        let default = 293.0;
        let rate = 0.1;
        let dt = 1.0;

        let new_value = apply_decay(old_value, default, rate, dt);

        // Value should move toward default
        assert!(new_value < old_value, "Value should decrease toward default");
        assert!(new_value > default, "Value should still be above default");

        // Check the formula: default + (old - default) * exp(-rate * dt)
        let expected = default + (old_value - default) * (-rate * dt).exp();
        assert!(
            (new_value - expected).abs() < EPSILON,
            "Expected {expected}, got {new_value}"
        );
    }

    #[test]
    fn test_decay_at_default_is_stable() {
        // A value already at default should stay at default
        let default = 293.0;
        let rate = 0.5;
        let dt = 10.0;

        let new_value = apply_decay(default, default, rate, dt);

        assert!(
            (new_value - default).abs() < EPSILON,
            "Value at default should remain stable, got {new_value}"
        );
    }

    #[test]
    fn test_diffusion_spreads_heat() {
        // Hot center (500K) surrounded by cold neighbors (293K) should cool down
        let old_value = 500.0;
        let neighbor_values = [293.0, 293.0, 293.0, 293.0]; // 4 neighbors
        let rate = 0.1;
        let dt = 1.0;

        let new_value = apply_diffusion(old_value, &neighbor_values, rate, dt);

        // Hot center should cool toward neighbors
        assert!(
            new_value < old_value,
            "Hot center should cool down, got {new_value}"
        );

        // Check the formula: old + rate * dt * (sum(neighbors) - n * old)
        let n = neighbor_values.len() as f32;
        let neighbor_sum: f32 = neighbor_values.iter().sum();
        let expected = old_value + rate * dt * (neighbor_sum - n * old_value);
        assert!(
            (new_value - expected).abs() < EPSILON,
            "Expected {expected}, got {new_value}"
        );
    }

    #[test]
    fn test_diffusion_uniform_is_stable() {
        // When all values are the same, diffusion should not change anything
        let uniform_value = 350.0;
        let neighbor_values = [uniform_value, uniform_value, uniform_value, uniform_value];
        let rate = 0.5;
        let dt = 1.0;

        let new_value = apply_diffusion(uniform_value, &neighbor_values, rate, dt);

        assert!(
            (new_value - uniform_value).abs() < EPSILON,
            "Uniform field should remain stable, got {new_value}"
        );
    }
}
