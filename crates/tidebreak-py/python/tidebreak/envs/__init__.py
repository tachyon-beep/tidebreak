"""Gymnasium environments for Tidebreak."""

from __future__ import annotations


# Lazy import to avoid issues when importing package __init__
def __getattr__(name: str):
    if name == "MurkEnv":
        from tidebreak.envs.murk_env import MurkEnv

        return MurkEnv
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")


__all__ = ["MurkEnv"]
