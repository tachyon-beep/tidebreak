"""Tidebreak - Naval strategy game with DRL agents."""

from __future__ import annotations

from typing import Any

__version__ = "0.1.0"

# Re-export Rust extension classes
# The Rust extension is built by maturin and installed with Python modules
# We import from the maturin package to allow usage like: from tidebreak import Field


def _load_rust_extension() -> Any:
    """Load Rust extension and return its exports."""
    import importlib.util
    import sys
    from pathlib import Path

    _this_file = Path(__file__).resolve()

    # Build list of candidate locations to search
    search_paths = list(sys.path)

    # Also check the maturin build directory (relative to repo root)
    repo_root = _this_file.parent.parent.parent  # src/tidebreak/__init__.py -> repo root
    maturin_path = repo_root / "crates" / "tidebreak-py" / "python"
    if maturin_path.exists():
        search_paths.insert(0, str(maturin_path))

    # Find the maturin-installed tidebreak package
    for site_path in search_paths:
        ext_path = Path(site_path) / "tidebreak"
        if ext_path.exists() and ext_path.resolve() != _this_file.parent:
            # Found installed tidebreak, look for the _tidebreak.so file
            so_files = list(ext_path.glob("_tidebreak.cpython-*.so"))
            if so_files:
                # Add this path to sys.path so submodule imports work
                ext_path_str = str(ext_path.parent)
                if ext_path_str not in sys.path:
                    sys.path.insert(0, ext_path_str)

                # Import the _tidebreak extension module
                spec = importlib.util.spec_from_file_location("tidebreak._tidebreak", so_files[0])
                if spec and spec.loader:
                    _rust = importlib.util.module_from_spec(spec)
                    sys.modules["tidebreak._tidebreak"] = _rust
                    spec.loader.exec_module(_rust)
                    return _rust
    return None


_rust = _load_rust_extension()

if _rust is not None:
    # Murk bindings (existing)
    Field = _rust.Field
    PyPointResult = _rust.PyPointResult
    PyQueryResult = _rust.PyQueryResult
    PyUniverse = _rust.PyUniverse

    # Tidebreak-core bindings (new)
    PyEntityId = _rust.PyEntityId
    PyEntityTag = _rust.PyEntityTag
    PyTransformState = _rust.PyTransformState
    PyPhysicsState = _rust.PyPhysicsState
    PyCombatState = _rust.PyCombatState
    PyEntity = _rust.PyEntity
    PySimulation = _rust.PySimulation
    PyObservation = _rust.PyObservation

    # Aliases for convenience
    Universe = PyUniverse
    Simulation = PySimulation
    EntityId = PyEntityId
    EntityTag = PyEntityTag
    Entity = PyEntity

    __all__ = [
        # Murk types
        "Field",
        "PyPointResult",
        "PyQueryResult",
        "PyUniverse",
        "Universe",
        # Entity types
        "PyEntityId",
        "PyEntityTag",
        "EntityId",
        "EntityTag",
        # Component types
        "PyTransformState",
        "PyPhysicsState",
        "PyCombatState",
        # Entity wrapper
        "PyEntity",
        "Entity",
        # Simulation
        "PySimulation",
        "Simulation",
        # DRL
        "PyObservation",
        # Envs submodule
        "envs",
    ]
    del _rust

del _load_rust_extension
