# Architecture

Mapper is an offline-first navigation app. It should not depend on a hosted map,
search, routing, or traffic API for its core experience.

## Product Shape

The user downloads a city or region pack. Once downloaded, the app can:

- render the map offline
- search places and addresses offline
- calculate routes offline
- snap GPS positions to the local path/road graph
- show turn-by-turn guidance offline

Network access is reserved for pack discovery, pack downloads, pack updates, and
optional community or account features.

## Runtime Components

### Flutter App

Flutter owns the cross-platform app shell:

- pack manager
- map screen chrome
- navigation HUD
- search UI
- settings
- onboarding
- Linux desktop integration
- Android and iOS application lifecycle

### MapLibre Native

MapLibre Native owns interactive GPU map rendering. The app should feed it local
tiles and a local style document from the downloaded pack.

The map style is where the pixel-city identity mostly lives. The underlying map
data stays real; the rendered world becomes simplified, chunky, and legible.

### Valhalla

Valhalla owns routing, guidance, costing, and map matching. It should run against
local graph tiles bundled in each city pack.

Initial routing modes:

- pedestrian
- bicycle
- car

Transit should be treated as a later milestone because it depends on GTFS
availability, freshness, and city-by-city quality.

### Mapper Core

Mapper-specific native code should stay focused:

- pack validation
- local search
- route/session state
- pack update metadata
- bridge APIs between Flutter and native engines

Rust is the preferred language for this layer.

## Build-Time Components

### Pack Builder

The pack builder converts regional source data into a shippable city pack:

- OpenStreetMap extract
- generated vector tiles
- generated Valhalla graph tiles
- generated local search index
- generated POI metadata
- generated pixel sprite atlas metadata
- manifest and integrity checks

### Pack Registry

A simple registry describes available packs and versions. It can start as static
JSON served from GitHub Pages or any plain HTTP endpoint.

