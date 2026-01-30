"""Gymnasium environments for Tidebreak.

This module provides Gymnasium-compatible environments for training DRL agents
on the Tidebreak combat simulation.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path
from typing import Any

__all__ = ["CombatEnv", "FlatActionWrapper", "MurkEnv", "NormalizedObsWrapper", "make_sb3_env"]


def _find_maturin_envs_path() -> Path | None:
    """Find the maturin-installed tidebreak envs package path."""
    _this_file = Path(__file__).resolve()
    for site_path in sys.path:
        ext_path = Path(site_path) / "tidebreak" / "envs"
        if ext_path.exists() and ext_path.resolve() != _this_file.parent:
            return ext_path
    return None


def _load_module_from_file(module_name: str, file_path: Path) -> Any:
    """Load a module from a file path."""
    spec = importlib.util.spec_from_file_location(module_name, file_path)
    if spec and spec.loader:
        module = importlib.util.module_from_spec(spec)
        sys.modules[module_name] = module
        spec.loader.exec_module(module)
        return module
    raise ImportError(f"Could not load module from {file_path}")


def __getattr__(name: str) -> Any:
    """Lazy load environments to avoid circular imports."""
    ext_path = _find_maturin_envs_path()
    if ext_path is None:
        raise ImportError(f"{name} not found. Make sure tidebreak is built with: maturin develop")

    if name == "CombatEnv":
        combat_env_file = ext_path / "combat_env.py"
        if combat_env_file.exists():
            module = _load_module_from_file("tidebreak.envs.combat_env", combat_env_file)
            return module.CombatEnv
        raise ImportError("CombatEnv not found. Make sure tidebreak is built with: maturin develop")

    if name == "MurkEnv":
        murk_env_file = ext_path / "murk_env.py"
        if murk_env_file.exists():
            module = _load_module_from_file("tidebreak.envs.murk_env", murk_env_file)
            return module.MurkEnv
        raise ImportError("MurkEnv not found. Make sure tidebreak is built with: maturin develop")

    if name == "FlatActionWrapper":
        wrappers_file = ext_path / "wrappers.py"
        if wrappers_file.exists():
            module = _load_module_from_file("tidebreak.envs.wrappers", wrappers_file)
            return module.FlatActionWrapper
        raise ImportError("FlatActionWrapper not found. Make sure tidebreak is built with: maturin develop")

    if name == "NormalizedObsWrapper":
        wrappers_file = ext_path / "wrappers.py"
        if wrappers_file.exists():
            if "tidebreak.envs.wrappers" not in sys.modules:
                _load_module_from_file("tidebreak.envs.wrappers", wrappers_file)
            return sys.modules["tidebreak.envs.wrappers"].NormalizedObsWrapper
        raise ImportError("NormalizedObsWrapper not found. Make sure tidebreak is built with: maturin develop")

    if name == "make_sb3_env":
        wrappers_file = ext_path / "wrappers.py"
        if wrappers_file.exists():
            if "tidebreak.envs.wrappers" not in sys.modules:
                _load_module_from_file("tidebreak.envs.wrappers", wrappers_file)
            return sys.modules["tidebreak.envs.wrappers"].make_sb3_env
        raise ImportError("make_sb3_env not found. Make sure tidebreak is built with: maturin develop")

    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
