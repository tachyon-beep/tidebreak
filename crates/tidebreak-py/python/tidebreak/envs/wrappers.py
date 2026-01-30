"""Action and observation space wrappers for SB3 compatibility.

Stable-baselines3 does not support Dict action/observation spaces. These wrappers
flatten and normalize Dict spaces to Box for compatibility with SB3 algorithms.
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


class NormalizedObsWrapper(gym.ObservationWrapper):
    """Normalize Dict observation to flat Box for better training.

    Normalizes positions, velocities, angles, and HP to [-1, 1] or [0, 1] range.
    Angles are encoded as [sin(theta), cos(theta)] for smooth gradients.

    Input: Dict with own_state (7,), contacts (max_contacts, 5), context (2,)
    Output: Box with shape (obs_dim,) where obs_dim depends on max_contacts

    Observation layout:
        own_state: [x_norm, y_norm, sin_h, cos_h, vx_norm, vy_norm, hp_ratio] = 7 dims
        contacts:  [x_norm, y_norm, sin_b, cos_b, dist_norm, quality_norm] * max_contacts = 6 dims each
        context:   [step_ratio, remaining_ratio] = 2 dims

    Contact fields from Rust (per contact, 5 values):
        [x, y, rel_heading (bearing TO contact), distance, quality (0-100)]
    """

    def __init__(
        self,
        env: gym.Env,
        world_size: float = 500.0,
        max_speed: float = 20.0,
    ) -> None:
        super().__init__(env)
        self._world_size = world_size
        self._max_speed = max_speed

        # Get max_contacts from wrapped env
        self._max_contacts = env.unwrapped.max_contacts
        self._max_steps = env.unwrapped.max_steps

        # Calculate observation dimension
        # own_state: 7 dims (x, y, sin_h, cos_h, vx, vy, hp_ratio)
        # contacts: 6 dims per contact (x, y, sin_b, cos_b, dist, quality)
        # context: 2 dims
        own_dim = 7
        contact_dim = 6 * self._max_contacts
        context_dim = 2
        total_dim = own_dim + contact_dim + context_dim

        self.observation_space = spaces.Box(
            low=-1.0,
            high=1.0,
            shape=(total_dim,),
            dtype=np.float32,
        )

    def observation(self, obs: dict[str, np.ndarray]) -> np.ndarray:
        """Convert dict observation to normalized flat array."""
        own = obs["own_state"]
        contacts = obs["contacts"]
        context = obs["context"]

        # Normalize own_state
        own_normalized = np.array(
            [
                own[0] / self._world_size,  # x normalized
                own[1] / self._world_size,  # y normalized
                np.sin(own[2]),  # sin(heading)
                np.cos(own[2]),  # cos(heading)
                own[3] / self._max_speed,  # vx normalized
                own[4] / self._max_speed,  # vy normalized
                own[5] / own[6],  # hp / max_hp
            ],
            dtype=np.float32,
        )

        # Normalize contacts
        # Rust contact layout: [x, y, rel_heading (bearing TO contact), distance, quality]
        contacts_normalized = []
        for i in range(self._max_contacts):
            c = contacts[i]
            contacts_normalized.extend(
                [
                    c[0] / self._world_size,  # x normalized
                    c[1] / self._world_size,  # y normalized
                    np.sin(c[2]),  # sin(bearing to contact)
                    np.cos(c[2]),  # cos(bearing to contact)
                    np.clip(c[3] / self._world_size, -1, 1),  # distance normalized
                    c[4] / 100.0,  # quality normalized (0-1)
                ]
            )
        contacts_flat = np.array(contacts_normalized, dtype=np.float32)

        # Normalize context
        context_normalized = np.array(
            [
                context[0] / self._max_steps,  # step ratio
                context[1] / self._max_steps,  # remaining ratio
            ],
            dtype=np.float32,
        )

        return np.concatenate([own_normalized, contacts_flat, context_normalized])


def make_sb3_env(
    max_contacts: int = 16,
    max_steps: int = 1000,
    world_size: float = 500.0,
    max_speed: float = 20.0,
    **kwargs,
) -> gym.Env:
    """Create a CombatEnv wrapped for SB3 compatibility.

    Returns an environment with:
    - Flat Box action space: (3,) for [throttle, turn_rate, fire_logit]
    - Flat Box observation space: normalized to [-1, 1]

    Args:
        max_contacts: Maximum tracked contacts
        max_steps: Episode length
        world_size: World size for position normalization
        max_speed: Max entity speed for velocity normalization
        **kwargs: Additional CombatEnv parameters

    Returns:
        Wrapped Gymnasium environment ready for SB3 training
    """
    from tidebreak.envs import CombatEnv

    env = CombatEnv(max_contacts=max_contacts, max_steps=max_steps, **kwargs)
    env = FlatActionWrapper(env)
    env = NormalizedObsWrapper(env, world_size=world_size, max_speed=max_speed)
    return env
