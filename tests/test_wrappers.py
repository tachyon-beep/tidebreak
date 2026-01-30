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
NormalizedObsWrapper = wrappers.NormalizedObsWrapper


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


class TestNormalizedObsWrapper:
    """Tests for NormalizedObsWrapper."""

    def test_observation_space_is_flat_box(self) -> None:
        """Wrapped env has flat Box observation space."""
        env = CombatEnv(max_contacts=16)
        wrapped = NormalizedObsWrapper(env)

        # own_state: 7 dims + contacts: 6 * 16 = 96 dims + context: 2 dims = 105
        expected_dim = 7 + 6 * 16 + 2
        assert wrapped.observation_space.shape == (expected_dim,)
        assert wrapped.observation_space.low[0] == -1.0
        assert wrapped.observation_space.high[0] == 1.0

    def test_observation_space_with_different_max_contacts(self) -> None:
        """Observation dimension scales with max_contacts."""
        env = CombatEnv(max_contacts=8)
        wrapped = NormalizedObsWrapper(env)

        # own_state: 7 dims + contacts: 6 * 8 = 48 dims + context: 2 dims = 57
        expected_dim = 7 + 6 * 8 + 2
        assert wrapped.observation_space.shape == (expected_dim,)

    def test_observation_normalization_positions(self) -> None:
        """Position values are normalized by world_size."""
        env = CombatEnv(max_contacts=4)
        wrapped = NormalizedObsWrapper(env, world_size=500.0)

        wrapped.reset(seed=42)
        obs, _, _, _, _ = wrapped.step({"throttle": np.array([0.0]), "turn_rate": np.array([0.0]), "fire": 0})

        # All values should be in [-1, 1] range
        assert obs.shape == (7 + 6 * 4 + 2,)
        assert np.all(obs >= -1.0)
        assert np.all(obs <= 1.0)

    def test_angle_encoding_sin_cos(self) -> None:
        """Angles are encoded as sin/cos pairs."""
        env = CombatEnv(max_contacts=4)
        wrapped = NormalizedObsWrapper(env)

        wrapped.reset(seed=42)
        obs, _, _, _, _ = wrapped.step({"throttle": np.array([0.0]), "turn_rate": np.array([0.0]), "fire": 0})

        # own_state indices 2,3 are sin_h, cos_h
        sin_h = obs[2]
        cos_h = obs[3]
        # sin^2 + cos^2 should equal 1
        np.testing.assert_almost_equal(sin_h**2 + cos_h**2, 1.0, decimal=5)

    def test_hp_ratio_normalized(self) -> None:
        """HP is normalized as hp/max_hp."""
        env = CombatEnv(max_contacts=4)
        wrapped = NormalizedObsWrapper(env)

        wrapped.reset(seed=42)
        obs, _, _, _, _ = wrapped.step({"throttle": np.array([0.0]), "turn_rate": np.array([0.0]), "fire": 0})

        # HP ratio at index 6 should be in [0, 1]
        hp_ratio = obs[6]
        assert 0.0 <= hp_ratio <= 1.0

    def test_context_normalized_by_max_steps(self) -> None:
        """Context values are normalized by max_steps."""
        env = CombatEnv(max_contacts=4, max_steps=100)
        wrapped = NormalizedObsWrapper(env)

        wrapped.reset(seed=42)
        # Take a step
        obs, _, _, _, _ = wrapped.step({"throttle": np.array([0.0]), "turn_rate": np.array([0.0]), "fire": 0})

        # Context is at the end: last 2 values
        step_ratio = obs[-2]  # step_count / max_steps
        remaining_ratio = obs[-1]  # remaining / max_steps

        # After 1 step with max_steps=100: step_ratio = 1/100 = 0.01, remaining = 99/100 = 0.99
        np.testing.assert_almost_equal(step_ratio, 0.01, decimal=3)
        np.testing.assert_almost_equal(remaining_ratio, 0.99, decimal=3)

    def test_step_with_normalized_observation(self) -> None:
        """Wrapped env returns normalized observations on step."""
        env = CombatEnv(max_contacts=4)
        wrapped = NormalizedObsWrapper(env)

        wrapped.reset(seed=42)

        action = {"throttle": np.array([0.5]), "turn_rate": np.array([0.2]), "fire": 0}
        obs, reward, terminated, truncated, _ = wrapped.step(action)

        assert isinstance(obs, np.ndarray)
        assert obs.dtype == np.float32
        assert isinstance(reward, float)
        assert isinstance(terminated, bool)
        assert isinstance(truncated, bool)

    def test_reset_returns_normalized_observation(self) -> None:
        """Wrapped env returns normalized observation on reset."""
        env = CombatEnv(max_contacts=4)
        wrapped = NormalizedObsWrapper(env)

        obs, _info = wrapped.reset(seed=42)

        assert isinstance(obs, np.ndarray)
        assert obs.dtype == np.float32
        expected_dim = 7 + 6 * 4 + 2
        assert obs.shape == (expected_dim,)

    def test_composed_with_flat_action_wrapper(self) -> None:
        """NormalizedObsWrapper composes with FlatActionWrapper for full SB3 compatibility."""
        env = CombatEnv(max_contacts=4)
        wrapped = NormalizedObsWrapper(FlatActionWrapper(env))

        wrapped.reset(seed=42)

        # Use flat action (from FlatActionWrapper)
        flat_action = np.array([0.5, 0.2, -0.5], dtype=np.float32)
        obs, _, _, _, _ = wrapped.step(flat_action)

        # Observation should be flat normalized array
        assert isinstance(obs, np.ndarray)
        expected_dim = 7 + 6 * 4 + 2
        assert obs.shape == (expected_dim,)

    def test_contacts_normalized_with_bearing(self) -> None:
        """Contact bearing is encoded as sin/cos."""
        env = CombatEnv(max_contacts=4)
        wrapped = NormalizedObsWrapper(env)

        wrapped.reset(seed=42)
        obs, _, _, _, _ = wrapped.step({"throttle": np.array([0.0]), "turn_rate": np.array([0.0]), "fire": 0})

        # Contact layout: [x, y, sin_b, cos_b, dist, quality] per contact (6 dims)
        # First contact starts at index 7 (after own_state)
        contact_start = 7
        for i in range(4):
            idx = contact_start + i * 6
            sin_b = obs[idx + 2]
            cos_b = obs[idx + 3]
            # sin^2 + cos^2 should equal 1 for valid angles
            np.testing.assert_almost_equal(sin_b**2 + cos_b**2, 1.0, decimal=5)

    def test_contact_quality_normalized_to_unit_range(self) -> None:
        """Contact quality is normalized to [0, 1] range, not treated as angle."""
        env = CombatEnv(max_contacts=4)
        wrapped = NormalizedObsWrapper(env)

        wrapped.reset(seed=42)
        obs, _, _, _, _ = wrapped.step({"throttle": np.array([0.0]), "turn_rate": np.array([0.0]), "fire": 0})

        # Contact layout: [x, y, sin_b, cos_b, dist, quality] per contact (6 dims)
        # Quality is the last element of each contact block
        contact_start = 7
        for i in range(4):
            idx = contact_start + i * 6
            quality_norm = obs[idx + 5]
            # Quality should be in [0, 1] range (from 0-100 raw)
            assert 0.0 <= quality_norm <= 1.0, f"Quality {quality_norm} not in [0, 1]"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
