"""
Smoke test: train a random policy for a few episodes.

This validates:
- Gymnasium contract compliance
- Rust->Python boundary stability
- Memory safety (no leaks, no segfaults)
- Determinism (optional)
"""

import numpy as np


def test_random_policy_10_episodes():
    """Train random policy for 10 episodes without crashing."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv(max_steps=100)

    total_reward = 0.0
    for episode in range(10):
        obs, info = env.reset(seed=episode)
        done = False
        episode_reward = 0.0

        while not done:
            action = env.action_space.sample()
            obs, reward, terminated, truncated, info = env.step(action)
            episode_reward += reward
            done = terminated or truncated

        total_reward += episode_reward

    env.close()

    # Should complete without crashes
    # Reward is -0.01 per step, so total will be negative
    # With max_steps=100 and 10 episodes, expect around -10.0 total
    assert total_reward <= 0, "Reward should be negative (step penalty)"
    assert total_reward > -20.0, "Reward should not be excessively negative"


def test_deterministic_episodes():
    """Same seed produces identical episode."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv(max_steps=50)

    # Episode 1
    obs1, _ = env.reset(seed=42)
    rewards1 = []
    for _ in range(10):
        action = {"move": np.array([0.5, 0.5], dtype=np.float32), "stamp": 1}
        obs, reward, _, _, _ = env.step(action)
        rewards1.append(reward)
    final_obs1 = obs.copy()

    # Episode 2 (same seed, same actions)
    obs2, _ = env.reset(seed=42)
    rewards2 = []
    for _ in range(10):
        action = {"move": np.array([0.5, 0.5], dtype=np.float32), "stamp": 1}
        obs, reward, _, _, _ = env.step(action)
        rewards2.append(reward)
    final_obs2 = obs.copy()

    env.close()

    np.testing.assert_array_equal(obs1, obs2, "Initial observations should match")
    np.testing.assert_array_almost_equal(rewards1, rewards2, decimal=5, err_msg="Rewards should match")
    np.testing.assert_array_almost_equal(final_obs1, final_obs2, decimal=5, err_msg="Final observations should match")


def test_longer_episode():
    """Run a longer episode to stress test memory."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv(max_steps=500)
    obs, _ = env.reset(seed=0)

    for step in range(500):
        action = env.action_space.sample()
        obs, reward, terminated, truncated, info = env.step(action)

        # Verify observation is still valid numpy array
        assert isinstance(obs, np.ndarray)
        assert obs.dtype == np.float32
        assert not np.any(np.isnan(obs)), f"NaN found in observation at step {step}"

        if terminated or truncated:
            break

    env.close()

    # Should complete without memory issues
    assert True


def test_multiple_resets_memory_stability():
    """Repeatedly reset environment to check for memory leaks."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv(max_steps=10)

    for i in range(50):
        obs, info = env.reset(seed=i)
        assert isinstance(obs, np.ndarray), f"Reset {i} failed to return numpy array"
        assert obs.shape == env.observation_space.shape

        # Take a few steps
        for _ in range(5):
            action = env.action_space.sample()
            obs, _, terminated, truncated, _ = env.step(action)
            if terminated or truncated:
                break

    env.close()


def test_rapid_action_sequence():
    """Execute many actions rapidly to stress test Rust<->Python boundary."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv(max_steps=1000)
    env.reset(seed=12345)

    # Execute 1000 rapid actions
    for step in range(1000):
        # Alternate between different action types
        stamp_type = step % 3  # 0=none, 1=fire, 2=sonar
        move_x = np.sin(step * 0.1)
        move_y = np.cos(step * 0.1)

        action = {
            "move": np.array([move_x, move_y], dtype=np.float32),
            "stamp": stamp_type,
        }
        obs, reward, terminated, truncated, info = env.step(action)

        # Verify state consistency
        assert "step" in info
        assert info["step"] == step + 1

        if terminated or truncated:
            break

    env.close()


def test_observation_bounds():
    """Verify observations remain within reasonable bounds over time."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv(max_steps=200)
    env.reset(seed=42)

    all_obs = []
    for _ in range(200):
        action = env.action_space.sample()
        obs, _, terminated, truncated, _ = env.step(action)
        all_obs.append(obs)
        if terminated or truncated:
            break

    env.close()

    # Stack all observations and check for reasonable values
    all_obs_array = np.stack(all_obs)

    # No NaN or Inf values
    assert not np.any(np.isnan(all_obs_array)), "NaN values found in observations"
    assert not np.any(np.isinf(all_obs_array)), "Inf values found in observations"

    # Values should be finite and reasonable (not exploding)
    max_abs_value = np.max(np.abs(all_obs_array))
    assert max_abs_value < 1e6, f"Observation values exploding: max={max_abs_value}"
