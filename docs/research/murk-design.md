# Murk: High-Level Design

**Status**: Research / Design Draft
**Purpose**: Hierarchical spatial substrate for agent perception, environmental simulation, and DRL training

---

## 1. Problem Statement

DRL projects and agent-based simulations repeatedly need:

- A world that can be **written to locally** (explosions, damage, fire)
- A world that can be **queried globally** (what's the temperature in this region?)
- Support for **diffuse phenomena** (smoke propagation, noise decay)
- **Scale-aware perception** (small agents need detail, large agents need summaries)
- **Fast iteration** on RL training without recompiling physics

Traditional approaches force a choice: either a detailed voxel grid (expensive to query at scale) or a sparse object list (can't represent continuous fields). Murk provides a third option: **hierarchical statistical summaries** over continuous fields.

---

## 2. Core Concepts

### 2.1 Fields, Not Objects

The world is represented as a set of **continuous scalar fields** sampled over space:

| Field | Type | Aggregation | Propagation |
|-------|------|-------------|-------------|
| Occupancy | [0, 1] | max | none |
| Material | enum | mode | none |
| Integrity | [0, 1] | mean | none |
| Temperature | ℝ⁺ | mean | diffusion |
| Smoke | [0, 1] | mean | diffusion + decay |
| Noise | ℝ⁺ | max | decay |
| Signal (generic) | ℝ | configurable | configurable |

**Objects are interpretations**, not storage primitives. A "wall" is a region where `occupancy > 0.5` and `material == CONCRETE`. A "fire" is where `temperature > ignition_threshold` and `fuel > 0`.

### 2.2 Hierarchical Compression

Space is stored as a **sparse octree** where:

- **Leaf nodes** store raw field values for their cell
- **Internal nodes** store **statistical summaries** of their children:
  - Mean
  - Variance
  - Min/Max (optional)
  - Dominant material (for enums)

This enables:
- **Cheap large-scale queries**: "average temperature in 100m radius" doesn't traverse to leaves
- **Adaptive detail**: boring regions collapse; interesting regions refine
- **Memory efficiency**: empty/uniform space costs almost nothing

### 2.3 Resolution-Aware Queries

All queries specify an **acceptable resolution** or **error tolerance**:

```
query_volume(center, radius, resolution="coarse")  → stops at depth 3
query_volume(center, radius, max_variance=0.1)    → refines until variance < 0.1
```

This allows agents to explicitly trade accuracy for speed.

---

## 3. Architecture

### 3.1 Layer Separation

```
┌─────────────────────────────────────────────────────────────────┐
│                        Python Layer                              │
│  ┌─────────────────┐  ┌─────────────────┐  ┌────────────────┐  │
│  │ Gymnasium Env   │  │ Reward Shaping  │  │ Visualization  │  │
│  └────────┬────────┘  └────────┬────────┘  └───────┬────────┘  │
│           │                    │                   │            │
│           └────────────────────┼───────────────────┘            │
│                                │                                 │
│                    ┌───────────▼───────────┐                    │
│                    │   PyO3 Bindings       │                    │
│                    │   (murk-py)    │                    │
│                    └───────────┬───────────┘                    │
└────────────────────────────────┼────────────────────────────────┘
                                 │ FFI boundary
┌────────────────────────────────┼────────────────────────────────┐
│                    ┌───────────▼───────────┐                    │
│                    │   Public Rust API     │                    │
│                    │   (murk crate) │                    │
│                    └───────────┬───────────┘                    │
│                                │                                 │
│  ┌─────────────────────────────┼─────────────────────────────┐  │
│  │                     Rust Core                              │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │  │
│  │  │ Octree   │  │ Fields   │  │ Physics  │  │ Buffers  │  │  │
│  │  │ Storage  │  │ & Stats  │  │ & Prop.  │  │ & Sync   │  │  │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘  │  │
│  └───────────────────────────────────────────────────────────┘  │
│                          Rust Layer                              │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 Component Responsibilities

| Component | Language | Responsibility |
|-----------|----------|----------------|
| **Octree Storage** | Rust | Sparse tree structure, node allocation, traversal |
| **Fields & Stats** | Rust | Per-node field values, statistical aggregation, merge/split logic |
| **Physics & Propagation** | Rust | Diffusion, decay, field update rules |
| **Buffers & Sync** | Rust | Triple-buffer management, observation serialization |
| **PyO3 Bindings** | Rust | Type conversion, GIL management, array views |
| **Gymnasium Env** | Python | `reset()`, `step()`, `render()`, action/observation spaces |
| **Reward Shaping** | Python | Task-specific reward computation |
| **Visualization** | Python | Debugging, training monitoring |

---

## 4. Synchronization Model

### 4.1 The Problem

- Rust physics can run at high frequency (100+ Hz)
- Python DRL runs at lower frequency (10-60 Hz, limited by inference)
- DRL must never see partially-written state
- Physics should not block on slow DRL consumers

### 4.2 Triple-Buffer Solution

```
┌─────────────────────────────────────────────────────────────────┐
│                         Rust Side                                │
│                                                                  │
│   Physics Loop                      Buffer Management            │
│   ┌──────────┐                     ┌──────────────────┐         │
│   │ Update   │                     │                  │         │
│   │ octree   │──── on tick ───────►│  Write Buffer    │         │
│   │ fields   │     complete        │  (owned by Rust) │         │
│   └──────────┘                     └────────┬─────────┘         │
│                                             │                    │
│                                    atomic   │                    │
│                                    swap     ▼                    │
│                                    ┌──────────────────┐         │
│                                    │  Ready Buffer    │         │
│                                    │  (latest frame)  │         │
│                                    └────────┬─────────┘         │
└─────────────────────────────────────────────┼───────────────────┘
                                              │
                              ┌───────────────┼───────────────┐
                              │ Python claims │ when ready    │
                              │               ▼               │
┌─────────────────────────────┼───────────────────────────────────┐
│                             │         Python Side               │
│                    ┌────────▼─────────┐                         │
│                    │  Read Buffer     │                         │
│                    │  (Python's view) │                         │
│                    └────────┬─────────┘                         │
│                             │                                    │
│                    ┌────────▼─────────┐                         │
│                    │  DRL Agent       │                         │
│                    │  Inference       │                         │
│                    └──────────────────┘                         │
└─────────────────────────────────────────────────────────────────┘
```

**Semantics**:
- Rust always has a buffer to write to (never blocks)
- If Rust produces faster than Python consumes, intermediate frames are dropped
- Python always gets the most recent complete frame
- No locks in the hot path (atomic pointer swaps only)

### 4.3 Synchronization Modes

```rust
pub enum SyncMode {
    /// Physics advances only when step() is called
    /// Guarantees: deterministic, reproducible
    /// Use for: training, replay, testing
    LockedStep {
        physics_ticks_per_step: u32,
        physics_dt: f64,
    },

    /// Physics runs continuously on background thread
    /// Guarantees: smooth real-time, latest-available observations
    /// Use for: deployment, human play, visualization
    Async {
        physics_hz: f64,
    },
}
```

**LockedStep** (for training):
```python
obs, info = env.reset(seed=42)
for _ in range(1000):
    action = agent.act(obs)
    obs, reward, term, trunc, info = env.step(action)
    # Physics advanced exactly N ticks, deterministically
```

**Async** (for deployment):
```python
env.start()  # Physics thread begins
while running:
    obs = env.observe()  # Latest available frame
    action = agent.act(obs)
    env.send_action(action)  # Non-blocking
```

---

## 5. Data Structures

### 5.1 Octree Node

```rust
pub struct OctreeNode {
    /// Spatial bounds (could be implicit from tree position)
    bounds: AABB,

    /// Node state
    state: NodeState,
}

pub enum NodeState {
    /// Leaf node with raw field values
    Leaf {
        fields: FieldValues,
    },

    /// Internal node with children and statistics
    Internal {
        children: [Option<Box<OctreeNode>>; 8],
        stats: FieldStatistics,
    },

    /// Implicit empty (not yet written)
    Empty,
}

pub struct FieldValues {
    pub occupancy: f32,
    pub material: MaterialId,
    pub integrity: f32,
    pub temperature: f32,
    pub smoke: f32,
    pub noise: f32,
    // Extensible via feature flags or generics
}

pub struct FieldStatistics {
    pub occupancy: ScalarStats,
    pub material: MaterialStats,  // Mode + distribution
    pub integrity: ScalarStats,
    pub temperature: ScalarStats,
    pub smoke: ScalarStats,
    pub noise: ScalarStats,
}

pub struct ScalarStats {
    pub mean: f32,
    pub variance: f32,
    pub min: f32,
    pub max: f32,
    pub sample_count: u32,
}
```

### 5.2 Observation Frame

The pre-vectorized observation sent to Python:

```rust
pub struct ObservationFrame {
    /// Tick number (for determinism tracking)
    pub tick: u64,

    /// Simulation time
    pub time: f64,

    /// Per-agent observations (foveated shells around each agent)
    pub agent_obs: Vec<AgentObservation>,

    /// Global state (optional, for centralized critics)
    pub global_state: Option<GlobalObservation>,
}

pub struct AgentObservation {
    pub agent_id: EntityId,

    /// Foveated perception shells (high-res near, low-res far)
    /// Shape: [num_shells, num_fields]
    pub shells: Vec<ShellObservation>,

    /// Agent's own state
    pub self_state: AgentState,
}

pub struct ShellObservation {
    pub radius_inner: f32,
    pub radius_outer: f32,
    pub resolution: u32,  // Angular divisions

    /// Per-sector field summaries
    /// Shape: [num_sectors, num_fields]
    pub sectors: Vec<FieldStatistics>,
}
```

### 5.3 Stamp (Mutation Primitive)

```rust
pub struct Stamp {
    pub shape: StampShape,
    pub modifications: Vec<FieldModification>,
}

pub enum StampShape {
    Sphere { center: Vec3, radius: f32 },
    Box { min: Vec3, max: Vec3 },
    Capsule { p0: Vec3, p1: Vec3, radius: f32 },
    // Future: arbitrary SDF
}

pub struct FieldModification {
    pub field: FieldId,
    pub operation: BlendOp,
    pub value: f32,
}

pub enum BlendOp {
    Set,                    // field = value
    Add,                    // field += value
    Multiply,               // field *= value
    Max,                    // field = max(field, value)
    Min,                    // field = min(field, value)
    Lerp { factor: f32 },   // field = lerp(field, value, factor)
}
```

---

## 6. Key Algorithms

### 6.1 Hierarchical Query

```
query_volume(center, radius, max_depth) → FieldStatistics:

    node = find_containing_node(center)

    if node.depth >= max_depth:
        return node.stats

    if node is Leaf:
        return stats_from_values(node.fields)

    if sphere_fully_contains(center, radius, node.bounds):
        return node.stats  # Early out: use cached stats

    # Partial overlap: recurse into relevant children
    result = empty_stats()
    for child in node.children:
        if child is not None and sphere_intersects(center, radius, child.bounds):
            child_stats = query_volume(center, radius, max_depth, child)
            result = merge_stats(result, child_stats)

    return result
```

### 6.2 Stamp Application

```
apply_stamp(stamp):

    affected_nodes = find_nodes_intersecting(stamp.shape)

    for node in affected_nodes:
        if node is Leaf:
            apply_modifications(node.fields, stamp.modifications)

        else if node is Internal:
            # Check if we need to split
            if stamp_introduces_gradient(stamp, node):
                split_node(node)
                apply_stamp(stamp)  # Retry with finer resolution
            else:
                # Apply to aggregate (lossy but fast)
                apply_modifications(node.stats, stamp.modifications)

        else:  # Empty
            materialize_node(node)
            apply_stamp(stamp)

    propagate_stats_upward(affected_nodes)
```

### 6.3 Adaptive Merge/Split

```
maybe_merge(node):
    if node is not Internal:
        return

    # Check if children are similar enough to merge
    variance_across_children = compute_inter_child_variance(node)

    if variance_across_children < MERGE_THRESHOLD for all fields:
        merge_children_into_leaf(node)

maybe_split(node, trigger_variance):
    if node is not Leaf:
        return

    if trigger_variance > SPLIT_THRESHOLD:
        convert_to_internal(node)
        distribute_values_to_children(node)
```

**Hysteresis**: `SPLIT_THRESHOLD > MERGE_THRESHOLD` to prevent oscillation.

### 6.4 Field Propagation (Diffusion)

```
propagate_field(field_id, dt):

    # Hierarchical where possible
    for node in nodes_at_depth(coarse_depth):
        if node.stats[field_id].variance < VARIANCE_THRESHOLD:
            # Uniform enough: diffuse at this level
            diffuse_with_neighbors(node, field_id, dt)
        else:
            # Too much variance: must go deeper
            for child in node.children:
                propagate_field_node(child, field_id, dt)
```

---

## 7. Python API

### 7.1 Universe Management

```python
from murk import Universe, SyncMode, Fields

# Create world
world = Universe(
    bounds=(1024, 1024, 256),
    base_resolution=1.0,
    merge_threshold=0.02,
    split_threshold=0.1,
)

# Configure fields
world.configure_field(
    Fields.TEMPERATURE,
    propagation="diffusion",
    diffusion_rate=0.1,
    decay_rate=0.01,
)

world.configure_field(
    Fields.NOISE,
    propagation="decay",
    decay_rate=0.5,
)
```

### 7.2 Mutation

```python
# Explosion
world.stamp(
    shape=Sphere(center=(500, 500, 20), radius=15),
    modifications={
        Fields.OCCUPANCY: ("subtract", 0.8),
        Fields.TEMPERATURE: ("add", 500),
        Fields.NOISE: ("add", 120),
        Fields.INTEGRITY: ("multiply", 0.2),
    }
)

# Fire spreading
world.stamp(
    shape=Box(min=(480, 480, 0), max=(520, 520, 40)),
    modifications={
        Fields.TEMPERATURE: ("lerp", 400, 0.1),
        Fields.SMOKE: ("add", 0.3),
    }
)
```

### 7.3 Query

```python
# Coarse regional query
stats = world.query_volume(
    center=(500, 500, 30),
    radius=50,
    resolution="coarse",  # or max_depth=3, or max_variance=0.1
)

print(f"Avg temperature: {stats.mean(Fields.TEMPERATURE)}")
print(f"Max noise: {stats.max(Fields.NOISE)}")
print(f"Heat gradient: {stats.gradient(Fields.TEMPERATURE)}")

# Foveated agent perception
obs = world.observe_foveated(
    agent_position=(500, 500, 10),
    agent_heading=(1, 0, 0),
    shells=[
        {"radius": 10, "resolution": 16},   # High-res nearby
        {"radius": 50, "resolution": 8},    # Medium-res mid-range
        {"radius": 200, "resolution": 4},   # Low-res far
    ]
)
# Returns numpy array ready for neural network
```

### 7.4 Gymnasium Environment

```python
import gymnasium as gym
from murk.gym import MorphoEnv

env = MorphoEnv(
    world_config={...},
    sync_mode=SyncMode.LockedStep(ticks_per_step=10, dt=0.01),
    observation_config={
        "foveated_shells": [...],
        "include_global": False,
    },
    action_space=gym.spaces.Dict({
        "move": gym.spaces.Box(-1, 1, shape=(3,)),
        "stamp": gym.spaces.Discrete(5),  # Predefined stamp types
    }),
)

obs, info = env.reset(seed=42)
for _ in range(1000):
    action = policy(obs)
    obs, reward, terminated, truncated, info = env.step(action)
```

---

## 8. Integration with Tidebreak

Murk could serve as the spatial substrate for Tidebreak's Combat Arena:

| Tidebreak Concept | Murk Mapping |
|-------------------|---------------------|
| Ocean layers (surface/submerged/abyssal) | Z-bands with different field profiles |
| Sensor detection | Field queries with resolution limits |
| Sonar propagation | Noise field with water-aware diffusion |
| Thermal layers | Temperature field with depth gradient |
| Weather effects | Field modifiers applied to surface layer |
| Fog of war | Query resolution limits based on sensor quality |
| Damage/destruction | Integrity field + occupancy stamps |

### 8.1 Potential Integration Points

```
┌─────────────────────────────────────────────────────────────────┐
│                     Tidebreak Combat Arena                       │
│                                                                  │
│   ┌──────────────┐    ┌──────────────┐    ┌──────────────┐     │
│   │ Entity       │    │ Plugins      │    │ Resolver     │     │
│   │ (ships, etc) │    │ (sensors,    │    │ (collision,  │     │
│   │              │    │  weapons)    │    │  damage)     │     │
│   └──────┬───────┘    └──────┬───────┘    └──────┬───────┘     │
│          │                   │                   │              │
│          └───────────────────┼───────────────────┘              │
│                              │                                   │
│                    ┌─────────▼─────────┐                        │
│                    │   WorldView       │                        │
│                    │   (immutable      │                        │
│                    │    snapshot)      │                        │
│                    └─────────┬─────────┘                        │
│                              │                                   │
└──────────────────────────────┼──────────────────────────────────┘
                               │
                    ┌──────────▼──────────┐
                    │    Murk      │
                    │    (spatial fields) │
                    └─────────────────────┘
```

**Key question**: Does Murk replace Tidebreak's entity system, or complement it?

**Proposed answer**: Complement. Entities (ships, weapons, people) remain discrete objects with identity. Murk handles the *environment* — terrain, water properties, ambient conditions, sensor occlusion. The WorldView snapshot includes both entity state and Murk field queries.

---

## 9. Open Questions

### 9.1 Architecture

- [ ] **Octree vs. other hierarchies**: Is cubic octree the right choice, or should we support other tessellations (e.g., for non-cubic worlds)?
- [ ] **Field extensibility**: Hardcode common fields, or make fully generic with runtime field registration?
- [ ] **Multi-agent observation**: One foveated observation per agent, or shared spatial features with per-agent masking?

### 9.2 Performance

- [ ] **SIMD for field updates**: How much does vectorization help for diffusion/decay?
- [ ] **GPU acceleration**: Is there value in moving field propagation to GPU, or is the CPU→GPU transfer overhead too high?
- [ ] **Memory layout**: Array-of-structs vs. struct-of-arrays for field storage?

### 9.3 Determinism

- [ ] **Cross-platform determinism**: Do we need fixed-point math, or is IEEE 754 + careful ordering sufficient?
- [ ] **Floating-point diffusion**: Iterative diffusion accumulates error. Accept it, or use exact rational arithmetic?

### 9.4 Merge/Split Heuristics

- [ ] **Hysteresis tuning**: What are good default values for merge/split thresholds?
- [ ] **Gradient detection**: How do we detect "sharp gradients" efficiently during stamp application?
- [ ] **Material merging**: When children have different materials, what's the merge policy?

---

## 10. Implementation Phases

### Phase 1: Proof of Concept (2-3 days)

**Goal**: Validate Rust↔Python boundary with trivial data structure.

- Flat 3D array (no octree)
- Two fields: temperature, occupancy
- One stamp shape: sphere
- One query: box mean
- PyO3 bindings with maturin
- Minimal Gymnasium wrapper

**Success criteria**: Can run a trivial "find the heat source" RL task.

### Phase 2: Hierarchical Storage (1 week)

**Goal**: Implement octree with merge/split.

- Sparse octree with lazy allocation
- Statistical aggregation on internal nodes
- Resolution-aware queries
- Stamp-triggered splits
- Variance-triggered merges

**Success criteria**: Large world (1024³) with localized detail is memory-efficient.

### Phase 3: Field Propagation (1 week)

**Goal**: Temporal field dynamics.

- Diffusion (temperature, smoke)
- Decay (noise)
- Hierarchical propagation where possible

**Success criteria**: Fire spreads, smoke dissipates, noise fades.

### Phase 4: Foveated Perception (3-5 days)

**Goal**: Multi-scale agent observations.

- Shell-based observation structure
- Configurable resolution per shell
- Efficient batch observation for multi-agent

**Success criteria**: Observation tensor size is O(shells × sectors), not O(world size).

### Phase 5: Tidebreak Integration (TBD)

**Goal**: Use Murk as Tidebreak's environmental substrate.

- Ocean layer modeling
- Sensor integration
- Weather field modifiers

---

## 11. References

### Proven Patterns

- **Sparse Voxel Octrees**: Standard in graphics (NVIDIA GVDB, OpenVDB)
- **Statistical spatial indices**: R-trees with aggregates, quadtree pyramids
- **Triple buffering**: Graphics swap chains, audio buffers
- **PyO3 + maturin**: Polars, tokenizers, candle

### Related Work

- **OpenVDB**: Production-quality sparse volumetric data structure
- **Mujoco**: Physics engine with good Python bindings (different domain, similar boundary)
- **Gymnasium**: De facto standard RL environment interface

---

## Appendix A: Glossary

| Term | Definition |
|------|------------|
| **Cell** | Fundamental spatial unit; a leaf node's extent |
| **Field** | Continuous scalar quantity sampled over space |
| **Internal node** | Octree node with children; stores statistical summaries |
| **Leaf node** | Octree node without children; stores raw field values |
| **Merge** | Collapsing children into a single leaf (lossy) |
| **Shell** | Annular region around an agent for foveated perception |
| **Split** | Converting a leaf into an internal node with children |
| **Stamp** | Mutation primitive: shape + field modifications |
| **Statistics** | Per-field summaries: mean, variance, min, max |

---

## Appendix B: Example Field Configurations

### Combat Environment

```yaml
fields:
  occupancy:
    type: scalar
    range: [0, 1]
    aggregation: max
    propagation: none

  temperature:
    type: scalar
    range: [0, 10000]  # Kelvin
    aggregation: mean
    propagation:
      type: diffusion
      rate: 0.05

  smoke:
    type: scalar
    range: [0, 1]
    aggregation: mean
    propagation:
      type: diffusion_decay
      diffusion_rate: 0.1
      decay_rate: 0.02

  noise:
    type: scalar
    range: [0, 200]  # dB
    aggregation: max
    propagation:
      type: decay
      rate: 0.3

  threat:
    type: scalar
    range: [0, 1]
    aggregation: max
    propagation:
      type: decay
      rate: 0.1
```

### Ocean Environment (Tidebreak)

```yaml
fields:
  depth:
    type: scalar
    range: [0, 10000]  # meters
    aggregation: mean
    propagation: none

  salinity:
    type: scalar
    range: [0, 50]  # ppt
    aggregation: mean
    propagation:
      type: diffusion
      rate: 0.001

  temperature:
    type: scalar
    range: [-2, 35]  # Celsius
    aggregation: mean
    propagation:
      type: diffusion
      rate: 0.01

  current_x:
    type: scalar
    range: [-10, 10]  # m/s
    aggregation: mean
    propagation: none

  current_y:
    type: scalar
    range: [-10, 10]
    aggregation: mean
    propagation: none

  acoustic_noise:
    type: scalar
    range: [0, 200]  # dB
    aggregation: max
    propagation:
      type: decay
      rate: 0.1

  sonar_return:
    type: scalar
    range: [0, 1]
    aggregation: max
    propagation:
      type: decay
      rate: 0.5
```
