# ADR 0006: Rust Core with Python Bindings

## Status
Accepted

## Context

Tidebreak has competing requirements:

1. **DRL Training Speed**: Training agents requires millions of environment steps. The simulation must run 100x+ faster than real-time.

2. **DRL Ecosystem**: The reinforcement learning ecosystem (Gymnasium, stable-baselines3, PyTorch, numpy) is Python-native.

3. **Design Iteration**: Game mechanics (governance, economy, combat) need rapid iteration. Compile-edit-run cycles slow exploration.

4. **Determinism**: Reproducible simulation requires careful control over memory, iteration order, and floating-point operations.

Pure Python cannot deliver (1) and (4) reliably. Pure Rust cannot deliver (2) and (3) ergonomically.

## Decision

Use a **hybrid architecture**:

- **Rust core** (`tidebreak-core`, `morphospace`): Performance-critical simulation — Combat Arena, spatial fields, physics, entity systems.
- **Python bindings** (`tidebreak-py`): PyO3 bindings exposing full Rust API to Python.
- **Python layer**: DRL training, strategic layer, visualization, tooling.

### The Two Roles of Python Bindings

Python bindings serve **dual purposes**:

1. **DRL Ecosystem Access**: Gymnasium environment wrapper, numpy observations, PyTorch policy networks.

2. **Rapid Prototyping**: Design iteration happens in Python. Once a system is stable and proven, port to Rust if it's performance-critical.

This enables a "**prototype in Python, harden in Rust**" workflow.

### Boundary Definition

```
Python Side                          Rust Side
─────────────────────────────────────────────────────────────
DRL Training (SB3, PyTorch)    │
Gymnasium Env Wrapper          │──► PyO3 ──► Combat Arena Core
Strategic Layer (Python)       │              MorphoSpace
Reward Shaping                 │              Entity System
Visualization                  │              Physics
Tooling, Analysis              │              Deterministic RNG
```

**Rust owns**:
- Combat Arena step loop
- MorphoSpace (spatial fields, octree, propagation)
- Entity-Plugin-Resolver execution
- Determinism-critical code (RNG streams, iteration order)
- Observation serialization to numpy arrays

**Python owns** (initially):
- Strategic layer (economy, factions, governance)
- DRL training orchestration
- Reward shaping and curriculum
- BattlePackage construction
- BattleResult interpretation
- Visualization and debugging tools

### Data Flow

```
Python                              Rust
───────────────────────────────────────────────────────
BattlePackage (pydantic)  ──►  serde deserialize
                               │
                               ▼
                          Arena.simulate()
                               │
                               ▼
BattleResult (pydantic)   ◄──  serde serialize

Gymnasium step:
action (numpy/dict)       ──►  Arena.step(action)
                               │
                               ▼
observation (numpy)       ◄──  Pre-vectorized buffer
reward (float)            ◄──  Computed in Rust or Python
done (bool)               ◄──  Termination check
```

### Observation Handoff

Observations are pre-vectorized in Rust and exposed as numpy arrays via PyO3:

```rust
// Rust side
#[pyclass]
struct Observation {
    #[pyo3(get)]
    own_state: PyArray1<f32>,
    #[pyo3(get)]
    contacts: PyArray2<f32>,
    #[pyo3(get)]
    environment: PyArray1<f32>,
}
```

No Python-side transformation needed in the hot path. The Rust→numpy boundary is zero-copy where possible.

## Consequences

### Enables

- **Fast training**: Rust simulation at 100x+ real-time; Python only touches observations/actions.
- **DRL ecosystem**: Native Gymnasium interface; works with SB3, CleanRL, RLlib out of the box.
- **Rapid iteration**: Strategic layer and reward shaping in Python; change without recompiling.
- **Gradual hardening**: Prototype in Python, port proven designs to Rust when performance matters.
- **Determinism**: Rust gives explicit control over memory layout, iteration, and floating-point.

### Costs

- **Two-language codebase**: Contributors need familiarity with both Rust and Python.
- **Build complexity**: Maturin/PyO3 adds build steps. CI must build wheels.
- **API boundary design**: Must carefully design what crosses the FFI boundary.
- **Debugging across boundary**: Stack traces and debugging are harder across FFI.

### Mitigations

- Clear ownership: Rust owns simulation, Python owns orchestration. Minimal crossing.
- Maturin is mature: Same tooling as Polars, tokenizers, etc.
- Typed interfaces: PyO3 + pydantic give type safety on both sides.
- Logging/tracing: Rust-side tracing with Python-visible output.

### Follow-ups Required

- Set up Cargo workspace (`tidebreak-core`, `morphospace`, `tidebreak-py`).
- Establish PyO3 binding patterns and conventions.
- Define observation/action schemas that work for both DRL and prototyping.
- Update CLAUDE.md with dual-language tech stack.
- CI pipeline for building and testing both layers.

## Alternatives Considered

### Pure Python (numpy + numba)

Use numpy for vectorized operations, numba for JIT compilation of hot loops.

**Rejected because**:
- Numba has limitations (no classes, limited numpy support).
- Determinism harder to guarantee (GC pauses, dict ordering edge cases).
- Still slower than Rust for complex branching logic.
- PyPy not viable due to numpy/PyTorch incompatibility.

### Pure Rust with Python Only for Training Script

Minimal Python — just the training loop calling a Rust library.

**Rejected because**:
- Loses rapid prototyping benefit.
- Strategic layer would need Rust implementation before design is stable.
- Reward shaping experimentation requires recompilation.

### C++ Core with pybind11

Traditional game engine approach.

**Rejected because**:
- Rust offers better safety guarantees for concurrent code.
- Cargo ecosystem is more ergonomic than CMake.
- PyO3 is more mature than pybind11 for the use cases we need.
- Personal familiarity/preference for Rust.

### Cython

Compile Python to C for hot paths.

**Rejected because**:
- Incremental approach that doesn't scale.
- Still fighting Python's runtime semantics.
- Harder to achieve determinism.
- Rust gives a cleaner separation of concerns.
