# Development Setup

How to set up a development environment for Tidebreak.

## Prerequisites

- Python 3.12+
- [uv](https://github.com/astral-sh/uv) package manager

## Quick Start

```bash
# Create virtual environment
uv venv

# Activate
source .venv/bin/activate  # Linux/Mac
# or: .venv\Scripts\activate  # Windows

# Install with all development dependencies
uv pip install -e ".[dev,arena,viz]"

# Set up pre-commit hooks
uv run pre-commit install
```

## Dependency Groups

| Group | Contents | When to Use |
|-------|----------|-------------|
| (default) | numpy, pydantic, pyyaml, rich | Always |
| `arena` | gymnasium, stable-baselines3, torch | Combat arena, DRL |
| `viz` | matplotlib, pygame-ce | Visualization |
| `dev` | mypy, pytest, ruff, pre-commit | Development |

Install specific groups:
```bash
uv pip install -e ".[arena]"      # Just arena
uv pip install -e ".[dev,arena]"  # Dev + arena
```

## Common Commands

### Testing

```bash
# Run all tests
uv run pytest

# Run with verbose output
uv run pytest -v

# Run single test file
uv run pytest tests/test_arena_determinism.py

# Run with coverage
uv run pytest --cov
```

### Linting and Formatting

```bash
# Check for issues
uv run ruff check .

# Auto-fix issues
uv run ruff check . --fix

# Format code
uv run ruff format .

# Type check
uv run mypy .
```

### Pre-commit

```bash
# Install hooks (run once)
uv run pre-commit install

# Run all hooks manually
uv run pre-commit run --all-files
```

### Running the Demo (Planned)

> **Note**: Arena code not yet implemented. These commands will work once `src/tidebreak/arena/` exists.

```bash
# Run demo battle (prints JSON result)
uv run python -m tidebreak.arena.demo
```

### DRL Training (Planned)

> **Note**: Training scripts not yet implemented. See `docs/requirements/drl.md` for requirements.

```bash
# Train PPO agent
uv run python scripts/train_arena_ppo.py --timesteps 200000

# With custom seed
uv run python scripts/train_arena_ppo.py --timesteps 200000 --seed 42

# Save model
uv run python scripts/train_arena_ppo.py --timesteps 200000 --model-out artifacts/ppo_model

# View training logs
tensorboard --logdir runs/ppo_arena
```

## Project Structure

**Current state** (documentation complete, implementation not started):

```
tidebreak/
├── src/tidebreak/           # Source code (skeleton only)
│   └── __init__.py          # Version string
├── tests/                   # Test suite
│   └── test_setup.py        # Smoke test
├── docs/                    # Documentation (complete)
│   ├── vision/              # Product vision
│   ├── design/              # Technical design
│   ├── requirements/        # Functional requirements
│   ├── technical/           # Formal specs and contracts
│   └── development/         # Dev guides (you are here)
├── pyproject.toml           # Package configuration
└── CLAUDE.md                # AI assistant guidance
```

**Planned structure** (after MVP implementation):

```
tidebreak/
├── src/tidebreak/
│   ├── arena/               # Combat arena (MVP focus)
│   │   ├── sim.py           # Physics engine
│   │   ├── schema.py        # Data contracts
│   │   ├── gym_env.py       # DRL environment
│   │   ├── controllers.py   # Scripted AI
│   │   └── demo.py          # Demo battle
│   ├── entity/              # Entity framework
│   ├── plugins/             # Plugin implementations
│   ├── resolvers/           # Resolver implementations
│   └── __init__.py
├── tests/
├── scripts/                 # Training and utility scripts
└── ...
```

## Code Standards

- **Python 3.12** — Use modern syntax (type hints, walrus operator, etc.)
- **mypy strict** — All code must pass strict type checking
- **ruff** — Line length 120, selected rule sets
- **pytest** — All new features need tests

## Troubleshooting

### Import errors after install

Try reinstalling in editable mode:
```bash
uv pip install -e ".[dev,arena,viz]" --force-reinstall
```

### Type errors from third-party packages

Check `pyproject.toml` for `ignore_missing_imports` settings. Third-party stubs may need to be added.

### Pre-commit fails on first run

Run the full hook suite once:
```bash
uv run pre-commit run --all-files
```

Then commit the auto-fixed changes.
