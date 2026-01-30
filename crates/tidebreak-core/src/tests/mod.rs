//! Test module for determinism and integration tests.
//!
//! This module provides comprehensive tests for the Entity-Plugin-Resolver system:
//! - **Determinism tests**: Verify same seed produces identical results
//! - **Integration tests**: Test the full simulation pipeline
//! - **Helper functions**: Utilities for test setup
//!
//! # Test Structure
//!
//! - `determinism.rs`: Tests that verify deterministic execution
//! - `integration.rs`: End-to-end tests of the simulation
//! - `helpers.rs`: Test setup utilities and factory functions

mod determinism;
mod helpers;
mod integration;

// Re-export for convenience
pub use helpers::*;
