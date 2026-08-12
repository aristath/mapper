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
  --out target/region.mapperpack \
  --id region \
  --name "Region" \
  --country ZZ \
  --bbox 1.0,2.0,3.0,4.0 \
  --version 2026.08.12 \
  --generated-at 2026-08-12T00:00:00Z \
  --osm-extract region.osm.pbf
```

Inspect a pack:

```bash
cargo run -p mapper-pack -- inspect target/region.mapperpack
```

Check whether the external open-source pack builders are installed:

```bash
cargo run -p mapper-pack -- toolchain
```

Download an OpenStreetMap extract:

```bash
scripts/download-osm-extract.sh europe/monaco-latest.osm.pbf monaco-latest.osm.pbf
```

Build local vector tiles from an extract:

```bash
scripts/build-vector-tiles.sh data/sources/monaco-latest.osm.pbf data/work/monaco.pmtiles
```

Inspect a PMTiles archive:

```bash
scripts/inspect-vector-tiles.sh data/work/monaco.pmtiles
```

Attach generated vector tiles to a pack:

```bash
cargo run -p mapper-pack -- add-file \
  --pack target/region.mapperpack \
  --source data/work/monaco.pmtiles \
  --pack-path map/tiles.pmtiles \
  --kind vector_tiles \
  --feature rendering
```

Generate a basic local MapLibre style for the pack:

```bash
cargo run -p mapper-pack -- add-default-style \
  --pack target/region.mapperpack
```

Install a pack into a local pack store:

```bash
cargo run -p mapper-pack -- install \
  --pack target/region.mapperpack \
  --store target/installed-packs
```

Bundle a pack into a downloadable file:

```bash
cargo run -p mapper-pack -- bundle \
  --pack target/region.mapperpack \
  --out target/region.mapperpack.tar
```

Create or update a registry entry for a bundle:

```bash
cargo run -p mapper-pack -- registry-add \
  --registry target/registry.json \
  --pack target/region.mapperpack \
  --archive target/region.mapperpack.tar \
  --url https://example.test/packs/region.mapperpack.tar \
  --generated-at 2026-08-12T00:00:00Z
```

Install a downloaded bundle:

```bash
cargo run -p mapper-pack -- install-bundle \
  --archive target/region.mapperpack.tar \
  --store target/installed-packs
```

List downloadable packs from a registry:

```bash
cargo run -p mapper-pack -- registry-list \
  --registry target/registry.json
```

Download, verify, and install a pack from a registry:

```bash
cargo run -p mapper-pack -- install-from-registry \
  --registry target/registry.json \
  --id region \
  --cache target/pack-cache \
  --store target/installed-packs
```

List installed packs:

```bash
cargo run -p mapper-pack -- list --store target/installed-packs
```

Resolve the local path for an app asset:

```bash
cargo run -p mapper-pack -- asset \
  --pack target/installed-packs/region \
  --kind vector_tiles
```

Emit app-facing runtime JSON for an installed pack:

```bash
cargo run -p mapper-pack -- runtime-config \
  --pack target/installed-packs/region
```
