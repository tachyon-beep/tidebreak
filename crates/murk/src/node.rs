//! Octree node structure.
//!
//! Nodes can be empty, leaf (with field values), or internal (with children and stats).

use serde::{Deserialize, Serialize};

use crate::field::FieldValues;
use crate::stats::FieldStats;
use crate::Bounds;

/// State of an octree node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeState {
    /// Empty node (not yet written, uses defaults)
    Empty,
    /// Leaf node with raw field values
    Leaf { values: FieldValues },
    /// Internal node with children and cached statistics
    Internal {
        children: [Option<Box<OctreeNode>>; 8],
        stats: FieldStats,
    },
}

impl Default for NodeState {
    fn default() -> Self {
        Self::Empty
    }
}

/// A node in the octree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OctreeNode {
    /// Spatial bounds of this node
    pub bounds: Bounds,
    /// Depth in the tree (0 = root)
    pub depth: u8,
    /// Node state (empty, leaf, or internal)
    pub state: NodeState,
}

impl OctreeNode {
    /// Create a new empty node.
    #[must_use]
    pub fn new(bounds: Bounds, depth: u8) -> Self {
        Self {
            bounds,
            depth,
            state: NodeState::Empty,
        }
    }

    /// Create a leaf node with values.
    #[must_use]
    pub fn leaf(bounds: Bounds, depth: u8, values: FieldValues) -> Self {
        Self {
            bounds,
            depth,
            state: NodeState::Leaf { values },
        }
    }

    /// Check if this node is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        matches!(self.state, NodeState::Empty)
    }

    /// Check if this node is a leaf.
    #[must_use]
    pub fn is_leaf(&self) -> bool {
        matches!(self.state, NodeState::Leaf { .. })
    }

    /// Check if this node is internal (has children).
    #[must_use]
    pub fn is_internal(&self) -> bool {
        matches!(self.state, NodeState::Internal { .. })
    }

    /// Get field values if this is a leaf node.
    #[must_use]
    pub fn values(&self) -> Option<&FieldValues> {
        match &self.state {
            NodeState::Leaf { values } => Some(values),
            _ => None,
        }
    }

    /// Get mutable field values if this is a leaf node.
    pub fn values_mut(&mut self) -> Option<&mut FieldValues> {
        match &mut self.state {
            NodeState::Leaf { values } => Some(values),
            _ => None,
        }
    }

    /// Get statistics (computed for leaves, cached for internal nodes).
    #[must_use]
    pub fn stats(&self) -> Option<FieldStats> {
        match &self.state {
            NodeState::Empty => None,
            NodeState::Leaf { values } => Some(FieldStats::from_values(values)),
            NodeState::Internal { stats, .. } => Some(stats.clone()),
        }
    }

    /// Get children if this is an internal node.
    #[must_use]
    pub fn children(&self) -> Option<&[Option<Box<OctreeNode>>; 8]> {
        match &self.state {
            NodeState::Internal { children, .. } => Some(children),
            _ => None,
        }
    }

    /// Get mutable children if this is an internal node.
    pub fn children_mut(&mut self) -> Option<&mut [Option<Box<OctreeNode>>; 8]> {
        match &mut self.state {
            NodeState::Internal { children, .. } => Some(children),
            _ => None,
        }
    }

    /// Convert this node to a leaf with the given values.
    pub fn make_leaf(&mut self, values: FieldValues) {
        self.state = NodeState::Leaf { values };
    }

    /// Convert this node to an internal node, distributing current value to children.
    pub fn split(&mut self) {
        let values = match &self.state {
            NodeState::Empty => FieldValues::new(),
            NodeState::Leaf { values } => *values,
            NodeState::Internal { .. } => return, // Already internal
        };

        let children: [Option<Box<OctreeNode>>; 8] = std::array::from_fn(|i| {
            let child_bounds = self.bounds.child_bounds(i);
            Some(Box::new(OctreeNode::leaf(child_bounds, self.depth + 1, values)))
        });

        self.state = NodeState::Internal {
            children,
            stats: FieldStats::from_values(&values),
        };
    }

    /// Try to merge children into a leaf if they're similar enough.
    ///
    /// Returns true if merge was performed.
    pub fn try_merge(&mut self, variance_threshold: f32) -> bool {
        let stats = match &self.state {
            NodeState::Internal { children, .. } => {
                // Collect stats from all non-empty children
                let child_stats: Vec<_> = children
                    .iter()
                    .filter_map(|c| c.as_ref().and_then(|node| node.stats()))
                    .collect();

                if child_stats.is_empty() {
                    self.state = NodeState::Empty;
                    return true;
                }

                FieldStats::merge_many(&child_stats)
            }
            _ => return false,
        };

        // Check if variance is low enough to merge
        if stats.is_uniform(variance_threshold) {
            // Create leaf with mean values
            let mut values = FieldValues::new();
            for (i, scalar_stats) in stats.scalars.iter().enumerate() {
                values.as_slice_mut()[i] = scalar_stats.mean;
            }
            self.state = NodeState::Leaf { values };
            true
        } else {
            // Update cached stats but don't merge
            if let NodeState::Internal { stats: ref mut s, .. } = &mut self.state {
                *s = stats;
            }
            false
        }
    }

    /// Update cached statistics from children.
    pub fn update_stats(&mut self) {
        if let NodeState::Internal { children, stats } = &mut self.state {
            let child_stats: Vec<_> = children
                .iter()
                .filter_map(|c| c.as_ref().and_then(|node| node.stats()))
                .collect();

            *stats = FieldStats::merge_many(&child_stats);
        }
    }

    /// Get the cell size at this depth.
    #[must_use]
    pub fn cell_size(&self) -> f32 {
        self.bounds.size().x // Assuming cubic cells
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn test_node_creation() {
        let bounds = Bounds::new(100.0, 100.0, 100.0);
        let node = OctreeNode::new(bounds, 0);
        assert!(node.is_empty());
    }

    #[test]
    fn test_node_split() {
        let bounds = Bounds::new(100.0, 100.0, 100.0);
        let mut node = OctreeNode::leaf(bounds, 0, FieldValues::new());

        node.split();
        assert!(node.is_internal());

        let children = node.children().unwrap();
        assert!(children.iter().all(|c| c.is_some()));
    }

    #[test]
    fn test_child_bounds() {
        let bounds = Bounds::new(100.0, 100.0, 100.0);
        let node = OctreeNode::new(bounds, 0);

        // Child 0 should be the -x, -y, -z octant
        let child_bounds = node.bounds.child_bounds(0);
        assert_eq!(child_bounds.min, Vec3::new(-50.0, -50.0, -50.0));
        assert_eq!(child_bounds.max, Vec3::new(0.0, 0.0, 0.0));
    }
}
