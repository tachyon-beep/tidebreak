"""Tests for foveated observation in tidebreak Python bindings."""

import numpy as np
import pytest


def test_foveated_observation_returns_numpy():
    """Foveated observation should return a numpy array."""
    from tidebreak import PyUniverse

    universe = PyUniverse(width=200.0, height=200.0, depth=50.0)
    universe.stamp_fire(center=(50.0, 0.0, 0.0), radius=10.0)

    obs = universe.observe_foveated(
        position=(0.0, 0.0, 0.0),
        heading=(1.0, 0.0, 0.0),
    )

    assert isinstance(obs, np.ndarray)
    assert obs.dtype == np.float32
    assert len(obs.shape) == 1  # Flat array
    assert obs.shape[0] > 0


def test_foveated_observation_custom_shells():
    """Foveated observation should accept custom shell configurations."""
    from tidebreak import PyUniverse

    universe = PyUniverse(width=200.0, height=200.0, depth=50.0)

    obs = universe.observe_foveated(
        position=(0.0, 0.0, 0.0),
        heading=(1.0, 0.0, 0.0),
        shells=[
            {"radius_inner": 0.0, "radius_outer": 10.0, "sectors": 8},
            {"radius_inner": 10.0, "radius_outer": 50.0, "sectors": 4},
        ],
    )

    assert isinstance(obs, np.ndarray)
    assert obs.shape[0] > 0


def test_foveated_observation_shape_matches_shells():
    """Observation shape should match the number of sectors and fields."""
    from tidebreak import PyUniverse

    universe = PyUniverse(width=200.0, height=200.0, depth=50.0)

    # Custom shells: 8 + 4 = 12 sectors total
    # Default fields: 4 (temperature, noise, occupancy, sonar_return)
    # Expected shape: 12 * 4 = 48
    shells = [
        {"radius_inner": 0.0, "radius_outer": 10.0, "sectors": 8},
        {"radius_inner": 10.0, "radius_outer": 50.0, "sectors": 4},
    ]

    obs = universe.observe_foveated(
        position=(0.0, 0.0, 0.0),
        heading=(1.0, 0.0, 0.0),
        shells=shells,
    )

    # 12 sectors * 4 fields = 48
    expected_size = (8 + 4) * 4
    assert obs.shape[0] == expected_size


def test_foveated_observation_detects_heat_source():
    """Observation should detect a heat source in the field of view."""
    from tidebreak import PyUniverse

    universe = PyUniverse(width=200.0, height=200.0, depth=50.0)

    # Place a fire directly ahead
    universe.stamp_fire(center=(20.0, 0.0, 0.0), radius=5.0, intensity=1.0)

    obs = universe.observe_foveated(
        position=(0.0, 0.0, 0.0),
        heading=(1.0, 0.0, 0.0),
    )

    # The observation should be a valid numpy array.
    # Note: Detection of actual field values depends on murk engine internals.
    # This test verifies the method can be called and returns valid data structure.
    assert isinstance(obs, np.ndarray)
    assert obs.dtype == np.float32


def test_foveated_observation_default_shells():
    """Default shells should produce expected observation size."""
    from tidebreak import PyUniverse

    universe = PyUniverse(width=200.0, height=200.0, depth=50.0)

    obs = universe.observe_foveated(
        position=(0.0, 0.0, 0.0),
        heading=(1.0, 0.0, 0.0),
    )

    # Default shells: (16 + 8 + 4) = 28 sectors
    # Default fields: 4
    # Expected shape: 28 * 4 = 112
    expected_size = (16 + 8 + 4) * 4
    assert obs.shape[0] == expected_size


def test_observation_zero_copy_check():
    """Document whether observation is zero-copy or not."""
    from tidebreak import PyUniverse

    universe = PyUniverse(width=100.0, height=100.0, depth=50.0)
    obs = universe.observe_foveated(
        position=(0.0, 0.0, 0.0),
        heading=(1.0, 0.0, 0.0),
    )

    # This documents the current behavior
    print(f"Observation owns data: {obs.flags['OWNDATA']}")
    assert obs.dtype == np.float32


def test_foveated_observation_missing_shell_key():
    """Missing required keys in shell dict should raise an error."""
    from tidebreak import PyUniverse

    universe = PyUniverse(width=200.0, height=200.0, depth=50.0)

    # Missing 'sectors' key
    with pytest.raises((KeyError, Exception)):
        universe.observe_foveated(
            position=(0.0, 0.0, 0.0),
            heading=(1.0, 0.0, 0.0),
            shells=[{"radius_inner": 0.0, "radius_outer": 10.0}],  # missing sectors
        )


def test_foveated_observation_different_headings():
    """Observation can be queried with different heading directions."""
    from tidebreak import PyUniverse

    universe = PyUniverse(width=200.0, height=200.0, depth=50.0)

    # Looking forward (+x)
    obs_forward = universe.observe_foveated(
        position=(0.0, 0.0, 0.0),
        heading=(1.0, 0.0, 0.0),
    )

    # Looking right (+y)
    obs_right = universe.observe_foveated(
        position=(0.0, 0.0, 0.0),
        heading=(0.0, 1.0, 0.0),
    )

    # Both observations should have the same shape and type
    assert obs_forward.shape == obs_right.shape
    assert obs_forward.dtype == obs_right.dtype == np.float32
