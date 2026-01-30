"""Single-agent combat environment."""

from __future__ import annotations

from typing import Any, ClassVar

import gymnasium as gym
import numpy as np
from gymnasium import spaces

# Import directly from the Rust extension to avoid circular imports
from tidebreak._tidebreak import PySimulation


class CombatEnv(gym.Env):
    """Single-agent combat training environment.

    Observation space:
        Dict with:
        - "own_state": Box(7,) - [x, y, heading, vx, vy, hp, max_hp]
        - "contacts": Box(max_contacts, 5) - contact info per track

    Action space:
        Dict with:
        - "velocity": Box(2,) - desired velocity (vx, vy)
        - "heading": Box(1,) - desired heading in radians
    """

    metadata: ClassVar[dict[str, Any]] = {"render_modes": ["human", "rgb_array"]}

    def __init__(
        self,
        max_contacts: int = 16,
        max_speed: float = 20.0,
        max_steps: int = 1000,
        render_mode: str | None = None,
    ) -> None:
        super().__init__()

        self.max_contacts = max_contacts
        self.max_speed = max_speed
        self.max_steps = max_steps
        self.render_mode = render_mode

        # Observation space
        self.observation_space = spaces.Dict(
            {
                "own_state": spaces.Box(low=-np.inf, high=np.inf, shape=(7,), dtype=np.float32),
                "contacts": spaces.Box(low=-np.inf, high=np.inf, shape=(max_contacts, 5), dtype=np.float32),
            }
        )

        # Action space
        self.action_space = spaces.Dict(
            {
                "velocity": spaces.Box(low=-max_speed, high=max_speed, shape=(2,), dtype=np.float32),
                "heading": spaces.Box(low=-np.pi, high=np.pi, shape=(1,), dtype=np.float32),
            }
        )

        self._sim: PySimulation | None = None
        self._agent_id = None
        self._step_count = 0

    def reset(
        self,
        *,
        seed: int | None = None,
        options: dict[str, Any] | None = None,
    ) -> tuple[dict[str, np.ndarray], dict[str, Any]]:
        super().reset(seed=seed)

        # Create new simulation
        sim_seed = seed if seed is not None else self.np_random.integers(0, 2**32)
        self._sim = PySimulation(seed=sim_seed)

        # Spawn agent ship at origin
        self._agent_id = self._sim.spawn_ship(0.0, 0.0, 0.0)

        # Spawn some enemies for training
        self._setup_scenario()

        self._step_count = 0

        obs = self._get_obs()
        info = {"tick": self._sim.tick}

        return obs, info

    def step(
        self,
        action: dict[str, np.ndarray],
    ) -> tuple[dict[str, np.ndarray], float, bool, bool, dict[str, Any]]:
        assert self._sim is not None and self._agent_id is not None

        # Convert action to dict format expected by Rust
        action_dict = {
            "velocity": (float(action["velocity"][0]), float(action["velocity"][1])),
            "heading": float(action["heading"][0]),
        }

        # Apply action
        self._sim.apply_action(self._agent_id, action_dict)

        # Step simulation
        self._sim.step()
        self._step_count += 1

        # Get observation
        obs = self._get_obs()

        # Compute reward
        reward = self._compute_reward()

        # Check termination
        terminated = self._is_terminated()
        truncated = self._step_count >= self.max_steps

        info = {
            "tick": self._sim.tick,
            "entity_count": self._sim.entity_count,
        }

        return obs, reward, terminated, truncated, info

    def _get_obs(self) -> dict[str, np.ndarray]:
        assert self._sim is not None and self._agent_id is not None

        py_obs = self._sim.get_observation(self._agent_id, self.max_contacts)
        if py_obs is None:
            # Agent was destroyed
            return {
                "own_state": np.zeros(7, dtype=np.float32),
                "contacts": np.zeros((self.max_contacts, 5), dtype=np.float32),
            }

        return {
            "own_state": np.asarray(py_obs.own_state(), dtype=np.float32),
            "contacts": np.asarray(py_obs.contacts(), dtype=np.float32),
        }

    def _compute_reward(self) -> float:
        # Placeholder reward - survival bonus
        return 0.1

    def _is_terminated(self) -> bool:
        assert self._sim is not None and self._agent_id is not None

        entity = self._sim.get_entity(self._agent_id)
        if entity is None:
            return True  # Despawned

        return bool(entity.is_destroyed())

    def _setup_scenario(self) -> None:
        """Spawn training scenario entities."""
        assert self._sim is not None

        # Spawn a few enemy ships in a circle around the agent
        for i in range(3):
            angle = i * 2.0 * np.pi / 3.0
            x = 100.0 * np.cos(angle)
            y = 100.0 * np.sin(angle)
            self._sim.spawn_ship(float(x), float(y), float(angle + np.pi))
