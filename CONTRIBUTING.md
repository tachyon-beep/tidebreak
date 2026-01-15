# Contributing to Tidebreak

Thank you for your interest in contributing to Tidebreak!

## Getting Started

1. Fork the repository
2. Clone your fork:

   ```bash
   git clone https://github.com/YOUR_USERNAME/tidebreak.git
   cd tidebreak
   ```

3. Set up the development environment:

   ```bash
   uv venv && source .venv/bin/activate
   uv pip install -e ".[dev]"
   uv run pre-commit install
   ```

## Development Workflow

### Before You Code

1. Check [existing issues](https://github.com/tidebreak/tidebreak/issues) to avoid duplicate work
2. For significant changes, open an issue first to discuss the approach
3. Read the relevant documentation in `docs/`

### Making Changes

1. Create a feature branch:

   ```bash
   git checkout -b feature/your-feature-name
   ```

2. Write your code following project conventions (see `CLAUDE.md`)

3. Run quality checks:

   ```bash
   uv run ruff check . --fix   # Lint
   uv run ruff format .        # Format
   uv run mypy .               # Type check
   uv run pytest               # Test
   ```

4. Commit with a clear message:

   ```bash
   git commit -m "Add feature X that does Y"
   ```

### Submitting a Pull Request

1. Push your branch:

   ```bash
   git push origin feature/your-feature-name
   ```

2. Open a pull request against `main`

3. Fill out the PR template

4. Wait for CI checks to pass and address any feedback

## Code Standards

- **Python 3.12+** with strict mypy type checking
- **Ruff** for linting and formatting (line length: 120)
- **Determinism**: Same inputs must produce identical outputs
- **No direct state mutation** in plugins (use the Resolver pattern)

See `CLAUDE.md` for complete coding guidelines.

## Project Structure

```text
tidebreak/
├── src/tidebreak/     # Main package
├── tests/             # Test suite
├── docs/              # Documentation
│   ├── vision/        # Product vision
│   ├── design/        # Technical designs
│   ├── technical/     # Formal specs
│   └── requirements/  # Prioritized requirements
└── pyproject.toml     # Project configuration
```

## Types of Contributions

### Bug Fixes

- Include a test that reproduces the bug
- Reference the issue number in your PR

### New Features

- Discuss in an issue first
- Add tests for new functionality
- Update documentation if needed

### Documentation

- Keep terminology consistent with `docs/vision/glossary.md`
- Documentation PRs are always welcome

### DRL / Training

- Training code should be reproducible (set seeds)
- Include hyperparameter configs
- Document any dependencies on arena extras

## Questions?

Open an issue with the question label or check existing documentation in `docs/`.
