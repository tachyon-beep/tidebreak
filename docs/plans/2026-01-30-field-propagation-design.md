# Field Propagation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement field diffusion and decay in Murk's sparse octree so that thermal gradients spread, smoke dissipates, and noise fades over time.

**Architecture:** Two-buffer leaf-only propagation with O(log N) neighbor finding. Read all leaf values, compute updates using discrete Laplacian diffusion and exponential decay, write updates back. Deterministic via sorted iteration order.

**Tech Stack:** Rust (murk crate), PyO3 bindings for Python exposure

---

## Background

Currently `Universe::step()` just increments tick/time. The propagation infrastructure exists:
- `Propagation` enum with `None`, `Diffusion`, `Decay`, `DiffusionDecay` variants
- Default field configs assign propagation types (Temperature→Diffusion, Smoke→DiffusionDecay, Noise→Decay)

What's missing:
- Neighbor finding in the sparse octree
- Actual diffusion/decay computation
- Integration into step()

## Design Decisions

**Leaf-Only vs Hierarchical:** Using leaf-only propagation for simplicity. Values only change at leaf nodes. Hierarchical propagation (where coarser levels diffuse faster) is more physically accurate but significantly more complex with split/merge operations.

**2D vs 3D Neighbors:** Using 4 face-neighbors in XY plane (±x, ±y) rather than 6 face-neighbors (±z adds complexity for depth layers per ADR-0002). Can extend to 3D later if needed.

**Two-Buffer Approach:** Read all old values first, then compute all new values, then write. This ensures iteration order doesn't affect results (determinism).

**Boundary Conditions:** At world edges, treat neighbors as having default field values (Neumann-like boundary).

---

## Task 1: Add Neighbor Finding to Octree

**Files:**
- Modify: `crates/murk/src/octree.rs`
- Create: `crates/murk/src/octree.rs` (add methods)

**Step 1: Write failing test for neighbor finding**

```rust
// In octree.rs tests module
#[test]
fn test_find_neighbor_simple() {
    let mut octree = Octree::with_bounds(Bounds::new(100.0, 100.0, 100.0), 10.0);

    // Set two adjacent cells
    let mut values_a = FieldValues::new();
    values_a.set(Field::Temperature, 500.0);
    octree.set_point(Vec3::new(-25.0, 0.0, 0.0), values_a);

    let mut values_b = FieldValues::new();
    values_b.set(Field::Temperature, 300.0);
    octree.set_point(Vec3::new(25.0, 0.0, 0.0), values_b);

    // Find +X neighbor of point at (-25, 0, 0)
    let neighbor = octree.find_neighbor(Vec3::new(-25.0, 0.0, 0.0), Direction::PosX);
    assert!(neighbor.is_some());
    assert!((neighbor.unwrap().get(Field::Temperature) - 300.0).abs() < 0.1);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p murk test_find_neighbor_simple`
Expected: FAIL with "find_neighbor not found"

**Step 3: Add Direction enum and find_neighbor method**

```rust
/// Direction for neighbor finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

impl Direction {
    /// Get the offset vector for this direction.
    pub fn offset(&self) -> Vec3 {
        match self {
            Direction::PosX => Vec3::X,
            Direction::NegX => Vec3::NEG_X,
            Direction::PosY => Vec3::Y,
            Direction::NegY => Vec3::NEG_Y,
            Direction::PosZ => Vec3::Z,
            Direction::NegZ => Vec3::NEG_Z,
        }
    }

    /// Get the 4 XY-plane directions.
    pub fn xy_directions() -> &'static [Direction] {
        &[Direction::PosX, Direction::NegX, Direction::PosY, Direction::NegY]
    }
}

impl Octree {
    /// Find the field values of the neighboring cell in the given direction.
    ///
    /// Returns None if the neighbor is outside world bounds.
    /// Returns default values if neighbor cell is empty.
    pub fn find_neighbor(&self, position: Vec3, direction: Direction) -> Option<FieldValues> {
        // Calculate neighbor position (offset by cell size at this position)
        let cell_size = self.cell_size_at(position);
        let neighbor_pos = position + direction.offset() * cell_size;

        // Check bounds
        if !self.config.bounds.contains(neighbor_pos) {
            return None;
        }

        // Query the neighbor position
        let result = self.query_point(&PointQuery::new(neighbor_pos));
        Some(result.values)
    }

    /// Get the cell size at a given position.
    fn cell_size_at(&self, position: Vec3) -> f32 {
        // Find the leaf depth at this position
        let result = self.query_point(&PointQuery::new(position));
        let depth = result.depth;

        // Cell size = world_size / 2^depth
        let world_size = self.config.bounds.size().x;
        world_size / (1 << depth) as f32
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p murk test_find_neighbor_simple`
Expected: PASS

**Step 5: Add more neighbor tests**

```rust
#[test]
fn test_find_neighbor_at_boundary() {
    let octree = Octree::with_bounds(Bounds::new(100.0, 100.0, 100.0), 10.0);

    // At +X boundary, +X neighbor should be None
    let neighbor = octree.find_neighbor(Vec3::new(45.0, 0.0, 0.0), Direction::PosX);
    assert!(neighbor.is_none());

    // At -X boundary, -X neighbor should be None
    let neighbor = octree.find_neighbor(Vec3::new(-45.0, 0.0, 0.0), Direction::NegX);
    assert!(neighbor.is_none());
}

#[test]
fn test_find_neighbor_empty_returns_default() {
    let octree = Octree::with_bounds(Bounds::new(100.0, 100.0, 100.0), 10.0);

    // Empty octree, neighbor should return default values
    let neighbor = octree.find_neighbor(Vec3::new(0.0, 0.0, 0.0), Direction::PosX);
    assert!(neighbor.is_some());
    // Default temperature is ~293K
    assert!((neighbor.unwrap().get(Field::Temperature) - 0.0).abs() < 0.1);
}
```

**Step 6: Run all neighbor tests**

Run: `cargo test -p murk find_neighbor`
Expected: All PASS

**Step 7: Commit**

```bash
git add crates/murk/src/octree.rs
git commit -m "feat(murk): add neighbor finding for diffusion support

Add Direction enum and find_neighbor() method to Octree for
locating adjacent cells. Supports 6-directional lookup with
boundary handling (returns None at world edges).

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 2: Create Propagation Module

**Files:**
- Create: `crates/murk/src/propagation.rs`
- Modify: `crates/murk/src/lib.rs` (add module)

**Step 1: Write failing test for decay**

```rust
// In propagation.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::Field;

    #[test]
    fn test_decay_reduces_value() {
        let old_value = 100.0;
        let default = 0.0;
        let rate = 0.3;
        let dt = 0.1;

        let new_value = apply_decay(old_value, default, rate, dt);

        // Value should move toward default
        assert!(new_value < old_value);
        assert!(new_value > default);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p murk test_decay_reduces_value`
Expected: FAIL with "propagation module not found"

**Step 3: Create propagation module with decay function**

```rust
//! Field propagation: diffusion and decay.
//!
//! Propagation updates field values over time:
//! - Diffusion: spreads values to neighbors (heat conduction)
//! - Decay: values fade toward default over time (noise dissipation)

use crate::field::{Field, FieldConfig, FieldValues, Propagation};
use crate::octree::{Direction, Octree};
use glam::Vec3;

/// Apply exponential decay toward default value.
///
/// Formula: new = default + (old - default) * exp(-rate * dt)
pub fn apply_decay(old_value: f32, default: f32, rate: f32, dt: f32) -> f32 {
    let decay_factor = (-rate * dt).exp();
    default + (old_value - default) * decay_factor
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p murk test_decay_reduces_value`
Expected: PASS

**Step 5: Add diffusion function and test**

```rust
#[test]
fn test_diffusion_spreads_heat() {
    // Hot center cell surrounded by cold neighbors
    let center_temp = 500.0;
    let neighbor_temps = [300.0, 300.0, 300.0, 300.0]; // 4 XY neighbors
    let rate = 0.1;
    let dt = 0.1;

    let new_temp = apply_diffusion(center_temp, &neighbor_temps, rate, dt);

    // Center should cool down (heat flows to cooler neighbors)
    assert!(new_temp < center_temp);
}

/// Apply discrete Laplacian diffusion.
///
/// Formula: new = old + rate * dt * Σ(neighbor_i - old)
///
/// For 4 neighbors in 2D: Σ(neighbor_i - old) = Σneighbor_i - 4*old
pub fn apply_diffusion(old_value: f32, neighbor_values: &[f32], rate: f32, dt: f32) -> f32 {
    let neighbor_sum: f32 = neighbor_values.iter().sum();
    let n = neighbor_values.len() as f32;
    let laplacian = neighbor_sum - n * old_value;
    old_value + rate * dt * laplacian
}
```

**Step 6: Run diffusion test**

Run: `cargo test -p murk test_diffusion_spreads_heat`
Expected: PASS

**Step 7: Commit**

```bash
git add crates/murk/src/propagation.rs crates/murk/src/lib.rs
git commit -m "feat(murk): add propagation module with diffusion and decay

Implement core propagation functions:
- apply_decay(): exponential decay toward default value
- apply_diffusion(): discrete Laplacian diffusion

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 3: Implement Leaf Collection

**Files:**
- Modify: `crates/murk/src/octree.rs`

**Step 1: Write failing test for leaf collection**

```rust
#[test]
fn test_collect_leaves() {
    let mut octree = Octree::with_bounds(Bounds::new(100.0, 100.0, 100.0), 10.0);

    // Create two leaf nodes
    octree.set_point(Vec3::new(-25.0, 0.0, 0.0), FieldValues::new());
    octree.set_point(Vec3::new(25.0, 0.0, 0.0), FieldValues::new());

    let leaves = octree.collect_leaves();

    // Should have at least 2 leaves (may have more due to splitting)
    assert!(leaves.len() >= 2);

    // Each leaf should have position and values
    for (pos, values) in &leaves {
        assert!(octree.config().bounds.contains(*pos));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p murk test_collect_leaves`
Expected: FAIL with "collect_leaves not found"

**Step 3: Implement collect_leaves**

```rust
impl Octree {
    /// Collect all leaf nodes as (center_position, values) pairs.
    ///
    /// Leaves are collected in deterministic order (depth-first, octant order)
    /// for reproducible propagation.
    pub fn collect_leaves(&self) -> Vec<(Vec3, FieldValues)> {
        let mut leaves = Vec::new();
        self.collect_leaves_recursive(&self.root, &mut leaves);
        leaves
    }

    fn collect_leaves_recursive(&self, node: &OctreeNode, leaves: &mut Vec<(Vec3, FieldValues)>) {
        match &node.state {
            NodeState::Empty => {
                // Empty nodes don't contribute to propagation
            }
            NodeState::Leaf { values } => {
                leaves.push((node.bounds.center(), *values));
            }
            NodeState::Internal { children, .. } => {
                // Recurse in deterministic octant order (0-7)
                for child in children.iter().flatten() {
                    self.collect_leaves_recursive(child, leaves);
                }
            }
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p murk test_collect_leaves`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/murk/src/octree.rs
git commit -m "feat(murk): add collect_leaves for propagation traversal

Collect all leaf nodes as (position, values) pairs in deterministic
octant order for reproducible field propagation.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 4: Implement Full Propagation Step

**Files:**
- Modify: `crates/murk/src/propagation.rs`
- Modify: `crates/murk/src/universe.rs`

**Step 1: Write integration test for propagation**

```rust
// In universe.rs tests
#[test]
fn test_step_propagates_temperature() {
    let config = UniverseConfig::with_bounds(100.0, 100.0, 50.0);
    let mut universe = Universe::new(config);

    // Create a hot spot
    universe.stamp(&Stamp::fire(Vec3::ZERO, 10.0, 1.0));

    // Get initial temperature at nearby point
    let initial = universe.query_point(Vec3::new(20.0, 0.0, 0.0));
    let initial_temp = initial.values.get(Field::Temperature);

    // Step multiple times
    for _ in 0..10 {
        universe.step(0.1);
    }

    // Temperature at nearby point should have increased (heat diffused)
    let after = universe.query_point(Vec3::new(20.0, 0.0, 0.0));
    let after_temp = after.values.get(Field::Temperature);

    assert!(after_temp > initial_temp, "Heat should diffuse to nearby cells");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p murk test_step_propagates_temperature`
Expected: FAIL (temperature unchanged because step() is empty)

**Step 3: Implement propagate_all in propagation module**

```rust
use crate::universe::Universe;

/// Propagate all fields for one timestep.
pub fn propagate_all(universe: &mut Universe, dt: f64) {
    let dt_f32 = dt as f32;

    // Phase 1: Collect all leaves
    let leaves = universe.octree().collect_leaves();

    if leaves.is_empty() {
        return;
    }

    // Phase 2: Compute updates for each leaf
    let updates: Vec<(Vec3, FieldValues)> = leaves
        .iter()
        .map(|(pos, old_values)| {
            let mut new_values = *old_values;

            for field in Field::all() {
                let config = universe.field_config(*field);
                let old_val = old_values.get(*field);

                let new_val = match config.propagation {
                    Propagation::None => old_val,
                    Propagation::Diffusion { rate } => {
                        let neighbors = get_xy_neighbor_values(universe, *pos, *field);
                        apply_diffusion(old_val, &neighbors, rate, dt_f32)
                    }
                    Propagation::Decay { rate } => {
                        apply_decay(old_val, config.default_value, rate, dt_f32)
                    }
                    Propagation::DiffusionDecay { diffusion_rate, decay_rate } => {
                        let neighbors = get_xy_neighbor_values(universe, *pos, *field);
                        let diffused = apply_diffusion(old_val, &neighbors, diffusion_rate, dt_f32);
                        apply_decay(diffused, config.default_value, decay_rate, dt_f32)
                    }
                };

                new_values.set(*field, config.clamp(new_val));
            }

            (*pos, new_values)
        })
        .collect();

    // Phase 3: Apply updates
    for (pos, values) in updates {
        universe.set_point(pos, values);
    }
}

/// Get neighbor field values in the XY plane (4 neighbors).
fn get_xy_neighbor_values(universe: &Universe, pos: Vec3, field: Field) -> Vec<f32> {
    Direction::xy_directions()
        .iter()
        .filter_map(|dir| {
            universe.octree().find_neighbor(pos, *dir)
                .map(|values| values.get(field))
        })
        .collect()
}
```

**Step 4: Wire propagation into Universe::step()**

```rust
// In universe.rs
use crate::propagation;

impl Universe {
    pub fn step(&mut self, dt: f64) {
        // Propagate fields (diffusion, decay)
        propagation::propagate_all(self, dt);

        self.tick += 1;
        self.time += dt;
    }
}
```

**Step 5: Run test to verify it passes**

Run: `cargo test -p murk test_step_propagates_temperature`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/murk/src/propagation.rs crates/murk/src/universe.rs
git commit -m "feat(murk): implement field propagation in Universe::step()

Wire diffusion and decay into simulation step:
- Temperature diffuses (spreads to neighbors)
- Smoke diffuses and decays
- Noise decays toward silence
- Two-buffer approach ensures determinism

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 5: Determinism Tests

**Files:**
- Modify: `crates/murk/src/universe.rs` (add tests)

**Step 1: Write determinism test for propagation**

```rust
#[test]
fn test_propagation_determinism() {
    let config = UniverseConfig::with_bounds(100.0, 100.0, 50.0);

    // Run 1
    let mut u1 = Universe::new_with_seed(config.clone(), 42);
    u1.stamp(&Stamp::fire(Vec3::new(10.0, 20.0, 0.0), 15.0, 0.8));
    u1.stamp(&Stamp::explosion(Vec3::new(-10.0, 0.0, 0.0), 8.0, 0.5));
    for _ in 0..20 {
        u1.step(0.1);
    }
    let hash1 = u1.state_hash();

    // Run 2 (identical operations)
    let mut u2 = Universe::new_with_seed(config, 42);
    u2.stamp(&Stamp::fire(Vec3::new(10.0, 20.0, 0.0), 15.0, 0.8));
    u2.stamp(&Stamp::explosion(Vec3::new(-10.0, 0.0, 0.0), 8.0, 0.5));
    for _ in 0..20 {
        u2.step(0.1);
    }
    let hash2 = u2.state_hash();

    assert_eq!(hash1, hash2, "Propagation must be deterministic (ADR-0003)");
}

#[test]
fn test_decay_noise_fades() {
    let config = UniverseConfig::with_bounds(100.0, 100.0, 50.0);
    let mut universe = Universe::new(config);

    // Create explosion (generates noise)
    universe.stamp(&Stamp::explosion(Vec3::ZERO, 10.0, 1.0));

    let initial_noise = universe.query_point(Vec3::ZERO).values.get(Field::Noise);
    assert!(initial_noise > 0.0, "Explosion should create noise");

    // Step many times
    for _ in 0..100 {
        universe.step(0.1);
    }

    let final_noise = universe.query_point(Vec3::ZERO).values.get(Field::Noise);
    assert!(final_noise < initial_noise * 0.1, "Noise should decay significantly");
}
```

**Step 2: Run determinism tests**

Run: `cargo test -p murk propagation_determinism`
Run: `cargo test -p murk decay_noise_fades`
Expected: Both PASS

**Step 3: Commit**

```bash
git add crates/murk/src/universe.rs
git commit -m "test(murk): add propagation determinism and decay tests

Verify that:
- Identical seed + operations produce identical state hashes
- Noise decays toward zero over time
- ADR-0003 compliance for propagation

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 6: Performance Benchmark

**Files:**
- Modify: `crates/murk/benches/murk_bench.rs` (or create if needed)

**Step 1: Add propagation benchmark**

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use glam::Vec3;
use murk::{Stamp, Universe, UniverseConfig};

fn bench_propagation(c: &mut Criterion) {
    let config = UniverseConfig::with_bounds(200.0, 200.0, 50.0);
    let mut universe = Universe::new(config);

    // Create some hot spots to have leaves to propagate
    for i in 0..10 {
        let x = (i as f32 - 5.0) * 30.0;
        universe.stamp(&Stamp::fire(Vec3::new(x, 0.0, 0.0), 15.0, 1.0));
    }

    c.bench_function("propagation_step", |b| {
        b.iter(|| {
            universe.step(black_box(0.1));
        })
    });
}

criterion_group!(benches, bench_propagation);
criterion_main!(benches);
```

**Step 2: Run benchmark**

Run: `cargo bench -p murk`
Expected: Results showing microseconds per step

**Step 3: Commit benchmark**

```bash
git add crates/murk/benches/
git commit -m "bench(murk): add propagation step benchmark

Measure propagation performance with ~10 fire stamps.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 7: Python Bindings Update

**Files:**
- Modify: `crates/tidebreak-py/src/lib.rs`
- Modify: `crates/tidebreak-py/tests/test_propagation.py` (create)

**Step 1: Write Python test for propagation**

```python
"""Tests for field propagation in tidebreak Python bindings."""

def test_temperature_diffusion():
    """Temperature should spread to nearby cells over time."""
    from tidebreak import PyUniverse, Field

    universe = PyUniverse(width=100.0, height=100.0, depth=50.0)

    # Create hot spot at center
    universe.stamp_fire(center=(0.0, 0.0, 0.0), radius=10.0, intensity=1.0)

    # Check initial temperature at distant point
    initial = universe.query_point(position=(30.0, 0.0, 0.0))
    initial_temp = initial.get(Field.TEMPERATURE)

    # Step simulation
    for _ in range(20):
        universe.step(0.1)

    # Temperature at distant point should have increased
    after = universe.query_point(position=(30.0, 0.0, 0.0))
    after_temp = after.get(Field.TEMPERATURE)

    assert after_temp > initial_temp, "Heat should diffuse outward"


def test_noise_decay():
    """Noise should fade toward zero over time."""
    from tidebreak import PyUniverse, Field

    universe = PyUniverse(width=100.0, height=100.0, depth=50.0)

    # Create explosion (generates noise)
    universe.stamp_explosion(center=(0.0, 0.0, 0.0), radius=10.0)

    initial = universe.query_point(position=(0.0, 0.0, 0.0))
    initial_noise = initial.get(Field.NOISE)
    assert initial_noise > 0, "Explosion should create noise"

    # Step many times
    for _ in range(50):
        universe.step(0.1)

    after = universe.query_point(position=(0.0, 0.0, 0.0))
    after_noise = after.get(Field.NOISE)

    assert after_noise < initial_noise * 0.5, "Noise should decay significantly"
```

**Step 2: Run Python tests**

Run: `uv run pytest crates/tidebreak-py/tests/test_propagation.py -v`
Expected: Both PASS (since Rust propagation is already wired in)

**Step 3: Commit Python tests**

```bash
git add crates/tidebreak-py/tests/test_propagation.py
git commit -m "test(tidebreak-py): add Python tests for field propagation

Verify that Python bindings expose working propagation:
- Temperature diffuses outward from heat sources
- Noise decays toward zero over time

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 8: Documentation Update

**Files:**
- Modify: `docs/research/murk-design.md`

**Step 1: Update murk design doc with propagation details**

Add a section describing the implemented propagation model:

```markdown
## Field Propagation

Fields update each simulation tick via propagation rules:

### Diffusion
Temperature and salinity spread to neighboring cells using discrete Laplacian diffusion:

```
new_value = old_value + rate × dt × Σ(neighbor_i - old_value)
```

This models heat conduction and concentration gradients. Hot cells cool as heat flows to colder neighbors.

### Decay
Noise, signal, and sonar returns fade exponentially toward their default values:

```
new_value = default + (old_value - default) × exp(-rate × dt)
```

This models transient phenomena that dissipate over time.

### Combined (Smoke)
Smoke both spreads (diffusion) and dissipates (decay), creating realistic smoke plumes that expand and fade.

### Implementation Notes
- Two-buffer approach: read all old values, compute all new values, write updates
- 4-neighbor diffusion in XY plane (±x, ±y)
- Deterministic iteration order (depth-first, octant order)
- Boundary cells treat out-of-bounds neighbors as having default values
```

**Step 2: Commit documentation**

```bash
git add docs/research/murk-design.md
git commit -m "docs(murk): add field propagation documentation

Document diffusion, decay, and implementation details.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Verification Checklist

After all tasks complete:

- [ ] `cargo test -p murk` - All Rust tests pass
- [ ] `cargo clippy -p murk -- -D warnings` - No warnings
- [ ] `uv run pytest crates/tidebreak-py/tests/` - All Python tests pass
- [ ] `cargo bench -p murk` - Benchmark runs and shows reasonable performance
- [ ] State hash matches between identical runs (determinism verified)
- [ ] Temperature visibly spreads from heat sources
- [ ] Noise audibly fades after explosions (conceptually)
