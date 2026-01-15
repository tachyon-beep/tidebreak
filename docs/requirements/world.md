# World Requirements

Requirements for economy, factions, governance, and world events.

See: [vision/pitch.md](../vision/pitch.md), [design/entity-framework.md](../design/entity-framework.md), [design/economy.md](../design/economy.md), [design/factions.md](../design/factions.md), [design/governance.md](../design/governance.md), [design/weather.md](../design/weather.md)

## Economy (P2)

- Support 6 core resource types:
  - Water (desalination, ice mining)
  - Food (kelp farms, fisheries, aquaculture)
  - Fuel (algae biofuel, salvaged reserves)
  - Materials (mining, salvage, recycling)
  - Salvage (wrecks, drowned cities, derelicts)
  - Luxuries (trade goods, culture, entertainment)
- Support production chains transforming inputs to outputs
- Support SupplyState tracking stockpiles and consumption
- Support days-of-supply calculation and shortage alerts
- Support ocean-based food production as primary food source
- Support land factions dependent on ocean food (blockade vulnerability)
- Support mega shipyards as land-controlled production monopoly

## Economy (P3)

- Support trade routes with capacity, risk, and political requirements
- Support convoy and merchant route types
- Support blockade mechanics reducing route capacity
- Support raiding and protection gameplay (economic warfare)
- Support EconomicPressure affecting governance (prosperity, stability)
- Support supply logistics affecting ship/platform operation
- Support arcology internal economics with sectors

## Factions (P2)

- Support FactionState component with holdings and resources
- Support faction Philosophy affecting AI decisions (militarism, commercialism, etc.)
- Support 2 test factions for MVP:
  - "The Thalassic Accord" (commercial, defensive)
  - "The Iron Dominion" (military, aggressive)
- Support faction identity (capital, philosophy, tech focus)
- Support land factions controlling shipyards
- Support ocean factions controlling food production
- Support player reputation per faction (-1.0 to 1.0)
- Support reputation events affecting standing

## Factions (P3)

- Support 4-6 distinct factions with unique philosophies
- Support Disposition system for faction-to-faction relations
- Support treaty system (non-aggression, trade, mutual defense, etc.)
- Support faction AI goal pursuit with action points
- Support faction defeat and respawn conditions
- Support strategic AI grace periods preventing early aggression

## Governance (P2)

- Support GovernanceState component for arcologies
- Support government types as parameterized bundles:
  - Autocracy (instant decisions, high instability risk)
  - Military Junta (fast military focus, coup risk)
  - Corporate Meritocracy (profit-driven, short-term bias)
  - Direct Democracy (slow, high legitimacy)
  - Representative Democracy (moderate speed, political gridlock)
  - Technocracy (efficiency focus, low popular engagement)
- Support decision latency varying by government type (strategic ticks, 1 tick = 1 day; see [glossary](../vision/glossary.md#time--ticks))
- Support Legitimacy (0.0-1.0) affecting compliance, morale, resistance
- Support Political Capital for override_modes (rush, delay, suppress)
- Support decision queue processing with government-specific procedures
- Support LeaderTraits affecting decision biases

## Governance (P3)

- Support InternalFactionsState tracking population subgroups
- Support internal faction radicalization and grievances
- Support crisis events stressing government systems
- Support government transitions (reform, revolution, coup)
- Support ConquestState machine (Occupied → Contested → Pacified → Integrated)
- Support InfoIntegrity linking isolation to governance outcomes
- Support council/assembly of mobile nations

## Weather (P1)

- Support WeatherState component with:
  - Sea state (0-9 Douglas scale)
  - Visibility (km, 0.0-20.0+)
  - Precipitation type (None, LightRain, HeavyRain, Storm)
  - Wind speed and direction
- Support weather effects on Surface layer:
  - Movement penalties at high sea state
  - Accuracy penalties in rough conditions
  - Small craft damage in severe weather
- Support weather effects on sensors:
  - Radar: High sensitivity (clutter, reduced range)
  - Visual: High sensitivity (fog, rain, spray)
  - Sonar: Low-Medium sensitivity (surface noise)
- Support static weather zones with base conditions
- Support weather in BattlePackage/observation space

## Weather (P2)

- Support moving storm systems (StormSystem entities)
- Support weather forecasting from meteorological equipment
- Support lightning effects (electronics damage, EMP-like)
- Support layer transition weather effects (timing, failure risk)
- Support weather-masked tactical operations

## Weather (P3)

- Support blueout events (magnetic storms disrupting electronics)
- Support seasonal weather patterns
- Support weather affecting TDM InfoIntegrity
- Support strategic weather (trade route hazards)

## World Events (P3)

- Support tide system affecting currents and shallow access
- Support large moving entities as terrain (whale migration)
- Support regenerative terrain (reef-forts)
- Support crisis events (plague, resource shortage, mutiny)
