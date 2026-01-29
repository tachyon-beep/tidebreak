//! # Tidebreak Python Bindings
//!
//! PyO3 bindings exposing Murk and Tidebreak Core to Python.
//!
//! ## Usage
//!
//! ```python
//! import tidebreak
//! from tidebreak import Field
//!
//! # Create a universe (spatial substrate)
//! universe = tidebreak.PyUniverse(
//!     width=1024.0,
//!     height=1024.0,
//!     depth=256.0,
//!     base_resolution=1.0,
//! )
//!
//! # Apply a stamp
//! universe.stamp_explosion(
//!     center=(500, 500, 20),
//!     radius=15,
//!     intensity=1.0,
//! )
//!
//! # Query with enum (type-safe)
//! stats = universe.query_volume(
//!     center=(500, 500, 30),
//!     radius=50,
//!     resolution="coarse",
//! )
//! print(f"Avg temperature: {stats.mean(Field.TEMPERATURE)}")
//!
//! # Or with string (backwards compatible)
//! print(f"Avg temperature: {stats.mean('temperature')}")
//! ```

use pyo3::prelude::*;

/// Field enum for Python.
///
/// Represents the different scalar fields that can be queried or modified
/// in the spatial substrate. Using this enum provides IDE autocomplete
/// and type checking benefits over string-based field names.
///
/// # Python Usage
///
/// ```python
/// from tidebreak import Field
///
/// # Use enum values for type-safe field access
/// temp = result.mean(Field.TEMPERATURE)
/// noise = result.max(Field.NOISE)
///
/// # Enums can be used as dict keys
/// field_names = {Field.TEMPERATURE: "temp", Field.NOISE: "noise"}
/// ```
#[pyclass(eq, eq_int, hash, frozen)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[allow(non_camel_case_types)] // Python convention uses SCREAMING_SNAKE_CASE for enums
pub enum Field {
    /// Solid vs empty space [0, 1]
    OCCUPANCY,
    /// Material type (encoded as float for storage uniformity)
    MATERIAL,
    /// Structural integrity [0, 1]
    INTEGRITY,
    /// Temperature in Kelvin [0, infinity)
    TEMPERATURE,
    /// Smoke density [0, 1]
    SMOKE,
    /// Acoustic noise level in dB [0, 200]
    NOISE,
    /// Generic signal field (configurable)
    SIGNAL,
    /// Water current X component [-10, 10] m/s
    CURRENT_X,
    /// Water current Y component [-10, 10] m/s
    CURRENT_Y,
    /// Water depth in meters [0, 10000]
    DEPTH,
    /// Salinity in ppt [0, 50]
    SALINITY,
    /// Sonar return strength [0, 1]
    SONAR_RETURN,
}

impl From<Field> for murk::Field {
    fn from(f: Field) -> Self {
        match f {
            Field::OCCUPANCY => murk::Field::Occupancy,
            Field::MATERIAL => murk::Field::Material,
            Field::INTEGRITY => murk::Field::Integrity,
            Field::TEMPERATURE => murk::Field::Temperature,
            Field::SMOKE => murk::Field::Smoke,
            Field::NOISE => murk::Field::Noise,
            Field::SIGNAL => murk::Field::Signal,
            Field::CURRENT_X => murk::Field::CurrentX,
            Field::CURRENT_Y => murk::Field::CurrentY,
            Field::DEPTH => murk::Field::Depth,
            Field::SALINITY => murk::Field::Salinity,
            Field::SONAR_RETURN => murk::Field::SonarReturn,
        }
    }
}

impl From<murk::Field> for Field {
    fn from(f: murk::Field) -> Self {
        match f {
            murk::Field::Occupancy => Field::OCCUPANCY,
            murk::Field::Material => Field::MATERIAL,
            murk::Field::Integrity => Field::INTEGRITY,
            murk::Field::Temperature => Field::TEMPERATURE,
            murk::Field::Smoke => Field::SMOKE,
            murk::Field::Noise => Field::NOISE,
            murk::Field::Signal => Field::SIGNAL,
            murk::Field::CurrentX => Field::CURRENT_X,
            murk::Field::CurrentY => Field::CURRENT_Y,
            murk::Field::Depth => Field::DEPTH,
            murk::Field::Salinity => Field::SALINITY,
            murk::Field::SonarReturn => Field::SONAR_RETURN,
        }
    }
}

/// Accept either Field enum or string for backwards compatibility.
#[derive(FromPyObject)]
enum FieldOrStr {
    Field(Field),
    Str(String),
}

impl From<FieldOrStr> for murk::Field {
    fn from(f: FieldOrStr) -> Self {
        match f {
            FieldOrStr::Field(field) => field.into(),
            FieldOrStr::Str(s) => str_to_field(&s),
        }
    }
}

/// Universe wrapper for Python.
#[pyclass]
pub struct PyUniverse {
    inner: murk::Universe,
}

#[pymethods]
impl PyUniverse {
    /// Create a new Universe.
    #[new]
    #[pyo3(signature = (width=1024.0, height=1024.0, depth=256.0, base_resolution=1.0))]
    fn new(width: f32, height: f32, depth: f32, base_resolution: f32) -> Self {
        let config = murk::UniverseConfig {
            bounds: murk::Bounds::new(width, height, depth),
            base_resolution,
            ..Default::default()
        };
        Self {
            inner: murk::Universe::new(config),
        }
    }

    /// Get current tick.
    #[getter]
    fn tick(&self) -> u64 {
        self.inner.tick()
    }

    /// Get current simulation time.
    #[getter]
    fn time(&self) -> f64 {
        self.inner.time()
    }

    /// Apply an explosion stamp.
    #[pyo3(signature = (center, radius, intensity=1.0))]
    fn stamp_explosion(&mut self, center: (f32, f32, f32), radius: f32, intensity: f32) {
        let center = glam::Vec3::new(center.0, center.1, center.2);
        self.inner
            .stamp(&murk::Stamp::explosion(center, radius, intensity));
    }

    /// Apply a fire stamp.
    #[pyo3(signature = (center, radius, intensity=1.0))]
    fn stamp_fire(&mut self, center: (f32, f32, f32), radius: f32, intensity: f32) {
        let center = glam::Vec3::new(center.0, center.1, center.2);
        self.inner
            .stamp(&murk::Stamp::fire(center, radius, intensity));
    }

    /// Apply a sonar ping stamp.
    #[pyo3(signature = (center, radius, strength=1.0))]
    fn stamp_sonar_ping(&mut self, center: (f32, f32, f32), radius: f32, strength: f32) {
        let center = glam::Vec3::new(center.0, center.1, center.2);
        self.inner
            .stamp(&murk::Stamp::sonar_ping(center, radius, strength));
    }

    /// Query a point.
    fn query_point(&self, position: (f32, f32, f32)) -> PyPointResult {
        let position = glam::Vec3::new(position.0, position.1, position.2);
        let result = self.inner.query_point(position);
        PyPointResult { inner: result }
    }

    /// Query a volume.
    #[pyo3(signature = (center, radius, resolution="medium"))]
    fn query_volume(
        &self,
        center: (f32, f32, f32),
        radius: f32,
        resolution: &str,
    ) -> PyQueryResult {
        let center = glam::Vec3::new(center.0, center.1, center.2);
        let res = match resolution {
            "coarse" => murk::QueryResolution::Coarse,
            "fine" => murk::QueryResolution::Fine,
            "full" => murk::QueryResolution::Full,
            _ => murk::QueryResolution::Medium,
        };
        let result = self.inner.query_volume(center, radius, res);
        PyQueryResult { inner: result }
    }

    /// Advance simulation by dt seconds.
    ///
    /// Releases the GIL during computation for better Python threading.
    fn step(&mut self, py: Python, dt: f64) {
        py.allow_threads(|| {
            self.inner.step(dt);
        });
    }

    /// Reset the universe.
    fn reset(&mut self) {
        self.inner.reset();
    }
}

/// Point query result wrapper.
#[pyclass]
pub struct PyPointResult {
    inner: murk::query::PointResult,
}

#[pymethods]
impl PyPointResult {
    /// Get value for a field.
    ///
    /// Accepts either a Field enum or a string for backwards compatibility.
    ///
    /// # Examples
    ///
    /// ```python
    /// from tidebreak import Field
    ///
    /// # Using enum (preferred)
    /// temp = result.get(Field.TEMPERATURE)
    ///
    /// # Using string (backwards compatible)
    /// temp = result.get("temperature")
    /// ```
    fn get(&self, field: FieldOrStr) -> f32 {
        let field: murk::Field = field.into();
        self.inner.get(field)
    }

    /// Get depth at which value was found.
    #[getter]
    fn depth(&self) -> u8 {
        self.inner.depth
    }

    /// Whether value is interpolated.
    #[getter]
    fn interpolated(&self) -> bool {
        self.inner.interpolated
    }
}

/// Volume query result wrapper.
#[pyclass]
pub struct PyQueryResult {
    inner: murk::query::QueryResult,
}

#[pymethods]
impl PyQueryResult {
    /// Get mean value for a field.
    ///
    /// Accepts either a Field enum or a string for backwards compatibility.
    ///
    /// # Examples
    ///
    /// ```python
    /// from tidebreak import Field
    ///
    /// # Using enum (preferred)
    /// temp = result.mean(Field.TEMPERATURE)
    ///
    /// # Using string (backwards compatible)
    /// temp = result.mean("temperature")
    /// ```
    fn mean(&self, field: FieldOrStr) -> f32 {
        let field: murk::Field = field.into();
        self.inner.mean(field)
    }

    /// Get variance for a field.
    ///
    /// Accepts either a Field enum or a string for backwards compatibility.
    fn variance(&self, field: FieldOrStr) -> f32 {
        let field: murk::Field = field.into();
        self.inner.variance(field)
    }

    /// Get min value for a field.
    ///
    /// Accepts either a Field enum or a string for backwards compatibility.
    fn min(&self, field: FieldOrStr) -> f32 {
        let field: murk::Field = field.into();
        self.inner.min(field)
    }

    /// Get max value for a field.
    ///
    /// Accepts either a Field enum or a string for backwards compatibility.
    fn max(&self, field: FieldOrStr) -> f32 {
        let field: murk::Field = field.into();
        self.inner.max(field)
    }

    /// Get nodes visited.
    #[getter]
    fn nodes_visited(&self) -> u32 {
        self.inner.nodes_visited
    }
}

/// Convert string to Field enum.
fn str_to_field(s: &str) -> murk::Field {
    match s.to_lowercase().as_str() {
        "occupancy" => murk::Field::Occupancy,
        "material" => murk::Field::Material,
        "integrity" => murk::Field::Integrity,
        "temperature" => murk::Field::Temperature,
        "smoke" => murk::Field::Smoke,
        "noise" => murk::Field::Noise,
        "signal" => murk::Field::Signal,
        "current_x" | "currentx" => murk::Field::CurrentX,
        "current_y" | "currenty" => murk::Field::CurrentY,
        "depth" => murk::Field::Depth,
        "salinity" => murk::Field::Salinity,
        "sonar_return" | "sonarreturn" | "sonar" => murk::Field::SonarReturn,
        _ => murk::Field::Signal, // Default fallback
    }
}

/// Python module definition.
#[pymodule]
fn tidebreak(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyUniverse>()?;
    m.add_class::<PyPointResult>()?;
    m.add_class::<PyQueryResult>()?;
    m.add_class::<Field>()?;
    Ok(())
}
