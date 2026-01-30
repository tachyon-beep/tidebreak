//! Resolver module for the Entity-Plugin-Resolver architecture.
//!
//! Resolvers process plugin outputs and mutate the next state. They are the final
//! phase of the execution loop where proposed changes become actual state mutations.
//!
//! # Architecture
//!
//! Each resolver declares which output kinds it handles via [`Resolver::handles()`].
//! During resolution:
//! 1. Outputs are collected from all plugins
//! 2. Outputs are routed to resolvers based on their kind
//! 3. Each resolver processes its outputs and mutates `NextState`
//!
//! # Invariants
//!
//! - Resolvers MUST NOT read from `next` (use `current` for lookups)
//! - Resolvers MUST be deterministic given the same inputs and output order
//! - Resolvers should process outputs in a consistent order for determinism
//!
//! # Available Resolvers
//!
//! - [`PhysicsResolver`]: Handles movement commands and physics integration
//! - [`CombatResolver`]: Handles damage, healing, and status effects
//! - [`EventResolver`]: Records events for telemetry (no state mutation)

mod combat;
mod event;
mod physics;

pub use combat::CombatResolver;
pub use event::EventResolver;
pub use physics::PhysicsResolver;

use crate::arena::Arena;
use crate::output::{OutputEnvelope, OutputKind};

/// Resolver processes outputs and mutates `NextState`.
///
/// Resolvers are the write phase of the Entity-Plugin-Resolver architecture.
/// They receive collected outputs from plugins and apply the appropriate
/// state mutations to the next frame's state.
///
/// # Implementation Guidelines
///
/// 1. **Determinism**: Given the same inputs and output order, a resolver must
///    produce identical results. Use deterministic iteration order and avoid
///    floating-point operations that vary by platform.
///
/// 2. **Read from current, write to next**: The `current` arena provides the
///    authoritative state for lookups. The `next` arena is where mutations
///    should be written. Never read from `next`.
///
/// 3. **Handle conflicts**: When multiple outputs target the same state, the
///    resolver must decide how to combine them (e.g., sum damage, last-write-wins
///    for position).
///
/// # Example
///
/// ```
/// use tidebreak_core::resolver::Resolver;
/// use tidebreak_core::output::{OutputKind, OutputEnvelope};
/// use tidebreak_core::arena::Arena;
///
/// struct MyResolver;
///
/// impl Resolver for MyResolver {
///     fn handles(&self) -> &[OutputKind] {
///         &[OutputKind::Command]
///     }
///
///     fn resolve(
///         &self,
///         outputs: &[&OutputEnvelope],
///         current: &Arena,
///         next: &mut Arena,
///     ) {
///         // Process outputs and mutate next
///     }
/// }
/// ```
pub trait Resolver: Send + Sync {
    /// Returns the output kinds this resolver handles.
    ///
    /// The execution loop uses this to route outputs to the appropriate resolver.
    /// A resolver may handle multiple output kinds.
    fn handles(&self) -> &[OutputKind];

    /// Resolves outputs into state mutations.
    ///
    /// # Arguments
    ///
    /// * `outputs` - The outputs routed to this resolver (filtered by `handles()`)
    /// * `current` - The current frame's state (read-only reference for lookups)
    /// * `next` - The next frame's state (mutate this)
    ///
    /// # Invariants
    ///
    /// - Only mutate `next`, never read from it (use `current` for lookups)
    /// - Must be deterministic given the same inputs + output order
    fn resolve(&self, outputs: &[&OutputEnvelope], current: &Arena, next: &mut Arena);
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test that the trait is object-safe
    #[test]
    fn resolver_is_object_safe() {
        fn _accepts_boxed(_resolver: Box<dyn Resolver>) {}
        fn _accepts_slice(_resolvers: &[Box<dyn Resolver>]) {}
    }
}
