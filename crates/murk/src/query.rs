//! Query interface for Murk.
//!
//! Queries specify a region and resolution, returning statistical summaries
//! that can trade accuracy for speed.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::field::Field;
use crate::stats::{FieldStats, ScalarStats};
use crate::Bounds;

/// Resolution specification for queries.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum QueryResolution {
    /// Use a specific tree depth (0 = root only, higher = more detail)
    Depth(u8),
    /// Stop when variance is below threshold
    Variance(f32),
    /// Coarse preset (depth 2-3)
    Coarse,
    /// Medium preset (depth 4-5)
    Medium,
    /// Fine preset (depth 6-7)
    Fine,
    /// Maximum detail (traverse to leaves)
    Full,
}

impl QueryResolution {
    /// Get the maximum depth for this resolution.
    #[must_use]
    pub fn max_depth(&self, tree_max_depth: u8) -> u8 {
        match self {
            QueryResolution::Depth(d) => *d,
            QueryResolution::Variance(_) => tree_max_depth, // Will stop when variance threshold met
            QueryResolution::Coarse => 3.min(tree_max_depth),
            QueryResolution::Medium => 5.min(tree_max_depth),
            QueryResolution::Fine => 7.min(tree_max_depth),
            QueryResolution::Full => tree_max_depth,
        }
    }

    /// Get the variance threshold (if any).
    #[must_use]
    pub fn variance_threshold(&self) -> Option<f32> {
        match self {
            QueryResolution::Variance(v) => Some(*v),
            _ => None,
        }
    }
}

impl Default for QueryResolution {
    fn default() -> Self {
        Self::Medium
    }
}

/// Volume query specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeQuery {
    /// Center of query region
    pub center: Vec3,
    /// Radius of query sphere
    pub radius: f32,
    /// Resolution/accuracy tradeoff
    pub resolution: QueryResolution,
    /// Optional: only query specific fields
    pub fields: Option<Vec<Field>>,
}

impl VolumeQuery {
    /// Create a new volume query.
    #[must_use]
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self {
            center,
            radius,
            resolution: QueryResolution::default(),
            fields: None,
        }
    }

    /// Set resolution.
    #[must_use]
    pub fn with_resolution(mut self, resolution: QueryResolution) -> Self {
        self.resolution = resolution;
        self
    }

    /// Set specific fields to query.
    #[must_use]
    pub fn with_fields(mut self, fields: Vec<Field>) -> Self {
        self.fields = Some(fields);
        self
    }

    /// Get the bounding box of this query.
    #[must_use]
    pub fn bounds(&self) -> Bounds {
        Bounds::from_min_max(
            self.center - Vec3::splat(self.radius),
            self.center + Vec3::splat(self.radius),
        )
    }
}

/// Result of a volume query.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryResult {
    /// Field statistics for the queried region
    pub stats: FieldStats,
    /// Number of nodes visited
    pub nodes_visited: u32,
    /// Maximum depth reached
    pub max_depth_reached: u8,
}

impl QueryResult {
    /// Get mean value for a field.
    #[must_use]
    pub fn mean(&self, field: Field) -> f32 {
        self.stats.get(field).mean
    }

    /// Get variance for a field.
    #[must_use]
    pub fn variance(&self, field: Field) -> f32 {
        self.stats.get(field).variance
    }

    /// Get min value for a field.
    #[must_use]
    pub fn min(&self, field: Field) -> f32 {
        self.stats.get(field).min
    }

    /// Get max value for a field.
    #[must_use]
    pub fn max(&self, field: Field) -> f32 {
        self.stats.get(field).max
    }

    /// Get the full scalar stats for a field.
    #[must_use]
    pub fn field_stats(&self, field: Field) -> &ScalarStats {
        self.stats.get(field)
    }
}

/// Point query (single location).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointQuery {
    /// Location to query
    pub position: Vec3,
    /// Optional: only query specific fields
    pub fields: Option<Vec<Field>>,
}

impl PointQuery {
    /// Create a new point query.
    #[must_use]
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            fields: None,
        }
    }

    /// Set specific fields to query.
    #[must_use]
    pub fn with_fields(mut self, fields: Vec<Field>) -> Self {
        self.fields = Some(fields);
        self
    }
}

/// Result of a point query.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PointResult {
    /// Field values at the queried point
    pub values: crate::field::FieldValues,
    /// Depth at which the value was found
    pub depth: u8,
    /// Whether the value is interpolated (vs exact leaf value)
    pub interpolated: bool,
}

impl PointResult {
    /// Get value for a field.
    #[must_use]
    pub fn get(&self, field: Field) -> f32 {
        self.values.get(field)
    }
}

/// Foveated observation shell for agent perception.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoveatedShell {
    /// Inner radius of shell
    pub radius_inner: f32,
    /// Outer radius of shell
    pub radius_outer: f32,
    /// Number of angular divisions (sectors)
    pub sectors: u32,
    /// Resolution for queries in this shell
    pub resolution: QueryResolution,
}

impl FoveatedShell {
    /// Create a new shell.
    #[must_use]
    pub fn new(radius_inner: f32, radius_outer: f32, sectors: u32) -> Self {
        Self {
            radius_inner,
            radius_outer,
            sectors,
            resolution: QueryResolution::Medium,
        }
    }

    /// Set resolution.
    #[must_use]
    pub fn with_resolution(mut self, resolution: QueryResolution) -> Self {
        self.resolution = resolution;
        self
    }
}

/// Foveated observation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoveatedQuery {
    /// Agent position
    pub position: Vec3,
    /// Agent heading (for sector orientation)
    pub heading: Vec3,
    /// Shells from inner to outer
    pub shells: Vec<FoveatedShell>,
    /// Fields to include in observation
    pub fields: Vec<Field>,
}

impl FoveatedQuery {
    /// Create a new foveated query with default shells.
    #[must_use]
    pub fn new(position: Vec3, heading: Vec3) -> Self {
        Self {
            position,
            heading,
            shells: vec![
                FoveatedShell::new(0.0, 10.0, 16).with_resolution(QueryResolution::Fine),
                FoveatedShell::new(10.0, 50.0, 8).with_resolution(QueryResolution::Medium),
                FoveatedShell::new(50.0, 200.0, 4).with_resolution(QueryResolution::Coarse),
            ],
            fields: vec![
                Field::Temperature,
                Field::Noise,
                Field::Occupancy,
                Field::SonarReturn,
            ],
        }
    }

    /// Set shells.
    #[must_use]
    pub fn with_shells(mut self, shells: Vec<FoveatedShell>) -> Self {
        self.shells = shells;
        self
    }

    /// Set fields.
    #[must_use]
    pub fn with_fields(mut self, fields: Vec<Field>) -> Self {
        self.fields = fields;
        self
    }
}

/// Result of a foveated observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoveatedResult {
    /// Per-shell, per-sector statistics
    /// Shape: [num_shells][num_sectors]
    pub shell_stats: Vec<Vec<FieldStats>>,
    /// Total nodes visited
    pub nodes_visited: u32,
}

impl FoveatedResult {
    /// Get the observation as a flat vector (for neural networks).
    ///
    /// Layout: [shell0_sector0_field0, shell0_sector0_field1, ..., shell0_sector1_field0, ...]
    #[must_use]
    pub fn to_flat_vec(&self, fields: &[Field]) -> Vec<f32> {
        let mut result = Vec::new();
        for shell in &self.shell_stats {
            for sector_stats in shell {
                for field in fields {
                    result.push(sector_stats.get(*field).mean);
                }
            }
        }
        result
    }

    /// Get shape of the observation tensor.
    #[must_use]
    pub fn shape(&self, num_fields: usize) -> (usize, usize, usize) {
        let num_shells = self.shell_stats.len();
        let num_sectors = self.shell_stats.first().map_or(0, |s| s.len());
        (num_shells, num_sectors, num_fields)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_query_bounds() {
        let query = VolumeQuery::new(Vec3::new(100.0, 100.0, 50.0), 25.0);
        let bounds = query.bounds();
        assert_eq!(bounds.min, Vec3::new(75.0, 75.0, 25.0));
        assert_eq!(bounds.max, Vec3::new(125.0, 125.0, 75.0));
    }

    #[test]
    fn test_resolution_max_depth() {
        assert_eq!(QueryResolution::Coarse.max_depth(10), 3);
        assert_eq!(QueryResolution::Depth(5).max_depth(10), 5);
        assert_eq!(QueryResolution::Full.max_depth(10), 10);
    }
}
