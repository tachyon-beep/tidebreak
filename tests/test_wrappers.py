"""Tests for action space wrappers."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import numpy as np
import pytest

from tidebreak.envs import CombatEnv


def _import_wrappers_module():
    """Import wrappers module from maturin location.

    This helper exists because the wrappers module is in the maturin-built
    package location, and the __init__.py exports haven't been updated yet.
    Once Task 3 is complete, this can be replaced with:
        from tidebreak.envs.wrappers import FlatActionWrapper
    """
    wrappers_file = (
        Path(__file__).parent.parent / "crates" / "tidebreak-py" / "python" / "tidebreak" / "envs" / "wrappers.py"
    )
    if "tidebreak.envs.wrappers" not in sys.modules:
        spec = importlib.util.spec_from_file_location("tidebreak.envs.wrappers", wrappers_file)
        if spec and spec.loader:
            module = importlib.util.module_from_spec(spec)
            sys.modules["tidebreak.envs.wrappers"] = module
            spec.loader.exec_module(module)
    return sys.modules["tidebreak.envs.wrappers"]


wrappers = _import_wrappers_module()
FlatActionWrapper = wrappers.FlatActionWrapper


class TestFlatActionWrapper:
    """Tests for FlatActionWrapper."""

    def test_action_space_is_box(self) -> None:
        """Wrapped env has Box action space."""
        env = CombatEnv()
        wrapped = FlatActionWrapper(env)

        assert wrapped.action_space.shape == (3,)
        assert wrapped.action_space.low[0] == -1.0
        assert wrapped.action_space.high[0] == 1.0

    def test_action_conversion_basic(self) -> None:
        """Flat action is converted to dict action."""
        env = CombatEnv()
        wrapped = FlatActionWrapper(env)

        flat_action = np.array([0.5, -0.3, 0.1], dtype=np.float32)
        dict_action = wrapped.action(flat_action)

        assert "throttle" in dict_action
        assert "turn_rate" in dict_action
        assert "fire" in dict_action

        np.testing.assert_almost_equal(dict_action["throttle"][0], 0.5)
        np.testing.assert_almost_equal(dict_action["turn_rate"][0], -0.3)
        assert dict_action["fire"] == 1  # 0.1 >= 0.0

    def test_fire_logit_positive(self) -> None:
        """Positive fire logit maps to fire=1."""
        env = CombatEnv()
        wrapped = FlatActionWrapper(env)

        flat_action = np.array([0.0, 0.0, 0.5], dtype=np.float32)
        dict_action = wrapped.action(flat_action)

        assert dict_action["fire"] == 1

    def test_fire_logit_zero(self) -> None:
        """Zero fire logit maps to fire=1 (boundary case)."""
        env = CombatEnv()
        wrapped = FlatActionWrapper(env)

        flat_action = np.array([0.0, 0.0, 0.0], dtype=np.float32)
        dict_action = wrapped.action(flat_action)

        assert dict_action["fire"] == 1

    def test_fire_logit_negative(self) -> None:
        """Negative fire logit maps to fire=0."""
        env = CombatEnv()
        wrapped = FlatActionWrapper(env)

        flat_action = np.array([0.0, 0.0, -0.1], dtype=np.float32)
        dict_action = wrapped.action(flat_action)

        assert dict_action["fire"] == 0

    def test_step_with_flat_action(self) -> None:
        """Wrapped env can step with flat action."""
        env = CombatEnv()
        wrapped = FlatActionWrapper(env)

        wrapped.reset(seed=42)

        flat_action = np.array([0.5, 0.2, -0.5], dtype=np.float32)
        obs, reward, terminated, truncated, _ = wrapped.step(flat_action)

        assert "own_state" in obs
        assert isinstance(reward, float)
        assert isinstance(terminated, bool)
        assert isinstance(truncated, bool)

    def test_action_space_sample(self) -> None:
        """Action space can be sampled and used."""
        env = CombatEnv()
        wrapped = FlatActionWrapper(env)

        wrapped.reset(seed=42)

        for _ in range(5):
            action = wrapped.action_space.sample()
            assert action.shape == (3,)
            obs, _, _, _, _ = wrapped.step(action)
            assert "own_state" in obs

    def test_observation_space_unchanged(self) -> None:
        """Observation space is unchanged by wrapper."""
        env = CombatEnv()
        wrapped = FlatActionWrapper(env)

        assert wrapped.observation_space == env.observation_space


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
