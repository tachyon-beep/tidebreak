//! State hashing for determinism verification.
//!
//! This module provides functions to compute deterministic hashes of universe state.
//! Two universes with identical inputs must produce identical state hashes.
//! This is used to verify determinism per ADR-0003.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::field::{Field, FieldValues};
use crate::node::{NodeState, OctreeNode};
use crate::stats::{FieldStats, ScalarStats};
use crate::Universe;

/// Compute a deterministic hash of universe state.
///
/// This hash includes:
/// - Current tick and time
/// - Octree structure and all field values
///
/// Two universes with identical operations from the same seed
/// will produce identical hashes.
#[must_use]
pub fn hash_universe(universe: &Universe) -> u64 {
    let mut hasher = DefaultHasher::new();

    // Hash tick
    universe.tick().hash(&mut hasher);

    // Hash time as bits to avoid float comparison issues
    universe.time().to_bits().hash(&mut hasher);

    // Hash seed (if present)
    universe.seed().hash(&mut hasher);

    // Hash octree state by traversing the tree
    hash_octree_node(universe.octree().root(), &mut hasher);

    hasher.finish()
}

/// Hash a single octree node and recursively hash its children.
fn hash_octree_node<H: Hasher>(node: &OctreeNode, hasher: &mut H) {
    // Hash node metadata
    node.depth.hash(hasher);

    // Hash bounds (as bits for deterministic float hashing)
    hash_bounds(&node.bounds, hasher);

    // Hash node state
    match &node.state {
        NodeState::Empty => {
            0u8.hash(hasher); // Discriminant for Empty
        }
        NodeState::Leaf { values } => {
            1u8.hash(hasher); // Discriminant for Leaf
            hash_field_values(values, hasher);
        }
        NodeState::Internal { children, stats } => {
            2u8.hash(hasher); // Discriminant for Internal

            // Hash stats
            hash_field_stats(stats, hasher);

            // Hash children in deterministic order (0..7)
            for (i, child) in children.iter().enumerate() {
                i.hash(hasher);
                match child {
                    Some(child_node) => {
                        true.hash(hasher);
                        hash_octree_node(child_node, hasher);
                    }
                    None => {
                        false.hash(hasher);
                    }
                }
            }
        }
    }
}

/// Hash bounds by converting floats to bits.
fn hash_bounds<H: Hasher>(bounds: &crate::Bounds, hasher: &mut H) {
    bounds.min.x.to_bits().hash(hasher);
    bounds.min.y.to_bits().hash(hasher);
    bounds.min.z.to_bits().hash(hasher);
    bounds.max.x.to_bits().hash(hasher);
    bounds.max.y.to_bits().hash(hasher);
    bounds.max.z.to_bits().hash(hasher);
}

/// Hash field values by converting each f32 to bits.
fn hash_field_values<H: Hasher>(values: &FieldValues, hasher: &mut H) {
    for field in Field::all() {
        values.get(*field).to_bits().hash(hasher);
    }
}

/// Hash field statistics.
fn hash_field_stats<H: Hasher>(stats: &FieldStats, hasher: &mut H) {
    // Hash scalar stats for each field
    for scalar_stats in &stats.scalars {
        hash_scalar_stats(scalar_stats, hasher);
    }

    // Hash material stats
    stats.material.mode.hash(hasher);
    stats.material.mode_count.hash(hasher);
    stats.material.sample_count.hash(hasher);
    for (material, count) in &stats.material.distribution {
        material.hash(hasher);
        count.hash(hasher);
    }
}

/// Hash scalar statistics.
fn hash_scalar_stats<H: Hasher>(stats: &ScalarStats, hasher: &mut H) {
    stats.mean.to_bits().hash(hasher);
    stats.variance.to_bits().hash(hasher);
    stats.min.to_bits().hash(hasher);
    stats.max.to_bits().hash(hasher);
    stats.sample_count.hash(hasher);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stamp::Stamp;
    use crate::UniverseConfig;
    use glam::Vec3;

    #[test]
    fn test_hash_empty_universes() {
        let config = UniverseConfig::with_bounds(100.0, 100.0, 50.0);
        let u1 = Universe::new_with_seed(config.clone(), 42);
        let u2 = Universe::new_with_seed(config, 42);

        assert_eq!(hash_universe(&u1), hash_universe(&u2));
    }

    #[test]
    fn test_hash_different_seeds() {
        let config = UniverseConfig::with_bounds(100.0, 100.0, 50.0);
        let u1 = Universe::new_with_seed(config.clone(), 42);
        let u2 = Universe::new_with_seed(config, 43);

        assert_ne!(hash_universe(&u1), hash_universe(&u2));
    }

    #[test]
    fn test_hash_after_stamp() {
        let config = UniverseConfig::with_bounds(100.0, 100.0, 50.0);
        let mut u1 = Universe::new_with_seed(config.clone(), 42);
        let mut u2 = Universe::new_with_seed(config, 42);

        let stamp = Stamp::explosion(Vec3::ZERO, 10.0, 1.0);
        u1.stamp(&stamp);
        u2.stamp(&stamp);

        assert_eq!(hash_universe(&u1), hash_universe(&u2));
    }

    #[test]
    fn test_hash_changes_after_step() {
        let config = UniverseConfig::with_bounds(100.0, 100.0, 50.0);
        let mut universe = Universe::new_with_seed(config, 42);

        let hash_before = hash_universe(&universe);
        universe.step(0.1);
        let hash_after = hash_universe(&universe);

        assert_ne!(hash_before, hash_after);
    }

    #[test]
    fn test_hash_different_stamps_differ() {
        let config = UniverseConfig::with_bounds(100.0, 100.0, 50.0);
        let mut u1 = Universe::new_with_seed(config.clone(), 42);
        let mut u2 = Universe::new_with_seed(config, 42);

        u1.stamp(&Stamp::explosion(Vec3::ZERO, 10.0, 1.0));
        u2.stamp(&Stamp::explosion(Vec3::new(50.0, 0.0, 0.0), 10.0, 1.0));

        assert_ne!(hash_universe(&u1), hash_universe(&u2));
    }
}
