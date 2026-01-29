//! # Murk
//!
//! Hierarchical spatial substrate for agent perception, environmental simulation,
//! and DRL training.
//!
//! Murk represents the world as continuous scalar fields stored in a sparse octree
//! with statistical summaries at each level. This enables:
//!
//! - **Scale-aware queries**: High resolution nearby, low resolution far away
//! - **Efficient memory**: Sparse storage means empty/uniform space costs nothing
//! - **Fast updates**: Localized "stamps" modify fields without full traversal
//! - **Field propagation**: Diffusion, decay for phenomena like heat, smoke, sound
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use murk::{Universe, Field, Stamp, SphereShape};
//!
//! // Create a universe
//! let mut universe = Universe::new(UniverseConfig {
//!     bounds: Bounds::new(1024.0, 1024.0, 256.0),
//!     base_resolution: 1.0,
//!     ..Default::default()
//! });
//!
//! // Apply an explosion (stamp)
//! universe.stamp(Stamp {
//!     shape: SphereShape::new(Vec3::new(500.0, 500.0, 20.0), 15.0).into(),
//!     modifications: vec![
//!         FieldMod::new(Field::Temperature, BlendOp::Add, 500.0),
//!         FieldMod::new(Field::Noise, BlendOp::Add, 120.0),
//!     ],
//! });
//!
//! // Query a region
//! let stats = universe.query_volume(
//!     Vec3::new(500.0, 500.0, 30.0),
//!     50.0,
//!     QueryResolution::Coarse,
//! );
//! println!("Average temperature: {}", stats.temperature.mean);
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod field;
pub mod hash;
pub mod node;
pub mod octree;
pub mod query;
pub mod stamp;
pub mod stats;
pub mod universe;

// Re-exports for convenience
pub use field::{Field, FieldConfig, FieldValues};
pub use hash::hash_universe;
pub use node::{NodeState, OctreeNode};
pub use octree::Octree;
pub use query::{QueryResolution, VolumeQuery};
pub use stamp::{BlendOp, FieldMod, Stamp, StampShape};
pub use stats::{FieldStats, ScalarStats};
pub use universe::{Universe, UniverseConfig};

/// Axis-aligned bounding box.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Bounds {
    /// Minimum corner
    pub min: glam::Vec3,
    /// Maximum corner
    pub max: glam::Vec3,
}

impl Bounds {
    /// Create bounds from dimensions (centered at origin).
    #[must_use]
    pub fn new(width: f32, height: f32, depth: f32) -> Self {
        Self {
            min: glam::Vec3::new(-width / 2.0, -height / 2.0, -depth / 2.0),
            max: glam::Vec3::new(width / 2.0, height / 2.0, depth / 2.0),
        }
    }

    /// Create bounds from min/max corners.
    #[must_use]
    pub fn from_min_max(min: glam::Vec3, max: glam::Vec3) -> Self {
        Self { min, max }
    }

    /// Get the center of the bounds.
    #[must_use]
    pub fn center(&self) -> glam::Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Get the size of the bounds.
    #[must_use]
    pub fn size(&self) -> glam::Vec3 {
        self.max - self.min
    }

    /// Check if a point is inside the bounds.
    #[must_use]
    pub fn contains(&self, point: glam::Vec3) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }

    /// Check if this bounds intersects a sphere.
    #[must_use]
    pub fn intersects_sphere(&self, center: glam::Vec3, radius: f32) -> bool {
        let closest = glam::Vec3::new(
            center.x.clamp(self.min.x, self.max.x),
            center.y.clamp(self.min.y, self.max.y),
            center.z.clamp(self.min.z, self.max.z),
        );
        center.distance_squared(closest) <= radius * radius
    }

    /// Check if a sphere fully contains this bounds.
    #[must_use]
    pub fn is_fully_inside_sphere(&self, center: glam::Vec3, radius: f32) -> bool {
        // Check all 8 corners
        let r2 = radius * radius;
        let corners = [
            glam::Vec3::new(self.min.x, self.min.y, self.min.z),
            glam::Vec3::new(self.max.x, self.min.y, self.min.z),
            glam::Vec3::new(self.min.x, self.max.y, self.min.z),
            glam::Vec3::new(self.max.x, self.max.y, self.min.z),
            glam::Vec3::new(self.min.x, self.min.y, self.max.z),
            glam::Vec3::new(self.max.x, self.min.y, self.max.z),
            glam::Vec3::new(self.min.x, self.max.y, self.max.z),
            glam::Vec3::new(self.max.x, self.max.y, self.max.z),
        ];
        corners
            .iter()
            .all(|&c| center.distance_squared(c) <= r2)
    }

    /// Get the octant index for a point (0-7).
    #[must_use]
    pub fn octant_index(&self, point: glam::Vec3) -> usize {
        let center = self.center();
        let mut index = 0;
        if point.x >= center.x {
            index |= 1;
        }
        if point.y >= center.y {
            index |= 2;
        }
        if point.z >= center.z {
            index |= 4;
        }
        index
    }

    /// Get the bounds of a child octant.
    #[must_use]
    pub fn child_bounds(&self, octant: usize) -> Self {
        let center = self.center();
        let min = glam::Vec3::new(
            if octant & 1 == 0 { self.min.x } else { center.x },
            if octant & 2 == 0 { self.min.y } else { center.y },
            if octant & 4 == 0 { self.min.z } else { center.z },
        );
        let max = glam::Vec3::new(
            if octant & 1 == 0 { center.x } else { self.max.x },
            if octant & 2 == 0 { center.y } else { self.max.y },
            if octant & 4 == 0 { center.z } else { self.max.z },
        );
        Self { min, max }
    }
}

impl Default for Bounds {
    fn default() -> Self {
        Self::new(100.0, 100.0, 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounds_contains() {
        let bounds = Bounds::new(10.0, 10.0, 10.0);
        assert!(bounds.contains(glam::Vec3::ZERO));
        assert!(bounds.contains(glam::Vec3::new(4.0, 4.0, 4.0)));
        assert!(!bounds.contains(glam::Vec3::new(10.0, 0.0, 0.0)));
    }

    #[test]
    fn test_bounds_octant() {
        let bounds = Bounds::new(10.0, 10.0, 10.0);
        assert_eq!(bounds.octant_index(glam::Vec3::new(-1.0, -1.0, -1.0)), 0);
        assert_eq!(bounds.octant_index(glam::Vec3::new(1.0, -1.0, -1.0)), 1);
        assert_eq!(bounds.octant_index(glam::Vec3::new(-1.0, 1.0, -1.0)), 2);
        assert_eq!(bounds.octant_index(glam::Vec3::new(1.0, 1.0, 1.0)), 7);
    }

    #[test]
    fn test_child_bounds() {
        let bounds = Bounds::new(10.0, 10.0, 10.0);
        let child = bounds.child_bounds(0);
        assert_eq!(child.min, glam::Vec3::new(-5.0, -5.0, -5.0));
        assert_eq!(child.max, glam::Vec3::new(0.0, 0.0, 0.0));
    }
}
