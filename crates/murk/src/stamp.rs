//! Stamps: the mutation primitive for Murk.
//!
//! A stamp describes a shape and a set of field modifications to apply within
//! that shape. Stamps are the primary way to modify the world.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::field::Field;
use crate::Bounds;

/// Blend operation for applying a modification.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BlendOp {
    /// Replace: field = value
    Set,
    /// Add: field += value
    Add,
    /// Subtract: field -= value
    Subtract,
    /// Multiply: field *= value
    Multiply,
    /// Maximum: field = max(field, value)
    Max,
    /// Minimum: field = min(field, value)
    Min,
    /// Linear interpolation: field = lerp(field, value, factor)
    Lerp { factor: f32 },
}

impl BlendOp {
    /// Apply the blend operation.
    #[must_use]
    pub fn apply(self, current: f32, value: f32) -> f32 {
        match self {
            BlendOp::Set => value,
            BlendOp::Add => current + value,
            BlendOp::Subtract => current - value,
            BlendOp::Multiply => current * value,
            BlendOp::Max => current.max(value),
            BlendOp::Min => current.min(value),
            BlendOp::Lerp { factor } => current + (value - current) * factor,
        }
    }
}

/// A single field modification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FieldMod {
    /// Which field to modify
    pub field: Field,
    /// How to apply the modification
    pub op: BlendOp,
    /// The value to apply
    pub value: f32,
}

impl FieldMod {
    /// Create a new field modification.
    #[must_use]
    pub fn new(field: Field, op: BlendOp, value: f32) -> Self {
        Self { field, op, value }
    }

    /// Shorthand for Set operation.
    #[must_use]
    pub fn set(field: Field, value: f32) -> Self {
        Self::new(field, BlendOp::Set, value)
    }

    /// Shorthand for Add operation.
    #[must_use]
    pub fn add(field: Field, value: f32) -> Self {
        Self::new(field, BlendOp::Add, value)
    }

    /// Shorthand for Multiply operation.
    #[must_use]
    pub fn mul(field: Field, value: f32) -> Self {
        Self::new(field, BlendOp::Multiply, value)
    }
}

/// Shape for a stamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StampShape {
    /// Sphere defined by center and radius
    Sphere { center: Vec3, radius: f32 },
    /// Axis-aligned box
    Box { bounds: Bounds },
    /// Capsule (two endpoints + radius)
    Capsule { p0: Vec3, p1: Vec3, radius: f32 },
}

impl StampShape {
    /// Create a sphere shape.
    #[must_use]
    pub fn sphere(center: Vec3, radius: f32) -> Self {
        Self::Sphere { center, radius }
    }

    /// Create a box shape from bounds.
    #[must_use]
    pub fn aabb(bounds: Bounds) -> Self {
        Self::Box { bounds }
    }

    /// Create a box shape from min/max corners.
    #[must_use]
    pub fn box_min_max(min: Vec3, max: Vec3) -> Self {
        Self::Box {
            bounds: Bounds::from_min_max(min, max),
        }
    }

    /// Create a capsule shape.
    #[must_use]
    pub fn capsule(p0: Vec3, p1: Vec3, radius: f32) -> Self {
        Self::Capsule { p0, p1, radius }
    }

    /// Get the bounding box of this shape.
    #[must_use]
    pub fn bounds(&self) -> Bounds {
        match self {
            StampShape::Sphere { center, radius } => Bounds::from_min_max(
                *center - Vec3::splat(*radius),
                *center + Vec3::splat(*radius),
            ),
            StampShape::Box { bounds } => *bounds,
            StampShape::Capsule { p0, p1, radius } => {
                let min = p0.min(*p1) - Vec3::splat(*radius);
                let max = p0.max(*p1) + Vec3::splat(*radius);
                Bounds::from_min_max(min, max)
            }
        }
    }

    /// Check if a point is inside this shape.
    #[must_use]
    pub fn contains(&self, point: Vec3) -> bool {
        match self {
            StampShape::Sphere { center, radius } => center.distance(point) <= *radius,
            StampShape::Box { bounds } => bounds.contains(point),
            StampShape::Capsule { p0, p1, radius } => {
                // Distance from point to line segment
                let ab = *p1 - *p0;
                let ap = point - *p0;
                let t = (ap.dot(ab) / ab.dot(ab)).clamp(0.0, 1.0);
                let closest = *p0 + ab * t;
                point.distance(closest) <= *radius
            }
        }
    }

    /// Check if this shape intersects a bounds.
    #[must_use]
    pub fn intersects(&self, bounds: &Bounds) -> bool {
        match self {
            StampShape::Sphere { center, radius } => bounds.intersects_sphere(*center, *radius),
            StampShape::Box { bounds: b } => {
                // AABB vs AABB intersection
                b.min.x <= bounds.max.x
                    && b.max.x >= bounds.min.x
                    && b.min.y <= bounds.max.y
                    && b.max.y >= bounds.min.y
                    && b.min.z <= bounds.max.z
                    && b.max.z >= bounds.min.z
            }
            StampShape::Capsule { p0, p1, radius } => {
                // Conservative: check if capsule bounding box intersects
                let capsule_bounds = self.bounds();
                capsule_bounds.min.x <= bounds.max.x
                    && capsule_bounds.max.x >= bounds.min.x
                    && capsule_bounds.min.y <= bounds.max.y
                    && capsule_bounds.max.y >= bounds.min.y
                    && capsule_bounds.min.z <= bounds.max.z
                    && capsule_bounds.max.z >= bounds.min.z
            }
        }
    }

    /// Get intensity at a point (1.0 = full effect, 0.0 = no effect).
    ///
    /// For shapes with falloff (like spheres), this can return values between 0 and 1.
    /// For hard shapes (like boxes), this returns 1.0 inside and 0.0 outside.
    #[must_use]
    pub fn intensity_at(&self, point: Vec3, falloff: bool) -> f32 {
        if !falloff {
            return if self.contains(point) { 1.0 } else { 0.0 };
        }

        match self {
            StampShape::Sphere { center, radius } => {
                let dist = center.distance(point);
                if dist >= *radius {
                    0.0
                } else {
                    1.0 - (dist / radius)
                }
            }
            StampShape::Box { bounds } => {
                if bounds.contains(point) {
                    1.0
                } else {
                    0.0
                }
            }
            StampShape::Capsule { p0, p1, radius } => {
                let ab = *p1 - *p0;
                let ap = point - *p0;
                let t = (ap.dot(ab) / ab.dot(ab)).clamp(0.0, 1.0);
                let closest = *p0 + ab * t;
                let dist = point.distance(closest);
                if dist >= *radius {
                    0.0
                } else {
                    1.0 - (dist / radius)
                }
            }
        }
    }
}

/// A stamp: shape + field modifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stamp {
    /// Shape defining where the stamp applies
    pub shape: StampShape,
    /// Modifications to apply
    pub modifications: Vec<FieldMod>,
    /// Whether to use distance-based falloff
    pub falloff: bool,
}

impl Stamp {
    /// Create a new stamp.
    #[must_use]
    pub fn new(shape: StampShape, modifications: Vec<FieldMod>) -> Self {
        Self {
            shape,
            modifications,
            falloff: false,
        }
    }

    /// Create a stamp with falloff enabled.
    #[must_use]
    pub fn with_falloff(mut self) -> Self {
        self.falloff = true;
        self
    }

    /// Create an explosion stamp.
    #[must_use]
    pub fn explosion(center: Vec3, radius: f32, intensity: f32) -> Self {
        Self::new(
            StampShape::sphere(center, radius),
            vec![
                FieldMod::new(Field::Occupancy, BlendOp::Subtract, 0.8 * intensity),
                FieldMod::new(Field::Temperature, BlendOp::Add, 500.0 * intensity),
                FieldMod::new(Field::Noise, BlendOp::Add, 120.0 * intensity),
                FieldMod::new(Field::Integrity, BlendOp::Multiply, 1.0 - 0.8 * intensity),
            ],
        )
        .with_falloff()
    }

    /// Create a fire stamp.
    #[must_use]
    pub fn fire(center: Vec3, radius: f32, intensity: f32) -> Self {
        Self::new(
            StampShape::sphere(center, radius),
            vec![
                FieldMod::new(Field::Temperature, BlendOp::Lerp { factor: 0.1 }, 800.0),
                FieldMod::new(Field::Smoke, BlendOp::Add, 0.3 * intensity),
            ],
        )
        .with_falloff()
    }

    /// Create a sonar ping stamp.
    #[must_use]
    pub fn sonar_ping(center: Vec3, radius: f32, strength: f32) -> Self {
        Self::new(
            StampShape::sphere(center, radius),
            vec![
                FieldMod::new(Field::SonarReturn, BlendOp::Max, strength),
                FieldMod::new(Field::Noise, BlendOp::Add, 80.0),
            ],
        )
        .with_falloff()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blend_op_apply() {
        assert_eq!(BlendOp::Set.apply(5.0, 10.0), 10.0);
        assert_eq!(BlendOp::Add.apply(5.0, 10.0), 15.0);
        assert_eq!(BlendOp::Multiply.apply(5.0, 2.0), 10.0);
        assert_eq!(BlendOp::Max.apply(5.0, 10.0), 10.0);
        assert_eq!(BlendOp::Max.apply(15.0, 10.0), 15.0);
        assert_eq!((BlendOp::Lerp { factor: 0.5 }).apply(0.0, 10.0), 5.0);
    }

    #[test]
    fn test_sphere_contains() {
        let shape = StampShape::sphere(Vec3::ZERO, 10.0);
        assert!(shape.contains(Vec3::ZERO));
        assert!(shape.contains(Vec3::new(5.0, 0.0, 0.0)));
        assert!(!shape.contains(Vec3::new(15.0, 0.0, 0.0)));
    }

    #[test]
    fn test_sphere_intensity() {
        let shape = StampShape::sphere(Vec3::ZERO, 10.0);
        assert_eq!(shape.intensity_at(Vec3::ZERO, true), 1.0);
        assert!((shape.intensity_at(Vec3::new(5.0, 0.0, 0.0), true) - 0.5).abs() < 0.001);
        assert_eq!(shape.intensity_at(Vec3::new(10.0, 0.0, 0.0), true), 0.0);
    }
}
