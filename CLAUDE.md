# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Tidebreak is a naval strategy game featuring multi-layer combat (surface/submerged/abyssal), fleet hierarchy from jetskis to mobile city-ships, and Deep Reinforcement Learning agents trained at tactical, operational, and strategic scales.

**Current Status**: Clean slate rebuild. Documentation complete in `docs/`, implementation not yet started.

## Technology Stack

Tidebreak uses a **Rust core with Python bindings** (see `docs/decisions/adr-0006-rust-core-python-bindings.md`):

**Rust Layer** (`tidebreak-core`, `morphospace`):
- Combat Arena simulation (entities, plugins, resolvers)
- MorphoSpace spatial substrate (octree, fields, propagation)
- Determinism-critical code (RNG, physics, iteration order)
- Pre-vectorized observations for DRL

**Python Layer** (`tidebreak-py`):
- PyO3 bindings to Rust core
- Gymnasium/PettingZoo environment wrappers
- DRL training orchestration (stable-baselines3, PyTorch)
- Strategic layer (economy, factions, governance)
- Visualization and tooling

**Development Workflow**: Prototype in Python, harden in Rust when stable.

## Documentation

All design and requirements live in `docs/`:

- **`docs/vision/`** — Product vision, design pillars, glossary
- **`docs/design/`** — Technical designs (architecture, combat, sensors, layers, boarding)
- **`docs/technical/`** — Formal specs (entity framework, contracts)
- **`docs/requirements/`** — Prioritized requirements (P0=MVP, P1=core, P2+=later)
- **`docs/decisions/`** — Architecture Decision Records (ADRs)
- **`docs/research/`** — Exploratory designs (e.g., MorphoSpace)

Start with: `docs/vision/pitch.md` → `docs/design/architecture.md` → `docs/requirements/`

## Commands

```bash
# === Rust Development ===

# Build Rust core
cargo build --release

# Run Rust tests
cargo test

# Build Python wheels (includes Rust compilation)
maturin develop --release

# Rust linting
cargo clippy -- -D warnings
cargo fmt --check

# === Python Development ===

# Development Setup (after maturin develop)
uv venv && source .venv/bin/activate
uv pip install -e ".[dev,arena,viz]"

# Lint and format
uv run ruff check . --fix
uv run ruff format .

# Type check
uv run mypy .

# Run Python tests
uv run pytest

# Pre-commit hooks (runs both Rust and Python checks)
uv run pre-commit install
uv run pre-commit run --all-files
```

## Architecture Summary

See `docs/design/entity-framework.md` for full details.

**Entity-Plugin-Resolver Pattern**:
- **Entity**: Container with identity, state components, and plugins
- **Plugin**: Reads WorldView (immutable), emits Outputs (proposals)
- **Resolver**: Collects outputs, resolves conflicts, writes to NextState

**Execution Loop** (per tick):
1. SNAPSHOT: Freeze state into immutable WorldView
2. PLUGIN: Each plugin reads WorldView, emits Outputs (parallelizable)
3. RESOLUTION: Collect outputs, resolve conflicts, write to NextState
4. APPLY: Swap NextState → CurrentState, emit telemetry

**Key Invariants**:
- Plugins MUST NOT mutate state directly
- Determinism: same seed + same platform + same inputs = identical results
- All state must be serializable for replay

## Code Standards

**Rust** (`tidebreak-core`, `morphospace`):
- Rust 2021 edition, stable toolchain
- `clippy` for linting (deny warnings)
- `rustfmt` for formatting
- No unsafe without justification comment
- All public APIs documented

**Python** (`tidebreak-py`, strategic layer):
- Python 3.12+ with strict mypy
- Ruff for linting (line-length: 120)
- Type hints required on all public functions

**Shared Conventions**:
- All physics calculations assume 2D (x, y coordinates)
- Heading in radians, counter-clockwise from +X axis
- Use terminology from `docs/vision/glossary.md`
- Determinism: same seed + same platform = identical results (see ADR-0003)

## Dependencies

**Rust Crates** (Cargo.toml):
- `pyo3`: Python bindings
- `numpy`: PyO3 numpy integration
- `serde`, `serde_json`: Serialization
- `rand`, `rand_chacha`: Deterministic RNG

**Python Packages** (pyproject.toml):
- **Core**: numpy, pydantic, pyyaml, rich
- **Arena/DRL** (`.[arena]`): gymnasium, pettingzoo, stable-baselines3, torch, tensorboard, shapely
- **Visualization** (`.[viz]`): matplotlib, pygame-ce
- **Build**: maturin
