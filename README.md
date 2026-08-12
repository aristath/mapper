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

