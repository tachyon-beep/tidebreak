"""Gymnasium environments for Tidebreak.

This module provides Gymnasium-compatible environments for training DRL agents
on the Tidebreak combat simulation.
"""

from __future__ import annotations

__all__ = ["MurkEnv"]


def __getattr__(name: str) -> type:
    """Lazy load MurkEnv to avoid circular imports."""
    if name == "MurkEnv":
        import importlib.util
        import sys
        from pathlib import Path

        _this_file = Path(__file__).resolve()

        # Find the maturin-installed tidebreak package
        for site_path in sys.path:
            ext_path = Path(site_path) / "tidebreak" / "envs"
            if ext_path.exists() and ext_path.resolve() != _this_file.parent:
                murk_env_file = ext_path / "murk_env.py"
                if murk_env_file.exists():
                    # Import the murk_env module
                    spec = importlib.util.spec_from_file_location("tidebreak.envs.murk_env", murk_env_file)
                    if spec and spec.loader:
                        module = importlib.util.module_from_spec(spec)
                        sys.modules["tidebreak.envs.murk_env"] = module
                        spec.loader.exec_module(module)
                        murk_env_cls: type = module.MurkEnv
                        return murk_env_cls
        raise ImportError("MurkEnv not found. Make sure tidebreak is built with: maturin develop")
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
