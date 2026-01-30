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

use glam::Vec2;
use numpy::{PyArray1, ToPyArray};
use pyo3::prelude::*;
use pyo3::types::PyList;
use tidebreak_core::entity::components::{CombatState, PhysicsState, StatusFlags, TransformState};
use tidebreak_core::entity::{Entity, EntityId, EntityInner, EntityTag, ShipComponents};
use tidebreak_core::simulation::Simulation;

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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

/// Read-only view of an entity.
#[pyclass(frozen)]
#[derive(Clone)]
pub struct PyEntity {
    id: PyEntityId,
    tag: PyEntityTag,
    transform: PyTransformState,
    physics: Option<PyPhysicsState>,
    combat: Option<PyCombatState>,
}

impl PyEntity {
    pub fn from_entity(entity: &Entity) -> Self {
        let (transform, physics, combat) = match entity.inner() {
            EntityInner::Ship(c) => (
                PyTransformState::from(&c.transform),
                Some(PyPhysicsState::from(&c.physics)),
                Some(PyCombatState::from(&c.combat)),
            ),
            EntityInner::Platform(c) => (PyTransformState::from(&c.transform), None, None),
            EntityInner::Projectile(c) => (
                PyTransformState::from(&c.transform),
                Some(PyPhysicsState::from(&c.physics)),
                None,
            ),
            EntityInner::Squadron(c) => (
                PyTransformState::from(&c.transform),
                Some(PyPhysicsState::from(&c.physics)),
                Some(PyCombatState::from(&c.combat)),
            ),
        };

        Self {
            id: entity.id().into(),
            tag: entity.tag().into(),
            transform,
            physics,
            combat,
        }
    }
}

#[pymethods]
impl PyEntity {
    /// Entity ID.
    #[getter]
    fn id(&self) -> PyEntityId {
        self.id
    }

    /// Entity tag (type).
    #[getter]
    fn tag(&self) -> PyEntityTag {
        self.tag
    }

    /// Transform state (always present).
    #[getter]
    fn transform(&self) -> PyTransformState {
        self.transform.clone()
    }

    /// Physics state (if entity has physics).
    #[getter]
    fn physics(&self) -> Option<PyPhysicsState> {
        self.physics.clone()
    }

    /// Combat state (if entity has combat).
    #[getter]
    fn combat(&self) -> Option<PyCombatState> {
        self.combat.clone()
    }

    /// Check if entity is a ship.
    fn is_ship(&self) -> bool {
        matches!(self.tag, PyEntityTag::Ship)
    }

    /// Check if entity is destroyed.
    fn is_destroyed(&self) -> bool {
        self.combat.as_ref().is_some_and(|c| c.is_destroyed)
    }

    fn __repr__(&self) -> String {
        format!("Entity(id={}, tag={:?})", self.id.value(), self.tag)
    }
}

/// Main simulation orchestrator.
#[pyclass]
pub struct PySimulation {
    inner: Simulation,
}

#[pymethods]
impl PySimulation {
    /// Create a new simulation with the given seed.
    #[new]
    #[pyo3(signature = (seed=42))]
    fn new(seed: u64) -> Self {
        Self {
            inner: Simulation::new(seed),
        }
    }

    /// Current tick number.
    #[getter]
    fn tick(&self) -> u64 {
        self.inner.tick()
    }

    /// Master seed.
    #[getter]
    fn seed(&self) -> u64 {
        self.inner.seed()
    }

    /// Number of entities in the arena.
    #[getter]
    fn entity_count(&self) -> usize {
        self.inner.arena().entity_count()
    }

    /// Execute one simulation step.
    ///
    /// Releases the GIL during execution for better Python threading.
    fn step(&mut self, py: Python) {
        py.allow_threads(|| {
            self.inner.step();
        });
    }

    /// Spawn a ship at the given position.
    #[pyo3(signature = (x, y, heading=0.0))]
    fn spawn_ship(&mut self, x: f32, y: f32, heading: f32) -> PyEntityId {
        let components = ShipComponents::at_position(Vec2::new(x, y), heading);
        let id = self
            .inner
            .arena_mut()
            .spawn(EntityTag::Ship, EntityInner::Ship(components));
        id.into()
    }

    /// Get entity by ID.
    fn get_entity(&self, id: PyEntityId) -> Option<PyEntity> {
        self.inner.arena().get(id.into()).map(PyEntity::from_entity)
    }

    /// Get all entity IDs.
    fn entity_ids(&self) -> Vec<PyEntityId> {
        self.inner
            .arena()
            .entity_ids_sorted()
            .map(|id| id.into())
            .collect()
    }

    /// Query entities within radius.
    fn query_radius(&self, x: f32, y: f32, radius: f32) -> Vec<PyEntityId> {
        self.inner
            .arena()
            .spatial()
            .query_radius(Vec2::new(x, y), radius)
            .into_iter()
            .map(|id| id.into())
            .collect()
    }

    /// Despawn an entity.
    fn despawn(&mut self, id: PyEntityId) -> bool {
        self.inner.arena_mut().despawn(id.into()).is_some()
    }

    /// Reset simulation with optional new seed.
    #[pyo3(signature = (seed=None))]
    fn reset(&mut self, seed: Option<u64>) {
        let s = seed.unwrap_or(self.inner.seed());
        self.inner = Simulation::new(s);
    }

    /// Apply an action dict to an entity.
    ///
    /// Action dict can contain:
    /// - "velocity": (vx, vy) tuple
    /// - "heading": float in radians
    fn apply_action(
        &mut self,
        entity_id: PyEntityId,
        action: &Bound<'_, pyo3::types::PyDict>,
    ) -> PyResult<()> {
        let id: EntityId = entity_id.into();

        // Parse velocity
        let velocity: Option<(f32, f32)> = action
            .get_item("velocity")?
            .map(|v| v.extract())
            .transpose()?;

        // Parse heading
        let heading: Option<f32> = action
            .get_item("heading")?
            .map(|h| h.extract())
            .transpose()?;

        if let Some(entity) = self.inner.arena_mut().get_mut(id) {
            if let EntityInner::Ship(c) = entity.inner_mut() {
                if let Some((vx, vy)) = velocity {
                    let vel = Vec2::new(vx, vy);
                    // Clamp to max speed
                    let clamped = if vel.length() > c.physics.max_speed {
                        vel.normalize() * c.physics.max_speed
                    } else {
                        vel
                    };
                    c.physics.velocity = clamped;
                }

                if let Some(h) = heading {
                    c.transform.heading = h;
                }
            }
        }

        // Update spatial index after position changes
        self.inner.arena_mut().update_spatial(id);

        Ok(())
    }

    /// Get observation for an entity.
    #[pyo3(signature = (entity_id, max_contacts=16))]
    fn get_observation(&self, entity_id: PyEntityId, max_contacts: usize) -> Option<PyObservation> {
        PyObservation::for_entity(self.inner.arena(), entity_id.into(), max_contacts)
    }
}

/// Observation for a single agent (ship).
///
/// Pre-vectorized observation suitable for DRL training. Contains:
/// - `own_state`: Position, heading, velocity, and health as a 1D array
/// - `contacts`: Detected contacts from the sensor track table as a 2D array
#[pyclass]
pub struct PyObservation {
    /// Own state: [x, y, heading, vx, vy, hp, max_hp]
    own_state: Vec<f32>,
    /// Contacts: [[x, y, rel_heading, distance, quality], ...]
    contacts: Vec<Vec<f32>>,
}

impl PyObservation {
    /// Build observation for a specific entity.
    pub fn for_entity(
        arena: &tidebreak_core::arena::Arena,
        entity_id: EntityId,
        max_contacts: usize,
    ) -> Option<Self> {
        let entity = arena.get(entity_id)?;

        // Build own state vector
        let own_state = Self::build_own_state(entity);

        // Build contacts from sensor track table
        let contacts = Self::build_contacts(entity, max_contacts);

        Some(Self {
            own_state,
            contacts,
        })
    }

    fn build_own_state(entity: &Entity) -> Vec<f32> {
        match entity.inner() {
            EntityInner::Ship(c) => vec![
                c.transform.position.x,
                c.transform.position.y,
                c.transform.heading,
                c.physics.velocity.x,
                c.physics.velocity.y,
                c.combat.hp,
                c.combat.max_hp,
            ],
            EntityInner::Squadron(c) => vec![
                c.transform.position.x,
                c.transform.position.y,
                c.transform.heading,
                c.physics.velocity.x,
                c.physics.velocity.y,
                c.combat.hp,
                c.combat.max_hp,
            ],
            _ => vec![0.0; 7], // Platforms/projectiles shouldn't be agents
        }
    }

    fn build_contacts(entity: &Entity, max_contacts: usize) -> Vec<Vec<f32>> {
        let mut contacts = Vec::with_capacity(max_contacts);

        // Get own position for relative calculations
        let own_pos = match entity.inner() {
            EntityInner::Ship(c) => c.transform.position,
            EntityInner::Squadron(c) => c.transform.position,
            _ => return Self::pad_contacts(contacts, max_contacts),
        };

        // Get track table if entity has sensors
        let tracks = match entity.inner() {
            EntityInner::Ship(c) => &c.sensor.track_table,
            _ => return Self::pad_contacts(contacts, max_contacts),
        };

        for track in tracks.iter().take(max_contacts) {
            let rel = track.position - own_pos;
            let distance = rel.length();
            let rel_heading = rel.y.atan2(rel.x);
            let quality = track.quality as i32 as f32;

            contacts.push(vec![
                track.position.x,
                track.position.y,
                rel_heading,
                distance,
                quality,
            ]);
        }

        Self::pad_contacts(contacts, max_contacts)
    }

    fn pad_contacts(mut contacts: Vec<Vec<f32>>, max_contacts: usize) -> Vec<Vec<f32>> {
        while contacts.len() < max_contacts {
            contacts.push(vec![0.0; 5]);
        }
        contacts
    }
}

#[pymethods]
impl PyObservation {
    /// Own state as numpy array.
    ///
    /// Returns a 1D array with shape (7,) containing:
    /// [x, y, heading, vx, vy, hp, max_hp]
    fn own_state<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f32>> {
        self.own_state.to_pyarray(py)
    }

    /// Contacts as 2D numpy array (max_contacts x 5).
    ///
    /// Each row contains: [x, y, rel_heading, distance, quality]
    /// Unused slots are zero-padded.
    fn contacts<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, numpy::PyArray2<f32>>> {
        numpy::PyArray2::from_vec2(py, &self.contacts)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{e}")))
    }

    /// Feature dimension for own_state.
    #[getter]
    fn own_state_dim(&self) -> usize {
        self.own_state.len()
    }

    /// Number of contact slots.
    #[getter]
    fn max_contacts(&self) -> usize {
        self.contacts.len()
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
    m.add_class::<PyEntity>()?;
    m.add_class::<PySimulation>()?;
    m.add_class::<PyObservation>()?;
    Ok(())
}
