"""Tidebreak: Naval combat with DRL agents.

This module provides the Python bindings for the Tidebreak naval strategy game,
including the Murk spatial substrate and Gymnasium environments for DRL training.
"""

from __future__ import annotations

# Import from the compiled Rust extension
from tidebreak._tidebreak import Field, PyPointResult, PyQueryResult, PyUniverse

# Convenience aliases
Universe = PyUniverse
PointResult = PyPointResult
QueryResult = PyQueryResult

__all__ = [
    "Universe",
    "PyUniverse",
    "PointResult",
    "PyPointResult",
    "QueryResult",
    "PyQueryResult",
    "Field",
]
