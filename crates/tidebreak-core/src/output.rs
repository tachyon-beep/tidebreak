//! Output system for the Entity-Plugin-Resolver architecture.
//!
//! This module provides the output types that plugins emit during the execution loop.
//! Outputs are proposals for state changes that are collected and resolved by the
//! resolution phase.
//!
//! # Architecture
//!
//! The output system uses a nested enum hierarchy for categorical routing:
//! - [`Command`]: Direct state change requests (`SetVelocity`, `FireWeapon`, etc.)
//! - [`Modifier`]: Value modifications (`ApplyDamage`, `ModifyStat`, etc.)
//! - [`Event`]: Notifications of things that happened (`WeaponFired`, `DamageDealt`, etc.)
//!
//! All outputs are wrapped in [`OutputEnvelope`] which provides causal chain metadata
//! for debugging, replay, and traceability.
//!
//! # Causal Chains
//!
//! Each output envelope includes:
//! - `source`: The plugin instance that emitted the output
//! - `cause`: Optional ID of the event that triggered this output
//! - `trace_id`: Unique ID for tracing related outputs across ticks
//!
//! # Example
//!
//! ```
//! use tidebreak_core::output::{
//!     Output, Command, OutputEnvelope, PluginInstanceId, PluginId, TraceId,
//! };
//! use tidebreak_core::entity::EntityId;
//! use glam::Vec2;
//!
//! // A plugin emits a command to set velocity
//! let command = Command::SetVelocity {
//!     target: EntityId::new(1),
//!     velocity: Vec2::new(10.0, 0.0),
//! };
//!
//! // Wrap it in an envelope with causal chain metadata
//! let envelope = OutputEnvelope::new(
//!     Output::Command(command),
//!     PluginInstanceId::new(EntityId::new(1), PluginId::new("movement")),
//!     TraceId::new(42),
//!     100, // tick
//!     0,   // sequence
//! );
//!
//! assert!(matches!(envelope.output(), Output::Command(_)));
//! ```

use glam::Vec2;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::entity::components::{StatId, StatusFlags, TrackQuality};
use crate::entity::EntityId;

// =============================================================================
// Plugin Identification Types
// =============================================================================

/// Unique identifier for a plugin type.
///
/// `PluginId` wraps a string that identifies a plugin by its registered name.
/// Plugin IDs can be created from static strings at compile time.
///
/// # Example
///
/// ```
/// use tidebreak_core::output::PluginId;
///
/// let movement_plugin = PluginId::new("movement");
/// let weapon_plugin = PluginId::new("weapon_control");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PluginId(String);

impl PluginId {
    /// Creates a new `PluginId` from a string.
    #[must_use]
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }

    /// Returns the plugin ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PluginId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for PluginId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for PluginId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Identifies a specific plugin instance (entity + plugin type).
///
/// A plugin instance is the combination of an entity and a plugin type.
/// For example, Ship #42's `MovementPlugin` is a distinct instance from
/// Ship #43's `MovementPlugin`.
///
/// # Example
///
/// ```
/// use tidebreak_core::output::{PluginInstanceId, PluginId};
/// use tidebreak_core::entity::EntityId;
///
/// let instance = PluginInstanceId::new(
///     EntityId::new(42),
///     PluginId::new("movement"),
/// );
///
/// assert_eq!(instance.entity_id(), EntityId::new(42));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PluginInstanceId {
    entity_id: EntityId,
    plugin_id: PluginId,
}

impl PluginInstanceId {
    /// Creates a new plugin instance identifier.
    #[must_use]
    pub fn new(entity_id: EntityId, plugin_id: PluginId) -> Self {
        Self {
            entity_id,
            plugin_id,
        }
    }

    /// Returns the entity ID of this instance.
    #[must_use]
    pub const fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    /// Returns the plugin ID of this instance.
    #[must_use]
    pub fn plugin_id(&self) -> &PluginId {
        &self.plugin_id
    }
}

impl fmt::Display for PluginInstanceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.plugin_id, self.entity_id)
    }
}

// =============================================================================
// Tracing Types
// =============================================================================

/// Unique identifier for tracing related outputs across ticks.
///
/// `TraceId` groups related outputs together for debugging and analysis.
/// For example, a player command might trigger multiple outputs across
/// several ticks, all sharing the same trace ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceId(u64);

impl TraceId {
    /// Creates a new trace ID.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Returns the raw value of this trace ID.
    #[must_use]
    pub const fn as_u64(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "trace:{}", self.0)
    }
}

impl From<u64> for TraceId {
    fn from(id: u64) -> Self {
        Self::new(id)
    }
}

impl From<TraceId> for u64 {
    fn from(id: TraceId) -> Self {
        id.0
    }
}

/// Unique identifier for an event.
///
/// `EventId` identifies a specific event output for causal chain tracking.
/// When an output is caused by a previous event, the cause field references
/// that event's ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(u64);

impl EventId {
    /// Creates a new event ID.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Returns the raw value of this event ID.
    #[must_use]
    pub const fn as_u64(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "event:{}", self.0)
    }
}

impl From<u64> for EventId {
    fn from(id: u64) -> Self {
        Self::new(id)
    }
}

impl From<EventId> for u64 {
    fn from(id: EventId) -> Self {
        id.0
    }
}

// =============================================================================
// Output Categories
// =============================================================================

/// Command outputs request direct state changes.
///
/// Commands are proposals to change entity state. They may be validated and
/// potentially rejected by resolvers (e.g., if the target is destroyed or
/// the action is invalid).
///
/// # Variants
///
/// - `SetVelocity`: Change an entity's velocity vector
/// - `SetHeading`: Change an entity's heading angle
/// - `FireWeapon`: Fire a weapon at a target entity
/// - `SpawnProjectile`: Create a new projectile entity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Command {
    /// Set the velocity of an entity.
    SetVelocity {
        /// Entity to modify
        target: EntityId,
        /// New velocity vector (m/s)
        velocity: Vec2,
    },
    /// Set the heading of an entity.
    SetHeading {
        /// Entity to modify
        target: EntityId,
        /// New heading in radians (counter-clockwise from +X)
        heading: f32,
    },
    /// Fire a weapon at a target.
    FireWeapon {
        /// Entity firing the weapon
        source: EntityId,
        /// Entity being targeted
        target: EntityId,
        /// Weapon slot index
        slot: usize,
    },
    /// Spawn a projectile from a weapon.
    SpawnProjectile {
        /// Entity spawning the projectile
        source: EntityId,
        /// Weapon slot being fired
        weapon_slot: usize,
        /// Target position for the projectile
        target_pos: Vec2,
    },
}

impl Command {
    /// Returns the target entity for this command, if applicable.
    #[must_use]
    pub const fn target(&self) -> Option<EntityId> {
        match self {
            Self::SetVelocity { target, .. }
            | Self::SetHeading { target, .. }
            | Self::FireWeapon { target, .. } => Some(*target),
            Self::SpawnProjectile { .. } => None,
        }
    }

    /// Returns the source entity for this command, if applicable.
    #[must_use]
    pub const fn source(&self) -> Option<EntityId> {
        match self {
            Self::FireWeapon { source, .. } | Self::SpawnProjectile { source, .. } => Some(*source),
            Self::SetVelocity { target, .. } | Self::SetHeading { target, .. } => Some(*target),
        }
    }
}

/// Modifier outputs request value changes to entity state.
///
/// Modifiers are more targeted than commands - they modify specific values
/// rather than trigger complex behaviors. Multiple modifiers to the same
/// target may be combined by resolvers.
///
/// # Variants
///
/// - `ApplyDamage`: Reduce an entity's HP
/// - `ApplyHealing`: Increase an entity's HP
/// - `SetStatusFlag`: Enable or disable a status flag
/// - `ModifyStat`: Add a delta to a stat value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Modifier {
    /// Apply damage to an entity.
    ApplyDamage {
        /// Entity to damage
        target: EntityId,
        /// Damage amount (positive value)
        amount: f32,
    },
    /// Apply healing to an entity.
    ApplyHealing {
        /// Entity to heal
        target: EntityId,
        /// Healing amount (positive value)
        amount: f32,
    },
    /// Set a status flag on an entity.
    SetStatusFlag {
        /// Entity to modify
        target: EntityId,
        /// Flag to set or clear
        flag: StatusFlags,
        /// True to set the flag, false to clear it
        value: bool,
    },
    /// Modify a stat value by a delta.
    ModifyStat {
        /// Entity to modify
        target: EntityId,
        /// Stat to modify
        stat: StatId,
        /// Delta to add (can be negative)
        delta: f32,
    },
}

impl Modifier {
    /// Returns the target entity for this modifier.
    #[must_use]
    pub const fn target(&self) -> EntityId {
        match self {
            Self::ApplyDamage { target, .. }
            | Self::ApplyHealing { target, .. }
            | Self::SetStatusFlag { target, .. }
            | Self::ModifyStat { target, .. } => *target,
        }
    }
}

/// Event outputs notify of things that happened.
///
/// Events are informational outputs that don't directly change state but
/// inform other systems about what occurred. They form the basis of causal
/// chains - other outputs can reference events as their cause.
///
/// # Variants
///
/// - `WeaponFired`: A weapon discharged
/// - `DamageDealt`: Damage was applied to an entity
/// - `EntityDestroyed`: An entity was destroyed
/// - `ContactDetected`: A sensor detected a contact
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Event {
    /// A weapon was fired.
    WeaponFired {
        /// Entity that fired
        source: EntityId,
        /// Weapon slot that fired
        weapon_slot: usize,
    },
    /// Damage was dealt to an entity.
    DamageDealt {
        /// Entity that caused the damage
        source: EntityId,
        /// Entity that received the damage
        target: EntityId,
        /// Amount of damage dealt
        amount: f32,
    },
    /// An entity was destroyed.
    EntityDestroyed {
        /// Entity that was destroyed
        entity: EntityId,
        /// Entity that destroyed it (if known)
        destroyer: Option<EntityId>,
    },
    /// A contact was detected by sensors.
    ContactDetected {
        /// Entity that detected the contact
        observer: EntityId,
        /// Entity that was detected
        target: EntityId,
        /// Quality of the detection
        quality: TrackQuality,
    },
}

impl Event {
    /// Returns the primary entity involved in this event.
    #[must_use]
    pub const fn primary_entity(&self) -> EntityId {
        match self {
            Self::WeaponFired { source, .. } => *source,
            Self::DamageDealt { target, .. } => *target,
            Self::EntityDestroyed { entity, .. } => *entity,
            Self::ContactDetected { observer, .. } => *observer,
        }
    }
}

// =============================================================================
// Top-Level Output Enum
// =============================================================================

/// Output kind for resolver routing.
///
/// Used to quickly categorize outputs for routing to the appropriate resolver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OutputKind {
    /// Command outputs (state change requests)
    Command,
    /// Modifier outputs (value modifications)
    Modifier,
    /// Event outputs (notifications)
    Event,
}

impl fmt::Display for OutputKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Command => write!(f, "Command"),
            Self::Modifier => write!(f, "Modifier"),
            Self::Event => write!(f, "Event"),
        }
    }
}

/// A plugin output - a proposal for state change or notification.
///
/// `Output` is the top-level enum containing all output categories.
/// Use the `kind()` method to get the `OutputKind` for routing.
///
/// # Example
///
/// ```
/// use tidebreak_core::output::{Output, Command, OutputKind};
/// use tidebreak_core::entity::EntityId;
/// use glam::Vec2;
///
/// let output = Output::Command(Command::SetVelocity {
///     target: EntityId::new(1),
///     velocity: Vec2::new(10.0, 5.0),
/// });
///
/// assert_eq!(output.kind(), OutputKind::Command);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Output {
    /// A command output (state change request)
    Command(Command),
    /// A modifier output (value modification)
    Modifier(Modifier),
    /// An event output (notification)
    Event(Event),
}

impl Output {
    /// Returns the kind of this output for resolver routing.
    #[must_use]
    pub const fn kind(&self) -> OutputKind {
        match self {
            Self::Command(_) => OutputKind::Command,
            Self::Modifier(_) => OutputKind::Modifier,
            Self::Event(_) => OutputKind::Event,
        }
    }

    /// Returns `true` if this is a command output.
    #[must_use]
    pub const fn is_command(&self) -> bool {
        matches!(self, Self::Command(_))
    }

    /// Returns `true` if this is a modifier output.
    #[must_use]
    pub const fn is_modifier(&self) -> bool {
        matches!(self, Self::Modifier(_))
    }

    /// Returns `true` if this is an event output.
    #[must_use]
    pub const fn is_event(&self) -> bool {
        matches!(self, Self::Event(_))
    }

    /// Returns the command if this is a command output.
    #[must_use]
    pub const fn as_command(&self) -> Option<&Command> {
        match self {
            Self::Command(cmd) => Some(cmd),
            _ => None,
        }
    }

    /// Returns the modifier if this is a modifier output.
    #[must_use]
    pub const fn as_modifier(&self) -> Option<&Modifier> {
        match self {
            Self::Modifier(m) => Some(m),
            _ => None,
        }
    }

    /// Returns the event if this is an event output.
    #[must_use]
    pub const fn as_event(&self) -> Option<&Event> {
        match self {
            Self::Event(e) => Some(e),
            _ => None,
        }
    }
}

impl From<Command> for Output {
    fn from(cmd: Command) -> Self {
        Self::Command(cmd)
    }
}

impl From<Modifier> for Output {
    fn from(m: Modifier) -> Self {
        Self::Modifier(m)
    }
}

impl From<Event> for Output {
    fn from(e: Event) -> Self {
        Self::Event(e)
    }
}

// =============================================================================
// Output Envelope
// =============================================================================

/// Wrapper for outputs with causal chain metadata.
///
/// `OutputEnvelope` wraps an `Output` with metadata for traceability:
/// - `source`: Which plugin instance emitted this output
/// - `cause`: The event that triggered this output (for causal chains)
/// - `trace_id`: Groups related outputs for debugging
/// - `tick`: When this output was emitted
/// - `sequence`: Ordering within the same tick/source
///
/// # Causal Chains
///
/// Outputs can form causal chains through the `cause` field. For example:
/// 1. `WeaponPlugin` emits `FireWeapon` command
/// 2. Resolver processes and emits `WeaponFired` event (cause: None)
/// 3. `DamagePlugin` sees the event, emits `ApplyDamage` modifier (cause: `WeaponFired` event ID)
/// 4. Resolver processes and emits `DamageDealt` event (cause: `ApplyDamage`'s `EventId`)
///
/// # Example
///
/// ```
/// use tidebreak_core::output::{
///     Output, Modifier, OutputEnvelope, PluginInstanceId, PluginId, TraceId, EventId,
/// };
/// use tidebreak_core::entity::EntityId;
///
/// let envelope = OutputEnvelope::new(
///     Output::Modifier(Modifier::ApplyDamage {
///         target: EntityId::new(2),
///         amount: 50.0,
///     }),
///     PluginInstanceId::new(EntityId::new(1), PluginId::new("weapon")),
///     TraceId::new(100),
///     42, // tick
///     0,  // sequence
/// ).with_cause(EventId::new(99));
///
/// assert!(envelope.cause().is_some());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputEnvelope {
    /// The output being wrapped
    output: Output,
    /// The plugin instance that emitted this output
    source: PluginInstanceId,
    /// The event that caused this output (for causal chains)
    cause: Option<EventId>,
    /// Trace ID for grouping related outputs
    trace_id: TraceId,
    /// Tick when this output was emitted
    tick: u64,
    /// Sequence number within the same tick/source
    sequence: u32,
}

impl OutputEnvelope {
    /// Creates a new output envelope.
    ///
    /// # Arguments
    ///
    /// * `output` - The output to wrap
    /// * `source` - The plugin instance that emitted this output
    /// * `trace_id` - Trace ID for grouping related outputs
    /// * `tick` - Current simulation tick
    /// * `sequence` - Sequence number within this tick for ordering
    #[must_use]
    pub fn new(
        output: Output,
        source: PluginInstanceId,
        trace_id: TraceId,
        tick: u64,
        sequence: u32,
    ) -> Self {
        Self {
            output,
            source,
            cause: None,
            trace_id,
            tick,
            sequence,
        }
    }

    /// Sets the cause event for this output.
    ///
    /// Returns a new envelope with the cause set (builder pattern).
    #[must_use]
    pub fn with_cause(mut self, cause: EventId) -> Self {
        self.cause = Some(cause);
        self
    }

    /// Returns a reference to the wrapped output.
    #[must_use]
    pub fn output(&self) -> &Output {
        &self.output
    }

    /// Consumes the envelope and returns the wrapped output.
    #[must_use]
    pub fn into_output(self) -> Output {
        self.output
    }

    /// Returns the source plugin instance.
    #[must_use]
    pub fn source(&self) -> &PluginInstanceId {
        &self.source
    }

    /// Returns the cause event ID, if any.
    #[must_use]
    pub const fn cause(&self) -> Option<EventId> {
        self.cause
    }

    /// Returns the trace ID.
    #[must_use]
    pub const fn trace_id(&self) -> TraceId {
        self.trace_id
    }

    /// Returns the tick when this output was emitted.
    #[must_use]
    pub const fn tick(&self) -> u64 {
        self.tick
    }

    /// Returns the sequence number within the tick.
    #[must_use]
    pub const fn sequence(&self) -> u32 {
        self.sequence
    }

    /// Returns the kind of the wrapped output.
    #[must_use]
    pub const fn kind(&self) -> OutputKind {
        self.output.kind()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod plugin_id_tests {
        use super::*;

        #[test]
        fn new_creates_id() {
            let id = PluginId::new("movement");
            assert_eq!(id.as_str(), "movement");
        }

        #[test]
        fn display_format() {
            let id = PluginId::new("weapon_control");
            assert_eq!(format!("{}", id), "weapon_control");
        }

        #[test]
        fn equality() {
            let id1 = PluginId::new("test");
            let id2 = PluginId::new("test");
            let id3 = PluginId::new("other");

            assert_eq!(id1, id2);
            assert_ne!(id1, id3);
        }

        #[test]
        fn hashing() {
            use std::collections::HashSet;

            let mut set = HashSet::new();
            set.insert(PluginId::new("a"));
            set.insert(PluginId::new("b"));
            set.insert(PluginId::new("a")); // Duplicate

            assert_eq!(set.len(), 2);
        }

        #[test]
        fn serialization_roundtrip() {
            let id = PluginId::new("test_plugin");
            let json = serde_json::to_string(&id).unwrap();
            let deserialized: PluginId = serde_json::from_str(&json).unwrap();
            assert_eq!(id, deserialized);
        }
    }

    mod plugin_instance_id_tests {
        use super::*;

        #[test]
        fn new_creates_instance() {
            let instance = PluginInstanceId::new(EntityId::new(42), PluginId::new("movement"));

            assert_eq!(instance.entity_id(), EntityId::new(42));
            assert_eq!(*instance.plugin_id(), PluginId::new("movement"));
        }

        #[test]
        fn display_format() {
            let instance = PluginInstanceId::new(EntityId::new(42), PluginId::new("weapon"));
            assert_eq!(format!("{}", instance), "weapon@42");
        }

        #[test]
        fn equality() {
            let i1 = PluginInstanceId::new(EntityId::new(1), PluginId::new("a"));
            let i2 = PluginInstanceId::new(EntityId::new(1), PluginId::new("a"));
            let i3 = PluginInstanceId::new(EntityId::new(2), PluginId::new("a"));
            let i4 = PluginInstanceId::new(EntityId::new(1), PluginId::new("b"));

            assert_eq!(i1, i2);
            assert_ne!(i1, i3); // Different entity
            assert_ne!(i1, i4); // Different plugin
        }

        #[test]
        fn serialization_roundtrip() {
            let instance = PluginInstanceId::new(EntityId::new(123), PluginId::new("test"));
            let json = serde_json::to_string(&instance).unwrap();
            let deserialized: PluginInstanceId = serde_json::from_str(&json).unwrap();
            assert_eq!(instance, deserialized);
        }
    }

    mod trace_id_tests {
        use super::*;

        #[test]
        fn new_creates_id() {
            let id = TraceId::new(12345);
            assert_eq!(id.as_u64(), 12345);
        }

        #[test]
        fn display_format() {
            let id = TraceId::new(42);
            assert_eq!(format!("{}", id), "trace:42");
        }

        #[test]
        fn from_u64() {
            let id: TraceId = 99u64.into();
            assert_eq!(id.as_u64(), 99);
        }

        #[test]
        fn into_u64() {
            let id = TraceId::new(42);
            let value: u64 = id.into();
            assert_eq!(value, 42);
        }

        #[test]
        fn serialization_roundtrip() {
            let id = TraceId::new(98765);
            let json = serde_json::to_string(&id).unwrap();
            let deserialized: TraceId = serde_json::from_str(&json).unwrap();
            assert_eq!(id, deserialized);
        }
    }

    mod event_id_tests {
        use super::*;

        #[test]
        fn new_creates_id() {
            let id = EventId::new(54321);
            assert_eq!(id.as_u64(), 54321);
        }

        #[test]
        fn display_format() {
            let id = EventId::new(100);
            assert_eq!(format!("{}", id), "event:100");
        }

        #[test]
        fn from_u64() {
            let id: EventId = 77u64.into();
            assert_eq!(id.as_u64(), 77);
        }

        #[test]
        fn into_u64() {
            let id = EventId::new(88);
            let value: u64 = id.into();
            assert_eq!(value, 88);
        }

        #[test]
        fn serialization_roundtrip() {
            let id = EventId::new(11111);
            let json = serde_json::to_string(&id).unwrap();
            let deserialized: EventId = serde_json::from_str(&json).unwrap();
            assert_eq!(id, deserialized);
        }
    }

    mod command_tests {
        use super::*;

        #[test]
        fn set_velocity() {
            let cmd = Command::SetVelocity {
                target: EntityId::new(1),
                velocity: Vec2::new(10.0, 5.0),
            };

            assert_eq!(cmd.target(), Some(EntityId::new(1)));
            assert_eq!(cmd.source(), Some(EntityId::new(1)));
        }

        #[test]
        fn set_heading() {
            let cmd = Command::SetHeading {
                target: EntityId::new(2),
                heading: 1.5,
            };

            assert_eq!(cmd.target(), Some(EntityId::new(2)));
        }

        #[test]
        fn fire_weapon() {
            let cmd = Command::FireWeapon {
                source: EntityId::new(1),
                target: EntityId::new(2),
                slot: 0,
            };

            assert_eq!(cmd.target(), Some(EntityId::new(2)));
            assert_eq!(cmd.source(), Some(EntityId::new(1)));
        }

        #[test]
        fn spawn_projectile() {
            let cmd = Command::SpawnProjectile {
                source: EntityId::new(1),
                weapon_slot: 0,
                target_pos: Vec2::new(1000.0, 2000.0),
            };

            assert_eq!(cmd.target(), None);
            assert_eq!(cmd.source(), Some(EntityId::new(1)));
        }

        #[test]
        fn serialization_roundtrip() {
            let cmd = Command::FireWeapon {
                source: EntityId::new(1),
                target: EntityId::new(2),
                slot: 3,
            };
            let json = serde_json::to_string(&cmd).unwrap();
            let deserialized: Command = serde_json::from_str(&json).unwrap();
            assert_eq!(cmd, deserialized);
        }
    }

    mod modifier_tests {
        use super::*;

        #[test]
        fn apply_damage() {
            let m = Modifier::ApplyDamage {
                target: EntityId::new(1),
                amount: 50.0,
            };

            assert_eq!(m.target(), EntityId::new(1));
        }

        #[test]
        fn apply_healing() {
            let m = Modifier::ApplyHealing {
                target: EntityId::new(2),
                amount: 25.0,
            };

            assert_eq!(m.target(), EntityId::new(2));
        }

        #[test]
        fn set_status_flag() {
            let m = Modifier::SetStatusFlag {
                target: EntityId::new(3),
                flag: StatusFlags::MOBILITY_DISABLED,
                value: true,
            };

            assert_eq!(m.target(), EntityId::new(3));
        }

        #[test]
        fn modify_stat() {
            let m = Modifier::ModifyStat {
                target: EntityId::new(4),
                stat: StatId::Hp,
                delta: -10.0,
            };

            assert_eq!(m.target(), EntityId::new(4));
        }

        #[test]
        fn serialization_roundtrip() {
            let m = Modifier::SetStatusFlag {
                target: EntityId::new(1),
                flag: StatusFlags::WEAPONS_DISABLED | StatusFlags::SENSORS_DISABLED,
                value: true,
            };
            let json = serde_json::to_string(&m).unwrap();
            let deserialized: Modifier = serde_json::from_str(&json).unwrap();
            assert_eq!(m, deserialized);
        }
    }

    mod event_tests {
        use super::*;

        #[test]
        fn weapon_fired() {
            let e = Event::WeaponFired {
                source: EntityId::new(1),
                weapon_slot: 0,
            };

            assert_eq!(e.primary_entity(), EntityId::new(1));
        }

        #[test]
        fn damage_dealt() {
            let e = Event::DamageDealt {
                source: EntityId::new(1),
                target: EntityId::new(2),
                amount: 100.0,
            };

            assert_eq!(e.primary_entity(), EntityId::new(2));
        }

        #[test]
        fn entity_destroyed() {
            let e = Event::EntityDestroyed {
                entity: EntityId::new(3),
                destroyer: Some(EntityId::new(1)),
            };

            assert_eq!(e.primary_entity(), EntityId::new(3));
        }

        #[test]
        fn entity_destroyed_no_destroyer() {
            let e = Event::EntityDestroyed {
                entity: EntityId::new(3),
                destroyer: None,
            };

            assert_eq!(e.primary_entity(), EntityId::new(3));
        }

        #[test]
        fn contact_detected() {
            let e = Event::ContactDetected {
                observer: EntityId::new(1),
                target: EntityId::new(2),
                quality: TrackQuality::FireControl,
            };

            assert_eq!(e.primary_entity(), EntityId::new(1));
        }

        #[test]
        fn serialization_roundtrip() {
            let e = Event::ContactDetected {
                observer: EntityId::new(1),
                target: EntityId::new(2),
                quality: TrackQuality::Shared,
            };
            let json = serde_json::to_string(&e).unwrap();
            let deserialized: Event = serde_json::from_str(&json).unwrap();
            assert_eq!(e, deserialized);
        }
    }

    mod output_tests {
        use super::*;

        #[test]
        fn kind_routing() {
            let cmd_output = Output::Command(Command::SetVelocity {
                target: EntityId::new(1),
                velocity: Vec2::ZERO,
            });
            assert_eq!(cmd_output.kind(), OutputKind::Command);

            let mod_output = Output::Modifier(Modifier::ApplyDamage {
                target: EntityId::new(1),
                amount: 10.0,
            });
            assert_eq!(mod_output.kind(), OutputKind::Modifier);

            let event_output = Output::Event(Event::WeaponFired {
                source: EntityId::new(1),
                weapon_slot: 0,
            });
            assert_eq!(event_output.kind(), OutputKind::Event);
        }

        #[test]
        fn is_type_predicates() {
            let cmd = Output::Command(Command::SetHeading {
                target: EntityId::new(1),
                heading: 0.0,
            });
            assert!(cmd.is_command());
            assert!(!cmd.is_modifier());
            assert!(!cmd.is_event());

            let m = Output::Modifier(Modifier::ApplyHealing {
                target: EntityId::new(1),
                amount: 10.0,
            });
            assert!(!m.is_command());
            assert!(m.is_modifier());
            assert!(!m.is_event());

            let e = Output::Event(Event::EntityDestroyed {
                entity: EntityId::new(1),
                destroyer: None,
            });
            assert!(!e.is_command());
            assert!(!e.is_modifier());
            assert!(e.is_event());
        }

        #[test]
        fn as_type_accessors() {
            let cmd = Output::Command(Command::SetVelocity {
                target: EntityId::new(1),
                velocity: Vec2::new(1.0, 2.0),
            });
            assert!(cmd.as_command().is_some());
            assert!(cmd.as_modifier().is_none());
            assert!(cmd.as_event().is_none());
        }

        #[test]
        fn from_command() {
            let cmd = Command::SetVelocity {
                target: EntityId::new(1),
                velocity: Vec2::ZERO,
            };
            let output: Output = cmd.into();
            assert!(output.is_command());
        }

        #[test]
        fn from_modifier() {
            let m = Modifier::ApplyDamage {
                target: EntityId::new(1),
                amount: 10.0,
            };
            let output: Output = m.into();
            assert!(output.is_modifier());
        }

        #[test]
        fn from_event() {
            let e = Event::WeaponFired {
                source: EntityId::new(1),
                weapon_slot: 0,
            };
            let output: Output = e.into();
            assert!(output.is_event());
        }

        #[test]
        fn serialization_roundtrip() {
            let outputs = vec![
                Output::Command(Command::FireWeapon {
                    source: EntityId::new(1),
                    target: EntityId::new(2),
                    slot: 0,
                }),
                Output::Modifier(Modifier::ApplyDamage {
                    target: EntityId::new(2),
                    amount: 50.0,
                }),
                Output::Event(Event::DamageDealt {
                    source: EntityId::new(1),
                    target: EntityId::new(2),
                    amount: 50.0,
                }),
            ];

            for output in outputs {
                let json = serde_json::to_string(&output).unwrap();
                let deserialized: Output = serde_json::from_str(&json).unwrap();
                assert_eq!(output, deserialized);
            }
        }
    }

    mod output_kind_tests {
        use super::*;

        #[test]
        fn all_variants_exist() {
            let _cmd = OutputKind::Command;
            let _m = OutputKind::Modifier;
            let _e = OutputKind::Event;
        }

        #[test]
        fn display_format() {
            assert_eq!(format!("{}", OutputKind::Command), "Command");
            assert_eq!(format!("{}", OutputKind::Modifier), "Modifier");
            assert_eq!(format!("{}", OutputKind::Event), "Event");
        }

        #[test]
        fn equality() {
            assert_eq!(OutputKind::Command, OutputKind::Command);
            assert_ne!(OutputKind::Command, OutputKind::Modifier);
        }

        #[test]
        fn serialization_roundtrip() {
            let kind = OutputKind::Modifier;
            let json = serde_json::to_string(&kind).unwrap();
            let deserialized: OutputKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, deserialized);
        }
    }

    mod output_envelope_tests {
        use super::*;

        fn make_test_envelope() -> OutputEnvelope {
            OutputEnvelope::new(
                Output::Command(Command::SetVelocity {
                    target: EntityId::new(1),
                    velocity: Vec2::new(10.0, 5.0),
                }),
                PluginInstanceId::new(EntityId::new(1), PluginId::new("movement")),
                TraceId::new(100),
                42,
                0,
            )
        }

        #[test]
        fn new_creates_envelope() {
            let envelope = make_test_envelope();

            assert_eq!(envelope.tick(), 42);
            assert_eq!(envelope.sequence(), 0);
            assert_eq!(envelope.trace_id(), TraceId::new(100));
            assert!(envelope.cause().is_none());
        }

        #[test]
        fn with_cause() {
            let envelope = make_test_envelope().with_cause(EventId::new(99));

            assert_eq!(envelope.cause(), Some(EventId::new(99)));
        }

        #[test]
        fn source_access() {
            let envelope = make_test_envelope();
            let source = envelope.source();

            assert_eq!(source.entity_id(), EntityId::new(1));
            assert_eq!(*source.plugin_id(), PluginId::new("movement"));
        }

        #[test]
        fn output_access() {
            let envelope = make_test_envelope();

            assert!(envelope.output().is_command());
            assert_eq!(envelope.kind(), OutputKind::Command);
        }

        #[test]
        fn into_output() {
            let envelope = make_test_envelope();
            let output = envelope.into_output();

            assert!(output.is_command());
        }

        #[test]
        fn serialization_roundtrip() {
            let envelope = OutputEnvelope::new(
                Output::Modifier(Modifier::ApplyDamage {
                    target: EntityId::new(2),
                    amount: 50.0,
                }),
                PluginInstanceId::new(EntityId::new(1), PluginId::new("weapon")),
                TraceId::new(200),
                100,
                5,
            )
            .with_cause(EventId::new(50));

            let json = serde_json::to_string(&envelope).unwrap();
            let deserialized: OutputEnvelope = serde_json::from_str(&json).unwrap();

            assert_eq!(envelope.tick(), deserialized.tick());
            assert_eq!(envelope.sequence(), deserialized.sequence());
            assert_eq!(envelope.trace_id(), deserialized.trace_id());
            assert_eq!(envelope.cause(), deserialized.cause());
            assert_eq!(envelope.source(), deserialized.source());
            assert_eq!(envelope.kind(), deserialized.kind());
        }

        #[test]
        fn causal_chain_example() {
            // Simulate a causal chain: FireWeapon -> WeaponFired -> ApplyDamage -> DamageDealt

            let fire_cmd = OutputEnvelope::new(
                Output::Command(Command::FireWeapon {
                    source: EntityId::new(1),
                    target: EntityId::new(2),
                    slot: 0,
                }),
                PluginInstanceId::new(EntityId::new(1), PluginId::new("weapon_control")),
                TraceId::new(1),
                100,
                0,
            );

            // Event emitted by resolver (no cause - it's the root)
            let weapon_fired = OutputEnvelope::new(
                Output::Event(Event::WeaponFired {
                    source: EntityId::new(1),
                    weapon_slot: 0,
                }),
                PluginInstanceId::new(EntityId::new(0), PluginId::new("resolver")),
                TraceId::new(1),
                100,
                1,
            );

            // Damage modifier caused by weapon fired event
            let apply_damage = OutputEnvelope::new(
                Output::Modifier(Modifier::ApplyDamage {
                    target: EntityId::new(2),
                    amount: 50.0,
                }),
                PluginInstanceId::new(EntityId::new(1), PluginId::new("damage_calc")),
                TraceId::new(1),
                100,
                2,
            )
            .with_cause(EventId::new(1)); // WeaponFired event ID

            // All share the same trace ID
            assert_eq!(fire_cmd.trace_id(), weapon_fired.trace_id());
            assert_eq!(fire_cmd.trace_id(), apply_damage.trace_id());

            // Causal chain is established
            assert!(fire_cmd.cause().is_none()); // Root command
            assert!(weapon_fired.cause().is_none()); // Root event
            assert_eq!(apply_damage.cause(), Some(EventId::new(1)));
        }
    }
}
