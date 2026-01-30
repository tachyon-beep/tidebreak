"""Gymnasium environments for Tidebreak."""

from __future__ import annotations

__all__ = ["CombatEnv", "FlatActionWrapper", "MurkEnv", "NormalizedObsWrapper", "make_sb3_env"]


# Lazy import to avoid issues when importing package __init__
def __getattr__(name: str):
    if name == "CombatEnv":
        from tidebreak.envs.combat_env import CombatEnv

        return CombatEnv
    if name == "MurkEnv":
        from tidebreak.envs.murk_env import MurkEnv

        return MurkEnv
    if name == "FlatActionWrapper":
        from tidebreak.envs.wrappers import FlatActionWrapper

        return FlatActionWrapper
    if name == "NormalizedObsWrapper":
        from tidebreak.envs.wrappers import NormalizedObsWrapper

        return NormalizedObsWrapper
    if name == "make_sb3_env":
        from tidebreak.envs.wrappers import make_sb3_env

        return make_sb3_env
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
