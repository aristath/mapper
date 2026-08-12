# Routing

Routing should be delegated to an existing open-source engine. Mapper should not
invent pathfinding, turn restriction handling, costing, or map matching.

## Preferred Engine

Valhalla is the first candidate because it is:

- OSM-based
- open source
- multimodal
- built around tiled graph data
- used on Linux, Android, and iOS
- suitable for offline regional routing

## Runtime Responsibilities

Valhalla should provide:

- route calculation
- turn-by-turn maneuvers
- route summaries
- costing per travel mode
- map matching for noisy GPS positions

Each routing-capable pack declares both `valhalla_tiles` and
`valhalla_config`. The config is generated inside the pack and points Valhalla at
the local tile archive, so app code can launch routing from resolved local asset
paths instead of guessing filenames.

Mapper should provide:

- UI route selection
- route preview rendering
- navigation session state
- off-route detection thresholds
- reroute triggers
- app-specific wording only where necessary

## Initial Modes

1. Walking
2. Cycling
3. Driving

Transit should wait until the pack pipeline can ingest and validate GTFS feeds.

## Trust Rules

- Critical instructions must stay clear and conventional.
- Pixel art must never hide the next maneuver.
- The debug build should include a normal geometry overlay for validating route
  correctness against the stylized map.
