"""Tidebreak: Naval combat with DRL agents.

This module provides the Python bindings for the Tidebreak naval strategy game,
including the Murk spatial substrate and Gymnasium environments for DRL training.
"""

from __future__ import annotations

# Import from the compiled Rust extension
from tidebreak._tidebreak import (
    # Murk bindings (existing)
    Field,
    PyCombatState,
    PyEntity,
    # Tidebreak-core bindings (new)
    PyEntityId,
    PyEntityTag,
    PyObservation,
    PyPhysicsState,
    PyPointResult,
    PyQueryResult,
    PySimulation,
    PyTransformState,
    PyUniverse,
)

# Aliases for convenience
Universe = PyUniverse
Simulation = PySimulation
EntityId = PyEntityId
EntityTag = PyEntityTag
Entity = PyEntity

__all__ = [
    # Murk types
    "Field",
    "PyPointResult",
    "PyQueryResult",
    "PyUniverse",
    "Universe",
    # Entity types
    "PyEntityId",
    "PyEntityTag",
    "EntityId",
    "EntityTag",
    # Component types
    "PyTransformState",
    "PyPhysicsState",
    "PyCombatState",
    # Entity wrapper
    "PyEntity",
    "Entity",
    # Simulation
    "PySimulation",
    "Simulation",
    # DRL
    "PyObservation",
    # Envs submodule
    "envs",
]
