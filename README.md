# Tidebreak

A naval strategy game featuring multi-layer combat, mobile city-ships, and Deep Reinforcement Learning agents.

## What is Tidebreak?

Command fleets across three depth layers (surface, submerged, abyssal) in a world where the ocean is everything. From jetskis to arcology-ships housing 10,000+ people, every vessel has a role. Train AI agents with DRL to master tactical, operational, and strategic decision-making.

**Core pillars**:

- **Depth creates tactical space** — Layers aren't just visuals; they're rule sets with different sensors, weapons, and vulnerabilities
- **Scale creates consequence** — Losing a corvette is a setback; losing an arcology-ship is a civilization event
- **Information is uncertain** — Fog of war isn't a veil to lift; it's the permanent state of combat

## Documentation

All design and requirements are in [`docs/`](docs/):

| Folder | Contents |
| -------- | ---------- |
| [`docs/vision/`](docs/vision/) | Product vision, design pillars, glossary |
| [`docs/design/`](docs/design/) | Technical designs (architecture, combat, sensors, layers) |
| [`docs/technical/`](docs/technical/) | Formal specs (entity framework, data contracts) |
| [`docs/requirements/`](docs/requirements/) | Prioritized requirements (P0–P3) |

**Start here**: [docs/vision/pitch.md](docs/vision/pitch.md)

## Development

```bash
# Setup
uv venv && source .venv/bin/activate
uv pip install -e ".[dev,arena,viz]"

# Install pre-commit hooks
uv run pre-commit install

# Quality checks
uv run ruff check . --fix
uv run ruff format .
uv run mypy .
uv run pytest

# Run all hooks manually
uv run pre-commit run --all-files
```

See [docs/development/setup.md](docs/development/setup.md) for detailed setup instructions.

## Project Status

**Current**: Clean slate rebuild. Documentation complete, implementation starting fresh.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.

## License

[MIT](LICENSE)
