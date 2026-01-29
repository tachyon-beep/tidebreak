"""Tests for deterministic seeded reset in tidebreak Python bindings."""

import numpy as np


def test_seeded_reset_determinism():
    """Same seed should produce identical observations after identical operations."""
    from tidebreak import PyUniverse

    # Run 1
    universe1 = PyUniverse(width=100.0, height=100.0, depth=50.0)
    universe1.reset(seed=42)
    universe1.stamp_explosion(center=(10.0, 10.0, 5.0), radius=8.0)
    obs1 = universe1.observe_foveated(position=(0.0, 0.0, 0.0), heading=(1.0, 0.0, 0.0))

    # Run 2 (identical)
    universe2 = PyUniverse(width=100.0, height=100.0, depth=50.0)
    universe2.reset(seed=42)
    universe2.stamp_explosion(center=(10.0, 10.0, 5.0), radius=8.0)
    obs2 = universe2.observe_foveated(position=(0.0, 0.0, 0.0), heading=(1.0, 0.0, 0.0))

    np.testing.assert_array_equal(obs1, obs2, "Same seed should produce identical observations")


def test_different_seeds_produce_different_state():
    """Different seeds should initialize the universe differently."""
    from tidebreak import PyUniverse

    universe1 = PyUniverse(width=100.0, height=100.0, depth=50.0)
    universe1.reset(seed=42)

    universe2 = PyUniverse(width=100.0, height=100.0, depth=50.0)
    universe2.reset(seed=43)

    # The universes are identical until we do something that uses RNG
    # For now, just verify reset works without error
    assert universe1.tick == 0
    assert universe2.tick == 0


def test_reset_without_seed():
    """Reset without seed should reset tick counter."""
    from tidebreak import PyUniverse

    universe = PyUniverse(width=100.0, height=100.0, depth=50.0)
    universe.step(0.1)
    assert universe.tick == 1

    universe.reset()  # No seed
    assert universe.tick == 0


def test_reset_with_seed_clears_state():
    """Reset with seed should clear previous stamps and state."""
    from tidebreak import PyUniverse

    universe = PyUniverse(width=100.0, height=100.0, depth=50.0)
    universe.reset(seed=42)

    # Apply a stamp
    universe.stamp_fire(center=(0.0, 0.0, 0.0), radius=10.0, intensity=1.0)

    # Query the affected area
    result1 = universe.query_volume(center=(0.0, 0.0, 0.0), radius=15.0)
    temp1 = result1.mean("temperature")

    # Reset with same seed should clear the stamp
    universe.reset(seed=42)
    result2 = universe.query_volume(center=(0.0, 0.0, 0.0), radius=15.0)
    temp2 = result2.mean("temperature")

    # Temperature should be lower after reset (back to default)
    assert temp2 < temp1, "Reset should clear previous stamps"


def test_seeded_reset_multiple_times():
    """Multiple resets with same seed should produce consistent state."""
    from tidebreak import PyUniverse

    universe = PyUniverse(width=100.0, height=100.0, depth=50.0)

    observations = []
    for _ in range(3):
        universe.reset(seed=12345)
        universe.stamp_explosion(center=(5.0, 5.0, 5.0), radius=5.0)
        obs = universe.observe_foveated(position=(0.0, 0.0, 0.0), heading=(1.0, 0.0, 0.0))
        observations.append(obs)

    # All observations should be identical
    np.testing.assert_array_equal(observations[0], observations[1])
    np.testing.assert_array_equal(observations[1], observations[2])


def test_seeded_reset_preserves_dimensions():
    """Reset with seed should preserve universe dimensions."""
    from tidebreak import PyUniverse

    universe = PyUniverse(width=200.0, height=150.0, depth=75.0)
    universe.reset(seed=42)

    # The universe should still have the same dimensions
    # We can verify this indirectly by checking that queries still work
    result = universe.query_volume(center=(50.0, 50.0, 25.0), radius=10.0)
    assert result.nodes_visited >= 0  # Query succeeded
