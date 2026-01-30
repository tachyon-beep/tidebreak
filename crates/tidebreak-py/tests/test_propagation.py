"""Tests for field propagation in tidebreak Python bindings."""


def test_temperature_diffusion():
    """Temperature should change over time due to diffusion."""
    from tidebreak import Field, PyUniverse

    # Use smaller world with coarser resolution for faster tests
    universe = PyUniverse(width=50.0, height=50.0, depth=25.0)

    # Create hot spot at center
    universe.stamp_fire(center=(0.0, 0.0, 0.0), radius=10.0, intensity=1.0)

    # Check initial temperature at center (where the fire is)
    initial = universe.query_point(position=(0.0, 0.0, 0.0))
    initial_temp = initial.get(Field.TEMPERATURE)

    # Step simulation multiple times
    for _ in range(10):
        universe.step(0.1)

    # Temperature should have changed due to diffusion
    # (may increase or decrease depending on neighbor values)
    after = universe.query_point(position=(0.0, 0.0, 0.0))
    after_temp = after.get(Field.TEMPERATURE)

    # With diffusion, the hot center should cool slightly as heat spreads
    assert after_temp != initial_temp, "Temperature should change due to diffusion"


def test_noise_decay():
    """Noise should fade toward zero over time."""
    from tidebreak import Field, PyUniverse

    universe = PyUniverse(width=50.0, height=50.0, depth=25.0)

    # Create explosion (generates noise)
    universe.stamp_explosion(center=(0.0, 0.0, 0.0), radius=10.0)

    initial = universe.query_point(position=(0.0, 0.0, 0.0))
    initial_noise = initial.get(Field.NOISE)
    assert initial_noise > 0, "Explosion should create noise"

    # Step simulation for 1 second (10 steps at 0.1s each)
    # With decay rate 0.3, after 1 second: exp(-0.3 * 1) ≈ 0.74
    for _ in range(10):
        universe.step(0.1)

    after = universe.query_point(position=(0.0, 0.0, 0.0))
    after_noise = after.get(Field.NOISE)

    # Noise should have decayed significantly
    assert after_noise < initial_noise, "Noise should decay over time"
    assert after_noise > 0, "Noise should still be positive (exponential decay)"


def test_multiple_steps_continue_propagation():
    """Propagation should continue with each step, not just the first."""
    from tidebreak import Field, PyUniverse

    universe = PyUniverse(width=50.0, height=50.0, depth=25.0)

    # Create explosion
    universe.stamp_explosion(center=(0.0, 0.0, 0.0), radius=10.0)

    noise_values = []
    noise_values.append(universe.query_point(position=(0.0, 0.0, 0.0)).get(Field.NOISE))

    # Track noise over several steps
    for _ in range(5):
        universe.step(0.1)
        noise_values.append(universe.query_point(position=(0.0, 0.0, 0.0)).get(Field.NOISE))

    # Each step should continue to decay noise
    for i in range(1, len(noise_values)):
        assert noise_values[i] < noise_values[i - 1], f"Noise should decrease at step {i}"
