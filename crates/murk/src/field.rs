//! Field definitions and configurations.
//!
//! Fields are continuous scalar quantities sampled over space. Each field has
//! a type, valid range, aggregation method, and optional propagation behavior.

use serde::{Deserialize, Serialize};

/// Field identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Field {
    /// Solid vs empty space [0, 1]
    Occupancy = 0,
    /// Material type (encoded as float for storage uniformity)
    Material = 1,
    /// Structural integrity [0, 1]
    Integrity = 2,
    /// Temperature in Kelvin [0, ∞)
    Temperature = 3,
    /// Smoke density [0, 1]
    Smoke = 4,
    /// Acoustic noise level in dB [0, 200]
    Noise = 5,
    /// Generic signal field (configurable)
    Signal = 6,
    /// Water current X component [-10, 10] m/s
    CurrentX = 7,
    /// Water current Y component [-10, 10] m/s
    CurrentY = 8,
    /// Water depth in meters [0, 10000]
    Depth = 9,
    /// Salinity in ppt [0, 50]
    Salinity = 10,
    /// Sonar return strength [0, 1]
    SonarReturn = 11,
}

impl Field {
    /// Total number of fields.
    pub const COUNT: usize = 12;

    /// Get all fields as a slice.
    #[must_use]
    pub const fn all() -> &'static [Field] {
        &[
            Field::Occupancy,
            Field::Material,
            Field::Integrity,
            Field::Temperature,
            Field::Smoke,
            Field::Noise,
            Field::Signal,
            Field::CurrentX,
            Field::CurrentY,
            Field::Depth,
            Field::Salinity,
            Field::SonarReturn,
        ]
    }

    /// Get the index of this field.
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// How values are aggregated when combining cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Aggregation {
    /// Use arithmetic mean
    Mean,
    /// Use maximum value
    Max,
    /// Use minimum value
    Min,
    /// Use mode (most common value, for discrete fields like Material)
    Mode,
}

/// How fields propagate over time.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Propagation {
    /// No propagation - static field
    None,
    /// Diffusion (heat spreading)
    Diffusion { rate: f32 },
    /// Decay (signal fading)
    Decay { rate: f32 },
    /// Both diffusion and decay
    DiffusionDecay {
        diffusion_rate: f32,
        decay_rate: f32,
    },
}

impl Default for Propagation {
    fn default() -> Self {
        Self::None
    }
}

/// Configuration for a single field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldConfig {
    /// Which field this configures
    pub field: Field,
    /// Valid range (min, max)
    pub range: (f32, f32),
    /// How to aggregate values
    pub aggregation: Aggregation,
    /// How values propagate over time
    pub propagation: Propagation,
    /// Default value for uninitialized cells
    pub default_value: f32,
}

impl FieldConfig {
    /// Create a new field configuration.
    #[must_use]
    pub fn new(field: Field) -> Self {
        Self::default_for(field)
    }

    /// Get default configuration for a field.
    #[must_use]
    pub fn default_for(field: Field) -> Self {
        match field {
            Field::Occupancy => Self {
                field,
                range: (0.0, 1.0),
                aggregation: Aggregation::Max,
                propagation: Propagation::None,
                default_value: 0.0,
            },
            Field::Material => Self {
                field,
                range: (0.0, 255.0),
                aggregation: Aggregation::Mode,
                propagation: Propagation::None,
                default_value: 0.0,
            },
            Field::Integrity => Self {
                field,
                range: (0.0, 1.0),
                aggregation: Aggregation::Mean,
                propagation: Propagation::None,
                default_value: 1.0,
            },
            Field::Temperature => Self {
                field,
                range: (0.0, 10000.0),
                aggregation: Aggregation::Mean,
                propagation: Propagation::Diffusion { rate: 0.05 },
                default_value: 293.0, // ~20°C
            },
            Field::Smoke => Self {
                field,
                range: (0.0, 1.0),
                aggregation: Aggregation::Mean,
                propagation: Propagation::DiffusionDecay {
                    diffusion_rate: 0.1,
                    decay_rate: 0.02,
                },
                default_value: 0.0,
            },
            Field::Noise => Self {
                field,
                range: (0.0, 200.0),
                aggregation: Aggregation::Max,
                propagation: Propagation::Decay { rate: 0.3 },
                default_value: 0.0,
            },
            Field::Signal => Self {
                field,
                range: (0.0, 1.0),
                aggregation: Aggregation::Max,
                propagation: Propagation::Decay { rate: 0.1 },
                default_value: 0.0,
            },
            Field::CurrentX | Field::CurrentY => Self {
                field,
                range: (-10.0, 10.0),
                aggregation: Aggregation::Mean,
                propagation: Propagation::None,
                default_value: 0.0,
            },
            Field::Depth => Self {
                field,
                range: (0.0, 10000.0),
                aggregation: Aggregation::Mean,
                propagation: Propagation::None,
                default_value: 100.0,
            },
            Field::Salinity => Self {
                field,
                range: (0.0, 50.0),
                aggregation: Aggregation::Mean,
                propagation: Propagation::Diffusion { rate: 0.001 },
                default_value: 35.0,
            },
            Field::SonarReturn => Self {
                field,
                range: (0.0, 1.0),
                aggregation: Aggregation::Max,
                propagation: Propagation::Decay { rate: 0.5 },
                default_value: 0.0,
            },
        }
    }

    /// Clamp a value to the valid range.
    #[must_use]
    pub fn clamp(&self, value: f32) -> f32 {
        value.clamp(self.range.0, self.range.1)
    }
}

/// Raw field values for a leaf node.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct FieldValues {
    values: [f32; Field::COUNT],
}

impl FieldValues {
    /// Create field values with all defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create field values from a slice (must be exactly `Field::COUNT` elements).
    ///
    /// # Panics
    /// Panics if slice length doesn't match `Field::COUNT`.
    #[must_use]
    pub fn from_slice(slice: &[f32]) -> Self {
        assert_eq!(slice.len(), Field::COUNT);
        let mut values = [0.0; Field::COUNT];
        values.copy_from_slice(slice);
        Self { values }
    }

    /// Get a field value.
    #[must_use]
    pub fn get(&self, field: Field) -> f32 {
        self.values[field.index()]
    }

    /// Set a field value.
    pub fn set(&mut self, field: Field, value: f32) {
        self.values[field.index()] = value;
    }

    /// Get mutable reference to a field value.
    pub fn get_mut(&mut self, field: Field) -> &mut f32 {
        &mut self.values[field.index()]
    }

    /// Get the raw values array.
    #[must_use]
    pub fn as_slice(&self) -> &[f32] {
        &self.values
    }

    /// Get mutable raw values array.
    pub fn as_slice_mut(&mut self) -> &mut [f32] {
        &mut self.values
    }
}

impl std::ops::Index<Field> for FieldValues {
    type Output = f32;

    fn index(&self, field: Field) -> &Self::Output {
        &self.values[field.index()]
    }
}

impl std::ops::IndexMut<Field> for FieldValues {
    fn index_mut(&mut self, field: Field) -> &mut Self::Output {
        &mut self.values[field.index()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_values() {
        let mut values = FieldValues::new();
        values.set(Field::Temperature, 500.0);
        assert_eq!(values.get(Field::Temperature), 500.0);
        assert_eq!(values[Field::Temperature], 500.0);
    }

    #[test]
    fn test_field_config_clamp() {
        let config = FieldConfig::default_for(Field::Occupancy);
        assert_eq!(config.clamp(-0.5), 0.0);
        assert_eq!(config.clamp(0.5), 0.5);
        assert_eq!(config.clamp(1.5), 1.0);
    }
}
