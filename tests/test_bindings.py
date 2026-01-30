"""Integration tests for Python bindings."""

from __future__ import annotations

import numpy as np
import pytest

# Import Rust bindings from the tidebreak package
import tidebreak


class TestEntityId:
    def test_value(self) -> None:
        sim = tidebreak.PySimulation(seed=42)
        ship_id = sim.spawn_ship(0.0, 0.0, 0.0)

        assert ship_id.value >= 0

    def test_equality(self) -> None:
        sim = tidebreak.PySimulation(seed=42)
        id1 = sim.spawn_ship(0.0, 0.0)
        id2 = sim.spawn_ship(10.0, 0.0)

        assert id1 != id2
        assert id1 == id1


class TestEntityTag:
    def test_ship(self) -> None:
        assert tidebreak.PyEntityTag.Ship is not None

    def test_all_tags(self) -> None:
        tags = [
            tidebreak.PyEntityTag.Ship,
            tidebreak.PyEntityTag.Platform,
            tidebreak.PyEntityTag.Projectile,
            tidebreak.PyEntityTag.Squadron,
        ]
        assert len(tags) == 4


class TestSimulation:
    def test_creation(self) -> None:
        sim = tidebreak.PySimulation(seed=123)
        assert sim.tick == 0
        assert sim.seed == 123

    def test_spawn_ship(self) -> None:
        sim = tidebreak.PySimulation()
        ship_id = sim.spawn_ship(10.0, 20.0, 1.5)

        entity = sim.get_entity(ship_id)
        assert entity is not None
        assert entity.tag == tidebreak.PyEntityTag.Ship

        transform = entity.transform
        assert abs(transform.x - 10.0) < 0.001
        assert abs(transform.y - 20.0) < 0.001
        assert abs(transform.heading - 1.5) < 0.001

    def test_step(self) -> None:
        sim = tidebreak.PySimulation()
        sim.spawn_ship(0.0, 0.0)

        sim.step()

        assert sim.tick == 1

    def test_reset(self) -> None:
        sim = tidebreak.PySimulation(seed=42)
        sim.spawn_ship(0.0, 0.0)
        sim.step()

        sim.reset(seed=99)

        assert sim.tick == 0
        assert sim.seed == 99
        assert sim.entity_count == 0


class TestObservation:
    def test_get_observation(self) -> None:
        sim = tidebreak.PySimulation()
        ship_id = sim.spawn_ship(50.0, 50.0, 0.0)

        obs = sim.get_observation(ship_id, max_contacts=8)

        assert obs is not None
        assert obs.own_state_dim == 7
        assert obs.max_contacts == 8

    def test_observation_arrays(self) -> None:
        sim = tidebreak.PySimulation()
        ship_id = sim.spawn_ship(50.0, 50.0, 0.0)

        obs = sim.get_observation(ship_id)

        own = obs.own_state()
        assert isinstance(own, np.ndarray)
        assert own.dtype == np.float32
        assert own.shape == (7,)

        contacts = obs.contacts()
        assert isinstance(contacts, np.ndarray)
        assert len(contacts.shape) == 2
        assert contacts.shape[1] == 5


class TestApplyAction:
    def test_velocity(self) -> None:
        sim = tidebreak.PySimulation()
        ship_id = sim.spawn_ship(0.0, 0.0, 0.0)

        sim.apply_action(ship_id, {"velocity": (5.0, 3.0)})

        entity = sim.get_entity(ship_id)
        assert entity is not None
        physics = entity.physics
        assert physics is not None
        assert abs(physics.vx - 5.0) < 0.001
        assert abs(physics.vy - 3.0) < 0.001

    def test_heading(self) -> None:
        sim = tidebreak.PySimulation()
        ship_id = sim.spawn_ship(0.0, 0.0, 0.0)

        sim.apply_action(ship_id, {"heading": 1.57})

        entity = sim.get_entity(ship_id)
        assert entity is not None
        assert abs(entity.transform.heading - 1.57) < 0.001


class TestDeterminism:
    def test_same_seed_same_result(self) -> None:
        """Simulations with same seed should produce identical results."""

        def run_sim(seed: int) -> tuple[tuple[float, float], ...]:
            sim = tidebreak.PySimulation(seed=seed)
            sim.spawn_ship(0.0, 0.0, 0.0)
            sim.spawn_ship(100.0, 0.0, 3.14)

            for _ in range(100):
                sim.step()

            # Get final positions
            ids = sim.entity_ids()
            positions = []
            for eid in ids:
                entity = sim.get_entity(eid)
                if entity:
                    t = entity.transform
                    positions.append((t.x, t.y))
            return tuple(positions)

        result1 = run_sim(42)
        result2 = run_sim(42)

        assert result1 == result2


class TestCombatEnv:
    def test_env_creation(self) -> None:
        from tidebreak.envs import CombatEnv

        env = CombatEnv()
        assert env.observation_space is not None
        assert env.action_space is not None

    def test_reset(self) -> None:
        from tidebreak.envs import CombatEnv

        env = CombatEnv()
        obs, _info = env.reset(seed=42)

        assert "own_state" in obs
        assert "contacts" in obs
        assert obs["own_state"].shape == (7,)
        assert obs["contacts"].shape == (16, 5)

    def test_step(self) -> None:
        from tidebreak.envs import CombatEnv

        env = CombatEnv()
        env.reset(seed=42)

        action = {
            "velocity": np.array([1.0, 0.0], dtype=np.float32),
            "heading": np.array([0.0], dtype=np.float32),
        }

        obs, reward, terminated, truncated, _info = env.step(action)

        assert obs["own_state"].shape == (7,)
        assert isinstance(reward, float)
        assert isinstance(terminated, bool)
        assert isinstance(truncated, bool)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
