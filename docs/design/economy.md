# Economy Design

The economy provides resources that fuel everything else: ships need supplies, arcologies need food, governments need prosperity, and factions need action points. Economic strength creates strategic options; scarcity creates pressure.

**Core Principle**: Economy supports gameplay pacing and meaningful choices, not academic simulation. Resources should be few enough to track mentally, scarce enough to matter, and interconnected enough to create interesting tradeoffs.

## Canonical Types Used

- **ResourceType**: Water, Food, Fuel, Materials, Salvage, Luxuries
- **ProductionChain**: Input → Process → Output transformation
- **TradeRoute**: Path between nodes with capacity and risk
- **SupplyState**: Per-entity inventory and consumption
- **EconomicPressure**: Shortage/surplus affecting governance

## Non-Goals

What economy does NOT model:

- **Microeconomic simulation**: No supply/demand curves, no price discovery algorithms
- **Individual transactions**: Trade happens at route level, not per-unit
- **Currency**: Resources are the currency; no abstract money layer
- **Complex logistics**: Supply is aggregate; no tracking individual cargo ships

## Resource Types

Six core resources, each with distinct sources and uses:

| Resource | Source | Use | Scarcity Creates |
|----------|--------|-----|------------------|
| **Water** | Desalination, ice mining | Life support, industry | Population crisis, forced relocation |
| **Food** | Kelp farms, fisheries, aquaculture | Population sustenance | Starvation, unrest, surrender |
| **Fuel** | Algae biofuel, salvaged reserves | Ship movement, production | Immobility, economic collapse |
| **Materials** | Mining, salvage, recycling | Construction, repairs | Degradation, no growth |
| **Salvage** | Wrecks, drowned cities, derelicts | Tech upgrades, components | Stagnation, vulnerability |
| **Luxuries** | Trade goods, culture, entertainment | Morale, political capital | Unrest, radicalization |

### Resource Properties

```rust
Resource {
    resource_type:  ResourceType
    quantity:       f32
    quality:        f32         // 0.0-1.0, affects efficiency
    decay_rate:     f32         // Per-tick loss (food spoils, fuel evaporates)
}
```

### Stockpile State

Every entity with population or operations maintains stockpiles:

```rust
SupplyState {
    stockpiles:         Map<ResourceType, Resource>
    consumption_rate:   Map<ResourceType, f32>      // Per-tick demand
    days_of_supply:     Map<ResourceType, f32>      // Derived: stockpile / consumption
    shortage_flags:     Set<ResourceType>           // Currently critical
}
```

## Production Chains

Resources transform through production chains. Chains require inputs, time, and sometimes specific facilities.

### Chain Definition

```rust
ProductionChain {
    chain_id:       ChainId
    name:           String
    inputs:         Vec<(ResourceType, f32)>    // What it consumes
    outputs:        Vec<(ResourceType, f32)>    // What it produces
    duration_ticks: u32                          // How long per batch
    facility_req:   Option<FacilityType>        // Required infrastructure
    crew_req:       u32                          // Crew to operate
}
```

### Core Chains

| Chain | Inputs | Outputs | Facility |
|-------|--------|---------|----------|
| **Desalination** | Fuel | Water | Desalination plant |
| **Kelp Farming** | Water | Food | Kelp farm |
| **Aquaculture** | Water, Food (seed) | Food (fish) | Fish pens |
| **Biofuel Refining** | Food (kelp) | Fuel | Refinery |
| **Salvage Processing** | Salvage (raw) | Materials, Salvage (refined) | Processing bay |
| **Manufacturing** | Materials, Fuel | Components | Factory |
| **Recycling** | Waste, Fuel | Materials | Recycler |

### Chain Efficiency

Production efficiency depends on:

- Crew skill and morale
- Facility condition
- Input quality
- Governor/manager competence (from People system)

```rust
efficiency = base_efficiency
           * crew_morale_modifier
           * facility_condition
           * input_quality_avg
           * manager_competence
```

## Trade Routes

Trade moves resources between nodes (arcologies, platforms, ports). Routes have capacity, risk, and political requirements.

### Route Definition

```rust
TradeRoute {
    route_id:       RouteId
    origin:         EntityId
    destination:    EntityId

    // Capacity
    capacity:       f32                 // Max throughput per tick
    current_flow:   Map<ResourceType, f32>

    // Risk
    threat_level:   f32                 // 0.0-1.0, piracy/hazard
    convoy_req:     bool                // Needs escort?

    // Politics
    treaty_req:     Option<TreatyId>    // Requires trade agreement?
    tariff:         f32                 // Percentage taken by route controller

    // State
    status:         RouteStatus         // Open, Contested, Blockaded, Destroyed
}
```

### Route Types

| Type | Capacity | Risk | Notes |
|------|----------|------|-------|
| **Convoy** | High | Medium | Scheduled bulk transport, escort-able |
| **Merchant** | Medium | Variable | Independent traders, opportunistic |
| **Smuggling** | Low | High | Bypasses blockades, illegal goods |
| **Emergency** | Low | High | Desperate measures, high cost |

### Blockades

Blockading a route requires:

- Military presence at chokepoint or destination
- Sustained patrol (costs fuel, ties up ships)
- Risk of combat with convoys/escorts

Blockade effects:

- Route capacity reduced or eliminated
- Prices spike at destination
- Political pressure on both sides

## Supply and Logistics

Ships and fleets consume supplies continuously. Running out has consequences.

### Consumption Model

```rust
ConsumptionRate {
    // Per tick, scales with crew/population
    water:      crew * 0.01
    food:       crew * 0.01
    fuel:       base_fuel + (speed_factor * mass)

    // Combat multipliers
    combat_fuel_mult:   2.0     // Combat burns more fuel
    combat_ammo:        variable // Weapon-dependent
}
```

### Supply States

| Days of Supply | Status | Effects |
|----------------|--------|---------|
| > 30 | Comfortable | Normal operations |
| 15-30 | Adequate | No penalty |
| 7-15 | Rationing | Morale penalty, efficiency drop |
| 1-7 | Critical | Severe penalties, unrest risk |
| 0 | Exhausted | Crisis: starvation, immobility, surrender |

### Resupply

Ships resupply at:

- **Friendly ports**: Full access, quick
- **Neutral ports**: Limited access, expensive
- **At sea**: From supply ships, slow and vulnerable
- **Salvage**: Emergency, unreliable

## Economic Pressure

Economy affects governance through **EconomicPressure**—a signal that translates resource state into political consequences.

### Pressure Calculation

```rust
EconomicPressure {
    prosperity:     f32     // -1.0 (crisis) to 1.0 (boom)
    stability:      f32     // 0.0-1.0, volatility inverse
    growth:         f32     // Rate of change

    // Per-resource breakdown
    shortages:      Vec<ResourceType>
    surpluses:      Vec<ResourceType>
}
```

### Effects on Governance

| Pressure | Governance Effect |
|----------|-------------------|
| High prosperity | +Legitimacy, +Political capital regen |
| Low prosperity | −Legitimacy, faction unrest |
| Critical shortage | Crisis event triggers |
| Trade disruption | InfoIntegrity drops (isolation) |
| Economic growth | +Action points for faction |

### Integration with Strategic AI

Faction action points (from governance.md) scale with economic strength:

```rust
action_point_rate = base_rate
                  * economy_mult       // From controlled resources
                  * trade_mult         // From active routes
                  * efficiency_mult    // From production chains
```

Economically dominant factions can act more often.

## Arcology Economics

Arcology-ships have internal economies that affect governance.

### Internal Economy State

```rust
ArcologyEconomy {
    // Production
    production_capacity:    Map<ChainId, f32>
    active_chains:          Vec<ChainId>

    // Population needs
    population:             u32
    consumption:            SupplyState
    satisfaction:           f32     // 0.0-1.0

    // Trade
    exports:                Vec<ResourceType>
    imports:                Vec<ResourceType>
    trade_balance:          f32

    // Reserves
    strategic_reserve:      SupplyState     // For emergencies
}
```

### Economic Sectors

Arcologies have sectors that compete for resources and political influence:

| Sector | Produces | Consumes | Political Faction |
|--------|----------|----------|-------------------|
| **Agriculture** | Food | Water, Fuel | Populists |
| **Industry** | Materials, Components | Fuel, Materials | Technocrats |
| **Commerce** | Luxuries, Services | Everything | Traders |
| **Military** | Security | Fuel, Materials | Militarists |
| **Administration** | Governance | Luxuries | Traditionalists |

Sector health affects the corresponding internal faction's influence and satisfaction.

## Land-Ocean Interdependence

The pitch establishes a critical asymmetry:

- **Land factions** control mega shipyards (only source of new capital ships)
- **Ocean factions** control food production (kelp farms, fisheries)

This creates mutual vulnerability:

### Blockade Dynamics

| Blockade Target | Effect | Political Pressure |
|-----------------|--------|-------------------|
| Land faction's food imports | Starvation, unrest | Land must negotiate or break blockade |
| Ocean faction's shipyard access | No new capital ships | Ocean must maintain relations or raid |

### Trade Leverage

Factions with control over critical chokepoints or resources have diplomatic leverage:

- **Food monopoly**: Can demand concessions
- **Shipyard access**: Can demand loyalty
- **Fuel reserves**: Can enable or disable fleets
- **Salvage sites**: Can trade technology access

## Data Contracts

### ResourceState Component

```rust
ResourceState {
    stockpiles:             Map<ResourceType, Resource>
    production_queues:      Vec<ProductionJob>
    consumption_forecast:   Map<ResourceType, f32>  // Next N ticks
    shortage_alerts:        Vec<ShortageAlert>
}

ProductionJob {
    chain_id:       ChainId
    progress:       f32         // 0.0-1.0
    efficiency:     f32
    assigned_crew:  u32
}

ShortageAlert {
    resource:       ResourceType
    severity:       f32         // 0.0-1.0
    days_until:     f32         // When stockpile exhausts
    cause:          ShortageCause
}
```

### TradeState Component

```rust
TradeState {
    routes:             Vec<TradeRoute>
    pending_shipments:  Vec<Shipment>
    trade_agreements:   Vec<TradeAgreement>
    blockade_status:    Map<RouteId, BlockadeInfo>
}

Shipment {
    shipment_id:    ShipmentId
    route_id:       RouteId
    cargo:          Map<ResourceType, f32>
    departure_tick: Tick
    arrival_tick:   Tick
    escort_id:      Option<FleetId>
}
```

## Plugins and Resolvers

### Economy Plugins

```yaml
ProductionPlugin:
  reads: [ResourceState, FacilityState, CrewState]
  emits: [ProductionProgress, ResourceDelta]
  behavior: "Advance production queues, emit resource changes"

ConsumptionPlugin:
  reads: [ResourceState, PopulationState, ShipState]
  emits: [ResourceDelta, ShortageAlert]
  behavior: "Calculate consumption, emit shortages"

TradePlugin:
  reads: [TradeState, RouteState, DiplomacyState]
  emits: [ShipmentComplete, TradeDisruption]
  behavior: "Process shipments, detect blockades"

EconomicPressurePlugin:
  reads: [ResourceState, TradeState, PopulationState]
  emits: [EconomicPressureUpdate]
  behavior: "Calculate prosperity, emit governance signals"
```

### Economy Resolver

The `EconomyResolver` handles:

- Applying resource deltas (production, consumption, trade)
- Processing shipment arrivals
- Triggering shortage events
- Updating economic pressure for governance

## MVP Staging

### P2 (Core Economy)

- [ ] 4 core resources: Water, Food, Fuel, Materials
- [ ] Basic production chains: Desalination, Kelp, Refining
- [ ] Simple trade routes between nodes
- [ ] Supply consumption for ships and arcologies
- [ ] Economic pressure → governance integration

### P3 (Full Economy)

- [ ] All 6 resources including Salvage and Luxuries
- [ ] Complex production chains with dependencies
- [ ] Trade route risk and convoy mechanics
- [ ] Blockade system
- [ ] Land-ocean interdependence
- [ ] Sector-based arcology economics

## Related Documents

- [Governance Design](governance.md) — How economy affects political systems
- [Factions Design](factions.md) — Faction economic specializations
- [World Requirements](../requirements/world.md) — Economy requirements
- [Architecture](architecture.md) — Strategic layer overview
- [Glossary](../vision/glossary.md) — Canonical terminology
