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

use numpy::{PyArray1, ToPyArray};
use pyo3::prelude::*;
use pyo3::types::PyList;
use tidebreak_core::entity::components::{CombatState, PhysicsState, StatusFlags, TransformState};
use tidebreak_core::entity::{EntityId, EntityTag};

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

    /// Reset the universe, optionally with a seed for determinism.
    ///
    /// If a seed is provided, the universe is recreated with that seed,
    /// ensuring deterministic replay of all subsequent operations.
    ///
    /// # Arguments
    ///
    /// * `seed` - Optional seed for deterministic RNG initialization
    ///
    /// # Example
    ///
    /// ```python
    /// universe = PyUniverse(width=100.0, height=100.0, depth=50.0)
    ///
    /// # Reset with seed for deterministic replay
    /// universe.reset(seed=42)
    ///
    /// # Reset without seed (uses previous seed if one existed)
    /// universe.reset()
    /// ```
    #[pyo3(signature = (seed=None))]
    fn reset(&mut self, seed: Option<u64>) {
        if let Some(s) = seed {
            // Re-create with seed
            let config = murk::UniverseConfig {
                bounds: self.inner.bounds(),
                ..Default::default()
            };
            self.inner = murk::Universe::new_with_seed(config, s);
        } else {
            self.inner.reset();
        }
    }

    /// Get foveated observation as numpy array.
    ///
    /// Returns a flat array of field means for each sector in each shell.
    /// Shape: (total_sectors * num_fields,)
    ///
    /// # Arguments
    ///
    /// * `position` - Agent position as (x, y, z) tuple
    /// * `heading` - Agent heading direction as (x, y, z) tuple
    /// * `shells` - Optional list of shell configurations as dicts with keys:
    ///   - `radius_inner`: Inner radius of shell
    ///   - `radius_outer`: Outer radius of shell
    ///   - `sectors`: Number of angular divisions
    ///
    /// # Returns
    ///
    /// A flat numpy array of f32 values with shape (total_sectors * num_fields,).
    /// Default fields are: temperature, noise, occupancy, sonar_return.
    /// Default shells are: (0-10, 16 sectors), (10-50, 8 sectors), (50-200, 4 sectors).
    ///
    /// # Example
    ///
    /// ```python
    /// obs = universe.observe_foveated(
    ///     position=(0.0, 0.0, 0.0),
    ///     heading=(1.0, 0.0, 0.0),
    ///     shells=[
    ///         {"radius_inner": 0.0, "radius_outer": 10.0, "sectors": 8},
    ///         {"radius_inner": 10.0, "radius_outer": 50.0, "sectors": 4},
    ///     ],
    /// )
    /// ```
    #[pyo3(signature = (position, heading, shells=None))]
    fn observe_foveated<'py>(
        &self,
        py: Python<'py>,
        position: (f32, f32, f32),
        heading: (f32, f32, f32),
        shells: Option<&Bound<'py, PyList>>,
    ) -> PyResult<Bound<'py, PyArray1<f32>>> {
        let position = glam::Vec3::new(position.0, position.1, position.2);
        let heading = glam::Vec3::new(heading.0, heading.1, heading.2);

        // Parse shells or use defaults
        let shell_configs: Vec<murk::query::FoveatedShell> = if let Some(shells) = shells {
            shells
                .iter()
                .map(|item| {
                    let dict = item.downcast::<pyo3::types::PyDict>()?;
                    let inner: f32 = dict
                        .get_item("radius_inner")?
                        .ok_or_else(|| {
                            PyErr::new::<pyo3::exceptions::PyKeyError, _>(
                                "missing key: radius_inner",
                            )
                        })?
                        .extract()?;
                    let outer: f32 = dict
                        .get_item("radius_outer")?
                        .ok_or_else(|| {
                            PyErr::new::<pyo3::exceptions::PyKeyError, _>(
                                "missing key: radius_outer",
                            )
                        })?
                        .extract()?;
                    let sectors: u32 = dict
                        .get_item("sectors")?
                        .ok_or_else(|| {
                            PyErr::new::<pyo3::exceptions::PyKeyError, _>("missing key: sectors")
                        })?
                        .extract()?;
                    Ok(murk::query::FoveatedShell::new(inner, outer, sectors))
                })
                .collect::<PyResult<Vec<_>>>()?
        } else {
            vec![
                murk::query::FoveatedShell::new(0.0, 10.0, 16),
                murk::query::FoveatedShell::new(10.0, 50.0, 8),
                murk::query::FoveatedShell::new(50.0, 200.0, 4),
            ]
        };

        let query = murk::query::FoveatedQuery::new(position, heading).with_shells(shell_configs);

        let result = self.inner.observe_foveated(&query);
        let flat = result.to_flat_vec(&query.fields);

        Ok(flat.to_pyarray(py))
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

/// Unique entity identifier exposed to Python.
#[pyclass(frozen, eq, hash)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PyEntityId(EntityId);

#[pymethods]
impl PyEntityId {
    /// Get the raw u64 value.
    #[getter]
    fn value(&self) -> u64 {
        self.0.as_u64()
    }

    fn __repr__(&self) -> String {
        format!("EntityId({})", self.0.as_u64())
    }
}

impl From<EntityId> for PyEntityId {
    fn from(id: EntityId) -> Self {
        Self(id)
    }
}

impl From<PyEntityId> for EntityId {
    fn from(id: PyEntityId) -> Self {
        id.0
    }
}

/// Entity type classification for Python.
#[pyclass(eq, eq_int, hash, frozen)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PyEntityTag {
    Ship,
    Platform,
    Projectile,
    Squadron,
}

impl From<EntityTag> for PyEntityTag {
    fn from(tag: EntityTag) -> Self {
        match tag {
            EntityTag::Ship => PyEntityTag::Ship,
            EntityTag::Platform => PyEntityTag::Platform,
            EntityTag::Projectile => PyEntityTag::Projectile,
            EntityTag::Squadron => PyEntityTag::Squadron,
        }
    }
}

impl From<PyEntityTag> for EntityTag {
    fn from(tag: PyEntityTag) -> Self {
        match tag {
            PyEntityTag::Ship => EntityTag::Ship,
            PyEntityTag::Platform => EntityTag::Platform,
            PyEntityTag::Projectile => EntityTag::Projectile,
            PyEntityTag::Squadron => EntityTag::Squadron,
        }
    }
}

/// Transform state (position and heading).
#[pyclass(frozen)]
#[derive(Clone)]
pub struct PyTransformState {
    /// X position.
    #[pyo3(get)]
    pub x: f32,
    /// Y position.
    #[pyo3(get)]
    pub y: f32,
    /// Heading in radians (CCW from +X).
    #[pyo3(get)]
    pub heading: f32,
}

impl From<&TransformState> for PyTransformState {
    fn from(t: &TransformState) -> Self {
        Self {
            x: t.position.x,
            y: t.position.y,
            heading: t.heading,
        }
    }
}

#[pymethods]
impl PyTransformState {
    /// Get position as (x, y) tuple.
    #[getter]
    fn position(&self) -> (f32, f32) {
        (self.x, self.y)
    }

    fn __repr__(&self) -> String {
        format!(
            "TransformState(x={:.2}, y={:.2}, heading={:.2})",
            self.x, self.y, self.heading
        )
    }
}

/// Physics state (velocity and limits).
#[pyclass(frozen)]
#[derive(Clone)]
pub struct PyPhysicsState {
    #[pyo3(get)]
    pub vx: f32,
    #[pyo3(get)]
    pub vy: f32,
    #[pyo3(get)]
    pub angular_velocity: f32,
    #[pyo3(get)]
    pub max_speed: f32,
    #[pyo3(get)]
    pub max_turn_rate: f32,
}

impl From<&PhysicsState> for PyPhysicsState {
    fn from(p: &PhysicsState) -> Self {
        Self {
            vx: p.velocity.x,
            vy: p.velocity.y,
            angular_velocity: p.angular_velocity,
            max_speed: p.max_speed,
            max_turn_rate: p.max_turn_rate,
        }
    }
}

#[pymethods]
impl PyPhysicsState {
    /// Get velocity as (vx, vy) tuple.
    #[getter]
    fn velocity(&self) -> (f32, f32) {
        (self.vx, self.vy)
    }

    /// Get current speed.
    #[getter]
    fn speed(&self) -> f32 {
        (self.vx * self.vx + self.vy * self.vy).sqrt()
    }

    fn __repr__(&self) -> String {
        format!(
            "PhysicsState(vx={:.2}, vy={:.2}, speed={:.2})",
            self.vx,
            self.vy,
            self.speed()
        )
    }
}

/// Combat state (HP and status).
#[pyclass(frozen)]
#[derive(Clone)]
pub struct PyCombatState {
    #[pyo3(get)]
    pub hp: f32,
    #[pyo3(get)]
    pub max_hp: f32,
    #[pyo3(get)]
    pub weapon_count: usize,
    #[pyo3(get)]
    pub is_destroyed: bool,
    #[pyo3(get)]
    pub is_mobility_disabled: bool,
}

impl From<&CombatState> for PyCombatState {
    fn from(c: &CombatState) -> Self {
        Self {
            hp: c.hp,
            max_hp: c.max_hp,
            weapon_count: c.weapons.len(),
            is_destroyed: c.status_flags.contains(StatusFlags::DESTROYED),
            is_mobility_disabled: c.status_flags.contains(StatusFlags::MOBILITY_DISABLED),
        }
    }
}

#[pymethods]
impl PyCombatState {
    /// Get health as percentage [0, 1].
    #[getter]
    fn health_pct(&self) -> f32 {
        if self.max_hp > 0.0 {
            self.hp / self.max_hp
        } else {
            0.0
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "CombatState(hp={:.1}/{:.1}, weapons={})",
            self.hp, self.max_hp, self.weapon_count
        )
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
fn _tidebreak(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyUniverse>()?;
    m.add_class::<PyPointResult>()?;
    m.add_class::<PyQueryResult>()?;
    m.add_class::<Field>()?;
    m.add_class::<PyEntityId>()?;
    m.add_class::<PyEntityTag>()?;
    m.add_class::<PyTransformState>()?;
    m.add_class::<PyPhysicsState>()?;
    m.add_class::<PyCombatState>()?;
    Ok(())
}
