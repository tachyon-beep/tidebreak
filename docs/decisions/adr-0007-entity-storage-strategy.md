# ADR 0007: Entity Storage Strategy

## Status
Accepted

## Context

The Entity-Plugin-Resolver architecture (ADR-0001) requires a storage strategy for entities and their components. Two primary approaches exist:

1. **Typed enum per entity type**: `enum Entity { Ship(ShipComponents), Platform(PlatformComponents) }`
2. **Component-based with dynamic composition**: `struct Entity { components: HashMap<TypeId, Box<dyn Component>> }`

The choice affects extensibility, type safety, performance, and maintenance burden.

### Requirements
- Support 4-6 entity types for MVP (Ship, Platform, Projectile, Squadron)
- Future expansion to 10+ types (Person, Faction, Enclave, Government)
- Deterministic iteration order
- Efficient component access in hot paths (plugin execution)
- Serialization for replay/save

## Decision

Use a **hybrid approach**: typed component structs with a tag-based entity wrapper.

```rust
/// Entity is a thin wrapper providing identity and tag-based dispatch
struct Entity {
    id: EntityId,
    tag: EntityTag,
    inner: EntityInner,
}

/// Tag determines entity type for plugin bundle selection
enum EntityTag {
    Ship,
    Platform,
    Projectile,
    Squadron,
}

/// Inner storage uses enum for MVP, migrates to archetype if needed
enum EntityInner {
    Ship(ShipComponents),
    Platform(PlatformComponents),
    Projectile(ProjectileComponents),
    Squadron(SquadronComponents),
}

/// Components are concrete structs, not trait objects
struct ShipComponents {
    pub transform: TransformState,
    pub physics: PhysicsState,
    pub combat: CombatState,
    pub sensor: SensorState,
    pub inventory: InventoryState,
}
```

### Key Design Elements

1. **EntityTag for plugin bundle selection**: Plugins are selected by tag, not by matching on EntityInner. This decouples plugin selection from storage.

2. **Concrete component structs**: No `Box<dyn Component>`. Components are known types with zero indirection.

3. **Shared components via traits**: Common access patterns use traits:
   ```rust
   trait HasTransform {
       fn transform(&self) -> &TransformState;
       fn transform_mut(&mut self) -> &mut TransformState;
   }

   impl HasTransform for ShipComponents { ... }
   impl HasTransform for PlatformComponents { ... }
   ```

4. **Migration path**: If entity types exceed 10 or component combinations become unwieldy, migrate EntityInner to archetype storage while keeping Entity wrapper stable.

## Consequences

### Enables
- **Type-safe component access**: No runtime type checking in hot paths
- **Exhaustive matching**: Compiler catches missing entity type handlers
- **Zero-cost abstractions**: No trait object indirection for component access
- **Clear extension point**: Add new EntityTag + EntityInner variant + component struct

### Costs
- **Boilerplate for new types**: Each entity type requires ~50 lines of struct + trait impls
- **Recompilation on type addition**: Adding entity types touches core enum
- **Limited runtime composition**: Can't create arbitrary component combinations at runtime

### Acceptable Because
- MVP needs only 4-6 entity types
- Performance in plugin execution is critical for DRL training
- Type safety catches errors at compile time
- Migration path exists if we outgrow this approach

## Alternatives Considered

### Pure Enum (No Tag Separation)
Rejected: Couples plugin selection to storage representation.

### HashMap<TypeId, Box<dyn Component>>
Rejected: Runtime type checking overhead in hot paths. Loses compile-time safety.

### Archetype-based (Bevy-style)
Deferred: Adds complexity we don't need for MVP. Can migrate later if entity count or component combinations grow significantly.

### Separate Storage per Component Type (SoA)
Deferred: Better cache locality for systems iterating one component type. Adds complexity. Consider for optimization phase.
