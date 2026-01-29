"""MurkEnv: A Gymnasium environment wrapping the Murk spatial substrate."""

from __future__ import annotations

from typing import Any, ClassVar

import gymnasium as gym
import numpy as np
from gymnasium import spaces

# Import directly from the Rust extension to avoid circular imports
from tidebreak._tidebreak import PyUniverse


class MurkEnv(gym.Env[np.ndarray, dict[str, Any]]):
    """Gymnasium environment wrapping the Murk spatial substrate.

    This environment provides a foveated observation of the Murk field
    substrate, with actions for movement and field stamping (fire/sonar).

    Observation Space:
        Box with shape (total_sectors * num_fields,) containing foveated
        field observations. Default fields are: temperature, noise,
        occupancy, sonar_return.

    Action Space:
        Dict with:
        - "move": Box(-1, 1, shape=(2,)) - 2D movement direction
        - "stamp": Discrete(3) - 0=none, 1=fire, 2=sonar

    Attributes:
        metadata: Environment metadata including render modes and FPS.
        world_size: Tuple of (width, height, depth) for the universe.
        max_steps: Maximum steps per episode before truncation.
        render_mode: Current render mode (rgb_array or None).
    """

    metadata: ClassVar[dict[str, Any]] = {"render_modes": ["rgb_array"], "render_fps": 10}

    def __init__(
        self,
        world_size: tuple[float, float, float] = (200.0, 200.0, 50.0),
        max_steps: int = 1000,
        render_mode: str | None = None,
        agent_speed: float = 2.0,
        stamp_radius: float = 10.0,
        stamp_intensity: float = 1.0,
        dt: float = 0.1,
    ) -> None:
        """Initialize the MurkEnv environment.

        Args:
            world_size: Tuple of (width, height, depth) for universe bounds.
            max_steps: Maximum steps per episode before truncation.
            render_mode: Render mode ("rgb_array" or None).
            agent_speed: Maximum movement speed per step.
            stamp_radius: Radius of fire/sonar stamps.
            stamp_intensity: Intensity of stamps.
            dt: Time step for simulation advancement.
        """
        super().__init__()

        self.world_size = world_size
        self.max_steps = max_steps
        self.render_mode = render_mode
        self.agent_speed = agent_speed
        self.stamp_radius = stamp_radius
        self.stamp_intensity = stamp_intensity
        self.dt = dt

        # Shell configuration for foveated observations
        self.shells = [
            {"radius_inner": 0.0, "radius_outer": 10.0, "sectors": 8},
            {"radius_inner": 10.0, "radius_outer": 50.0, "sectors": 4},
        ]

        # Calculate observation size
        # total_sectors = 8 + 4 = 12
        # num_fields = 4 (temperature, noise, occupancy, sonar_return)
        # obs_size = 12 * 4 = 48
        total_sectors = sum(s["sectors"] for s in self.shells)
        num_fields = 4  # Default fields in FoveatedQuery
        obs_size = total_sectors * num_fields

        self.observation_space = spaces.Box(
            low=-np.inf,
            high=np.inf,
            shape=(obs_size,),
            dtype=np.float32,
        )

        self.action_space = spaces.Dict(
            {
                "move": spaces.Box(-1.0, 1.0, shape=(2,), dtype=np.float32),
                "stamp": spaces.Discrete(3),  # 0=none, 1=fire, 2=sonar
            }
        )

        # Internal state (initialized in reset)
        self._universe: PyUniverse | None = None
        self._agent_position: np.ndarray = np.zeros(3, dtype=np.float32)
        self._agent_heading: np.ndarray = np.array([1.0, 0.0, 0.0], dtype=np.float32)
        self._step_count: int = 0
        self._np_random: np.random.Generator | None = None

    def reset(
        self,
        *,
        seed: int | None = None,
        options: dict[str, Any] | None = None,
    ) -> tuple[np.ndarray, dict[str, Any]]:
        """Reset the environment to initial state.

        Args:
            seed: Random seed for reproducibility.
            options: Additional options (unused).

        Returns:
            Tuple of (observation, info dict).
        """
        super().reset(seed=seed)

        # Create or reset the universe
        width, height, depth = self.world_size
        self._universe = PyUniverse(
            width=width,
            height=height,
            depth=depth,
            base_resolution=1.0,
        )

        if seed is not None:
            self._universe.reset(seed=seed)

        # Reset agent position to center of world
        self._agent_position = np.array(
            [width / 2, height / 2, depth / 2],
            dtype=np.float32,
        )
        self._agent_heading = np.array([1.0, 0.0, 0.0], dtype=np.float32)
        self._step_count = 0

        obs = self._get_observation()
        info = self._get_info()

        return obs, info

    def step(
        self,
        action: dict[str, Any],
    ) -> tuple[np.ndarray, float, bool, bool, dict[str, Any]]:
        """Execute one environment step.

        Args:
            action: Dict with "move" (2D direction) and "stamp" (0/1/2).

        Returns:
            Tuple of (observation, reward, terminated, truncated, info).
        """
        if self._universe is None:
            raise RuntimeError("Environment not initialized. Call reset() first.")

        # Extract action components
        move_action = np.array(action["move"], dtype=np.float32)
        stamp_action = int(action["stamp"])

        # Normalize and apply movement
        move_norm = np.linalg.norm(move_action)
        if move_norm > 0:
            direction = move_action / move_norm
            # Update heading (2D, extend to 3D with z=0)
            self._agent_heading = np.array(
                [direction[0], direction[1], 0.0],
                dtype=np.float32,
            )
            # Move agent
            displacement = direction * self.agent_speed * min(move_norm, 1.0)
            self._agent_position[0] += displacement[0]
            self._agent_position[1] += displacement[1]

            # Clamp to world bounds
            self._agent_position[0] = np.clip(self._agent_position[0], 0, self.world_size[0])
            self._agent_position[1] = np.clip(self._agent_position[1], 0, self.world_size[1])

        # Apply stamp action
        center = tuple(self._agent_position)
        if stamp_action == 1:  # Fire
            self._universe.stamp_fire(
                center=center,
                radius=self.stamp_radius,
                intensity=self.stamp_intensity,
            )
        elif stamp_action == 2:  # Sonar
            self._universe.stamp_sonar_ping(
                center=center,
                radius=self.stamp_radius,
                strength=self.stamp_intensity,
            )

        # Advance simulation
        self._universe.step(self.dt)

        self._step_count += 1

        # Get observation and compute reward
        obs = self._get_observation()
        reward = self._compute_reward()
        terminated = False  # No terminal state in this basic environment
        truncated = self._step_count >= self.max_steps
        info = self._get_info()

        return obs, reward, terminated, truncated, info

    def _get_observation(self) -> np.ndarray:
        """Get foveated observation from current agent position."""
        if self._universe is None:
            raise RuntimeError("Universe not initialized.")

        obs = self._universe.observe_foveated(
            position=tuple(self._agent_position),
            heading=tuple(self._agent_heading),
            shells=self.shells,
        )
        return obs

    def _compute_reward(self) -> float:
        """Compute reward for current state.

        This basic implementation returns a constant small negative reward
        to encourage efficiency. Override in subclasses for custom rewards.
        """
        return -0.01  # Small step penalty to encourage efficiency

    def _get_info(self) -> dict[str, Any]:
        """Get info dict for current state."""
        return {
            "step": self._step_count,
            "agent_position": self._agent_position.copy(),
            "agent_heading": self._agent_heading.copy(),
            "time": self._universe.time if self._universe else 0.0,
        }

    def render(self) -> np.ndarray | None:
        """Render the environment.

        Returns:
            RGB array if render_mode is "rgb_array", None otherwise.
        """
        if self.render_mode != "rgb_array":
            return None

        # Basic visualization: render agent position on a grid
        # This is a placeholder - full visualization would use the murk
        # field data
        width = int(self.world_size[0])
        height = int(self.world_size[1])
        img = np.zeros((height, width, 3), dtype=np.uint8)

        # Draw agent position as a white dot
        ax = int(np.clip(self._agent_position[0], 0, width - 1))
        ay = int(np.clip(self._agent_position[1], 0, height - 1))
        img[ay, ax] = [255, 255, 255]

        return img

    def close(self) -> None:
        """Clean up environment resources."""
        self._universe = None
