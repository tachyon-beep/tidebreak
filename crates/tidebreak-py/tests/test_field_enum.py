"""Tests for the Field enum in tidebreak Python bindings."""


def test_field_enum_exists():
    """Field enum should be accessible from tidebreak module."""
    import tidebreak

    assert hasattr(tidebreak, "Field")


def test_field_enum_values():
    """All field enum variants should exist."""
    from tidebreak import Field

    # Test all expected enum values exist
    assert Field.OCCUPANCY is not None
    assert Field.MATERIAL is not None
    assert Field.INTEGRITY is not None
    assert Field.TEMPERATURE is not None
    assert Field.SMOKE is not None
    assert Field.NOISE is not None
    assert Field.SIGNAL is not None
    assert Field.CURRENT_X is not None
    assert Field.CURRENT_Y is not None
    assert Field.DEPTH is not None
    assert Field.SALINITY is not None
    assert Field.SONAR_RETURN is not None


def test_field_enum_equality():
    """Field enum values should support equality comparison."""
    from tidebreak import Field

    # Same value should be equal
    assert Field.TEMPERATURE == Field.TEMPERATURE
    assert Field.NOISE == Field.NOISE

    # Different values should not be equal
    assert Field.TEMPERATURE != Field.NOISE
    assert Field.OCCUPANCY != Field.INTEGRITY


def test_field_enum_hash():
    """Field enum values should be hashable (usable as dict keys)."""
    from tidebreak import Field

    # Should be usable as dictionary keys
    field_dict = {
        Field.TEMPERATURE: "temp",
        Field.NOISE: "noise",
        Field.DEPTH: "depth",
    }

    assert field_dict[Field.TEMPERATURE] == "temp"
    assert field_dict[Field.NOISE] == "noise"
    assert field_dict[Field.DEPTH] == "depth"


def test_field_enum_used_in_query():
    """Field enum should work with query_volume methods."""
    from tidebreak import Field, PyUniverse

    universe = PyUniverse(width=100.0, height=100.0, depth=50.0)
    universe.stamp_explosion(center=(0.0, 0.0, 0.0), radius=10.0)
    result = universe.query_volume(center=(0.0, 0.0, 0.0), radius=15.0)

    # Should work with enum
    temp = result.mean(Field.TEMPERATURE)
    assert temp > 0  # Explosion should increase temperature

    # Test other methods with enum
    noise = result.max(Field.NOISE)
    assert noise >= 0

    variance = result.variance(Field.TEMPERATURE)
    assert variance >= 0

    min_temp = result.min(Field.TEMPERATURE)
    assert min_temp >= 0


def test_field_enum_used_in_point_query():
    """Field enum should work with query_point methods."""
    from tidebreak import Field, PyUniverse

    universe = PyUniverse(width=100.0, height=100.0, depth=50.0)
    universe.stamp_explosion(center=(0.0, 0.0, 0.0), radius=10.0)
    result = universe.query_point(position=(0.0, 0.0, 0.0))

    # Should work with enum
    temp = result.get(Field.TEMPERATURE)
    assert temp > 0  # Explosion should increase temperature


def test_backwards_compatibility_with_strings():
    """String-based field names should still work for backwards compatibility."""
    from tidebreak import PyUniverse

    universe = PyUniverse(width=100.0, height=100.0, depth=50.0)
    universe.stamp_explosion(center=(0.0, 0.0, 0.0), radius=10.0)
    result = universe.query_volume(center=(0.0, 0.0, 0.0), radius=15.0)

    # Should work with strings (backwards compatible)
    temp_str = result.mean("temperature")
    noise_str = result.max("noise")

    assert temp_str > 0
    assert noise_str >= 0


def test_enum_and_string_produce_same_results():
    """Using enum or string for same field should produce identical results."""
    from tidebreak import Field, PyUniverse

    universe = PyUniverse(width=100.0, height=100.0, depth=50.0)
    universe.stamp_explosion(center=(0.0, 0.0, 0.0), radius=10.0)
    result = universe.query_volume(center=(0.0, 0.0, 0.0), radius=15.0)

    # Mean values should match
    assert result.mean(Field.TEMPERATURE) == result.mean("temperature")
    assert result.mean(Field.NOISE) == result.mean("noise")
    assert result.mean(Field.SMOKE) == result.mean("smoke")

    # Max values should match
    assert result.max(Field.TEMPERATURE) == result.max("temperature")

    # Min values should match
    assert result.min(Field.TEMPERATURE) == result.min("temperature")

    # Variance should match
    assert result.variance(Field.TEMPERATURE) == result.variance("temperature")


def test_point_query_enum_and_string_same_results():
    """Point query should produce same results with enum or string."""
    from tidebreak import Field, PyUniverse

    universe = PyUniverse(width=100.0, height=100.0, depth=50.0)
    universe.stamp_explosion(center=(0.0, 0.0, 0.0), radius=10.0)
    result = universe.query_point(position=(0.0, 0.0, 0.0))

    assert result.get(Field.TEMPERATURE) == result.get("temperature")
    assert result.get(Field.NOISE) == result.get("noise")
