#!/usr/bin/env python3
"""Train a simple heat-seeking agent using PPO.

The agent's goal is to find and stay near a heat source.
Reward = mean temperature in forward sectors - step penalty.

This is a minimal proof-of-concept to verify the training loop works.
"""

from __future__ import annotations

import numpy as np

# Check if stable-baselines3 is available
try:
    from stable_baselines3 import PPO
    from stable_baselines3.common.vec_env import DummyVecEnv

    HAS_SB3 = True
except ImportError:
    HAS_SB3 = False
    print("stable-baselines3 not installed. Install with: uv pip install stable-baselines3")

import gymnasium as gym
from gymnasium import spaces

from tidebreak import PyUniverse


class HeatSeekerEnv(gym.Env):
    """Environment where agent seeks a randomly placed heat source.

    Observation: 48 floats from foveated sensing (12 sectors x 4 fields)
    Action: 2D continuous movement direction
    Reward: Temperature sensed in forward direction + small step penalty
    """

    def __init__(self, world_size: float = 100.0, max_steps: int = 200):
        super().__init__()

        self.world_size = world_size
        self.max_steps = max_steps
        self.dt = 0.1
        self.agent_speed = 3.0

        # Observation: foveated sensing (12 sectors x 4 fields = 48)
        self.shells = [
            {"radius_inner": 0.0, "radius_outer": 15.0, "sectors": 8},
            {"radius_inner": 15.0, "radius_outer": 40.0, "sectors": 4},
        ]
        obs_size = (8 + 4) * 4  # 48

        self.observation_space = spaces.Box(low=-np.inf, high=np.inf, shape=(obs_size,), dtype=np.float32)

        # Action: 2D movement direction (continuous)
        self.action_space = spaces.Box(low=-1.0, high=1.0, shape=(2,), dtype=np.float32)

        self._universe: PyUniverse | None = None
        self._agent_pos = np.zeros(3, dtype=np.float32)
        self._agent_heading = np.array([1.0, 0.0, 0.0], dtype=np.float32)
        self._heat_pos = np.zeros(3, dtype=np.float32)
        self._step_count = 0

    def reset(self, *, seed=None, options=None):
        super().reset(seed=seed)

        # Create universe with coarse resolution for speed
        self._universe = PyUniverse(
            width=self.world_size,
            height=self.world_size,
            depth=25.0,
            base_resolution=4.0,  # Coarse for fast training
        )

        # Random agent starting position (center-ish)
        self._agent_pos = np.array(
            [
                self.np_random.uniform(30, 70),
                self.np_random.uniform(30, 70),
                12.5,
            ],
            dtype=np.float32,
        )

        self._agent_heading = np.array([1.0, 0.0, 0.0], dtype=np.float32)

        # Place heat source at random location (not too close to agent)
        while True:
            self._heat_pos = np.array(
                [
                    self.np_random.uniform(10, 90),
                    self.np_random.uniform(10, 90),
                    12.5,
                ],
                dtype=np.float32,
            )
            dist = np.linalg.norm(self._heat_pos[:2] - self._agent_pos[:2])
            if dist > 30:  # At least 30 units away
                break

        # Stamp the heat source
        self._universe.stamp_fire(
            center=tuple(self._heat_pos),
            radius=15.0,
            intensity=1.0,
        )

        self._step_count = 0

        obs = self._get_obs()
        info = {"distance_to_heat": self._distance_to_heat()}

        return obs, info

    def step(self, action):
        if self._universe is None:
            raise RuntimeError("Call reset() first")

        # Apply movement
        move = np.array(action, dtype=np.float32)
        norm = np.linalg.norm(move)
        if norm > 0:
            direction = move / norm
            self._agent_heading = np.array([direction[0], direction[1], 0.0], dtype=np.float32)
            displacement = direction * self.agent_speed * min(norm, 1.0)
            self._agent_pos[0] += displacement[0]
            self._agent_pos[1] += displacement[1]

            # Clamp to world
            self._agent_pos[0] = np.clip(self._agent_pos[0], 5, self.world_size - 5)
            self._agent_pos[1] = np.clip(self._agent_pos[1], 5, self.world_size - 5)

        # Step physics (propagates heat)
        self._universe.step(self.dt)

        self._step_count += 1

        # Compute reward based on temperature sensing
        obs = self._get_obs()

        # Extract temperature from observation (first field in each sector)
        # Observation layout: [temp0, noise0, occ0, sonar0, temp1, noise1, ...]
        temps = obs[0::4]  # Every 4th value starting from 0
        forward_temp = temps[0]  # First sector is forward

        # Reward: temperature sensed - step penalty
        # Normalize temperature: ambient is ~293K, fire can reach 800K+
        temp_reward = (forward_temp - 293) / 500  # Normalize to roughly [-1, 1]
        step_penalty = -0.01

        # Bonus for being very close to heat source
        dist = self._distance_to_heat()
        proximity_bonus = 0.1 if dist < 20 else 0.0

        reward = temp_reward + step_penalty + proximity_bonus

        terminated = dist < 10  # Success: reached heat source
        truncated = self._step_count >= self.max_steps

        info = {
            "distance_to_heat": dist,
            "forward_temp": forward_temp,
            "success": terminated,
        }

        return obs, reward, terminated, truncated, info

    def _get_obs(self):
        return self._universe.observe_foveated(
            position=tuple(self._agent_pos),
            heading=tuple(self._agent_heading),
            shells=self.shells,
        )

    def _distance_to_heat(self):
        return float(np.linalg.norm(self._heat_pos[:2] - self._agent_pos[:2]))


def main():
    print("=" * 60)
    print("Heat-Seeking Agent Training Test")
    print("=" * 60)

    # Test environment first
    print("\n1. Testing environment...")
    env = HeatSeekerEnv()
    obs, info = env.reset(seed=42)
    print(f"   Observation shape: {obs.shape}")
    print(f"   Initial distance to heat: {info['distance_to_heat']:.1f}")

    # Random rollout
    total_reward = 0
    for _ in range(50):
        action = env.action_space.sample()
        obs, reward, terminated, truncated, info = env.step(action)
        total_reward += reward
        if terminated or truncated:
            break
    print(f"   Random policy 50-step reward: {total_reward:.2f}")
    print("   Environment OK!")

    if not HAS_SB3:
        print("\n2. Skipping training (stable-baselines3 not installed)")
        print("   Install with: uv pip install stable-baselines3")
        return

    # Train with PPO
    print("\n2. Training PPO agent...")
    print("   This is a quick test - not expecting great performance")

    vec_env = DummyVecEnv([lambda: HeatSeekerEnv()])

    model = PPO(
        "MlpPolicy",
        vec_env,
        learning_rate=3e-4,
        n_steps=256,
        batch_size=64,
        n_epochs=4,
        verbose=1,
    )

    # Train for a short time
    print("\n   Training for 10,000 steps...")
    model.learn(total_timesteps=10_000)

    # Evaluate
    print("\n3. Evaluating trained agent...")
    eval_env = HeatSeekerEnv()

    successes = 0
    total_rewards = []

    for episode in range(10):
        obs, info = eval_env.reset(seed=episode)
        episode_reward = 0

        for _ in range(200):
            action, _ = model.predict(obs, deterministic=True)
            obs, reward, terminated, truncated, info = eval_env.step(action)
            episode_reward += reward

            if terminated:
                successes += 1
                break
            if truncated:
                break

        total_rewards.append(episode_reward)
        status = "✓ found heat!" if terminated else "✗ timeout"
        print(f"   Episode {episode + 1}: reward={episode_reward:.2f}, dist={info['distance_to_heat']:.1f} {status}")

    print(f"\n   Success rate: {successes}/10 ({successes * 10}%)")
    print(f"   Mean reward: {np.mean(total_rewards):.2f}")

    print("\n" + "=" * 60)
    print("Training test complete!")
    print("=" * 60)


if __name__ == "__main__":
    main()
