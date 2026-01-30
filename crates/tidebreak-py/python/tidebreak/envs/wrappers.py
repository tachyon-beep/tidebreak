"""Action space wrappers for SB3 compatibility.

Stable-baselines3 does not support Dict action spaces. These wrappers
flatten the Dict action space to Box for compatibility with SB3 algorithms.
"""

from __future__ import annotations

import gymnasium as gym
import numpy as np
from gymnasium import spaces


class FlatActionWrapper(gym.ActionWrapper):
    """Flatten Dict action space to Box for SB3 compatibility.

    Input action: Box(-1, 1, shape=(3,)) as [throttle, turn_rate, fire_logit]
    Output action: Dict with throttle, turn_rate, fire

    Fire logit mapping: fire_logit >= 0.0 -> fire=1, else fire=0
    """

    def __init__(self, env: gym.Env) -> None:
        super().__init__(env)
        # Flat action space: [throttle, turn_rate, fire_logit]
        self.action_space = spaces.Box(
            low=-1.0,
            high=1.0,
            shape=(3,),
            dtype=np.float32,
        )

    def action(self, action: np.ndarray) -> dict[str, np.ndarray | int]:
        """Convert flat action to dict action."""
        return {
            "throttle": np.array([action[0]], dtype=np.float32),
            "turn_rate": np.array([action[1]], dtype=np.float32),
            "fire": 1 if action[2] >= 0.0 else 0,
        }
