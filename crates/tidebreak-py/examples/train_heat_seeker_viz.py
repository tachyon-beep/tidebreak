#!/usr/bin/env python3
"""Train a heat-seeking agent with visualization.

This script:
1. Trains PPO for longer (50k steps)
2. Visualizes episodes showing agent path and temperature field
3. Saves training curves and episode recordings

Usage:
    python train_heat_seeker_viz.py
"""

from __future__ import annotations

from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np

try:
    from stable_baselines3 import PPO
    from stable_baselines3.common.callbacks import BaseCallback
    from stable_baselines3.common.vec_env import DummyVecEnv
except ImportError as err:
    raise ImportError("Install stable-baselines3: uv pip install stable-baselines3") from err

import gymnasium as gym
from gymnasium import spaces

from tidebreak import Field, PyUniverse


class HeatSeekerEnv(gym.Env):
    """Environment where agent seeks a randomly placed heat source.

    Observation: 48 floats from foveated sensing (12 sectors x 4 fields)
    Action: 2D continuous movement direction
    Reward: Temperature sensed + distance shaping + step penalty
    """

    def __init__(
        self,
        world_size: float = 100.0,
        max_steps: int = 200,
        record_trajectory: bool = False,
    ):
        super().__init__()

        self.world_size = world_size
        self.max_steps = max_steps
        self.dt = 0.1
        self.agent_speed = 3.0
        self.record_trajectory = record_trajectory

        # Foveated observation shells
        self.shells = [
            {"radius_inner": 0.0, "radius_outer": 15.0, "sectors": 8},
            {"radius_inner": 15.0, "radius_outer": 40.0, "sectors": 4},
        ]
        obs_size = (8 + 4) * 4  # 48

        self.observation_space = spaces.Box(low=-np.inf, high=np.inf, shape=(obs_size,), dtype=np.float32)
        self.action_space = spaces.Box(low=-1.0, high=1.0, shape=(2,), dtype=np.float32)

        self._universe: PyUniverse | None = None
        self._agent_pos = np.zeros(3, dtype=np.float32)
        self._agent_heading = np.array([1.0, 0.0, 0.0], dtype=np.float32)
        self._heat_pos = np.zeros(3, dtype=np.float32)
        self._step_count = 0
        self._prev_distance = 0.0

        # For recording
        self.trajectory: list[tuple[float, float]] = []

    def reset(self, *, seed=None, options=None):
        super().reset(seed=seed)

        # Create universe with coarse resolution for speed
        self._universe = PyUniverse(
            width=self.world_size,
            height=self.world_size,
            depth=25.0,
            base_resolution=4.0,
        )

        # Random agent starting position
        self._agent_pos = np.array(
            [
                self.np_random.uniform(20, 80),
                self.np_random.uniform(20, 80),
                12.5,
            ],
            dtype=np.float32,
        )
        self._agent_heading = np.array([1.0, 0.0, 0.0], dtype=np.float32)

        # Place heat source far from agent
        while True:
            self._heat_pos = np.array(
                [
                    self.np_random.uniform(15, 85),
                    self.np_random.uniform(15, 85),
                    12.5,
                ],
                dtype=np.float32,
            )
            dist = np.linalg.norm(self._heat_pos[:2] - self._agent_pos[:2])
            if dist > 35:
                break

        # Stamp heat source
        self._universe.stamp_fire(
            center=tuple(self._heat_pos),
            radius=12.0,
            intensity=1.0,
        )

        self._step_count = 0
        self._prev_distance = self._distance_to_heat()

        # Reset trajectory recording
        if self.record_trajectory:
            self.trajectory = [(float(self._agent_pos[0]), float(self._agent_pos[1]))]

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

        # Record trajectory
        if self.record_trajectory:
            self.trajectory.append((float(self._agent_pos[0]), float(self._agent_pos[1])))

        # Step physics
        self._universe.step(self.dt)
        self._step_count += 1

        # Compute reward with distance shaping
        obs = self._get_obs()
        dist = self._distance_to_heat()

        # Temperature reward (normalized)
        temps = obs[0::4]
        forward_temp = temps[0]
        temp_reward = (forward_temp - 293) / 500

        # Distance shaping: reward for getting closer
        distance_reward = (self._prev_distance - dist) * 0.1
        self._prev_distance = dist

        # Proximity bonus
        proximity_bonus = 0.5 if dist < 15 else (0.2 if dist < 25 else 0.0)

        # Step penalty
        step_penalty = -0.01

        reward = temp_reward + distance_reward + proximity_bonus + step_penalty

        terminated = dist < 8  # Success
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

    def get_temperature_grid(self, resolution: int = 50) -> np.ndarray:
        """Sample temperature field for visualization."""
        if self._universe is None:
            return np.zeros((resolution, resolution))

        grid = np.zeros((resolution, resolution))
        for i in range(resolution):
            for j in range(resolution):
                x = (i / resolution) * self.world_size
                y = (j / resolution) * self.world_size
                result = self._universe.query_point(position=(x, y, 12.5))
                grid[j, i] = result.get(Field.TEMPERATURE)

        return grid

    @property
    def heat_position(self):
        return self._heat_pos[:2].copy()

    @property
    def agent_position(self):
        return self._agent_pos[:2].copy()


class TrainingCallback(BaseCallback):
    """Callback to track training metrics."""

    def __init__(self, verbose=0):
        super().__init__(verbose)
        self.episode_rewards = []
        self.episode_lengths = []
        self.successes = []

    def _on_step(self):
        # Check for episode completion
        if self.locals.get("dones") is not None:
            for i, done in enumerate(self.locals["dones"]):
                if done:
                    info = self.locals["infos"][i]
                    if "episode" in info:
                        self.episode_rewards.append(info["episode"]["r"])
                        self.episode_lengths.append(info["episode"]["l"])
                    self.successes.append(info.get("success", False))
        return True


def visualize_episode(
    env: HeatSeekerEnv,
    model: PPO,
    episode_num: int,
    save_path: Path,
):
    """Run one episode and visualize."""
    env.record_trajectory = True
    obs, info = env.reset(seed=episode_num * 100)

    total_reward = 0
    for _ in range(200):
        action, _ = model.predict(obs, deterministic=True)
        obs, reward, terminated, truncated, info = env.step(action)
        total_reward += reward
        if terminated or truncated:
            break

    # Create visualization
    fig, axes = plt.subplots(1, 2, figsize=(14, 6))

    # Left: Temperature field with trajectory
    ax1 = axes[0]
    temp_grid = env.get_temperature_grid(50)
    im = ax1.imshow(
        temp_grid,
        origin="lower",
        extent=[0, env.world_size, 0, env.world_size],
        cmap="hot",
        vmin=290,
        vmax=500,
    )
    plt.colorbar(im, ax=ax1, label="Temperature (K)")

    # Plot trajectory
    traj = np.array(env.trajectory)
    ax1.plot(traj[:, 0], traj[:, 1], "b-", linewidth=1.5, alpha=0.7, label="Path")
    ax1.plot(traj[0, 0], traj[0, 1], "go", markersize=10, label="Start")
    ax1.plot(traj[-1, 0], traj[-1, 1], "bs", markersize=10, label="End")

    # Plot heat source
    heat_pos = env.heat_position
    circle = plt.Circle(heat_pos, 12, fill=False, color="red", linewidth=2)
    ax1.add_patch(circle)
    ax1.plot(heat_pos[0], heat_pos[1], "r*", markersize=15, label="Heat Source")

    ax1.set_xlim(0, env.world_size)
    ax1.set_ylim(0, env.world_size)
    ax1.set_xlabel("X")
    ax1.set_ylabel("Y")
    ax1.set_title(f"Episode {episode_num + 1}")
    ax1.legend(loc="upper right")
    ax1.set_aspect("equal")

    # Right: Episode stats
    ax2 = axes[1]
    ax2.axis("off")

    status = "SUCCESS" if info.get("success") else "TIMEOUT"
    stats_text = f"""
Episode {episode_num + 1} Results
{'=' * 30}

Status: {status}
Total Reward: {total_reward:.2f}
Steps Taken: {len(env.trajectory)}
Final Distance: {info['distance_to_heat']:.1f}

Starting Distance: {np.linalg.norm(np.array(env.trajectory[0]) - heat_pos):.1f}
Distance Traveled: {sum(np.linalg.norm(np.diff(traj, axis=0), axis=1)):.1f}
"""
    ax2.text(
        0.1,
        0.9,
        stats_text,
        transform=ax2.transAxes,
        fontsize=12,
        verticalalignment="top",
        fontfamily="monospace",
    )

    plt.tight_layout()
    plt.savefig(save_path / f"episode_{episode_num + 1:02d}.png", dpi=100)
    plt.close()

    return info.get("success", False), total_reward


def plot_training_curves(callback: TrainingCallback, save_path: Path):
    """Plot training curves."""
    fig, axes = plt.subplots(2, 2, figsize=(12, 10))

    # Episode rewards (smoothed)
    ax1 = axes[0, 0]
    rewards = callback.episode_rewards
    if len(rewards) > 10:
        smoothed = np.convolve(rewards, np.ones(10) / 10, mode="valid")
        ax1.plot(rewards, alpha=0.3, label="Raw")
        ax1.plot(range(9, len(rewards)), smoothed, label="Smoothed (10-ep)")
    else:
        ax1.plot(rewards)
    ax1.set_xlabel("Episode")
    ax1.set_ylabel("Reward")
    ax1.set_title("Episode Rewards")
    ax1.legend()

    # Episode lengths
    ax2 = axes[0, 1]
    lengths = callback.episode_lengths
    if len(lengths) > 10:
        smoothed = np.convolve(lengths, np.ones(10) / 10, mode="valid")
        ax2.plot(lengths, alpha=0.3, label="Raw")
        ax2.plot(range(9, len(lengths)), smoothed, label="Smoothed (10-ep)")
    else:
        ax2.plot(lengths)
    ax2.set_xlabel("Episode")
    ax2.set_ylabel("Steps")
    ax2.set_title("Episode Lengths")
    ax2.legend()

    # Success rate (rolling)
    ax3 = axes[1, 0]
    successes = [1 if s else 0 for s in callback.successes]
    if len(successes) > 20:
        rolling = np.convolve(successes, np.ones(20) / 20, mode="valid")
        ax3.plot(range(19, len(successes)), rolling * 100)
        ax3.set_ylabel("Success Rate (%)")
    else:
        ax3.bar(range(len(successes)), successes)
        ax3.set_ylabel("Success (0/1)")
    ax3.set_xlabel("Episode")
    ax3.set_title("Success Rate (20-episode rolling)")

    # Summary stats
    ax4 = axes[1, 1]
    ax4.axis("off")

    total_episodes = len(callback.episode_rewards)
    total_successes = sum(callback.successes)
    if total_episodes > 0:
        recent_rewards = callback.episode_rewards[-50:] if len(rewards) >= 50 else rewards
        recent_successes = callback.successes[-50:] if len(successes) >= 50 else successes
        summary = f"""
Training Summary
{'=' * 30}

Total Episodes: {total_episodes}
Total Successes: {total_successes} ({100 * total_successes / total_episodes:.1f}%)

Last 50 Episodes:
  Mean Reward: {np.mean(recent_rewards):.2f}
  Success Rate: {100 * sum(recent_successes) / len(recent_successes):.1f}%

Overall:
  Best Reward: {max(rewards):.2f}
  Mean Reward: {np.mean(rewards):.2f}
"""
    else:
        summary = "No episodes completed yet"

    ax4.text(
        0.1,
        0.9,
        summary,
        transform=ax4.transAxes,
        fontsize=11,
        verticalalignment="top",
        fontfamily="monospace",
    )

    plt.tight_layout()
    plt.savefig(save_path / "training_curves.png", dpi=150)
    plt.close()


def main():
    print("=" * 60)
    print("Heat-Seeking Agent Training with Visualization")
    print("=" * 60)

    # Create output directory
    output_dir = Path("training_output")
    output_dir.mkdir(exist_ok=True)

    # Test environment
    print("\n1. Testing environment...")
    env = HeatSeekerEnv()
    obs, info = env.reset(seed=42)
    print(f"   Observation shape: {obs.shape}")
    print(f"   Initial distance: {info['distance_to_heat']:.1f}")

    # Create vectorized environment
    print("\n2. Setting up training...")
    vec_env = DummyVecEnv([lambda: HeatSeekerEnv()])

    callback = TrainingCallback()

    model = PPO(
        "MlpPolicy",
        vec_env,
        learning_rate=3e-4,
        n_steps=512,
        batch_size=64,
        n_epochs=10,
        gamma=0.99,
        gae_lambda=0.95,
        clip_range=0.2,
        ent_coef=0.01,
        verbose=1,
    )

    # Train
    total_timesteps = 50_000
    print(f"\n3. Training for {total_timesteps:,} steps...")
    print("   (This may take a few minutes)")

    model.learn(total_timesteps=total_timesteps, callback=callback)

    # Plot training curves
    print("\n4. Generating training curves...")
    plot_training_curves(callback, output_dir)
    print(f"   Saved to {output_dir}/training_curves.png")

    # Evaluate and visualize episodes
    print("\n5. Evaluating and visualizing episodes...")
    eval_env = HeatSeekerEnv()

    successes = 0
    total_rewards = []

    for episode in range(10):
        success, reward = visualize_episode(eval_env, model, episode, output_dir)
        total_rewards.append(reward)
        if success:
            successes += 1
        status = "SUCCESS" if success else "timeout"
        print(f"   Episode {episode + 1}: reward={reward:.2f} [{status}]")

    print(f"\n   Final Success Rate: {successes}/10 ({successes * 10}%)")
    print(f"   Mean Reward: {np.mean(total_rewards):.2f}")
    print(f"\n   Episode visualizations saved to {output_dir}/")

    # Save model
    model_path = output_dir / "heat_seeker_model"
    model.save(model_path)
    print(f"\n6. Model saved to {model_path}")

    print("\n" + "=" * 60)
    print("Training complete!")
    print(f"Check {output_dir}/ for visualizations")
    print("=" * 60)


if __name__ == "__main__":
    main()
