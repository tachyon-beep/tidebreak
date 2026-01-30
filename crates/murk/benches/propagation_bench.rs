use criterion::{black_box, criterion_group, criterion_main, Criterion};
use glam::Vec3;
use murk::{Stamp, Universe, UniverseConfig};

fn bench_propagation_step(c: &mut Criterion) {
    // Create universe with coarse resolution for fast benchmarks
    // Using 8.0 base_resolution keeps leaf count manageable
    let mut config = UniverseConfig::with_bounds(64.0, 64.0, 32.0);
    config.base_resolution = 8.0; // Coarse for reasonable benchmark time
    let mut universe = Universe::new(config);

    // Create a few stamps to have leaves to propagate
    for i in 0..3 {
        let x = (i as f32 - 1.0) * 20.0;
        universe.stamp(&Stamp::fire(Vec3::new(x, 0.0, 0.0), 12.0, 1.0));
    }

    c.bench_function("propagation_step", |b| {
        b.iter(|| {
            universe.step(black_box(0.1));
        })
    });
}

fn bench_collect_leaves(c: &mut Criterion) {
    let mut config = UniverseConfig::with_bounds(64.0, 64.0, 32.0);
    config.base_resolution = 8.0;
    let mut universe = Universe::new(config);

    for i in 0..5 {
        let x = (i as f32 - 2.0) * 12.0;
        universe.stamp(&Stamp::fire(Vec3::new(x, 0.0, 0.0), 8.0, 1.0));
    }

    c.bench_function("collect_leaves", |b| {
        b.iter(|| black_box(universe.octree().collect_leaves()))
    });
}

fn bench_propagation_step_larger(c: &mut Criterion) {
    // Slightly larger benchmark for stress testing
    // Uses finer resolution but smaller bounds
    let mut config = UniverseConfig::with_bounds(100.0, 100.0, 32.0);
    config.base_resolution = 4.0;
    let mut universe = Universe::new(config);

    // Single stamp to limit leaf count
    universe.stamp(&Stamp::fire(Vec3::ZERO, 20.0, 1.0));

    c.bench_function("propagation_step_larger", |b| {
        b.iter(|| {
            universe.step(black_box(0.1));
        })
    });
}

criterion_group!(benches, bench_propagation_step, bench_collect_leaves, bench_propagation_step_larger);
criterion_main!(benches);
