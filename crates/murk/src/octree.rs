//! Sparse octree storage.
//!
//! The octree provides hierarchical spatial storage with lazy allocation
//! and statistical aggregation at each level.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::field::FieldValues;
use crate::node::{NodeState, OctreeNode};
use crate::query::{PointQuery, PointResult, QueryResult, VolumeQuery};
use crate::stamp::Stamp;
use crate::stats::FieldStats;
use crate::Bounds;

/// Configuration for the octree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OctreeConfig {
    /// World bounds
    pub bounds: Bounds,
    /// Base resolution (cell size at maximum depth)
    pub base_resolution: f32,
    /// Maximum tree depth
    pub max_depth: u8,
    /// Variance threshold for merging cells
    pub merge_threshold: f32,
    /// Variance threshold for splitting cells
    pub split_threshold: f32,
}

impl Default for OctreeConfig {
    fn default() -> Self {
        Self {
            bounds: Bounds::new(1024.0, 1024.0, 256.0),
            base_resolution: 1.0,
            max_depth: 10,
            merge_threshold: 0.02,
            split_threshold: 0.1,
        }
    }
}

impl OctreeConfig {
    /// Calculate max depth from bounds and base resolution.
    #[must_use]
    pub fn calculate_max_depth(bounds: &Bounds, base_resolution: f32) -> u8 {
        let max_dim = bounds.size().max_element();
        let levels = (max_dim / base_resolution).log2().ceil() as u8;
        levels.min(16) // Cap at 16 to avoid excessive depth
    }
}

/// Sparse octree for field storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Octree {
    /// Root node
    root: OctreeNode,
    /// Configuration
    config: OctreeConfig,
    /// Number of nodes allocated
    node_count: usize,
    /// Number of leaf nodes
    leaf_count: usize,
}

impl Octree {
    /// Create a new octree.
    #[must_use]
    pub fn new(config: OctreeConfig) -> Self {
        let root = OctreeNode::new(config.bounds, 0);
        Self {
            root,
            config,
            node_count: 1,
            leaf_count: 0,
        }
    }

    /// Create an octree with default config.
    #[must_use]
    pub fn with_bounds(bounds: Bounds, base_resolution: f32) -> Self {
        let max_depth = OctreeConfig::calculate_max_depth(&bounds, base_resolution);
        Self::new(OctreeConfig {
            bounds,
            base_resolution,
            max_depth,
            ..Default::default()
        })
    }

    /// Get the configuration.
    #[must_use]
    pub fn config(&self) -> &OctreeConfig {
        &self.config
    }

    /// Get the root node.
    #[must_use]
    pub fn root(&self) -> &OctreeNode {
        &self.root
    }

    /// Get statistics.
    #[must_use]
    pub fn stats(&self) -> OctreeStats {
        OctreeStats {
            node_count: self.node_count,
            leaf_count: self.leaf_count,
            max_depth: self.config.max_depth,
        }
    }

    /// Query a single point.
    #[must_use]
    pub fn query_point(&self, query: &PointQuery) -> PointResult {
        if !self.config.bounds.contains(query.position) {
            return PointResult::default();
        }

        self.query_point_recursive(&self.root, query)
    }

    fn query_point_recursive(&self, node: &OctreeNode, query: &PointQuery) -> PointResult {
        match &node.state {
            NodeState::Empty => PointResult {
                values: FieldValues::new(),
                depth: node.depth,
                interpolated: true,
            },
            NodeState::Leaf { values } => PointResult {
                values: *values,
                depth: node.depth,
                interpolated: false,
            },
            NodeState::Internal { children, stats } => {
                let octant = node.bounds.octant_index(query.position);
                if let Some(child) = &children[octant] {
                    self.query_point_recursive(child, query)
                } else {
                    // No child at this octant, use stats
                    let mut values = FieldValues::new();
                    for (i, s) in stats.scalars.iter().enumerate() {
                        values.as_slice_mut()[i] = s.mean;
                    }
                    PointResult {
                        values,
                        depth: node.depth,
                        interpolated: true,
                    }
                }
            }
        }
    }

    /// Query a volume.
    #[must_use]
    pub fn query_volume(&self, query: &VolumeQuery) -> QueryResult {
        let mut result = QueryResult::default();
        self.query_volume_recursive(&self.root, query, &mut result);
        result
    }

    fn query_volume_recursive(
        &self,
        node: &OctreeNode,
        query: &VolumeQuery,
        result: &mut QueryResult,
    ) {
        result.nodes_visited += 1;
        result.max_depth_reached = result.max_depth_reached.max(node.depth);

        // Check if this node intersects the query sphere
        if !node.bounds.intersects_sphere(query.center, query.radius) {
            return;
        }

        let max_depth = query.resolution.max_depth(self.config.max_depth);
        let variance_threshold = query.resolution.variance_threshold();

        match &node.state {
            NodeState::Empty => {
                // Use default values
                let empty_stats = FieldStats::from_values(&FieldValues::new());
                result.stats = FieldStats::merge(&result.stats, &empty_stats);
            }
            NodeState::Leaf { values } => {
                let leaf_stats = FieldStats::from_values(values);
                result.stats = FieldStats::merge(&result.stats, &leaf_stats);
            }
            NodeState::Internal { children, stats } => {
                // Check early-out conditions
                let use_cached_stats = node.depth >= max_depth
                    || node.bounds.is_fully_inside_sphere(query.center, query.radius)
                    || variance_threshold.map_or(false, |t| stats.is_uniform(t));

                if use_cached_stats {
                    result.stats = FieldStats::merge(&result.stats, stats);
                } else {
                    // Recurse into children
                    for child in children.iter().flatten() {
                        self.query_volume_recursive(child, query, result);
                    }
                }
            }
        }
    }

    /// Apply a stamp to the octree.
    pub fn apply_stamp(&mut self, stamp: &Stamp) {
        let config = self.config.clone();
        Self::apply_stamp_recursive(&mut self.root, stamp, &config, &mut self.node_count, &mut self.leaf_count);
    }

    fn apply_stamp_recursive(
        node: &mut OctreeNode,
        stamp: &Stamp,
        config: &OctreeConfig,
        node_count: &mut usize,
        leaf_count: &mut usize,
    ) {
        // Check if stamp intersects this node
        if !stamp.shape.intersects(&node.bounds) {
            return;
        }

        match &mut node.state {
            NodeState::Empty => {
                // Materialize as leaf and apply
                node.state = NodeState::Leaf {
                    values: FieldValues::new(),
                };
                *leaf_count += 1;
                Self::apply_stamp_to_leaf(node, stamp);
            }
            NodeState::Leaf { .. } => {
                // Check if we need to split
                if node.depth < config.max_depth && Self::should_split_for_stamp(node, stamp, config) {
                    node.split();
                    *node_count += 8;
                    *leaf_count += 7; // Was 1 leaf, now 8 leaves
                    Self::apply_stamp_recursive(node, stamp, config, node_count, leaf_count);
                } else {
                    Self::apply_stamp_to_leaf(node, stamp);
                }
            }
            NodeState::Internal { children, .. } => {
                // Recurse into children
                for child in children.iter_mut().flatten() {
                    Self::apply_stamp_recursive(child, stamp, config, node_count, leaf_count);
                }
                // Update cached stats
                node.update_stats();
                // Try to merge if variance is low
                if node.try_merge(config.merge_threshold) {
                    *node_count -= 8;
                    *leaf_count -= 7;
                }
            }
        }
    }

    fn should_split_for_stamp(node: &OctreeNode, stamp: &Stamp, config: &OctreeConfig) -> bool {
        // Split if the stamp would create a significant gradient across the cell
        // For now, use a simple heuristic: split if stamp doesn't cover entire cell
        let cell_fully_covered = match &stamp.shape {
            crate::stamp::StampShape::Sphere { center, radius } => {
                node.bounds.is_fully_inside_sphere(*center, *radius)
            }
            crate::stamp::StampShape::Box { bounds } => {
                bounds.min.x <= node.bounds.min.x
                    && bounds.max.x >= node.bounds.max.x
                    && bounds.min.y <= node.bounds.min.y
                    && bounds.max.y >= node.bounds.max.y
                    && bounds.min.z <= node.bounds.min.z
                    && bounds.max.z >= node.bounds.max.z
            }
            crate::stamp::StampShape::Capsule { .. } => false, // Conservative
        };

        !cell_fully_covered && node.cell_size() > config.base_resolution * 2.0
    }

    fn apply_stamp_to_leaf(node: &mut OctreeNode, stamp: &Stamp) {
        if let NodeState::Leaf { values } = &mut node.state {
            // Sample at cell center
            let center = node.bounds.center();
            let intensity = stamp.shape.intensity_at(center, stamp.falloff);

            if intensity > 0.0 {
                for modification in &stamp.modifications {
                    let current = values.get(modification.field);
                    let new_value = if stamp.falloff {
                        // Interpolate based on intensity
                        let target = modification.op.apply(current, modification.value);
                        current + (target - current) * intensity
                    } else {
                        modification.op.apply(current, modification.value)
                    };
                    values.set(modification.field, new_value);
                }
            }
        }
    }

    /// Get the cell size at a given position.
    ///
    /// Returns the size of the cell containing the given position.
    /// For empty octrees, this returns the world size.
    #[must_use]
    pub fn cell_size_at(&self, position: Vec3) -> f32 {
        if !self.config.bounds.contains(position) {
            return 0.0;
        }

        let result = self.query_point(&PointQuery::new(position));
        // Cell size = world_size / 2^depth
        let world_size = self.config.bounds.size().x; // Assuming cubic cells
        world_size / 2.0_f32.powi(i32::from(result.depth))
    }

    /// Find the neighbor of a cell in the given direction.
    ///
    /// Returns `None` if the neighbor would be outside world bounds.
    /// Returns the field values at the neighboring cell position.
    #[must_use]
    pub fn find_neighbor(&self, position: Vec3, direction: Direction) -> Option<FieldValues> {
        // Get the cell size at this position
        let cell_size = self.cell_size_at(position);
        if cell_size == 0.0 {
            return None; // Position is outside bounds
        }

        // Calculate neighbor position (center of adjacent cell)
        let neighbor_pos = position + direction.offset() * cell_size;

        // Check if neighbor is within world bounds
        if !self.config.bounds.contains(neighbor_pos) {
            return None;
        }

        // Query the field values at the neighbor position
        let result = self.query_point(&PointQuery::new(neighbor_pos));
        Some(result.values)
    }

    /// Collect all leaf nodes for propagation traversal.
    ///
    /// Returns all leaf nodes as (center_position, values) pairs.
    /// Uses deterministic depth-first, octant-order traversal.
    /// Empty nodes are skipped (they don't contribute to propagation).
    #[must_use]
    pub fn collect_leaves(&self) -> Vec<(Vec3, FieldValues)> {
        let mut leaves = Vec::new();
        self.collect_leaves_recursive(&self.root, &mut leaves);
        leaves
    }

    /// Recursive helper for collecting leaf nodes.
    ///
    /// Traverses the tree in deterministic depth-first, octant-order.
    fn collect_leaves_recursive(&self, node: &OctreeNode, leaves: &mut Vec<(Vec3, FieldValues)>) {
        match &node.state {
            NodeState::Empty => {
                // Skip empty nodes - they don't contribute to propagation
            }
            NodeState::Leaf { values } => {
                // Add this leaf to the collection
                let center = node.bounds.center();
                leaves.push((center, *values));
            }
            NodeState::Internal { children, .. } => {
                // Recurse into children in octant order (0..7) for determinism
                for child in children.iter().flatten() {
                    self.collect_leaves_recursive(child, leaves);
                }
            }
        }
    }

    /// Set a single point value (useful for initialization).
    pub fn set_point(&mut self, position: Vec3, values: FieldValues) {
        if !self.config.bounds.contains(position) {
            return;
        }
        let max_depth = self.config.max_depth;
        Self::set_point_recursive(&mut self.root, position, values, max_depth, &mut self.node_count, &mut self.leaf_count);
    }

    fn set_point_recursive(
        node: &mut OctreeNode,
        position: Vec3,
        values: FieldValues,
        max_depth: u8,
        node_count: &mut usize,
        leaf_count: &mut usize,
    ) {
        match &mut node.state {
            NodeState::Empty => {
                if node.depth >= max_depth {
                    node.state = NodeState::Leaf { values };
                    *leaf_count += 1;
                } else {
                    node.split();
                    *node_count += 8;
                    *leaf_count += 8;
                    Self::set_point_recursive(node, position, values, max_depth, node_count, leaf_count);
                }
            }
            NodeState::Leaf { values: v } => {
                if node.depth >= max_depth {
                    *v = values;
                } else {
                    node.split();
                    *node_count += 8;
                    *leaf_count += 7;
                    Self::set_point_recursive(node, position, values, max_depth, node_count, leaf_count);
                }
            }
            NodeState::Internal { children, .. } => {
                let octant = node.bounds.octant_index(position);
                if children[octant].is_none() {
                    let child_bounds = node.bounds.child_bounds(octant);
                    children[octant] = Some(Box::new(OctreeNode::new(child_bounds, node.depth + 1)));
                    *node_count += 1;
                }
                if let Some(child) = &mut children[octant] {
                    Self::set_point_recursive(child, position, values, max_depth, node_count, leaf_count);
                }
                node.update_stats();
            }
        }
    }
}

/// Statistics about the octree structure.
#[derive(Debug, Clone, Copy, Default)]
pub struct OctreeStats {
    /// Total number of nodes
    pub node_count: usize,
    /// Number of leaf nodes
    pub leaf_count: usize,
    /// Maximum depth
    pub max_depth: u8,
}

/// Direction for neighbor finding.
///
/// Represents the 6 cardinal directions in 3D space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Positive X direction (+1, 0, 0)
    PosX,
    /// Negative X direction (-1, 0, 0)
    NegX,
    /// Positive Y direction (0, +1, 0)
    PosY,
    /// Negative Y direction (0, -1, 0)
    NegY,
    /// Positive Z direction (0, 0, +1)
    PosZ,
    /// Negative Z direction (0, 0, -1)
    NegZ,
}

impl Direction {
    /// Get the offset vector for this direction.
    #[must_use]
    pub fn offset(self) -> Vec3 {
        match self {
            Direction::PosX => Vec3::new(1.0, 0.0, 0.0),
            Direction::NegX => Vec3::new(-1.0, 0.0, 0.0),
            Direction::PosY => Vec3::new(0.0, 1.0, 0.0),
            Direction::NegY => Vec3::new(0.0, -1.0, 0.0),
            Direction::PosZ => Vec3::new(0.0, 0.0, 1.0),
            Direction::NegZ => Vec3::new(0.0, 0.0, -1.0),
        }
    }

    /// Get the 4 directions in the XY plane (for 2D operations).
    #[must_use]
    pub fn xy_directions() -> [Direction; 4] {
        [Direction::PosX, Direction::NegX, Direction::PosY, Direction::NegY]
    }

    /// Get all 6 directions.
    #[must_use]
    pub fn all() -> [Direction; 6] {
        [
            Direction::PosX,
            Direction::NegX,
            Direction::PosY,
            Direction::NegY,
            Direction::PosZ,
            Direction::NegZ,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::Field;
    use crate::stamp::{BlendOp, FieldMod, StampShape};

    #[test]
    fn test_octree_creation() {
        let octree = Octree::with_bounds(Bounds::new(100.0, 100.0, 100.0), 1.0);
        assert_eq!(octree.node_count, 1);
        assert!(octree.root().is_empty());
    }

    #[test]
    fn test_stamp_application() {
        let mut octree = Octree::with_bounds(Bounds::new(100.0, 100.0, 100.0), 1.0);

        let stamp = Stamp::new(
            StampShape::sphere(Vec3::ZERO, 10.0),
            vec![FieldMod::new(Field::Temperature, BlendOp::Set, 500.0)],
        );

        octree.apply_stamp(&stamp);

        let result = octree.query_point(&PointQuery::new(Vec3::ZERO));
        assert!(result.values.get(Field::Temperature) > 0.0);
    }

    #[test]
    fn test_volume_query() {
        let mut octree = Octree::with_bounds(Bounds::new(100.0, 100.0, 100.0), 1.0);

        let stamp = Stamp::new(
            StampShape::sphere(Vec3::ZERO, 20.0),
            vec![FieldMod::new(Field::Temperature, BlendOp::Set, 500.0)],
        );

        octree.apply_stamp(&stamp);

        let result = octree.query_volume(&VolumeQuery::new(Vec3::ZERO, 30.0));
        assert!(result.mean(Field::Temperature) > 0.0);
    }

    // ===== Neighbor Finding Tests =====

    #[test]
    fn test_find_neighbor_simple() {
        // Create a small octree with two adjacent cells
        let mut octree = Octree::with_bounds(Bounds::new(100.0, 100.0, 100.0), 10.0);

        // Set values at two adjacent positions
        let mut values_a = FieldValues::new();
        values_a.set(Field::Temperature, 100.0);
        octree.set_point(Vec3::new(-5.0, 0.0, 0.0), values_a);

        let mut values_b = FieldValues::new();
        values_b.set(Field::Temperature, 200.0);
        octree.set_point(Vec3::new(5.0, 0.0, 0.0), values_b);

        // Query neighbor in PosX direction from the first cell
        let neighbor = octree.find_neighbor(Vec3::new(-5.0, 0.0, 0.0), Direction::PosX);
        assert!(neighbor.is_some(), "Should find neighbor in PosX direction");
        let neighbor_values = neighbor.unwrap();
        assert!(
            neighbor_values.get(Field::Temperature) > 150.0,
            "Neighbor should have high temperature (got {})",
            neighbor_values.get(Field::Temperature)
        );
    }

    #[test]
    fn test_find_neighbor_at_boundary() {
        // Create an octree and try to find neighbor outside world bounds
        let octree = Octree::with_bounds(Bounds::new(100.0, 100.0, 100.0), 10.0);

        // Query from near the edge in the direction that would go out of bounds
        // World is from -50 to +50, so querying PosX from +45 should go outside
        let neighbor = octree.find_neighbor(Vec3::new(45.0, 0.0, 0.0), Direction::PosX);
        assert!(
            neighbor.is_none(),
            "Should return None when neighbor is outside world bounds"
        );

        // Also test NegX from -45
        let neighbor = octree.find_neighbor(Vec3::new(-45.0, 0.0, 0.0), Direction::NegX);
        assert!(
            neighbor.is_none(),
            "Should return None when neighbor is outside world bounds (NegX)"
        );
    }

    #[test]
    fn test_find_neighbor_empty_returns_default() {
        // Create an octree and set a value at one cell
        let mut octree = Octree::with_bounds(Bounds::new(100.0, 100.0, 100.0), 10.0);

        let mut values = FieldValues::new();
        values.set(Field::Temperature, 500.0);
        octree.set_point(Vec3::new(0.0, 0.0, 0.0), values);

        // Query neighbor in a direction where there's no data (empty cell)
        // The neighbor should return default values (all zeros)
        let neighbor = octree.find_neighbor(Vec3::new(0.0, 0.0, 0.0), Direction::NegY);
        assert!(neighbor.is_some(), "Should find neighbor even if empty");
        let neighbor_values = neighbor.unwrap();
        // Empty cells return default FieldValues (all zeros)
        assert_eq!(
            neighbor_values.get(Field::Temperature),
            0.0,
            "Empty neighbor should return default values"
        );
    }

    #[test]
    fn test_cell_size_at() {
        let octree = Octree::with_bounds(Bounds::new(100.0, 100.0, 100.0), 1.0);

        // At root level, cell size should be the world size
        let size = octree.cell_size_at(Vec3::ZERO);
        assert!(size > 0.0, "Cell size should be positive");
        // For an empty octree, the root covers the whole world
        assert_eq!(size, 100.0, "Cell size at root should be world size");
    }

    #[test]
    fn test_direction_offset() {
        assert_eq!(Direction::PosX.offset(), Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(Direction::NegX.offset(), Vec3::new(-1.0, 0.0, 0.0));
        assert_eq!(Direction::PosY.offset(), Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(Direction::NegY.offset(), Vec3::new(0.0, -1.0, 0.0));
        assert_eq!(Direction::PosZ.offset(), Vec3::new(0.0, 0.0, 1.0));
        assert_eq!(Direction::NegZ.offset(), Vec3::new(0.0, 0.0, -1.0));
    }

    #[test]
    fn test_xy_directions() {
        let xy = Direction::xy_directions();
        assert_eq!(xy.len(), 4);
        assert!(xy.contains(&Direction::PosX));
        assert!(xy.contains(&Direction::NegX));
        assert!(xy.contains(&Direction::PosY));
        assert!(xy.contains(&Direction::NegY));
        assert!(!xy.contains(&Direction::PosZ));
        assert!(!xy.contains(&Direction::NegZ));
    }

    // ===== Leaf Collection Tests =====

    #[test]
    fn test_collect_leaves() {
        // Create an octree and add some leaf nodes
        let mut octree = Octree::with_bounds(Bounds::new(100.0, 100.0, 100.0), 10.0);

        // Set values at a few positions to create leaf nodes
        let mut values_a = FieldValues::new();
        values_a.set(Field::Temperature, 100.0);
        octree.set_point(Vec3::new(-25.0, -25.0, 0.0), values_a);

        let mut values_b = FieldValues::new();
        values_b.set(Field::Temperature, 200.0);
        octree.set_point(Vec3::new(25.0, 25.0, 0.0), values_b);

        // Collect leaves
        let leaves = octree.collect_leaves();

        // Should have at least 2 leaves with non-default temperature values
        assert!(!leaves.is_empty(), "Should have collected some leaves");

        // Check that we got leaves with the expected temperature values
        let temps: Vec<f32> = leaves
            .iter()
            .map(|(_, values)| values.get(Field::Temperature))
            .filter(|&t| t > 0.0)
            .collect();

        assert!(
            temps.contains(&100.0) || temps.contains(&200.0),
            "Should contain at least one of our set temperature values"
        );
    }

    #[test]
    fn test_collect_leaves_empty_octree() {
        // Create an empty octree
        let octree = Octree::with_bounds(Bounds::new(100.0, 100.0, 100.0), 10.0);

        // Collect leaves - should return empty vec
        let leaves = octree.collect_leaves();

        assert!(
            leaves.is_empty(),
            "Empty octree should return empty leaf collection"
        );
    }

    #[test]
    fn test_collect_leaves_deterministic() {
        // Create two identical octrees
        let mut octree1 = Octree::with_bounds(Bounds::new(100.0, 100.0, 100.0), 10.0);
        let mut octree2 = Octree::with_bounds(Bounds::new(100.0, 100.0, 100.0), 10.0);

        // Add the same values in the same order
        let positions = [
            Vec3::new(-25.0, -25.0, 0.0),
            Vec3::new(25.0, 25.0, 0.0),
            Vec3::new(-25.0, 25.0, 0.0),
            Vec3::new(25.0, -25.0, 0.0),
        ];

        for (i, pos) in positions.iter().enumerate() {
            let mut values = FieldValues::new();
            values.set(Field::Temperature, (i as f32 + 1.0) * 100.0);
            octree1.set_point(*pos, values);
            octree2.set_point(*pos, values);
        }

        // Collect leaves from both
        let leaves1 = octree1.collect_leaves();
        let leaves2 = octree2.collect_leaves();

        // Should have same number of leaves
        assert_eq!(
            leaves1.len(),
            leaves2.len(),
            "Both octrees should have same number of leaves"
        );

        // Should be in the same order (deterministic traversal)
        for (i, ((pos1, values1), (pos2, values2))) in
            leaves1.iter().zip(leaves2.iter()).enumerate()
        {
            assert_eq!(
                pos1, pos2,
                "Leaf positions should match at index {}",
                i
            );
            assert_eq!(
                values1.as_slice(),
                values2.as_slice(),
                "Leaf values should match at index {}",
                i
            );
        }
    }
}
