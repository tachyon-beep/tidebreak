//! Universe: top-level API for Murk.
//!
//! The Universe wraps the octree and provides a convenient high-level interface
//! for common operations.

use glam::Vec3;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::field::{Field, FieldConfig, FieldValues};
use crate::octree::{Octree, OctreeConfig, OctreeStats};
use crate::query::{
    FoveatedQuery, FoveatedResult, PointQuery, PointResult, QueryResolution, QueryResult,
    VolumeQuery,
};
use crate::stamp::Stamp;
// FieldStats imported via query module
use crate::Bounds;

/// Configuration for a Universe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniverseConfig {
    /// World bounds
    pub bounds: Bounds,
    /// Base resolution (cell size at maximum depth)
    pub base_resolution: f32,
    /// Variance threshold for merging cells
    pub merge_threshold: f32,
    /// Variance threshold for splitting cells
    pub split_threshold: f32,
    /// Field configurations (optional overrides)
    pub field_configs: Vec<FieldConfig>,
}

impl Default for UniverseConfig {
    fn default() -> Self {
        Self {
            bounds: Bounds::new(1024.0, 1024.0, 256.0),
            base_resolution: 1.0,
            merge_threshold: 0.02,
            split_threshold: 0.1,
            field_configs: Vec::new(),
        }
    }
}

impl UniverseConfig {
    /// Create a new config with specified bounds.
    #[must_use]
    pub fn with_bounds(width: f32, height: f32, depth: f32) -> Self {
        Self {
            bounds: Bounds::new(width, height, depth),
            ..Default::default()
        }
    }
}

/// The Universe: top-level container for spatial fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Universe {
    /// Octree storage
    octree: Octree,
    /// Field configurations
    field_configs: [FieldConfig; Field::COUNT],
    /// Current simulation tick
    tick: u64,
    /// Simulation time in seconds
    time: f64,
    /// Deterministic RNG (optional, skipped in serialization)
    #[serde(skip)]
    rng: Option<ChaCha8Rng>,
    /// Original seed for replay
    seed: Option<u64>,
}

impl Universe {
    /// Create a new Universe.
    #[must_use]
    pub fn new(config: UniverseConfig) -> Self {
        let max_depth = OctreeConfig::calculate_max_depth(&config.bounds, config.base_resolution);

        let octree = Octree::new(OctreeConfig {
            bounds: config.bounds,
            base_resolution: config.base_resolution,
            max_depth,
            merge_threshold: config.merge_threshold,
            split_threshold: config.split_threshold,
        });

        // Initialize field configs with defaults, then apply overrides
        let mut field_configs: [FieldConfig; Field::COUNT] =
            std::array::from_fn(|i| FieldConfig::default_for(Field::all()[i]));

        for override_config in &config.field_configs {
            field_configs[override_config.field.index()] = override_config.clone();
        }

        Self {
            octree,
            field_configs,
            tick: 0,
            time: 0.0,
            rng: None,
            seed: None,
        }
    }

    /// Create a new Universe with deterministic seeded RNG.
    #[must_use]
    pub fn new_with_seed(config: UniverseConfig, seed: u64) -> Self {
        let mut universe = Self::new(config);
        universe.rng = Some(ChaCha8Rng::seed_from_u64(seed));
        universe.seed = Some(seed);
        universe
    }

    /// Get the seed used to create this universe.
    #[must_use]
    pub fn seed(&self) -> Option<u64> {
        self.seed
    }

    /// Get mutable access to RNG (for internal use).
    ///
    /// This will be used by propagation and stochastic operations.
    #[allow(dead_code)]
    pub(crate) fn rng_mut(&mut self) -> Option<&mut ChaCha8Rng> {
        self.rng.as_mut()
    }

    /// Get the current tick.
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Get the current simulation time.
    #[must_use]
    pub fn time(&self) -> f64 {
        self.time
    }

    /// Get octree statistics.
    #[must_use]
    pub fn stats(&self) -> OctreeStats {
        self.octree.stats()
    }

    /// Get the world bounds.
    #[must_use]
    pub fn bounds(&self) -> Bounds {
        self.octree.config().bounds
    }

    /// Get read access to the octree (for hashing and advanced queries).
    #[must_use]
    pub fn octree(&self) -> &Octree {
        &self.octree
    }

    /// Compute a deterministic hash of the current state.
    ///
    /// Used for verifying determinism: identical inputs should produce identical hashes.
    /// See ADR-0003 for determinism strategy.
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        crate::hash::hash_universe(self)
    }

    /// Get field configuration.
    #[must_use]
    pub fn field_config(&self, field: Field) -> &FieldConfig {
        &self.field_configs[field.index()]
    }

    // ========================================================================
    // Mutation
    // ========================================================================

    /// Apply a stamp to the universe.
    pub fn stamp(&mut self, stamp: &Stamp) {
        self.octree.apply_stamp(stamp);
    }

    /// Apply multiple stamps.
    pub fn stamp_many(&mut self, stamps: &[Stamp]) {
        for stamp in stamps {
            self.octree.apply_stamp(stamp);
        }
    }

    /// Set field values at a point.
    pub fn set_point(&mut self, position: Vec3, values: FieldValues) {
        self.octree.set_point(position, values);
    }

    // ========================================================================
    // Queries
    // ========================================================================

    /// Query a single point.
    #[must_use]
    pub fn query_point(&self, position: Vec3) -> PointResult {
        self.octree.query_point(&PointQuery::new(position))
    }

    /// Query a volume.
    #[must_use]
    pub fn query_volume(&self, center: Vec3, radius: f32, resolution: QueryResolution) -> QueryResult {
        self.octree.query_volume(
            &VolumeQuery::new(center, radius).with_resolution(resolution),
        )
    }

    /// Get a foveated observation for an agent.
    #[must_use]
    pub fn observe_foveated(&self, query: &FoveatedQuery) -> FoveatedResult {
        let mut shell_stats = Vec::with_capacity(query.shells.len());
        let mut total_nodes_visited = 0;

        for shell in &query.shells {
            let mut sector_stats = Vec::with_capacity(shell.sectors as usize);

            // For each sector in this shell
            for sector_idx in 0..shell.sectors {
                // Calculate sector center
                let angle = (sector_idx as f32 / shell.sectors as f32) * std::f32::consts::TAU;
                let mid_radius = (shell.radius_inner + shell.radius_outer) / 2.0;

                // Rotate by heading
                let heading_angle = query.heading.y.atan2(query.heading.x);
                let sector_angle = heading_angle + angle;

                let sector_center = query.position
                    + Vec3::new(sector_angle.cos(), sector_angle.sin(), 0.0) * mid_radius;

                let sector_radius = (shell.radius_outer - shell.radius_inner) / 2.0;

                // Query this sector
                let result = self.octree.query_volume(
                    &VolumeQuery::new(sector_center, sector_radius)
                        .with_resolution(shell.resolution),
                );

                total_nodes_visited += result.nodes_visited;
                sector_stats.push(result.stats);
            }

            shell_stats.push(sector_stats);
        }

        FoveatedResult {
            shell_stats,
            nodes_visited: total_nodes_visited,
        }
    }

    // ========================================================================
    // Simulation
    // ========================================================================

    /// Advance simulation by one tick.
    ///
    /// This propagates fields (diffusion, decay) according to their configurations.
    pub fn step(&mut self, dt: f64) {
        // Propagate fields (diffusion, decay)
        crate::propagation::propagate_all(self, dt);

        self.tick += 1;
        self.time += dt;
    }

    /// Reset the universe to initial state.
    ///
    /// If the universe was created with a seed, the RNG is re-seeded
    /// to ensure deterministic replay.
    pub fn reset(&mut self) {
        let config = self.octree.config().clone();
        self.octree = Octree::new(config);
        self.tick = 0;
        self.time = 0.0;
        // Re-seed RNG if a seed exists (for deterministic replay)
        if let Some(seed) = self.seed {
            self.rng = Some(ChaCha8Rng::seed_from_u64(seed));
        }
    }
}

impl Default for Universe {
    fn default() -> Self {
        Self::new(UniverseConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_universe_creation() {
        let universe = Universe::new(UniverseConfig::with_bounds(100.0, 100.0, 50.0));
        assert_eq!(universe.tick(), 0);
        assert_eq!(universe.time(), 0.0);
    }

    #[test]
    fn test_universe_stamp_and_query() {
        let mut universe = Universe::new(UniverseConfig::with_bounds(100.0, 100.0, 50.0));

        // Apply an explosion
        universe.stamp(&Stamp::explosion(Vec3::ZERO, 10.0, 1.0));

        // Query the affected area
        let result = universe.query_volume(Vec3::ZERO, 15.0, QueryResolution::Fine);
        assert!(result.mean(Field::Temperature) > 293.0); // Above default
        assert!(result.mean(Field::Noise) > 0.0);
    }

    #[test]
    fn test_universe_foveated_observation() {
        let mut universe = Universe::new(UniverseConfig::with_bounds(200.0, 200.0, 50.0));

        // Create a heat source
        universe.stamp(&Stamp::fire(Vec3::new(50.0, 0.0, 0.0), 10.0, 1.0));

        // Get foveated observation from origin looking toward heat
        let query = FoveatedQuery::new(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0));
        let result = universe.observe_foveated(&query);

        // Should have 3 shells (default config)
        assert_eq!(result.shell_stats.len(), 3);
    }

    #[test]
    fn test_universe_step() {
        let mut universe = Universe::default();
        assert_eq!(universe.tick(), 0);

        universe.step(0.1);
        assert_eq!(universe.tick(), 1);
        assert!((universe.time() - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_universe_seeded_creation() {
        let config = UniverseConfig::with_bounds(100.0, 100.0, 50.0);
        let universe = Universe::new_with_seed(config, 42);
        assert_eq!(universe.seed(), Some(42));
    }

    #[test]
    fn test_seeded_rng_determinism() {
        use rand::Rng;

        let config = UniverseConfig::with_bounds(100.0, 100.0, 50.0);
        let mut u1 = Universe::new_with_seed(config.clone(), 42);
        let mut u2 = Universe::new_with_seed(config, 42);

        // Both should produce identical sequences
        let v1: f64 = u1.rng_mut().unwrap().gen();
        let v2: f64 = u2.rng_mut().unwrap().gen();
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_reset_restores_rng() {
        use rand::Rng;

        let config = UniverseConfig::with_bounds(100.0, 100.0, 50.0);
        let mut universe = Universe::new_with_seed(config, 42);

        // Generate some values
        let initial: f64 = universe.rng_mut().unwrap().gen();
        let _: f64 = universe.rng_mut().unwrap().gen(); // Advance state

        // Reset should restore RNG
        universe.reset();
        let after_reset: f64 = universe.rng_mut().unwrap().gen();

        assert_eq!(initial, after_reset);
    }

    #[test]
    fn test_universe_state_hash() {
        let config = UniverseConfig::with_bounds(100.0, 100.0, 50.0);
        let mut universe = Universe::new_with_seed(config.clone(), 42);

        universe.stamp(&Stamp::explosion(Vec3::ZERO, 10.0, 1.0));
        let hash1 = universe.state_hash();

        let mut universe2 = Universe::new_with_seed(config, 42);
        universe2.stamp(&Stamp::explosion(Vec3::ZERO, 10.0, 1.0));
        let hash2 = universe2.state_hash();

        assert_eq!(
            hash1, hash2,
            "Identical operations should produce identical hashes"
        );
    }

    #[test]
    fn test_determinism_same_platform() {
        let config = UniverseConfig::with_bounds(100.0, 100.0, 50.0);

        // Run 1
        let mut universe1 = Universe::new_with_seed(config.clone(), 12345);
        universe1.stamp(&Stamp::explosion(Vec3::new(10.0, 20.0, 5.0), 15.0, 0.8));
        universe1.stamp(&Stamp::fire(Vec3::new(-5.0, 0.0, 0.0), 8.0, 0.5));
        for _ in 0..10 {
            universe1.step(0.1);
        }
        let hash1 = universe1.state_hash();

        // Run 2 (identical operations)
        let mut universe2 = Universe::new_with_seed(config, 12345);
        universe2.stamp(&Stamp::explosion(Vec3::new(10.0, 20.0, 5.0), 15.0, 0.8));
        universe2.stamp(&Stamp::fire(Vec3::new(-5.0, 0.0, 0.0), 8.0, 0.5));
        for _ in 0..10 {
            universe2.step(0.1);
        }
        let hash2 = universe2.state_hash();

        assert_eq!(hash1, hash2, "Same seed + same operations must produce identical state (ADR-0003)");
    }

    #[test]
    fn test_step_propagates_temperature() {
        use crate::stamp::{BlendOp, FieldMod, StampShape};

        // Use a small world with coarse resolution for fast tests
        let mut config = UniverseConfig::with_bounds(64.0, 64.0, 32.0);
        config.base_resolution = 8.0;
        let mut universe = Universe::new(config);

        // Use a box stamp to create a hot region at the center
        // This avoids creating many leaf nodes with zero temperature
        let hot_stamp = Stamp::new(
            StampShape::sphere(Vec3::ZERO, 15.0),
            vec![FieldMod::new(Field::Temperature, BlendOp::Set, 800.0)],
        );
        universe.stamp(&hot_stamp);

        // Query a point inside the hot region and outside it
        let center_temp_before = universe.query_point(Vec3::ZERO).values.get(Field::Temperature);
        let edge_temp_before = universe.query_point(Vec3::new(10.0, 0.0, 0.0)).values.get(Field::Temperature);

        // The temperature at edge should be affected by the stamp
        // (it's within the 15-unit radius sphere)
        assert!(
            center_temp_before > 500.0,
            "Center should be hot: {}",
            center_temp_before
        );
        assert!(
            edge_temp_before > 0.0,
            "Edge should have some heat from stamp: {}",
            edge_temp_before
        );

        // Step multiple times for diffusion to occur
        // Heat should spread outward (edge gets cooler as heat spreads)
        // and decay toward ambient (293K)
        for _ in 0..10 {
            universe.step(0.5);
        }

        let center_temp_after = universe.query_point(Vec3::ZERO).values.get(Field::Temperature);
        let _edge_temp_after = universe.query_point(Vec3::new(10.0, 0.0, 0.0)).values.get(Field::Temperature);

        // Both temperatures should be closer to ambient (293K) after diffusion/decay
        // Since temperature has Diffusion propagation (rate 0.05), values should move toward equilibrium
        // The center should cool down as heat spreads out
        assert!(
            center_temp_after < center_temp_before,
            "Center should cool as heat diffuses (before: {}, after: {})",
            center_temp_before,
            center_temp_after
        );

        // Verify that propagation actually happened (temperatures changed)
        assert!(
            (center_temp_after - center_temp_before).abs() > 1.0,
            "Temperature should have changed significantly"
        );
    }
}
