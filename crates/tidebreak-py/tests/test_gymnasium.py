"""Tests for the MurkEnv Gymnasium environment."""

import numpy as np
import pytest


def test_env_creation():
    """MurkEnv should be creatable with default parameters."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv()
    assert env is not None
    assert env.world_size == (200.0, 200.0, 50.0)
    assert env.max_steps == 1000
    env.close()


def test_env_creation_custom_params():
    """MurkEnv should accept custom parameters."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv(
        world_size=(100.0, 100.0, 25.0),
        max_steps=500,
        render_mode="rgb_array",
        agent_speed=3.0,
    )
    assert env.world_size == (100.0, 100.0, 25.0)
    assert env.max_steps == 500
    assert env.render_mode == "rgb_array"
    assert env.agent_speed == 3.0
    env.close()


def test_observation_space():
    """Observation space should be correctly configured."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv()
    # Default shells: 8 + 4 = 12 sectors, 4 fields
    # Expected size: 12 * 4 = 48
    assert env.observation_space.shape == (48,)
    assert env.observation_space.dtype == np.float32
    env.close()


def test_action_space():
    """Action space should be a Dict with move and stamp."""
    from gymnasium import spaces

    from tidebreak.envs import MurkEnv

    env = MurkEnv()
    assert isinstance(env.action_space, spaces.Dict)
    assert "move" in env.action_space.spaces
    assert "stamp" in env.action_space.spaces
    assert env.action_space["move"].shape == (2,)
    assert isinstance(env.action_space["stamp"], spaces.Discrete)
    assert env.action_space["stamp"].n == 3
    env.close()


def test_reset_returns_obs_and_info():
    """reset() should return (observation, info) tuple."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv()
    obs, info = env.reset()

    assert isinstance(obs, np.ndarray)
    assert obs.shape == env.observation_space.shape
    assert obs.dtype == np.float32
    assert isinstance(info, dict)
    assert "step" in info
    assert "agent_position" in info
    env.close()


def test_reset_with_seed():
    """reset() with seed should produce deterministic results."""
    from tidebreak.envs import MurkEnv

    env1 = MurkEnv()
    env2 = MurkEnv()

    obs1, _ = env1.reset(seed=42)
    obs2, _ = env2.reset(seed=42)

    # Same seed should produce same initial observation
    np.testing.assert_array_equal(obs1, obs2)

    env1.close()
    env2.close()


def test_step_returns_correct_tuple():
    """step() should return (obs, reward, terminated, truncated, info)."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv()
    env.reset(seed=42)

    action = {"move": np.array([1.0, 0.0], dtype=np.float32), "stamp": 0}
    obs, reward, terminated, truncated, info = env.step(action)

    assert isinstance(obs, np.ndarray)
    assert obs.shape == env.observation_space.shape
    assert isinstance(reward, int | float)
    assert isinstance(terminated, bool)
    assert isinstance(truncated, bool)
    assert isinstance(info, dict)
    env.close()


def test_step_truncates_at_max_steps():
    """Environment should truncate after max_steps."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv(max_steps=5)
    env.reset()

    action = {"move": np.array([0.0, 0.0], dtype=np.float32), "stamp": 0}

    for i in range(4):
        _, _, _, truncated, _ = env.step(action)
        assert not truncated, f"Truncated too early at step {i + 1}"

    _, _, _, truncated, _ = env.step(action)
    assert truncated, "Should be truncated at max_steps"
    env.close()


def test_step_without_reset_raises():
    """step() without reset() should raise RuntimeError."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv()

    action = {"move": np.array([0.0, 0.0], dtype=np.float32), "stamp": 0}
    with pytest.raises(RuntimeError, match="not initialized"):
        env.step(action)
    env.close()


def test_movement_action():
    """Movement action should update agent position."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv()
    _, info_before = env.reset(seed=42)
    pos_before = info_before["agent_position"].copy()

    # Move in +x direction
    action = {"move": np.array([1.0, 0.0], dtype=np.float32), "stamp": 0}
    _, _, _, _, info_after = env.step(action)
    pos_after = info_after["agent_position"]

    # Position should have changed in x direction
    assert pos_after[0] > pos_before[0]
    assert pos_after[1] == pytest.approx(pos_before[1], abs=1e-5)
    env.close()


def test_stamp_fire_action():
    """Fire stamp action should not crash."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv()
    env.reset(seed=42)

    # Stamp fire
    action = {"move": np.array([0.0, 0.0], dtype=np.float32), "stamp": 1}
    obs, _, _, _, _ = env.step(action)

    assert isinstance(obs, np.ndarray)
    env.close()


def test_stamp_sonar_action():
    """Sonar stamp action should not crash."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv()
    env.reset(seed=42)

    # Stamp sonar
    action = {"move": np.array([0.0, 0.0], dtype=np.float32), "stamp": 2}
    obs, _, _, _, _ = env.step(action)

    assert isinstance(obs, np.ndarray)
    env.close()


def test_render_rgb_array():
    """render() with rgb_array mode should return an image."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv(world_size=(100.0, 100.0, 50.0), render_mode="rgb_array")
    env.reset()

    img = env.render()

    assert isinstance(img, np.ndarray)
    assert img.shape == (100, 100, 3)
    assert img.dtype == np.uint8
    env.close()


def test_render_none_mode():
    """render() without rgb_array mode should return None."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv(render_mode=None)
    env.reset()

    result = env.render()
    assert result is None
    env.close()


def test_close():
    """close() should clean up resources."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv()
    env.reset()
    env.close()

    # Universe should be None after close
    assert env._universe is None


def test_info_contains_time():
    """Info dict should contain simulation time."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv(dt=0.1)
    env.reset()

    action = {"move": np.array([0.0, 0.0], dtype=np.float32), "stamp": 0}
    _, _, _, _, info = env.step(action)

    assert "time" in info
    assert info["time"] > 0
    env.close()


def test_position_clamped_to_world_bounds():
    """Agent position should be clamped to world bounds."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv(world_size=(100.0, 100.0, 50.0), agent_speed=1000.0)
    env.reset()

    # Try to move far past bounds
    action = {"move": np.array([1.0, 1.0], dtype=np.float32), "stamp": 0}
    for _ in range(100):
        _, _, _, _, info = env.step(action)

    pos = info["agent_position"]
    assert 0 <= pos[0] <= 100.0
    assert 0 <= pos[1] <= 100.0
    env.close()


def test_gymnasium_env_checker():
    """Environment should pass gymnasium's env_checker."""
    from gymnasium.utils.env_checker import check_env

    from tidebreak.envs import MurkEnv

    env = MurkEnv()
    # check_env will raise if there are issues
    check_env(env, warn=True, skip_render_check=True)
    env.close()


def test_action_space_sample():
    """Sampled actions should be valid."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv()
    env.reset(seed=42)

    for _ in range(10):
        action = env.action_space.sample()
        obs, _, _, _, _ = env.step(action)
        assert obs.shape == env.observation_space.shape
    env.close()


def test_multiple_episodes():
    """Environment should handle multiple reset/episode cycles."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv(max_steps=10)

    for episode in range(3):
        obs, info = env.reset(seed=episode)
        assert obs.shape == env.observation_space.shape

        step_count = 0
        for _ in range(15):
            action = env.action_space.sample()
            obs, _, terminated, truncated, _ = env.step(action)
            step_count += 1
            if terminated or truncated:
                break

        # Should have truncated after 10 steps
        assert step_count >= 10

    env.close()
