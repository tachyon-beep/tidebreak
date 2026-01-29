"""
Benchmark Rust->Python boundary performance.

Key metrics:
- observe_foveated() latency
- step() latency
- Memory stability over many steps

Run as script: python bench_boundary.py
Run as tests:  pytest bench_boundary.py -v -s
"""

import gc
import time
import tracemalloc

import numpy as np


def benchmark_observation_latency():
    """Measure observation creation time."""
    from tidebreak import PyUniverse

    universe = PyUniverse(width=200.0, height=200.0, depth=50.0)
    universe.reset(seed=42)

    # Warmup
    for _ in range(10):
        universe.observe_foveated(
            position=(0.0, 0.0, 0.0),
            heading=(1.0, 0.0, 0.0),
        )

    # Benchmark
    n_iterations = 1000
    start = time.perf_counter()
    for _ in range(n_iterations):
        universe.observe_foveated(
            position=(0.0, 0.0, 0.0),
            heading=(1.0, 0.0, 0.0),
        )
    elapsed = time.perf_counter() - start

    latency_us = (elapsed / n_iterations) * 1_000_000
    print(f"observe_foveated latency: {latency_us:.1f} us ({n_iterations / elapsed:.0f} Hz)")

    # Target: < 1000 us for training viability
    assert latency_us < 1000, f"Observation too slow: {latency_us:.1f} us"


def benchmark_step_latency():
    """Measure step time."""
    from tidebreak.envs import MurkEnv

    env = MurkEnv(max_steps=10000)
    env.reset(seed=42)

    action = {"move": np.array([0.1, 0.1], dtype=np.float32), "stamp": 0}

    # Warmup
    for _ in range(10):
        env.step(action)
    env.reset(seed=42)

    # Benchmark
    n_iterations = 1000
    start = time.perf_counter()
    for _ in range(n_iterations):
        env.step(action)
    elapsed = time.perf_counter() - start

    latency_us = (elapsed / n_iterations) * 1_000_000
    print(f"step latency: {latency_us:.1f} us ({n_iterations / elapsed:.0f} Hz)")

    # Target: < 10000 us for real-time training
    assert latency_us < 10000, f"Step too slow: {latency_us:.1f} us"

    env.close()


def benchmark_memory_stability():
    """Check for memory leaks over many steps."""
    from tidebreak.envs import MurkEnv

    tracemalloc.start()

    env = MurkEnv(max_steps=10000)

    for episode in range(5):
        env.reset(seed=episode)
        for _ in range(1000):
            action = env.action_space.sample()
            env.step(action)

    gc.collect()
    current, peak = tracemalloc.get_traced_memory()
    tracemalloc.stop()

    print(f"Memory: current={current / 1024 / 1024:.1f} MB, peak={peak / 1024 / 1024:.1f} MB")

    # Should not grow unboundedly
    assert peak < 500 * 1024 * 1024, f"Memory usage too high: {peak / 1024 / 1024:.1f} MB"

    env.close()


# Pytest test wrappers


def test_observation_latency_acceptable():
    """Test that observation latency is acceptable for training."""
    benchmark_observation_latency()


def test_step_latency_acceptable():
    """Test that step latency is acceptable for training."""
    benchmark_step_latency()


def test_memory_stability():
    """Test that memory doesn't grow unboundedly."""
    benchmark_memory_stability()


if __name__ == "__main__":
    print("=== Rust->Python Boundary Benchmarks ===\n")
    benchmark_observation_latency()
    benchmark_step_latency()
    benchmark_memory_stability()
    print("\n=== All benchmarks passed ===")
