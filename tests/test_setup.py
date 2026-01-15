"""Verify package is importable."""

import tidebreak


def test_version() -> None:
    """Package has a version."""
    assert tidebreak.__version__ == "0.1.0"
