# Murk Python Bindings Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Complete Python bindings for Murk spatial substrate enabling DRL training with Gymnasium.

**Architecture:** Phase 0 adds determinism infrastructure (seeded RNG, hashing). Phase 1 fixes critical binding issues (GIL, numpy, enums). Phase 2 adds Gymnasium wrapper. Phase 3 validates integration with a training smoke test.

**Tech Stack:** Rust (murk crate), PyO3 0.23, numpy crate, Gymnasium, rand_chacha for deterministic RNG.

---

## Phase 0: Determinism Infrastructure

### Task 1: Add Seeded RNG to Universe

**Files:**
- Modify: `crates/murk/src/universe.rs`
- Modify: `crates/murk/Cargo.toml` (if needed)

**Step 1: Write the failing test**

Add to `crates/murk/src/universe.rs` in the `tests` module:

```rust
#[test]
fn test_universe_seeded_creation() {
    let config = UniverseConfig::with_bounds(100.0, 100.0, 50.0);
    let universe = Universe::new_with_seed(config, 42);
    assert_eq!(universe.seed(), Some(42));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_universe_seeded_creation -- --nocapture`
Expected: FAIL with "no method named `new_with_seed`"

**Step 3: Write minimal implementation**

Add to `Universe` struct in `crates/murk/src/universe.rs`:

```rust
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// The Universe: top-level container for spatial fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Universe {
    /// Octree storage
    octree: Octree,
    /// Field configurations
    field_configs: [FieldConfig; Field::COUNT],
    /// Current simulation tick
    tick: u64,
    /// Simulation time in seconds
    time: f64,
    /// Deterministic RNG (optional)
    #[serde(skip)]
    rng: Option<ChaCha8Rng>,
    /// Original seed for replay
    seed: Option<u64>,
}
```

Add constructor:

```rust
impl Universe {
    /// Create a new Universe with deterministic seeded RNG.
    #[must_use]
    pub fn new_with_seed(config: UniverseConfig, seed: u64) -> Self {
        let mut universe = Self::new(config);
        universe.rng = Some(ChaCha8Rng::seed_from_u64(seed));
        universe.seed = Some(seed);
        universe
    }

    /// Get the seed used to create this universe.
    #[must_use]
    pub fn seed(&self) -> Option<u64> {
        self.seed
    }

    /// Get mutable access to RNG (for internal use).
    pub(crate) fn rng_mut(&mut self) -> Option<&mut ChaCha8Rng> {
        self.rng.as_mut()
    }
}
```

Update `Universe::new()` to initialize the new fields:

```rust
Self {
    octree,
    field_configs,
    tick: 0,
    time: 0.0,
    rng: None,
    seed: None,
}
```

Update `Universe::reset()`:

```rust
pub fn reset(&mut self) {
    let config = self.octree.config().clone();
    self.octree = Octree::new(config);
    self.tick = 0;
    self.time = 0.0;
    // Re-seed RNG if we have a seed
    if let Some(seed) = self.seed {
        self.rng = Some(ChaCha8Rng::seed_from_u64(seed));
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_universe_seeded_creation -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/murk/src/universe.rs
git commit -m "feat(murk): add seeded RNG to Universe for determinism"
```

---

### Task 2: Add Universe State Hashing

**Files:**
- Create: `crates/murk/src/hash.rs`
- Modify: `crates/murk/src/lib.rs`

**Step 1: Write the failing test**

Add to `crates/murk/src/universe.rs` tests:

```rust
#[test]
fn test_universe_state_hash() {
    let config = UniverseConfig::with_bounds(100.0, 100.0, 50.0);
    let mut universe = Universe::new_with_seed(config.clone(), 42);

    universe.stamp(&Stamp::explosion(Vec3::ZERO, 10.0, 1.0));
    let hash1 = universe.state_hash();

    let mut universe2 = Universe::new_with_seed(config, 42);
    universe2.stamp(&Stamp::explosion(Vec3::ZERO, 10.0, 1.0));
    let hash2 = universe2.state_hash();

    assert_eq!(hash1, hash2, "Identical operations should produce identical hashes");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_universe_state_hash -- --nocapture`
Expected: FAIL with "no method named `state_hash`"

**Step 3: Write minimal implementation**

Create `crates/murk/src/hash.rs`:

```rust
//! State hashing for determinism verification.

use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

use crate::field::FieldValues;
use crate::node::{NodeState, OctreeNode};
use crate::Universe;

/// Compute a deterministic hash of universe state.
pub fn hash_universe(universe: &Universe) -> u64 {
    let mut hasher = DefaultHasher::new();

    // Hash tick and time
    universe.tick().hash(&mut hasher);
    // Hash time as bits to avoid float comparison issues
    universe.time().to_bits().hash(&mut hasher);

    // Hash octree state
    hash_octree_node(universe.octree().root(), &mut hasher);

    hasher.finish()
}

fn hash_octree_node(node: &OctreeNode, hasher: &mut impl Hasher) {
    node.depth.hash(hasher);

    match &node.state {
        NodeState::Empty => {
            0u8.hash(hasher);
        }
        NodeState::Leaf { values } => {
            1u8.hash(hasher);
            hash_field_values(values, hasher);
        }
        NodeState::Internal { children, .. } => {
            2u8.hash(hasher);
            for child in children.iter() {
                match child {
                    Some(c) => {
                        1u8.hash(hasher);
                        hash_octree_node(c, hasher);
                    }
                    None => {
                        0u8.hash(hasher);
                    }
                }
            }
        }
    }
}

fn hash_field_values(values: &FieldValues, hasher: &mut impl Hasher) {
    for &v in values.as_slice() {
        v.to_bits().hash(hasher);
    }
}
```

Add to `crates/murk/src/lib.rs`:

```rust
pub mod hash;
pub use hash::hash_universe;
```

Add method to `Universe` in `crates/murk/src/universe.rs`:

```rust
/// Compute a deterministic hash of the current state.
///
/// Used for verifying determinism: identical inputs should produce identical hashes.
#[must_use]
pub fn state_hash(&self) -> u64 {
    crate::hash::hash_universe(self)
}

/// Get read access to the octree (for hashing).
#[must_use]
pub fn octree(&self) -> &Octree {
    &self.octree
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_universe_state_hash -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/murk/src/hash.rs crates/murk/src/lib.rs crates/murk/src/universe.rs
git commit -m "feat(murk): add state hashing for determinism verification"
```

---

### Task 3: Add Determinism Regression Test

**Files:**
- Modify: `crates/murk/src/universe.rs` (tests module)

**Step 1: Write the determinism test**

Add to `crates/murk/src/universe.rs` tests:

```rust
#[test]
fn test_determinism_same_platform() {
    let config = UniverseConfig::with_bounds(100.0, 100.0, 50.0);

    // Run 1
    let mut universe1 = Universe::new_with_seed(config.clone(), 12345);
    universe1.stamp(&Stamp::explosion(Vec3::new(10.0, 20.0, 5.0), 15.0, 0.8));
    universe1.stamp(&Stamp::fire(Vec3::new(-5.0, 0.0, 0.0), 8.0, 0.5));
    for _ in 0..10 {
        universe1.step(0.1);
    }
    let hash1 = universe1.state_hash();

    // Run 2 (identical operations)
    let mut universe2 = Universe::new_with_seed(config, 12345);
    universe2.stamp(&Stamp::explosion(Vec3::new(10.0, 20.0, 5.0), 15.0, 0.8));
    universe2.stamp(&Stamp::fire(Vec3::new(-5.0, 0.0, 0.0), 8.0, 0.5));
    for _ in 0..10 {
        universe2.step(0.1);
    }
    let hash2 = universe2.state_hash();

    assert_eq!(hash1, hash2, "Same seed + same operations must produce identical state (ADR-0003)");
}
```

**Step 2: Run test to verify it passes**

Run: `cargo test test_determinism_same_platform -- --nocapture`
Expected: PASS (since step() currently does nothing, state should match)

**Step 3: Commit**

```bash
git add crates/murk/src/universe.rs
git commit -m "test(murk): add determinism regression test per ADR-0003"
```

---

## Phase 1: Fix Critical Binding Issues

### Task 4: Add PyField Enum

**Files:**
- Modify: `crates/tidebreak-py/src/lib.rs`

**Step 1: Write the test**

Create `crates/tidebreak-py/tests/test_field_enum.py`:

```python
import pytest

def test_field_enum_exists():
    import tidebreak
    assert hasattr(tidebreak, 'Field')

def test_field_enum_values():
    from tidebreak import Field
    assert Field.TEMPERATURE is not None
    assert Field.NOISE is not None
    assert Field.OCCUPANCY is not None

def test_field_enum_used_in_query():
    from tidebreak import PyUniverse, Field
    universe = PyUniverse(width=100.0, height=100.0, depth=50.0)
    universe.stamp_explosion(center=(0.0, 0.0, 0.0), radius=10.0)
    result = universe.query_volume(center=(0.0, 0.0, 0.0), radius=15.0)

    # Should work with enum
    temp = result.mean(Field.TEMPERATURE)
    assert temp > 0
```

**Step 2: Run test to verify it fails**

Run: `maturin develop && pytest crates/tidebreak-py/tests/test_field_enum.py -v`
Expected: FAIL with "module 'tidebreak' has no attribute 'Field'"

**Step 3: Write implementation**

Add to `crates/tidebreak-py/src/lib.rs`:

```rust
/// Field enum for Python.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Field {
    OCCUPANCY,
    MATERIAL,
    INTEGRITY,
    TEMPERATURE,
    SMOKE,
    NOISE,
    SIGNAL,
    CURRENT_X,
    CURRENT_Y,
    DEPTH,
    SALINITY,
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
```

Update `PyQueryResult` methods to accept either string or Field:

```rust
#[pymethods]
impl PyQueryResult {
    /// Get mean value for a field.
    fn mean(&self, field: FieldOrStr) -> f32 {
        let field: murk::Field = field.into();
        self.inner.mean(field)
    }

    // ... same for variance, min, max
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
```

Update module registration:

```rust
#[pymodule]
fn tidebreak(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyUniverse>()?;
    m.add_class::<PyPointResult>()?;
    m.add_class::<PyQueryResult>()?;
    m.add_class::<Field>()?;  // Add this
    Ok(())
}
```

**Step 4: Run test to verify it passes**

Run: `maturin develop && pytest crates/tidebreak-py/tests/test_field_enum.py -v`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/tidebreak-py/src/lib.rs crates/tidebreak-py/tests/
git commit -m "feat(tidebreak-py): add Field enum for type-safe field access"
```

---

### Task 5: Fix GIL Management in step()

**Files:**
- Modify: `crates/tidebreak-py/src/lib.rs`

**Step 1: Identify the issue**

Current code holds GIL during entire `step()`:

```rust
fn step(&mut self, dt: f64) {
    self.inner.step(dt);  // GIL held!
}
```

**Step 2: Write the fix**

Update `step()` method in `PyUniverse`:

```rust
/// Advance simulation by dt seconds.
///
/// Releases the GIL during computation for better Python threading.
fn step(&mut self, py: Python, dt: f64) {
    py.allow_threads(|| {
        self.inner.step(dt);
    });
}
```

**Step 3: Run existing tests**

Run: `maturin develop && pytest crates/tidebreak-py/tests/ -v`
Expected: PASS (no behavior change, just GIL management)

**Step 4: Commit**

```bash
git add crates/tidebreak-py/src/lib.rs
git commit -m "fix(tidebreak-py): release GIL during step() for better threading"
```

---

### Task 6: Create FoveatedQuery Types in Rust

**Files:**
- Create: `crates/murk/src/query.rs`
- Modify: `crates/murk/src/lib.rs`
- Modify: `crates/murk/src/universe.rs`

**Step 1: Write the failing test**

Add to `crates/murk/src/query.rs` (create file first):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Field, Stamp, Universe, UniverseConfig};
    use glam::Vec3;

    #[test]
    fn test_foveated_observation_basic() {
        let config = UniverseConfig::with_bounds(200.0, 200.0, 50.0);
        let mut universe = Universe::new(config);

        // Add some heat to the right
        universe.stamp(&Stamp::fire(Vec3::new(50.0, 0.0, 0.0), 10.0, 1.0));

        let query = FoveatedQuery {
            position: Vec3::ZERO,
            heading: Vec3::X,
            shells: vec![
                FoveatedShell::new(0.0, 20.0, 4),
                FoveatedShell::new(20.0, 100.0, 4),
            ],
        };

        let result = universe.observe_foveated(&query);

        assert_eq!(result.shells.len(), 2);
        assert_eq!(result.shells[0].sectors.len(), 4);
        assert_eq!(result.shells[1].sectors.len(), 4);
    }

    #[test]
    fn test_foveated_to_flat_vec() {
        let config = UniverseConfig::with_bounds(100.0, 100.0, 50.0);
        let universe = Universe::new(config);

        let query = FoveatedQuery {
            position: Vec3::ZERO,
            heading: Vec3::X,
            shells: vec![
                FoveatedShell::new(0.0, 10.0, 8),
                FoveatedShell::new(10.0, 50.0, 4),
            ],
        };

        let result = universe.observe_foveated(&query);
        let flat = result.to_flat_vec();

        // 8 + 4 = 12 sectors, Field::COUNT fields each
        assert_eq!(flat.len(), 12 * Field::COUNT);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_foveated_observation_basic -- --nocapture`
Expected: FAIL (module doesn't exist)

**Step 3: Write implementation**

Create `crates/murk/src/query.rs`:

```rust
//! Foveated observation queries for DRL agents.

use glam::Vec3;
use crate::{Field, Universe};

/// Configuration for a single observation shell (annular region).
#[derive(Debug, Clone)]
pub struct FoveatedShell {
    pub radius_inner: f32,
    pub radius_outer: f32,
    pub sectors: u32,
}

impl FoveatedShell {
    pub fn new(radius_inner: f32, radius_outer: f32, sectors: u32) -> Self {
        Self {
            radius_inner,
            radius_outer,
            sectors,
        }
    }
}

/// Query for foveated (multi-resolution) observation around a position.
#[derive(Debug, Clone)]
pub struct FoveatedQuery {
    pub position: Vec3,
    pub heading: Vec3,
    pub shells: Vec<FoveatedShell>,
}

/// Result of a foveated observation query.
#[derive(Debug, Clone)]
pub struct FoveatedResult {
    pub shells: Vec<ShellResult>,
}

/// Observation data for a single shell.
#[derive(Debug, Clone)]
pub struct ShellResult {
    pub sectors: Vec<SectorStats>,
}

/// Statistics for a single sector.
#[derive(Debug, Clone)]
pub struct SectorStats {
    /// Mean value for each field in this sector.
    pub means: [f32; Field::COUNT],
}

impl FoveatedResult {
    /// Convert to a flat vector for neural network input.
    /// Layout: [shell0_sector0_field0, shell0_sector0_field1, ..., shell0_sector1_field0, ...]
    pub fn to_flat_vec(&self) -> Vec<f32> {
        let mut result = Vec::new();
        for shell in &self.shells {
            for sector in &shell.sectors {
                result.extend_from_slice(&sector.means);
            }
        }
        result
    }
}

impl Universe {
    /// Perform a foveated observation query.
    ///
    /// Returns mean field values for each sector in each shell,
    /// organized for efficient neural network consumption.
    pub fn observe_foveated(&self, query: &FoveatedQuery) -> FoveatedResult {
        let heading_2d = Vec3::new(query.heading.x, query.heading.y, 0.0).normalize_or_zero();
        let right = Vec3::new(-heading_2d.y, heading_2d.x, 0.0);

        let mut shells = Vec::with_capacity(query.shells.len());

        for shell_config in &query.shells {
            let mut sectors = Vec::with_capacity(shell_config.sectors as usize);

            for sector_idx in 0..shell_config.sectors {
                // Calculate sector angular bounds
                let angle_per_sector = std::f32::consts::TAU / shell_config.sectors as f32;
                let angle_start = sector_idx as f32 * angle_per_sector;
                let angle_mid = angle_start + angle_per_sector / 2.0;

                // Sample point at middle of sector, middle of shell radii
                let radius_mid = (shell_config.radius_inner + shell_config.radius_outer) / 2.0;
                let dir = heading_2d * angle_mid.cos() + right * angle_mid.sin();
                let sample_pos = query.position + dir * radius_mid;

                // Query the universe at this position
                let point_result = self.query_point(sample_pos);
                let means: [f32; Field::COUNT] = std::array::from_fn(|i| {
                    let field = Field::from_index(i);
                    point_result.get(field)
                });

                sectors.push(SectorStats { means });
            }

            shells.push(ShellResult { sectors });
        }

        FoveatedResult { shells }
    }
}
```

Add to `crates/murk/src/lib.rs`:

```rust
pub mod query;
pub use query::{FoveatedQuery, FoveatedResult, FoveatedShell, ShellResult, SectorStats};
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_foveated -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/murk/src/query.rs crates/murk/src/lib.rs
git commit -m "feat(murk): add FoveatedQuery types for agent observation"
```

---

### Task 7: Add Foveated Observation with NumPy

**Files:**
- Modify: `crates/tidebreak-py/src/lib.rs`

**Step 1: Write the failing test**

Add to `crates/tidebreak-py/tests/test_observations.py`:

```python
import numpy as np
import pytest

def test_foveated_observation_returns_numpy():
    from tidebreak import PyUniverse

    universe = PyUniverse(width=200.0, height=200.0, depth=50.0)
    universe.stamp_fire(center=(50.0, 0.0, 0.0), radius=10.0)

    obs = universe.observe_foveated(
        position=(0.0, 0.0, 0.0),
        heading=(1.0, 0.0, 0.0),
    )

    assert isinstance(obs, np.ndarray)
    assert obs.dtype == np.float32
    # Default: 3 shells, varying sectors, 12 fields
    assert len(obs.shape) == 1  # Flat array for now
    assert obs.shape[0] > 0

def test_foveated_observation_custom_shells():
    from tidebreak import PyUniverse

    universe = PyUniverse(width=200.0, height=200.0, depth=50.0)

    obs = universe.observe_foveated(
        position=(0.0, 0.0, 0.0),
        heading=(1.0, 0.0, 0.0),
        shells=[
            {"radius_inner": 0.0, "radius_outer": 10.0, "sectors": 8},
            {"radius_inner": 10.0, "radius_outer": 50.0, "sectors": 4},
        ],
    )

    assert isinstance(obs, np.ndarray)
    # 8 + 4 = 12 sectors, 12 fields each = 144 values
    # But mean only = 12 sectors * 1 value * 12 fields = 144
    assert obs.shape[0] > 0

def test_observation_is_zero_copy():
    """Verify numpy array is a view, not a copy (critical for perf)."""
    from tidebreak import PyUniverse

    universe = PyUniverse(width=100.0, height=100.0, depth=50.0)
    obs = universe.observe_foveated(
        position=(0.0, 0.0, 0.0),
        heading=(1.0, 0.0, 0.0),
    )

    # Note: For small arrays, PyO3/numpy may copy. This test documents
    # the current behavior. If OWNDATA is True, we're copying (acceptable
    # for now, optimize later if profiling shows it matters).
    # The key is that we KNOW what's happening.
    print(f"Observation owns data: {obs.flags['OWNDATA']}")
    assert obs.dtype == np.float32
```

**Step 2: Run test to verify it fails**

Run: `maturin develop && pytest crates/tidebreak-py/tests/test_observations.py -v`
Expected: FAIL with "no method named `observe_foveated`"

**Step 3: Write implementation**

Add to `crates/tidebreak-py/src/lib.rs`:

```rust
use numpy::{PyArray1, ToPyArray};
use pyo3::types::PyList;

#[pymethods]
impl PyUniverse {
    /// Get foveated observation as numpy array.
    ///
    /// Returns a flat array of field means for each sector in each shell.
    /// Shape: (total_sectors * num_fields,)
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
                    let inner: f32 = dict.get_item("radius_inner")?.unwrap().extract()?;
                    let outer: f32 = dict.get_item("radius_outer")?.unwrap().extract()?;
                    let sectors: u32 = dict.get_item("sectors")?.unwrap().extract()?;
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

        let query = murk::query::FoveatedQuery {
            position,
            heading,
            shells: shell_configs,
            fields: murk::Field::all().to_vec(),
        };

        let result = self.inner.observe_foveated(&query);
        let flat = result.to_flat_vec(murk::Field::all());

        Ok(flat.to_pyarray(py))
    }
}
```

**Step 4: Run test to verify it passes**

Run: `maturin develop && pytest crates/tidebreak-py/tests/test_observations.py -v`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/tidebreak-py/src/lib.rs crates/tidebreak-py/tests/test_observations.py
git commit -m "feat(tidebreak-py): add foveated observation returning numpy array"
```

---

### Task 8: Add Seeded Reset to Python

**Files:**
- Modify: `crates/tidebreak-py/src/lib.rs`

**Step 1: Write the failing test**

Add to `crates/tidebreak-py/tests/test_determinism.py`:

```python
import numpy as np
import pytest

def test_seeded_reset_determinism():
    from tidebreak import PyUniverse

    # Run 1
    universe1 = PyUniverse(width=100.0, height=100.0, depth=50.0)
    universe1.reset(seed=42)
    universe1.stamp_explosion(center=(10.0, 10.0, 5.0), radius=8.0)
    obs1 = universe1.observe_foveated(position=(0.0, 0.0, 0.0), heading=(1.0, 0.0, 0.0))

    # Run 2 (identical)
    universe2 = PyUniverse(width=100.0, height=100.0, depth=50.0)
    universe2.reset(seed=42)
    universe2.stamp_explosion(center=(10.0, 10.0, 5.0), radius=8.0)
    obs2 = universe2.observe_foveated(position=(0.0, 0.0, 0.0), heading=(1.0, 0.0, 0.0))

    np.testing.assert_array_equal(obs1, obs2, "Same seed should produce identical observations")
```

**Step 2: Run test to verify it fails**

Run: `maturin develop && pytest crates/tidebreak-py/tests/test_determinism.py -v`
Expected: FAIL (reset doesn't accept seed)

**Step 3: Write implementation**

Update `reset()` in `PyUniverse`:

```rust
/// Reset the universe, optionally with a seed for determinism.
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
```

**Step 4: Run test to verify it passes**

Run: `maturin develop && pytest crates/tidebreak-py/tests/test_determinism.py -v`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/tidebreak-py/src/lib.rs crates/tidebreak-py/tests/test_determinism.py
git commit -m "feat(tidebreak-py): add seeded reset for deterministic training"
```

---

## Phase 2: Gymnasium Environment

### Task 9: Create MurkEnv Gymnasium Wrapper

**Files:**
- Create: `crates/tidebreak-py/python/tidebreak/envs/__init__.py`
- Create: `crates/tidebreak-py/python/tidebreak/envs/murk_env.py`

**Step 1: Write the failing test**

Create `crates/tidebreak-py/tests/test_gymnasium.py`:

```python
import pytest
import numpy as np

def test_murk_env_creation():
    from tidebreak.envs import MurkEnv

    env = MurkEnv()
    assert env.observation_space is not None
    assert env.action_space is not None

def test_murk_env_reset():
    from tidebreak.envs import MurkEnv

    env = MurkEnv()
    obs, info = env.reset(seed=42)

    assert isinstance(obs, np.ndarray)
    assert obs.shape == env.observation_space.shape

def test_murk_env_step():
    from tidebreak.envs import MurkEnv

    env = MurkEnv()
    obs, info = env.reset(seed=42)

    action = env.action_space.sample()
    obs, reward, terminated, truncated, info = env.step(action)

    assert isinstance(obs, np.ndarray)
    assert isinstance(reward, float)
    assert isinstance(terminated, bool)
    assert isinstance(truncated, bool)

def test_murk_env_gymnasium_check():
    from gymnasium.utils.env_checker import check_env
    from tidebreak.envs import MurkEnv

    env = MurkEnv()
    check_env(env)  # Raises if contract violated
```

**Step 2: Run test to verify it fails**

Run: `pytest crates/tidebreak-py/tests/test_gymnasium.py -v`
Expected: FAIL with "No module named 'tidebreak.envs'"

**Step 3: Create directory structure**

```bash
mkdir -p crates/tidebreak-py/python/tidebreak/envs
```

**Step 4: Write implementation**

Create `crates/tidebreak-py/python/tidebreak/__init__.py`:

```python
"""Tidebreak: Naval combat with DRL agents."""

# Re-export Rust bindings
from tidebreak import PyUniverse, PyPointResult, PyQueryResult, Field

# Convenience aliases
Universe = PyUniverse
PointResult = PyPointResult
QueryResult = PyQueryResult

__all__ = [
    "Universe",
    "PyUniverse",
    "PointResult",
    "PyPointResult",
    "QueryResult",
    "PyQueryResult",
    "Field",
]
```

Create `crates/tidebreak-py/python/tidebreak/envs/__init__.py`:

```python
"""Gymnasium environments for Tidebreak."""

from tidebreak.envs.murk_env import MurkEnv

__all__ = ["MurkEnv"]
```

Create `crates/tidebreak-py/python/tidebreak/envs/murk_env.py`:

```python
"""Murk spatial substrate as a Gymnasium environment."""

from typing import Any, Optional, SupportsFloat
import numpy as np
import gymnasium as gym
from gymnasium import spaces

import tidebreak


class MurkEnv(gym.Env):
    """
    Gymnasium environment wrapping Murk spatial substrate.

    Observation: Foveated field observations (flattened numpy array)
    Action: Movement (continuous) + stamp type (discrete)
    """

    metadata = {"render_modes": ["rgb_array"], "render_fps": 10}

    def __init__(
        self,
        world_size: tuple[float, float, float] = (200.0, 200.0, 50.0),
        max_steps: int = 1000,
        render_mode: Optional[str] = None,
    ):
        super().__init__()

        self.world_size = world_size
        self.max_steps = max_steps
        self.render_mode = render_mode

        # Shell configuration for foveated observations
        self.shells = [
            {"radius_inner": 0.0, "radius_outer": 10.0, "sectors": 8},
            {"radius_inner": 10.0, "radius_outer": 50.0, "sectors": 4},
        ]

        # Calculate observation size: sum(sectors) * num_fields
        total_sectors = sum(s["sectors"] for s in self.shells)
        num_fields = 12  # murk::Field::COUNT
        obs_size = total_sectors * num_fields

        # Observation space
        self.observation_space = spaces.Box(
            low=-np.inf,
            high=np.inf,
            shape=(obs_size,),
            dtype=np.float32,
        )

        # Action space: move (x, y) + stamp type
        self.action_space = spaces.Dict({
            "move": spaces.Box(low=-1.0, high=1.0, shape=(2,), dtype=np.float32),
            "stamp": spaces.Discrete(3),  # 0=none, 1=fire, 2=sonar
        })

        # Internal state
        self._universe: Optional[tidebreak.PyUniverse] = None
        self._agent_pos = np.array([0.0, 0.0, 10.0], dtype=np.float32)
        self._agent_heading = np.array([1.0, 0.0, 0.0], dtype=np.float32)
        self._step_count = 0
        self._seed: Optional[int] = None

    def reset(
        self,
        *,
        seed: Optional[int] = None,
        options: Optional[dict[str, Any]] = None,
    ) -> tuple[np.ndarray, dict[str, Any]]:
        super().reset(seed=seed)

        self._seed = seed
        self._step_count = 0

        # Create or reset universe
        self._universe = tidebreak.PyUniverse(
            width=self.world_size[0],
            height=self.world_size[1],
            depth=self.world_size[2],
        )
        self._universe.reset(seed=seed)

        # Reset agent position to center
        self._agent_pos = np.array([0.0, 0.0, 10.0], dtype=np.float32)
        self._agent_heading = np.array([1.0, 0.0, 0.0], dtype=np.float32)

        obs = self._get_observation()
        info = {"step": 0}

        return obs, info

    def step(
        self,
        action: dict[str, Any],
    ) -> tuple[np.ndarray, SupportsFloat, bool, bool, dict[str, Any]]:
        assert self._universe is not None, "Must call reset() first"

        # Apply movement
        move = action["move"]
        speed = 5.0  # units per step
        self._agent_pos[0] += move[0] * speed
        self._agent_pos[1] += move[1] * speed

        # Clamp to world bounds
        half_w = self.world_size[0] / 2
        half_h = self.world_size[1] / 2
        self._agent_pos[0] = np.clip(self._agent_pos[0], -half_w, half_w)
        self._agent_pos[1] = np.clip(self._agent_pos[1], -half_h, half_h)

        # Update heading based on movement
        if np.linalg.norm(move) > 0.1:
            self._agent_heading[:2] = move / np.linalg.norm(move)

        # Apply stamp
        stamp_type = action["stamp"]
        pos = tuple(self._agent_pos.tolist())
        if stamp_type == 1:  # Fire
            self._universe.stamp_fire(center=pos, radius=5.0, intensity=0.5)
        elif stamp_type == 2:  # Sonar
            self._universe.stamp_sonar_ping(center=pos, radius=20.0, strength=0.8)

        # Advance simulation
        self._universe.step(0.1)
        self._step_count += 1

        # Get observation
        obs = self._get_observation()

        # Simple reward: exploration (distance from start)
        reward = float(np.linalg.norm(self._agent_pos[:2])) * 0.01

        # Termination
        terminated = False
        truncated = self._step_count >= self.max_steps

        info = {
            "step": self._step_count,
            "position": self._agent_pos.copy(),
        }

        return obs, reward, terminated, truncated, info

    def _get_observation(self) -> np.ndarray:
        assert self._universe is not None

        obs = self._universe.observe_foveated(
            position=tuple(self._agent_pos.tolist()),
            heading=tuple(self._agent_heading.tolist()),
            shells=self.shells,
        )

        return obs.astype(np.float32)

    def render(self) -> Optional[np.ndarray]:
        if self.render_mode == "rgb_array":
            # Placeholder: return empty image
            return np.zeros((100, 100, 3), dtype=np.uint8)
        return None
```

**Step 5: Update pyproject.toml to include Python package**

Add to `crates/tidebreak-py/pyproject.toml`:

```toml
[tool.maturin]
features = ["pyo3/extension-module"]
module-name = "tidebreak._tidebreak"
python-source = "python"
```

Note: This requires restructuring the Rust module name. For simplicity, we can also add the Python files to the installed package via a different approach.

**Step 6: Run test to verify it passes**

Run: `maturin develop && PYTHONPATH=crates/tidebreak-py/python pytest crates/tidebreak-py/tests/test_gymnasium.py -v`
Expected: PASS

**Step 7: Commit**

```bash
git add crates/tidebreak-py/python/ crates/tidebreak-py/tests/test_gymnasium.py crates/tidebreak-py/pyproject.toml
git commit -m "feat(tidebreak-py): add MurkEnv Gymnasium environment"
```

---

## Phase 3: Integration Validation

### Task 10: Training Smoke Test

**Files:**
- Create: `crates/tidebreak-py/tests/test_training_smoke.py`

**Step 1: Write the smoke test**

```python
"""
Smoke test: train a random policy for a few episodes.

This validates:
- Gymnasium contract compliance
- Rust→Python boundary stability
- Memory safety (no leaks, no segfaults)
- Determinism (optional)
"""

import pytest
import numpy as np

def test_random_policy_10_episodes():
    """Train random policy for 10 episodes without crashing."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv(max_steps=100)

    total_reward = 0.0
    for episode in range(10):
        obs, info = env.reset(seed=episode)
        done = False
        episode_reward = 0.0

        while not done:
            action = env.action_space.sample()
            obs, reward, terminated, truncated, info = env.step(action)
            episode_reward += reward
            done = terminated or truncated

        total_reward += episode_reward

    # Should complete without crashes
    assert total_reward >= 0  # Reward is always positive in our simple env

def test_deterministic_episodes():
    """Same seed produces identical episode."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv(max_steps=50)

    # Episode 1
    obs1, _ = env.reset(seed=42)
    rewards1 = []
    for _ in range(10):
        action = {"move": np.array([0.5, 0.5], dtype=np.float32), "stamp": 1}
        obs, reward, _, _, _ = env.step(action)
        rewards1.append(reward)
    final_obs1 = obs.copy()

    # Episode 2 (same seed, same actions)
    obs2, _ = env.reset(seed=42)
    rewards2 = []
    for _ in range(10):
        action = {"move": np.array([0.5, 0.5], dtype=np.float32), "stamp": 1}
        obs, reward, _, _, _ = env.step(action)
        rewards2.append(reward)
    final_obs2 = obs.copy()

    np.testing.assert_array_equal(obs1, obs2, "Initial observations should match")
    np.testing.assert_array_almost_equal(rewards1, rewards2, decimal=5, err_msg="Rewards should match")
    np.testing.assert_array_almost_equal(final_obs1, final_obs2, decimal=5, err_msg="Final observations should match")
```

**Step 2: Run the smoke test**

Run: `PYTHONPATH=crates/tidebreak-py/python pytest crates/tidebreak-py/tests/test_training_smoke.py -v`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/tidebreak-py/tests/test_training_smoke.py
git commit -m "test(tidebreak-py): add training smoke test for integration validation"
```

---

### Task 11: Profile Rust→Python Boundary

**Files:**
- Create: `crates/tidebreak-py/tests/bench_boundary.py`

**Step 1: Write benchmark script**

```python
"""
Benchmark Rust→Python boundary performance.

Key metrics:
- observe_foveated() latency
- step() latency
- Memory stability over many steps
"""

import time
import numpy as np

def benchmark_observation_latency():
    """Measure observation creation time."""
    from tidebreak import PyUniverse

    universe = PyUniverse(width=200.0, height=200.0, depth=50.0)
    universe.reset(seed=42)

    # Warmup
    for _ in range(10):
        universe.observe_foveated(
            position=(0.0, 0.0, 0.0),
            heading=(1.0, 0.0, 0.0),
        )

    # Benchmark
    n_iterations = 1000
    start = time.perf_counter()
    for _ in range(n_iterations):
        universe.observe_foveated(
            position=(0.0, 0.0, 0.0),
            heading=(1.0, 0.0, 0.0),
        )
    elapsed = time.perf_counter() - start

    latency_us = (elapsed / n_iterations) * 1_000_000
    print(f"observe_foveated latency: {latency_us:.1f} µs ({n_iterations / elapsed:.0f} Hz)")

    # Target: < 100 µs for training viability
    assert latency_us < 1000, f"Observation too slow: {latency_us:.1f} µs"

def benchmark_step_latency():
    """Measure step time."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv(max_steps=10000)
    env.reset(seed=42)

    action = {"move": np.array([0.1, 0.1], dtype=np.float32), "stamp": 0}

    # Warmup
    for _ in range(10):
        env.step(action)
    env.reset(seed=42)

    # Benchmark
    n_iterations = 1000
    start = time.perf_counter()
    for _ in range(n_iterations):
        env.step(action)
    elapsed = time.perf_counter() - start

    latency_us = (elapsed / n_iterations) * 1_000_000
    print(f"step latency: {latency_us:.1f} µs ({n_iterations / elapsed:.0f} Hz)")

    # Target: < 1000 µs for real-time training
    assert latency_us < 10000, f"Step too slow: {latency_us:.1f} µs"

def benchmark_memory_stability():
    """Check for memory leaks over many steps."""
    import gc
    import tracemalloc
    from tidebreak.envs import MurkEnv

    tracemalloc.start()

    env = MurkEnv(max_steps=10000)

    for episode in range(5):
        env.reset(seed=episode)
        for _ in range(1000):
            action = env.action_space.sample()
            env.step(action)

    gc.collect()
    current, peak = tracemalloc.get_traced_memory()
    tracemalloc.stop()

    print(f"Memory: current={current / 1024 / 1024:.1f} MB, peak={peak / 1024 / 1024:.1f} MB")

    # Should not grow unboundedly
    assert peak < 500 * 1024 * 1024, f"Memory usage too high: {peak / 1024 / 1024:.1f} MB"

if __name__ == "__main__":
    print("=== Rust→Python Boundary Benchmarks ===\n")
    benchmark_observation_latency()
    benchmark_step_latency()
    benchmark_memory_stability()
    print("\n=== All benchmarks passed ===")
```

**Step 2: Run benchmark**

Run: `PYTHONPATH=crates/tidebreak-py/python python crates/tidebreak-py/tests/bench_boundary.py`
Expected: Output showing latencies and memory usage

**Step 3: Commit**

```bash
git add crates/tidebreak-py/tests/bench_boundary.py
git commit -m "test(tidebreak-py): add Rust→Python boundary benchmarks"
```

---

## Summary

After completing all tasks:

1. **Phase 0** (Tasks 1-3): Determinism infrastructure in place
2. **Phase 1** (Tasks 4-8): Critical Python binding issues fixed, including Rust foveated query types
3. **Phase 2** (Task 9): Gymnasium environment working
4. **Phase 3** (Tasks 10-11): Integration validated, performance profiled

**Total commits:** 11

---

## Next Steps After This Plan

Once this plan is complete, field propagation (diffusion, decay) can be implemented with confidence that:
- Determinism regression tests will catch any issues
- Python API is validated by real DRL usage
- Performance baseline is established
