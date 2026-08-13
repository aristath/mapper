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

This repository is at the technical-spike stage. The current implementation
focuses on generic downloadable region packs that can be installed, selected,
updated, and opened without network access.

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

Build local Valhalla routing tiles from an extract:

```bash
scripts/build-valhalla-tiles.sh \
  data/sources/monaco-latest.osm.pbf \
  data/work/valhalla_tiles.tar \
  data/work/valhalla
```

Inspect a PMTiles archive:

```bash
scripts/inspect-vector-tiles.sh data/work/monaco.pmtiles
```

Assemble a pack from generated artifacts:

```bash
scripts/assemble-pack.sh \
  --out target/region.mapperpack \
  --id region \
  --name "Region" \
  --country ZZ \
  --bbox 1.0,2.0,3.0,4.0 \
  --version 2026.08.12 \
  --generated-at 2026-08-12T00:00:00Z \
  --osm-extract region.osm.pbf \
  --vector-tiles data/work/monaco.pmtiles \
  --valhalla-tiles data/work/valhalla_tiles.tar \
  --routing-modes pedestrian,bicycle,auto \
  --bundle target/region.mapperpack.tar
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

Attach generated Valhalla graph tiles to a pack:

```bash
cargo run -p mapper-pack -- add-file \
  --pack target/region.mapperpack \
  --source data/work/valhalla_tiles.tar \
  --pack-path routing/valhalla_tiles.tar \
  --kind valhalla_tiles \
  --feature routing:pedestrian
```

Generate a local Valhalla config for the pack:

```bash
cargo run -p mapper-pack -- add-default-valhalla-config \
  --pack target/region.mapperpack
```

Materialize a Valhalla runtime config with local absolute paths:

```bash
cargo run -p mapper-pack -- valhalla-runtime-config \
  --pack target/region.mapperpack \
  --out target/valhalla-runtime/region.json
```

Materialize a Valhalla runtime config for the smallest route-capable installed pack:

```bash
cargo run -p mapper-pack -- valhalla-runtime-config-at \
  --store target/installed-packs \
  --out target/valhalla-runtime/route.json \
  --from-lon 1.5 \
  --from-lat 2.5 \
  --to-lon 2.5 \
  --to-lat 3.5 \
  --mode walking
```

Start a local Valhalla service for a pack:

```bash
scripts/run-valhalla-service.sh \
  target/region.mapperpack \
  target/valhalla-runtime/region.json
```

Start a local Valhalla service for the smallest route-capable installed pack:

```bash
scripts/run-valhalla-route-service.sh \
  --store target/installed-packs \
  --from-lon 1.5 \
  --from-lat 2.5 \
  --to-lon 2.5 \
  --to-lat 3.5 \
  --mode walking \
  --runtime-config target/valhalla-runtime/route.json
```

Emit a validated Valhalla route request for a pack:

```bash
cargo run -p mapper-pack -- route-request \
  --pack target/region.mapperpack \
  --from-lon 1.5 \
  --from-lat 2.5 \
  --to-lon 2.5 \
  --to-lat 3.5 \
  --mode walking
```

Send a validated route request to a local Valhalla service:

```bash
cargo run -p mapper-pack -- route \
  --pack target/region.mapperpack \
  --endpoint http://127.0.0.1:8002 \
  --from-lon 1.5 \
  --from-lat 2.5 \
  --to-lon 2.5 \
  --to-lat 3.5 \
  --mode walking
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

Compare a registry with the local installed pack store:

```bash
cargo run -p mapper-pack -- registry-status \
  --registry target/registry.json \
  --store target/installed-packs
```

Download, verify, and install a pack from a registry:

```bash
cargo run -p mapper-pack -- install-from-registry \
  --registry target/registry.json \
  --id region \
  --cache target/pack-cache \
  --store target/installed-packs
```

Download, verify, and replace an installed pack from a registry:

```bash
cargo run -p mapper-pack -- update-from-registry \
  --registry target/registry.json \
  --id region \
  --cache target/pack-cache \
  --store target/installed-packs
```

List installed packs:

```bash
cargo run -p mapper-pack -- list --store target/installed-packs
```

Select the installed pack the app should open:

```bash
cargo run -p mapper-pack -- active-set \
  --store target/installed-packs \
  --id region
```

Show the selected pack:

```bash
cargo run -p mapper-pack -- active-get --store target/installed-packs
```

Find installed packs that cover a position:

```bash
cargo run -p mapper-pack -- covering \
  --store target/installed-packs \
  --lon 2.0 \
  --lat 3.0
```

Select the smallest installed pack covering a position:

```bash
cargo run -p mapper-pack -- active-set-at \
  --store target/installed-packs \
  --lon 2.0 \
  --lat 3.0
```

List installed packs that can route between two points:

```bash
cargo run -p mapper-pack -- route-pack \
  --store target/installed-packs \
  --from-lon 1.5 \
  --from-lat 2.5 \
  --to-lon 2.5 \
  --to-lat 3.5 \
  --mode walking
```

Remove an installed pack:

```bash
cargo run -p mapper-pack -- uninstall \
  --store target/installed-packs \
  --id region
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

Emit runtime JSON for the selected installed pack:

```bash
cargo run -p mapper-pack -- active-runtime-config \
  --store target/installed-packs
```

Emit a validated Valhalla route request for the selected installed pack:

```bash
cargo run -p mapper-pack -- active-route-request \
  --store target/installed-packs \
  --from-lon 1.5 \
  --from-lat 2.5 \
  --to-lon 2.5 \
  --to-lat 3.5 \
  --mode walking
```

Emit a validated Valhalla route request using the smallest route-capable installed pack:

```bash
cargo run -p mapper-pack -- route-request-at \
  --store target/installed-packs \
  --from-lon 1.5 \
  --from-lat 2.5 \
  --to-lon 2.5 \
  --to-lat 3.5 \
  --mode walking
```

Send a validated active-pack route request to a local Valhalla service:

```bash
cargo run -p mapper-pack -- active-route \
  --store target/installed-packs \
  --endpoint http://127.0.0.1:8002 \
  --from-lon 1.5 \
  --from-lat 2.5 \
  --to-lon 2.5 \
  --to-lat 3.5 \
  --mode walking
```

Send a validated route request from the smallest route-capable installed pack
to a local Valhalla service:

```bash
cargo run -p mapper-pack -- route-at \
  --store target/installed-packs \
  --endpoint http://127.0.0.1:8002 \
  --from-lon 1.5 \
  --from-lat 2.5 \
  --to-lon 2.5 \
  --to-lat 3.5 \
  --mode walking
```

Emit one app-facing JSON snapshot of the local pack store:

```bash
cargo run -p mapper-pack -- store-snapshot \
  --store target/installed-packs
```
