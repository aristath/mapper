# Mapper

Mapper is an offline-first maps and navigation app concept built around open
map infrastructure and a pixel-city visual language.

The product goal is simple:

> Download a city. Explore and navigate it offline as a readable game-like map.

## Target Platforms

- Linux
- Android
- iOS

## Stack Direction

- App shell: Flutter
- Map rendering: MapLibre Native
- Routing: Valhalla
- Pack tooling: Rust
- Offline storage: PMTiles/MBTiles, SQLite, Valhalla graph tiles
- Source data: OpenStreetMap, plus optional GTFS and elevation datasets

## Repository Layout

- `docs/` - architecture, pack format, rendering, routing, and roadmap notes.
- `crates/mapper-pack/` - Rust CLI entry point for offline city-pack tooling.

## Current Status

This repository is at the planning and technical-spike stage. The first
implementation milestone is a single-city offline prototype that can render a
local pixel-styled map and calculate a route without network access.

## Pack Tooling

Create an offline pack skeleton:

```bash
cargo run -p mapper-pack -- init \
  --out target/athens-metro.mapperpack \
  --id athens-metro \
  --name "Athens Metro" \
  --country GR \
  --bbox 23.45,37.75,24.15,38.25 \
  --version 2026.08.12 \
  --generated-at 2026-08-12T00:00:00Z \
  --osm-extract greece-latest.osm.pbf
```

Inspect a pack:

```bash
cargo run -p mapper-pack -- inspect target/athens-metro.mapperpack
```
