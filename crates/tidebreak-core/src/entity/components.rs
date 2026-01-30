//! State components for entity types.
//!
//! This module provides the core state components that make up entities in the
//! Tidebreak combat simulation:
//!
//! - [`TransformState`]: Position and heading
//! - [`PhysicsState`]: Velocity and movement limits
//! - [`CombatState`]: Health, weapons, and status
//! - [`SensorState`]: Detection capabilities and track table
//! - [`InventoryState`]: Fuel and ammunition
//!
//! Composite structs group these components by entity type:
//! - [`ShipComponents`]: All components (transform, physics, combat, sensor, inventory)
//! - [`PlatformComponents`]: Transform and sensor (stationary installations)
//! - [`ProjectileComponents`]: Transform and physics (in-flight weapons)
//! - [`SquadronComponents`]: Transform, physics, and combat (grouped aircraft)
//!
//! Access traits provide uniform access to components across different entity types:
//! - [`HasTransform`], [`HasPhysics`], [`HasCombat`], [`HasSensor`], [`HasInventory`]

use std::collections::BTreeMap;

use bitflags::bitflags;
use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::entity::EntityId;

// =============================================================================
// Supporting Types
// =============================================================================

/// Ammunition types for weapons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AmmoType {
    /// Standard kinetic rounds (cannons, autocannons)
    Bullet,
    /// Guided surface-to-surface or surface-to-air missiles
    Missile,
    /// Underwater guided weapons
    Torpedo,
    /// Unguided artillery shells
    Shell,
    /// Anti-submarine depth charges
    DepthCharge,
    /// Countermeasure flares and chaff
    Countermeasure,
}

/// Emissions mode for sensor systems.
///
/// Controls the tradeoff between detection capability and signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum EmissionsMode {
    /// No active emissions - relies on passive sensors only.
    /// Minimizes own signature but severely limits detection.
    Silent,
    /// Passive sensors only - no active radar/sonar.
    /// Low signature but limited detection capability.
    #[default]
    Passive,
    /// Active radar and sonar - full detection capability.
    /// High signature - easily detected by ESM.
    Active,
}

/// Track quality levels per sensor design.
///
/// Quality determines what actions can be taken on a track.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub enum TrackQuality {
    /// Q0: "Something exists" - bearing-only cue.
    /// Unlocks: Investigation
    #[default]
    Cue,
    /// Q1: Usable for maneuvering.
    /// Unlocks: Navigation, patrol
    Coarse,
    /// Q2: Engageable by own weapons.
    /// Unlocks: Local firing solutions
    FireControl,
    /// Q3: Engageable via shared data.
    /// Unlocks: Remote engagement, cooperative targeting
    Shared,
}

/// A sensor track representing a detected entity.
///
/// Tracks are fused, time-evolving estimates with uncertainty.
/// They represent what a ship believes about a contact, not ground truth.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    /// Entity ID of the tracked target (may be incorrect if misidentified)
    pub target_id: EntityId,
    /// Estimated position
    pub position: Vec2,
    /// Estimated velocity (if known)
    pub velocity: Option<Vec2>,
    /// Track quality level
    pub quality: TrackQuality,
    /// Seconds since last sensor update
    pub age: f32,
    /// Classification confidence (0.0-1.0)
    pub classification_confidence: f32,
}

impl Track {
    /// Creates a new track with the given parameters.
    #[must_use]
    pub fn new(target_id: EntityId, position: Vec2, quality: TrackQuality) -> Self {
        Self {
            target_id,
            position,
            velocity: None,
            quality,
            age: 0.0,
            classification_confidence: 0.0,
        }
    }
}

impl Default for Track {
    fn default() -> Self {
        Self {
            target_id: EntityId::new(0),
            position: Vec2::ZERO,
            velocity: None,
            quality: TrackQuality::default(),
            age: 0.0,
            classification_confidence: 0.0,
        }
    }
}

/// Weapon state for a single weapon slot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeaponState {
    /// Weapon slot index
    pub slot: usize,
    /// Current cooldown remaining (seconds)
    pub cooldown: f32,
    /// Maximum cooldown between shots (seconds)
    pub max_cooldown: f32,
    /// Type of ammunition this weapon uses
    pub ammo_type: AmmoType,
    /// Whether this weapon is operational
    pub operational: bool,
}

impl WeaponState {
    /// Creates a new weapon state.
    #[must_use]
    pub fn new(slot: usize, max_cooldown: f32, ammo_type: AmmoType) -> Self {
        Self {
            slot,
            cooldown: 0.0,
            max_cooldown,
            ammo_type,
            operational: true,
        }
    }

    /// Returns true if the weapon is ready to fire.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.operational && self.cooldown <= 0.0
    }
}

impl Default for WeaponState {
    fn default() -> Self {
        Self {
            slot: 0,
            cooldown: 0.0,
            max_cooldown: 1.0,
            ammo_type: AmmoType::Bullet,
            operational: true,
        }
    }
}

/// Stat identifiers for the effect system.
///
/// Used by `ApplyModifier` outputs to target specific stats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatId {
    // Transform stats
    /// Position X coordinate
    PositionX,
    /// Position Y coordinate
    PositionY,
    /// Heading in radians
    Heading,

    // Physics stats
    /// Velocity X component
    VelocityX,
    /// Velocity Y component
    VelocityY,
    /// Angular velocity
    AngularVelocity,
    /// Maximum speed
    MaxSpeed,
    /// Maximum turn rate
    MaxTurnRate,

    // Combat stats
    /// Current hit points
    Hp,
    /// Maximum hit points
    MaxHp,

    // Sensor stats
    /// Radar detection range
    RadarRange,
    /// Sonar detection range
    SonarRange,

    // Inventory stats
    /// Fuel amount
    Fuel,
}

// =============================================================================
// Status Flags
// =============================================================================

bitflags! {
    /// Status flags indicating various disabled or special states.
    ///
    /// These flags are used by the damage system (Tier 0) and can be set
    /// by critical hits without requiring a full component damage model.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
    pub struct StatusFlags: u32 {
        /// Propulsion is disabled - cannot move
        const MOBILITY_DISABLED = 0b0000_0001;
        /// Weapons are disabled - cannot fire
        const WEAPONS_DISABLED = 0b0000_0010;
        /// Sensors are disabled - cannot detect
        const SENSORS_DISABLED = 0b0000_0100;
        /// Entity is destroyed - pending removal
        const DESTROYED = 0b0000_1000;
        /// Entity is on fire - takes damage over time
        const ON_FIRE = 0b0001_0000;
        /// Entity is flooding - affects buoyancy/depth
        const FLOODING = 0b0010_0000;
        /// Entity has surrendered
        const SURRENDERED = 0b0100_0000;
    }
}

// =============================================================================
// Core State Components
// =============================================================================

/// Transform state - position and orientation.
///
/// Uses 2D coordinates with heading in radians (counter-clockwise from +X axis).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TransformState {
    /// Position in world coordinates (meters)
    pub position: Vec2,
    /// Heading in radians (counter-clockwise from +X axis)
    pub heading: f32,
}

impl TransformState {
    /// Creates a new transform state at the given position and heading.
    #[must_use]
    pub fn new(position: Vec2, heading: f32) -> Self {
        Self { position, heading }
    }

    /// Returns the forward direction vector based on the current heading.
    #[must_use]
    pub fn forward(&self) -> Vec2 {
        Vec2::new(self.heading.cos(), self.heading.sin())
    }

    /// Returns the right direction vector (perpendicular to forward).
    #[must_use]
    pub fn right(&self) -> Vec2 {
        Vec2::new(self.heading.sin(), -self.heading.cos())
    }
}

impl Default for TransformState {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            heading: 0.0,
        }
    }
}

/// Physics state - velocity and movement constraints.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PhysicsState {
    /// Current velocity in m/s
    pub velocity: Vec2,
    /// Current angular velocity in rad/s
    pub angular_velocity: f32,
    /// Maximum speed in m/s
    pub max_speed: f32,
    /// Maximum turn rate in rad/s
    pub max_turn_rate: f32,
}

impl PhysicsState {
    /// Creates a new physics state with the given limits.
    #[must_use]
    pub fn new(max_speed: f32, max_turn_rate: f32) -> Self {
        Self {
            velocity: Vec2::ZERO,
            angular_velocity: 0.0,
            max_speed,
            max_turn_rate,
        }
    }

    /// Returns the current speed (magnitude of velocity).
    #[must_use]
    pub fn speed(&self) -> f32 {
        self.velocity.length()
    }

    /// Returns true if the entity is stationary (very low speed).
    #[must_use]
    pub fn is_stationary(&self) -> bool {
        self.velocity.length_squared() < 0.01
    }
}

impl Default for PhysicsState {
    fn default() -> Self {
        Self {
            velocity: Vec2::ZERO,
            angular_velocity: 0.0,
            max_speed: 10.0,
            max_turn_rate: 1.0,
        }
    }
}

/// Combat state - health, weapons, and status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CombatState {
    /// Current hit points
    pub hp: f32,
    /// Maximum hit points
    pub max_hp: f32,
    /// Weapon states by slot
    pub weapons: Vec<WeaponState>,
    /// Status flags (disabled systems, destroyed, etc.)
    pub status_flags: StatusFlags,
}

impl CombatState {
    /// Creates a new combat state with the given max HP.
    #[must_use]
    pub fn new(max_hp: f32) -> Self {
        Self {
            hp: max_hp,
            max_hp,
            weapons: Vec::new(),
            status_flags: StatusFlags::empty(),
        }
    }

    /// Creates a combat state with weapons.
    #[must_use]
    pub fn with_weapons(max_hp: f32, weapons: Vec<WeaponState>) -> Self {
        Self {
            hp: max_hp,
            max_hp,
            weapons,
            status_flags: StatusFlags::empty(),
        }
    }

    /// Returns the health percentage (0.0-1.0).
    #[must_use]
    pub fn health_percent(&self) -> f32 {
        if self.max_hp > 0.0 {
            self.hp / self.max_hp
        } else {
            0.0
        }
    }

    /// Returns true if the entity is destroyed.
    #[must_use]
    pub fn is_destroyed(&self) -> bool {
        self.status_flags.contains(StatusFlags::DESTROYED) || self.hp <= 0.0
    }

    /// Returns true if mobility is disabled.
    #[must_use]
    pub fn is_mobility_disabled(&self) -> bool {
        self.status_flags.contains(StatusFlags::MOBILITY_DISABLED)
    }

    /// Returns true if weapons are disabled.
    #[must_use]
    pub fn are_weapons_disabled(&self) -> bool {
        self.status_flags.contains(StatusFlags::WEAPONS_DISABLED)
    }

    /// Returns true if sensors are disabled.
    #[must_use]
    pub fn are_sensors_disabled(&self) -> bool {
        self.status_flags.contains(StatusFlags::SENSORS_DISABLED)
    }

    /// Returns a weapon by slot index.
    #[must_use]
    pub fn get_weapon(&self, slot: usize) -> Option<&WeaponState> {
        self.weapons.iter().find(|w| w.slot == slot)
    }

    /// Returns a mutable weapon by slot index.
    #[must_use]
    pub fn get_weapon_mut(&mut self, slot: usize) -> Option<&mut WeaponState> {
        self.weapons.iter_mut().find(|w| w.slot == slot)
    }
}

impl Default for CombatState {
    fn default() -> Self {
        Self {
            hp: 100.0,
            max_hp: 100.0,
            weapons: Vec::new(),
            status_flags: StatusFlags::empty(),
        }
    }
}

/// Sensor state - detection capabilities and track table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensorState {
    /// Maximum radar detection range (meters)
    pub radar_range: f32,
    /// Maximum sonar detection range (meters)
    pub sonar_range: f32,
    /// Current emissions mode
    pub emissions_mode: EmissionsMode,
    /// Track table - known contacts
    pub track_table: Vec<Track>,
}

impl SensorState {
    /// Creates a new sensor state with the given ranges.
    #[must_use]
    pub fn new(radar_range: f32, sonar_range: f32) -> Self {
        Self {
            radar_range,
            sonar_range,
            emissions_mode: EmissionsMode::default(),
            track_table: Vec::new(),
        }
    }

    /// Returns the effective radar range based on emissions mode.
    #[must_use]
    pub fn effective_radar_range(&self) -> f32 {
        match self.emissions_mode {
            EmissionsMode::Silent | EmissionsMode::Passive => 0.0,
            EmissionsMode::Active => self.radar_range,
        }
    }

    /// Returns the effective sonar range based on emissions mode.
    #[must_use]
    pub fn effective_sonar_range(&self) -> f32 {
        match self.emissions_mode {
            EmissionsMode::Silent => self.sonar_range * 0.5, // Passive only, reduced range
            EmissionsMode::Passive => self.sonar_range * 0.75,
            EmissionsMode::Active => self.sonar_range,
        }
    }

    /// Finds a track by target ID.
    #[must_use]
    pub fn find_track(&self, target_id: EntityId) -> Option<&Track> {
        self.track_table.iter().find(|t| t.target_id == target_id)
    }

    /// Finds a mutable track by target ID.
    #[must_use]
    pub fn find_track_mut(&mut self, target_id: EntityId) -> Option<&mut Track> {
        self.track_table
            .iter_mut()
            .find(|t| t.target_id == target_id)
    }

    /// Returns tracks at or above the given quality level.
    #[must_use]
    pub fn tracks_at_quality(&self, min_quality: TrackQuality) -> Vec<&Track> {
        self.track_table
            .iter()
            .filter(|t| t.quality >= min_quality)
            .collect()
    }
}

impl Default for SensorState {
    fn default() -> Self {
        Self {
            radar_range: 10000.0,
            sonar_range: 5000.0,
            emissions_mode: EmissionsMode::default(),
            track_table: Vec::new(),
        }
    }
}

/// Inventory state - consumables and ammunition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InventoryState {
    /// Current fuel amount (normalized 0.0-1.0 or absolute)
    pub fuel: f32,
    /// Maximum fuel capacity
    pub max_fuel: f32,
    /// Ammunition by type
    pub ammo: BTreeMap<AmmoType, u32>,
}

impl InventoryState {
    /// Creates a new inventory state with the given fuel capacity.
    #[must_use]
    pub fn new(max_fuel: f32) -> Self {
        Self {
            fuel: max_fuel,
            max_fuel,
            ammo: BTreeMap::new(),
        }
    }

    /// Creates an inventory state with ammunition.
    #[must_use]
    pub fn with_ammo(max_fuel: f32, ammo: BTreeMap<AmmoType, u32>) -> Self {
        Self {
            fuel: max_fuel,
            max_fuel,
            ammo,
        }
    }

    /// Returns the fuel percentage (0.0-1.0).
    #[must_use]
    pub fn fuel_percent(&self) -> f32 {
        if self.max_fuel > 0.0 {
            self.fuel / self.max_fuel
        } else {
            0.0
        }
    }

    /// Returns the ammunition count for a given type.
    #[must_use]
    pub fn get_ammo(&self, ammo_type: AmmoType) -> u32 {
        self.ammo.get(&ammo_type).copied().unwrap_or(0)
    }

    /// Checks if there is ammunition of the given type.
    #[must_use]
    pub fn has_ammo(&self, ammo_type: AmmoType) -> bool {
        self.get_ammo(ammo_type) > 0
    }

    /// Consumes ammunition, returning true if successful.
    pub fn consume_ammo(&mut self, ammo_type: AmmoType, amount: u32) -> bool {
        if let Some(count) = self.ammo.get_mut(&ammo_type) {
            if *count >= amount {
                *count -= amount;
                return true;
            }
        }
        false
    }
}

impl Default for InventoryState {
    fn default() -> Self {
        Self {
            fuel: 1000.0,
            max_fuel: 1000.0,
            ammo: BTreeMap::new(),
        }
    }
}

// =============================================================================
// Composite Component Structs
// =============================================================================

/// Components for Ship entities.
///
/// Ships are the primary naval units in combat, ranging from jetskis to mobile
/// city-ships. They have full physics, combat, sensor, and inventory systems.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShipComponents {
    /// Position and heading
    pub transform: TransformState,
    /// Velocity and movement limits
    pub physics: PhysicsState,
    /// Health, weapons, and status
    pub combat: CombatState,
    /// Detection capabilities and track table
    pub sensor: SensorState,
    /// Fuel and ammunition
    pub inventory: InventoryState,
}

impl ShipComponents {
    /// Creates a new ship with default components.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a ship at the given position with the specified heading.
    #[must_use]
    pub fn at_position(position: Vec2, heading: f32) -> Self {
        Self {
            transform: TransformState::new(position, heading),
            ..Default::default()
        }
    }

    /// Builder method to set max HP.
    #[must_use]
    pub fn with_max_hp(mut self, max_hp: f32) -> Self {
        self.combat = CombatState::new(max_hp);
        self
    }

    /// Builder method to set physics limits.
    #[must_use]
    pub fn with_physics(mut self, max_speed: f32, max_turn_rate: f32) -> Self {
        self.physics = PhysicsState::new(max_speed, max_turn_rate);
        self
    }

    /// Builder method to set sensor ranges.
    #[must_use]
    pub fn with_sensors(mut self, radar_range: f32, sonar_range: f32) -> Self {
        self.sensor = SensorState::new(radar_range, sonar_range);
        self
    }
}

impl Default for ShipComponents {
    fn default() -> Self {
        Self {
            transform: TransformState::default(),
            physics: PhysicsState::default(),
            combat: CombatState::default(),
            sensor: SensorState::default(),
            inventory: InventoryState::default(),
        }
    }
}

/// Components for Platform entities.
///
/// Platforms are static or semi-static installations (buoys, oil rigs, bases).
/// They have position and sensors but no physics for movement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlatformComponents {
    /// Position and heading
    pub transform: TransformState,
    /// Detection capabilities and track table
    pub sensor: SensorState,
}

impl PlatformComponents {
    /// Creates a new platform with default components.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a platform at the given position.
    #[must_use]
    pub fn at_position(position: Vec2) -> Self {
        Self {
            transform: TransformState::new(position, 0.0),
            sensor: SensorState::default(),
        }
    }

    /// Builder method to set sensor ranges.
    #[must_use]
    pub fn with_sensors(mut self, radar_range: f32, sonar_range: f32) -> Self {
        self.sensor = SensorState::new(radar_range, sonar_range);
        self
    }
}

impl Default for PlatformComponents {
    fn default() -> Self {
        Self {
            transform: TransformState::default(),
            sensor: SensorState::default(),
        }
    }
}

/// Components for Projectile entities.
///
/// Projectiles are in-flight weapons (missiles, torpedoes, shells). They have
/// physics for movement but no complex systems.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectileComponents {
    /// Position and heading
    pub transform: TransformState,
    /// Velocity and movement limits
    pub physics: PhysicsState,
}

impl ProjectileComponents {
    /// Creates a new projectile with default components.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a projectile at the given position with velocity.
    #[must_use]
    pub fn at_position_with_velocity(position: Vec2, heading: f32, velocity: Vec2) -> Self {
        Self {
            transform: TransformState::new(position, heading),
            physics: PhysicsState {
                velocity,
                angular_velocity: 0.0,
                max_speed: velocity.length() * 1.5, // Some margin for guidance
                max_turn_rate: 0.5,                 // Limited maneuverability
            },
        }
    }
}

impl Default for ProjectileComponents {
    fn default() -> Self {
        Self {
            transform: TransformState::default(),
            physics: PhysicsState {
                velocity: Vec2::ZERO,
                angular_velocity: 0.0,
                max_speed: 500.0, // Fast by default
                max_turn_rate: 0.5,
            },
        }
    }
}

/// Components for Squadron entities.
///
/// Squadrons are groups of aircraft or small craft that operate as a unit.
/// They have aggregate state rather than individual member tracking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SquadronComponents {
    /// Position and heading (center of formation)
    pub transform: TransformState,
    /// Velocity and movement limits
    pub physics: PhysicsState,
    /// Aggregate health and weapons
    pub combat: CombatState,
}

impl SquadronComponents {
    /// Creates a new squadron with default components.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a squadron at the given position.
    #[must_use]
    pub fn at_position(position: Vec2, heading: f32) -> Self {
        Self {
            transform: TransformState::new(position, heading),
            physics: PhysicsState::default(),
            combat: CombatState::default(),
        }
    }

    /// Builder method to set the number of craft (affects HP).
    #[must_use]
    pub fn with_craft_count(mut self, count: u32, hp_per_craft: f32) -> Self {
        let total_hp = count as f32 * hp_per_craft;
        self.combat = CombatState::new(total_hp);
        self
    }
}

impl Default for SquadronComponents {
    fn default() -> Self {
        Self {
            transform: TransformState::default(),
            physics: PhysicsState {
                velocity: Vec2::ZERO,
                angular_velocity: 0.0,
                max_speed: 150.0,   // Aircraft are fast
                max_turn_rate: 2.0, // And maneuverable
            },
            combat: CombatState::default(),
        }
    }
}

// =============================================================================
// Access Traits
// =============================================================================

/// Trait for entities that have a transform component.
pub trait HasTransform {
    /// Returns a reference to the transform state.
    fn transform(&self) -> &TransformState;
    /// Returns a mutable reference to the transform state.
    fn transform_mut(&mut self) -> &mut TransformState;
}

/// Trait for entities that have a physics component.
pub trait HasPhysics {
    /// Returns a reference to the physics state.
    fn physics(&self) -> &PhysicsState;
    /// Returns a mutable reference to the physics state.
    fn physics_mut(&mut self) -> &mut PhysicsState;
}

/// Trait for entities that have a combat component.
pub trait HasCombat {
    /// Returns a reference to the combat state.
    fn combat(&self) -> &CombatState;
    /// Returns a mutable reference to the combat state.
    fn combat_mut(&mut self) -> &mut CombatState;
}

/// Trait for entities that have a sensor component.
pub trait HasSensor {
    /// Returns a reference to the sensor state.
    fn sensor(&self) -> &SensorState;
    /// Returns a mutable reference to the sensor state.
    fn sensor_mut(&mut self) -> &mut SensorState;
}

/// Trait for entities that have an inventory component.
pub trait HasInventory {
    /// Returns a reference to the inventory state.
    fn inventory(&self) -> &InventoryState;
    /// Returns a mutable reference to the inventory state.
    fn inventory_mut(&mut self) -> &mut InventoryState;
}

// =============================================================================
// Trait Implementations
// =============================================================================

// ShipComponents has all traits
impl HasTransform for ShipComponents {
    fn transform(&self) -> &TransformState {
        &self.transform
    }
    fn transform_mut(&mut self) -> &mut TransformState {
        &mut self.transform
    }
}

impl HasPhysics for ShipComponents {
    fn physics(&self) -> &PhysicsState {
        &self.physics
    }
    fn physics_mut(&mut self) -> &mut PhysicsState {
        &mut self.physics
    }
}

impl HasCombat for ShipComponents {
    fn combat(&self) -> &CombatState {
        &self.combat
    }
    fn combat_mut(&mut self) -> &mut CombatState {
        &mut self.combat
    }
}

impl HasSensor for ShipComponents {
    fn sensor(&self) -> &SensorState {
        &self.sensor
    }
    fn sensor_mut(&mut self) -> &mut SensorState {
        &mut self.sensor
    }
}

impl HasInventory for ShipComponents {
    fn inventory(&self) -> &InventoryState {
        &self.inventory
    }
    fn inventory_mut(&mut self) -> &mut InventoryState {
        &mut self.inventory
    }
}

// PlatformComponents has transform and sensor
impl HasTransform for PlatformComponents {
    fn transform(&self) -> &TransformState {
        &self.transform
    }
    fn transform_mut(&mut self) -> &mut TransformState {
        &mut self.transform
    }
}

impl HasSensor for PlatformComponents {
    fn sensor(&self) -> &SensorState {
        &self.sensor
    }
    fn sensor_mut(&mut self) -> &mut SensorState {
        &mut self.sensor
    }
}

// ProjectileComponents has transform and physics
impl HasTransform for ProjectileComponents {
    fn transform(&self) -> &TransformState {
        &self.transform
    }
    fn transform_mut(&mut self) -> &mut TransformState {
        &mut self.transform
    }
}

impl HasPhysics for ProjectileComponents {
    fn physics(&self) -> &PhysicsState {
        &self.physics
    }
    fn physics_mut(&mut self) -> &mut PhysicsState {
        &mut self.physics
    }
}

// SquadronComponents has transform, physics, and combat
impl HasTransform for SquadronComponents {
    fn transform(&self) -> &TransformState {
        &self.transform
    }
    fn transform_mut(&mut self) -> &mut TransformState {
        &mut self.transform
    }
}

impl HasPhysics for SquadronComponents {
    fn physics(&self) -> &PhysicsState {
        &self.physics
    }
    fn physics_mut(&mut self) -> &mut PhysicsState {
        &mut self.physics
    }
}

impl HasCombat for SquadronComponents {
    fn combat(&self) -> &CombatState {
        &self.combat
    }
    fn combat_mut(&mut self) -> &mut CombatState {
        &mut self.combat
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod transform_state_tests {
        use super::*;
        use std::f32::consts::PI;

        #[test]
        fn default_at_origin() {
            let transform = TransformState::default();
            assert_eq!(transform.position, Vec2::ZERO);
            assert_eq!(transform.heading, 0.0);
        }

        #[test]
        fn new_at_position() {
            let transform = TransformState::new(Vec2::new(100.0, 200.0), PI / 2.0);
            assert_eq!(transform.position, Vec2::new(100.0, 200.0));
            assert!((transform.heading - PI / 2.0).abs() < 0.001);
        }

        #[test]
        fn forward_direction() {
            // Heading 0 = facing +X
            let transform = TransformState::new(Vec2::ZERO, 0.0);
            let forward = transform.forward();
            assert!((forward.x - 1.0).abs() < 0.001);
            assert!(forward.y.abs() < 0.001);

            // Heading PI/2 = facing +Y
            let transform = TransformState::new(Vec2::ZERO, PI / 2.0);
            let forward = transform.forward();
            assert!(forward.x.abs() < 0.001);
            assert!((forward.y - 1.0).abs() < 0.001);
        }

        #[test]
        fn serialization_roundtrip() {
            let transform = TransformState::new(Vec2::new(1.0, 2.0), 0.5);
            let json = serde_json::to_string(&transform).unwrap();
            let deserialized: TransformState = serde_json::from_str(&json).unwrap();
            assert_eq!(transform, deserialized);
        }
    }

    mod physics_state_tests {
        use super::*;

        #[test]
        fn default_values() {
            let physics = PhysicsState::default();
            assert_eq!(physics.velocity, Vec2::ZERO);
            assert_eq!(physics.angular_velocity, 0.0);
            assert_eq!(physics.max_speed, 10.0);
            assert_eq!(physics.max_turn_rate, 1.0);
        }

        #[test]
        fn new_with_limits() {
            let physics = PhysicsState::new(50.0, 2.0);
            assert_eq!(physics.max_speed, 50.0);
            assert_eq!(physics.max_turn_rate, 2.0);
        }

        #[test]
        fn speed_calculation() {
            let mut physics = PhysicsState::default();
            physics.velocity = Vec2::new(3.0, 4.0);
            assert!((physics.speed() - 5.0).abs() < 0.001);
        }

        #[test]
        fn stationary_check() {
            let mut physics = PhysicsState::default();
            assert!(physics.is_stationary());

            physics.velocity = Vec2::new(0.001, 0.001);
            assert!(physics.is_stationary()); // Below threshold

            physics.velocity = Vec2::new(1.0, 0.0);
            assert!(!physics.is_stationary());
        }

        #[test]
        fn serialization_roundtrip() {
            let physics = PhysicsState::new(25.0, 1.5);
            let json = serde_json::to_string(&physics).unwrap();
            let deserialized: PhysicsState = serde_json::from_str(&json).unwrap();
            assert_eq!(physics, deserialized);
        }
    }

    mod combat_state_tests {
        use super::*;

        #[test]
        fn default_values() {
            let combat = CombatState::default();
            assert_eq!(combat.hp, 100.0);
            assert_eq!(combat.max_hp, 100.0);
            assert!(combat.weapons.is_empty());
            assert!(combat.status_flags.is_empty());
        }

        #[test]
        fn new_with_hp() {
            let combat = CombatState::new(500.0);
            assert_eq!(combat.hp, 500.0);
            assert_eq!(combat.max_hp, 500.0);
        }

        #[test]
        fn health_percent() {
            let mut combat = CombatState::new(100.0);
            assert!((combat.health_percent() - 1.0).abs() < 0.001);

            combat.hp = 50.0;
            assert!((combat.health_percent() - 0.5).abs() < 0.001);

            combat.hp = 0.0;
            assert!(combat.health_percent().abs() < 0.001);
        }

        #[test]
        fn status_flag_checks() {
            let mut combat = CombatState::default();
            assert!(!combat.is_destroyed());
            assert!(!combat.is_mobility_disabled());
            assert!(!combat.are_weapons_disabled());
            assert!(!combat.are_sensors_disabled());

            combat.status_flags.insert(StatusFlags::DESTROYED);
            assert!(combat.is_destroyed());

            combat.status_flags.insert(StatusFlags::MOBILITY_DISABLED);
            assert!(combat.is_mobility_disabled());
        }

        #[test]
        fn destroyed_by_hp() {
            let mut combat = CombatState::default();
            combat.hp = 0.0;
            assert!(combat.is_destroyed());
        }

        #[test]
        fn weapons_access() {
            let weapons = vec![
                WeaponState::new(0, 1.0, AmmoType::Bullet),
                WeaponState::new(1, 5.0, AmmoType::Missile),
            ];
            let mut combat = CombatState::with_weapons(100.0, weapons);

            assert!(combat.get_weapon(0).is_some());
            assert!(combat.get_weapon(1).is_some());
            assert!(combat.get_weapon(2).is_none());

            let weapon = combat.get_weapon_mut(0).unwrap();
            weapon.cooldown = 0.5;
            assert!(!weapon.is_ready());
        }

        #[test]
        fn serialization_roundtrip() {
            let combat =
                CombatState::with_weapons(200.0, vec![WeaponState::new(0, 2.0, AmmoType::Torpedo)]);
            let json = serde_json::to_string(&combat).unwrap();
            let deserialized: CombatState = serde_json::from_str(&json).unwrap();
            assert_eq!(combat, deserialized);
        }
    }

    mod sensor_state_tests {
        use super::*;

        #[test]
        fn default_values() {
            let sensor = SensorState::default();
            assert_eq!(sensor.radar_range, 10000.0);
            assert_eq!(sensor.sonar_range, 5000.0);
            assert_eq!(sensor.emissions_mode, EmissionsMode::Passive);
            assert!(sensor.track_table.is_empty());
        }

        #[test]
        fn effective_ranges_by_mode() {
            let mut sensor = SensorState::new(10000.0, 5000.0);

            // Silent mode
            sensor.emissions_mode = EmissionsMode::Silent;
            assert_eq!(sensor.effective_radar_range(), 0.0);
            assert!((sensor.effective_sonar_range() - 2500.0).abs() < 0.001);

            // Passive mode
            sensor.emissions_mode = EmissionsMode::Passive;
            assert_eq!(sensor.effective_radar_range(), 0.0);
            assert!((sensor.effective_sonar_range() - 3750.0).abs() < 0.001);

            // Active mode
            sensor.emissions_mode = EmissionsMode::Active;
            assert_eq!(sensor.effective_radar_range(), 10000.0);
            assert_eq!(sensor.effective_sonar_range(), 5000.0);
        }

        #[test]
        fn track_table_operations() {
            let mut sensor = SensorState::default();

            let track = Track::new(
                EntityId::new(42),
                Vec2::new(1000.0, 2000.0),
                TrackQuality::Coarse,
            );
            sensor.track_table.push(track);

            assert!(sensor.find_track(EntityId::new(42)).is_some());
            assert!(sensor.find_track(EntityId::new(999)).is_none());

            let track_mut = sensor.find_track_mut(EntityId::new(42)).unwrap();
            track_mut.quality = TrackQuality::FireControl;

            assert_eq!(
                sensor.find_track(EntityId::new(42)).unwrap().quality,
                TrackQuality::FireControl
            );
        }

        #[test]
        fn tracks_at_quality() {
            let mut sensor = SensorState::default();
            sensor
                .track_table
                .push(Track::new(EntityId::new(1), Vec2::ZERO, TrackQuality::Cue));
            sensor.track_table.push(Track::new(
                EntityId::new(2),
                Vec2::ZERO,
                TrackQuality::Coarse,
            ));
            sensor.track_table.push(Track::new(
                EntityId::new(3),
                Vec2::ZERO,
                TrackQuality::FireControl,
            ));

            let fc_tracks = sensor.tracks_at_quality(TrackQuality::FireControl);
            assert_eq!(fc_tracks.len(), 1);

            let coarse_tracks = sensor.tracks_at_quality(TrackQuality::Coarse);
            assert_eq!(coarse_tracks.len(), 2);

            let all_tracks = sensor.tracks_at_quality(TrackQuality::Cue);
            assert_eq!(all_tracks.len(), 3);
        }

        #[test]
        fn serialization_roundtrip() {
            let mut sensor = SensorState::new(15000.0, 8000.0);
            sensor.emissions_mode = EmissionsMode::Active;
            sensor.track_table.push(Track::new(
                EntityId::new(1),
                Vec2::new(100.0, 200.0),
                TrackQuality::Shared,
            ));

            let json = serde_json::to_string(&sensor).unwrap();
            let deserialized: SensorState = serde_json::from_str(&json).unwrap();
            assert_eq!(sensor, deserialized);
        }
    }

    mod inventory_state_tests {
        use super::*;

        #[test]
        fn default_values() {
            let inventory = InventoryState::default();
            assert_eq!(inventory.fuel, 1000.0);
            assert_eq!(inventory.max_fuel, 1000.0);
            assert!(inventory.ammo.is_empty());
        }

        #[test]
        fn fuel_percent() {
            let mut inventory = InventoryState::new(500.0);
            assert!((inventory.fuel_percent() - 1.0).abs() < 0.001);

            inventory.fuel = 250.0;
            assert!((inventory.fuel_percent() - 0.5).abs() < 0.001);
        }

        #[test]
        fn ammo_operations() {
            let mut ammo = BTreeMap::new();
            ammo.insert(AmmoType::Missile, 10);
            ammo.insert(AmmoType::Torpedo, 5);

            let mut inventory = InventoryState::with_ammo(1000.0, ammo);

            assert_eq!(inventory.get_ammo(AmmoType::Missile), 10);
            assert_eq!(inventory.get_ammo(AmmoType::Torpedo), 5);
            assert_eq!(inventory.get_ammo(AmmoType::Bullet), 0);

            assert!(inventory.has_ammo(AmmoType::Missile));
            assert!(!inventory.has_ammo(AmmoType::Bullet));

            assert!(inventory.consume_ammo(AmmoType::Missile, 3));
            assert_eq!(inventory.get_ammo(AmmoType::Missile), 7);

            assert!(!inventory.consume_ammo(AmmoType::Missile, 10)); // Not enough
            assert_eq!(inventory.get_ammo(AmmoType::Missile), 7); // Unchanged
        }

        #[test]
        fn serialization_roundtrip() {
            let mut ammo = BTreeMap::new();
            ammo.insert(AmmoType::Bullet, 100);
            let inventory = InventoryState::with_ammo(500.0, ammo);

            let json = serde_json::to_string(&inventory).unwrap();
            let deserialized: InventoryState = serde_json::from_str(&json).unwrap();
            assert_eq!(inventory, deserialized);
        }
    }

    mod status_flags_tests {
        use super::*;

        #[test]
        fn empty_by_default() {
            let flags = StatusFlags::default();
            assert!(flags.is_empty());
        }

        #[test]
        fn insert_and_contains() {
            let mut flags = StatusFlags::empty();
            flags.insert(StatusFlags::MOBILITY_DISABLED);
            flags.insert(StatusFlags::WEAPONS_DISABLED);

            assert!(flags.contains(StatusFlags::MOBILITY_DISABLED));
            assert!(flags.contains(StatusFlags::WEAPONS_DISABLED));
            assert!(!flags.contains(StatusFlags::SENSORS_DISABLED));
        }

        #[test]
        fn remove() {
            let mut flags = StatusFlags::MOBILITY_DISABLED | StatusFlags::WEAPONS_DISABLED;
            flags.remove(StatusFlags::MOBILITY_DISABLED);

            assert!(!flags.contains(StatusFlags::MOBILITY_DISABLED));
            assert!(flags.contains(StatusFlags::WEAPONS_DISABLED));
        }

        #[test]
        fn serialization_roundtrip() {
            let flags = StatusFlags::DESTROYED | StatusFlags::ON_FIRE;
            let json = serde_json::to_string(&flags).unwrap();
            let deserialized: StatusFlags = serde_json::from_str(&json).unwrap();
            assert_eq!(flags, deserialized);
        }
    }

    mod ship_components_tests {
        use super::*;

        #[test]
        fn default_construction() {
            let ship = ShipComponents::default();
            assert_eq!(ship.transform.position, Vec2::ZERO);
            assert_eq!(ship.physics.max_speed, 10.0);
            assert_eq!(ship.combat.hp, 100.0);
            assert_eq!(ship.sensor.radar_range, 10000.0);
            assert_eq!(ship.inventory.fuel, 1000.0);
        }

        #[test]
        fn builder_pattern() {
            let ship = ShipComponents::at_position(Vec2::new(100.0, 200.0), 1.0)
                .with_max_hp(500.0)
                .with_physics(30.0, 0.5)
                .with_sensors(20000.0, 10000.0);

            assert_eq!(ship.transform.position, Vec2::new(100.0, 200.0));
            assert_eq!(ship.combat.max_hp, 500.0);
            assert_eq!(ship.physics.max_speed, 30.0);
            assert_eq!(ship.sensor.radar_range, 20000.0);
        }

        #[test]
        fn has_all_traits() {
            let mut ship = ShipComponents::default();

            // HasTransform
            let _ = ship.transform();
            let _ = ship.transform_mut();

            // HasPhysics
            let _ = ship.physics();
            let _ = ship.physics_mut();

            // HasCombat
            let _ = ship.combat();
            let _ = ship.combat_mut();

            // HasSensor
            let _ = ship.sensor();
            let _ = ship.sensor_mut();

            // HasInventory
            let _ = ship.inventory();
            let _ = ship.inventory_mut();
        }

        #[test]
        fn serialization_roundtrip() {
            let ship = ShipComponents::at_position(Vec2::new(1.0, 2.0), 0.5).with_max_hp(200.0);
            let json = serde_json::to_string(&ship).unwrap();
            let deserialized: ShipComponents = serde_json::from_str(&json).unwrap();
            assert_eq!(ship, deserialized);
        }
    }

    mod platform_components_tests {
        use super::*;

        #[test]
        fn default_construction() {
            let platform = PlatformComponents::default();
            assert_eq!(platform.transform.position, Vec2::ZERO);
            assert_eq!(platform.sensor.radar_range, 10000.0);
        }

        #[test]
        fn at_position() {
            let platform = PlatformComponents::at_position(Vec2::new(500.0, 600.0));
            assert_eq!(platform.transform.position, Vec2::new(500.0, 600.0));
        }

        #[test]
        fn has_transform_and_sensor_traits() {
            let mut platform = PlatformComponents::default();

            // HasTransform
            let _ = platform.transform();
            let _ = platform.transform_mut();

            // HasSensor
            let _ = platform.sensor();
            let _ = platform.sensor_mut();
        }

        #[test]
        fn serialization_roundtrip() {
            let platform =
                PlatformComponents::at_position(Vec2::new(1.0, 2.0)).with_sensors(5000.0, 2000.0);
            let json = serde_json::to_string(&platform).unwrap();
            let deserialized: PlatformComponents = serde_json::from_str(&json).unwrap();
            assert_eq!(platform, deserialized);
        }
    }

    mod projectile_components_tests {
        use super::*;

        #[test]
        fn default_construction() {
            let projectile = ProjectileComponents::default();
            assert_eq!(projectile.transform.position, Vec2::ZERO);
            assert_eq!(projectile.physics.max_speed, 500.0);
        }

        #[test]
        fn at_position_with_velocity() {
            let projectile = ProjectileComponents::at_position_with_velocity(
                Vec2::new(100.0, 200.0),
                0.0,
                Vec2::new(300.0, 0.0),
            );
            assert_eq!(projectile.transform.position, Vec2::new(100.0, 200.0));
            assert_eq!(projectile.physics.velocity, Vec2::new(300.0, 0.0));
        }

        #[test]
        fn has_transform_and_physics_traits() {
            let mut projectile = ProjectileComponents::default();

            // HasTransform
            let _ = projectile.transform();
            let _ = projectile.transform_mut();

            // HasPhysics
            let _ = projectile.physics();
            let _ = projectile.physics_mut();
        }

        #[test]
        fn serialization_roundtrip() {
            let projectile = ProjectileComponents::at_position_with_velocity(
                Vec2::new(1.0, 2.0),
                0.5,
                Vec2::new(100.0, 50.0),
            );
            let json = serde_json::to_string(&projectile).unwrap();
            let deserialized: ProjectileComponents = serde_json::from_str(&json).unwrap();
            assert_eq!(projectile, deserialized);
        }
    }

    mod squadron_components_tests {
        use super::*;

        #[test]
        fn default_construction() {
            let squadron = SquadronComponents::default();
            assert_eq!(squadron.transform.position, Vec2::ZERO);
            assert_eq!(squadron.physics.max_speed, 150.0); // Aircraft speed
            assert_eq!(squadron.combat.hp, 100.0);
        }

        #[test]
        fn with_craft_count() {
            let squadron = SquadronComponents::at_position(Vec2::new(100.0, 200.0), 0.0)
                .with_craft_count(4, 25.0);
            assert_eq!(squadron.combat.max_hp, 100.0); // 4 * 25
        }

        #[test]
        fn has_transform_physics_combat_traits() {
            let mut squadron = SquadronComponents::default();

            // HasTransform
            let _ = squadron.transform();
            let _ = squadron.transform_mut();

            // HasPhysics
            let _ = squadron.physics();
            let _ = squadron.physics_mut();

            // HasCombat
            let _ = squadron.combat();
            let _ = squadron.combat_mut();
        }

        #[test]
        fn serialization_roundtrip() {
            let squadron =
                SquadronComponents::at_position(Vec2::new(1.0, 2.0), 0.5).with_craft_count(6, 20.0);
            let json = serde_json::to_string(&squadron).unwrap();
            let deserialized: SquadronComponents = serde_json::from_str(&json).unwrap();
            assert_eq!(squadron, deserialized);
        }
    }

    mod trait_access_tests {
        use super::*;

        fn modify_transform<T: HasTransform>(entity: &mut T) {
            entity.transform_mut().position = Vec2::new(999.0, 888.0);
        }

        fn modify_physics<T: HasPhysics>(entity: &mut T) {
            entity.physics_mut().velocity = Vec2::new(10.0, 20.0);
        }

        fn modify_combat<T: HasCombat>(entity: &mut T) {
            entity.combat_mut().hp = 50.0;
        }

        fn modify_sensor<T: HasSensor>(entity: &mut T) {
            entity.sensor_mut().emissions_mode = EmissionsMode::Active;
        }

        fn modify_inventory<T: HasInventory>(entity: &mut T) {
            entity.inventory_mut().fuel = 500.0;
        }

        #[test]
        fn generic_transform_access() {
            let mut ship = ShipComponents::default();
            let mut platform = PlatformComponents::default();
            let mut projectile = ProjectileComponents::default();
            let mut squadron = SquadronComponents::default();

            modify_transform(&mut ship);
            modify_transform(&mut platform);
            modify_transform(&mut projectile);
            modify_transform(&mut squadron);

            assert_eq!(ship.transform().position, Vec2::new(999.0, 888.0));
            assert_eq!(platform.transform().position, Vec2::new(999.0, 888.0));
            assert_eq!(projectile.transform().position, Vec2::new(999.0, 888.0));
            assert_eq!(squadron.transform().position, Vec2::new(999.0, 888.0));
        }

        #[test]
        fn generic_physics_access() {
            let mut ship = ShipComponents::default();
            let mut projectile = ProjectileComponents::default();
            let mut squadron = SquadronComponents::default();

            modify_physics(&mut ship);
            modify_physics(&mut projectile);
            modify_physics(&mut squadron);

            assert_eq!(ship.physics().velocity, Vec2::new(10.0, 20.0));
            assert_eq!(projectile.physics().velocity, Vec2::new(10.0, 20.0));
            assert_eq!(squadron.physics().velocity, Vec2::new(10.0, 20.0));
        }

        #[test]
        fn generic_combat_access() {
            let mut ship = ShipComponents::default();
            let mut squadron = SquadronComponents::default();

            modify_combat(&mut ship);
            modify_combat(&mut squadron);

            assert_eq!(ship.combat().hp, 50.0);
            assert_eq!(squadron.combat().hp, 50.0);
        }

        #[test]
        fn generic_sensor_access() {
            let mut ship = ShipComponents::default();
            let mut platform = PlatformComponents::default();

            modify_sensor(&mut ship);
            modify_sensor(&mut platform);

            assert_eq!(ship.sensor().emissions_mode, EmissionsMode::Active);
            assert_eq!(platform.sensor().emissions_mode, EmissionsMode::Active);
        }

        #[test]
        fn generic_inventory_access() {
            let mut ship = ShipComponents::default();

            modify_inventory(&mut ship);

            assert_eq!(ship.inventory().fuel, 500.0);
        }
    }

    mod weapon_state_tests {
        use super::*;

        #[test]
        fn new_weapon() {
            let weapon = WeaponState::new(0, 5.0, AmmoType::Missile);
            assert_eq!(weapon.slot, 0);
            assert_eq!(weapon.cooldown, 0.0);
            assert_eq!(weapon.max_cooldown, 5.0);
            assert_eq!(weapon.ammo_type, AmmoType::Missile);
            assert!(weapon.operational);
        }

        #[test]
        fn is_ready() {
            let mut weapon = WeaponState::new(0, 1.0, AmmoType::Bullet);
            assert!(weapon.is_ready());

            weapon.cooldown = 0.5;
            assert!(!weapon.is_ready());

            weapon.cooldown = 0.0;
            weapon.operational = false;
            assert!(!weapon.is_ready());
        }

        #[test]
        fn serialization_roundtrip() {
            let weapon = WeaponState::new(1, 3.0, AmmoType::Torpedo);
            let json = serde_json::to_string(&weapon).unwrap();
            let deserialized: WeaponState = serde_json::from_str(&json).unwrap();
            assert_eq!(weapon, deserialized);
        }
    }

    mod track_tests {
        use super::*;

        #[test]
        fn new_track() {
            let track = Track::new(
                EntityId::new(42),
                Vec2::new(1000.0, 2000.0),
                TrackQuality::Coarse,
            );
            assert_eq!(track.target_id, EntityId::new(42));
            assert_eq!(track.position, Vec2::new(1000.0, 2000.0));
            assert_eq!(track.quality, TrackQuality::Coarse);
            assert_eq!(track.age, 0.0);
            assert!(track.velocity.is_none());
        }

        #[test]
        fn track_quality_ordering() {
            assert!(TrackQuality::Cue < TrackQuality::Coarse);
            assert!(TrackQuality::Coarse < TrackQuality::FireControl);
            assert!(TrackQuality::FireControl < TrackQuality::Shared);
        }

        #[test]
        fn serialization_roundtrip() {
            let mut track = Track::new(
                EntityId::new(1),
                Vec2::new(100.0, 200.0),
                TrackQuality::FireControl,
            );
            track.velocity = Some(Vec2::new(10.0, 5.0));
            track.age = 2.5;
            track.classification_confidence = 0.8;

            let json = serde_json::to_string(&track).unwrap();
            let deserialized: Track = serde_json::from_str(&json).unwrap();
            assert_eq!(track, deserialized);
        }
    }
}
