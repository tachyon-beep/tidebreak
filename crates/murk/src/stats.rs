//! Statistical aggregates for hierarchical compression.
//!
//! Internal octree nodes store statistical summaries of their children,
//! enabling cheap large-scale queries without traversing to leaves.

use serde::{Deserialize, Serialize};

use crate::field::Field;

/// Statistics for a single scalar field.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ScalarStats {
    /// Arithmetic mean
    pub mean: f32,
    /// Variance (σ²)
    pub variance: f32,
    /// Minimum value
    pub min: f32,
    /// Maximum value
    pub max: f32,
    /// Number of samples contributing to these stats
    pub sample_count: u32,
}

impl ScalarStats {
    /// Create stats from a single value.
    #[must_use]
    pub fn from_value(value: f32) -> Self {
        Self {
            mean: value,
            variance: 0.0,
            min: value,
            max: value,
            sample_count: 1,
        }
    }

    /// Create empty stats.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            mean: 0.0,
            variance: 0.0,
            min: f32::INFINITY,
            max: f32::NEG_INFINITY,
            sample_count: 0,
        }
    }

    /// Merge two stats using weighted combination.
    ///
    /// Uses Welford's online algorithm for combining variances.
    #[must_use]
    pub fn merge(a: &Self, b: &Self) -> Self {
        if a.sample_count == 0 {
            return *b;
        }
        if b.sample_count == 0 {
            return *a;
        }

        let n_a = a.sample_count as f32;
        let n_b = b.sample_count as f32;
        let n_total = n_a + n_b;

        let delta = b.mean - a.mean;
        let mean = a.mean + delta * (n_b / n_total);

        // Combined variance using parallel algorithm
        let variance = (a.variance * n_a + b.variance * n_b + delta * delta * n_a * n_b / n_total)
            / n_total;

        Self {
            mean,
            variance,
            min: a.min.min(b.min),
            max: a.max.max(b.max),
            sample_count: a.sample_count + b.sample_count,
        }
    }

    /// Merge multiple stats.
    #[must_use]
    pub fn merge_many(stats: &[Self]) -> Self {
        stats
            .iter()
            .fold(Self::empty(), |acc, s| Self::merge(&acc, s))
    }

    /// Standard deviation.
    #[must_use]
    pub fn std_dev(&self) -> f32 {
        self.variance.sqrt()
    }

    /// Check if variance is below threshold (uniform enough to skip detail).
    #[must_use]
    pub fn is_uniform(&self, threshold: f32) -> bool {
        self.variance < threshold
    }
}

/// Statistics for a material/enum field (tracks mode/distribution).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MaterialStats {
    /// Most common material ID
    pub mode: u8,
    /// Count of the most common material
    pub mode_count: u32,
    /// Total sample count
    pub sample_count: u32,
    /// Distribution (sparse: only materials with non-zero count)
    /// Limited to top N for memory efficiency
    pub distribution: [(u8, u32); 4],
}

impl MaterialStats {
    /// Create stats from a single material.
    #[must_use]
    pub fn from_value(material: u8) -> Self {
        Self {
            mode: material,
            mode_count: 1,
            sample_count: 1,
            distribution: [(material, 1), (0, 0), (0, 0), (0, 0)],
        }
    }

    /// Create empty stats.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Fraction of samples that are the mode.
    #[must_use]
    pub fn mode_fraction(&self) -> f32 {
        if self.sample_count == 0 {
            0.0
        } else {
            self.mode_count as f32 / self.sample_count as f32
        }
    }

    /// Check if material is uniform (single material dominates).
    #[must_use]
    pub fn is_uniform(&self, threshold: f32) -> bool {
        self.mode_fraction() >= threshold
    }
}

/// Complete statistics for all fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FieldStats {
    /// Stats for scalar fields
    pub scalars: [ScalarStats; Field::COUNT],
    /// Stats for material field (special handling)
    pub material: MaterialStats,
}

impl FieldStats {
    /// Create empty stats.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            scalars: [ScalarStats::empty(); Field::COUNT],
            material: MaterialStats::empty(),
        }
    }

    /// Create stats from field values.
    #[must_use]
    pub fn from_values(values: &crate::field::FieldValues) -> Self {
        let mut scalars = [ScalarStats::empty(); Field::COUNT];
        for field in Field::all() {
            scalars[field.index()] = ScalarStats::from_value(values.get(*field));
        }

        Self {
            scalars,
            material: MaterialStats::from_value(values.get(Field::Material) as u8),
        }
    }

    /// Get stats for a specific field.
    #[must_use]
    pub fn get(&self, field: Field) -> &ScalarStats {
        &self.scalars[field.index()]
    }

    /// Merge two field stats.
    #[must_use]
    pub fn merge(a: &Self, b: &Self) -> Self {
        let mut scalars = [ScalarStats::empty(); Field::COUNT];
        for i in 0..Field::COUNT {
            scalars[i] = ScalarStats::merge(&a.scalars[i], &b.scalars[i]);
        }

        // Material stats merging is more complex; simplified here
        let material = if a.material.mode_count >= b.material.mode_count {
            MaterialStats {
                mode: a.material.mode,
                mode_count: a.material.mode_count,
                sample_count: a.material.sample_count + b.material.sample_count,
                distribution: a.material.distribution, // Simplified
            }
        } else {
            MaterialStats {
                mode: b.material.mode,
                mode_count: b.material.mode_count,
                sample_count: a.material.sample_count + b.material.sample_count,
                distribution: b.material.distribution,
            }
        };

        Self { scalars, material }
    }

    /// Merge many field stats.
    #[must_use]
    pub fn merge_many(stats: &[Self]) -> Self {
        stats
            .iter()
            .fold(Self::empty(), |acc, s| Self::merge(&acc, s))
    }

    /// Check if all fields are uniform enough to stop recursion.
    #[must_use]
    pub fn is_uniform(&self, variance_threshold: f32) -> bool {
        self.scalars.iter().all(|s| s.is_uniform(variance_threshold))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_stats_merge() {
        let a = ScalarStats::from_value(10.0);
        let b = ScalarStats::from_value(20.0);
        let merged = ScalarStats::merge(&a, &b);

        assert_eq!(merged.mean, 15.0);
        assert_eq!(merged.min, 10.0);
        assert_eq!(merged.max, 20.0);
        assert_eq!(merged.sample_count, 2);
        // Variance should be 25.0 ((10-15)² + (20-15)²) / 2
        assert!((merged.variance - 25.0).abs() < 0.001);
    }

    #[test]
    fn test_scalar_stats_merge_empty() {
        let a = ScalarStats::empty();
        let b = ScalarStats::from_value(10.0);
        let merged = ScalarStats::merge(&a, &b);

        assert_eq!(merged.mean, 10.0);
        assert_eq!(merged.sample_count, 1);
    }
}
