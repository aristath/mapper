# Offline Packs

An offline pack is the unit the user downloads.

Examples:

- `monaco`
- `berlin`
- `attica`
- `greece`

## Pack Goals

- Install and remove cleanly.
- Work without network after download.
- Support verified replacement updates.
- Keep enough metadata to explain freshness and attribution.
- Keep rendering, routing, and search versions in sync.
- Move across the network as one downloadable archive.

## App Runtime Contract

The app downloads a `.mapperpack.tar` bundle, unpacks it into a temporary local
directory, validates it, then installs it into the local pack store. An installed
pack is a local directory with a validated `manifest.json`.

The app should resolve assets by declared `kind`, not by guessing filenames.
The selected pack is store state, held in `active-pack.json`, not a file inside
the pack itself.
The app can read one store snapshot to get installed packs, the active pack, the
active runtime config, and warnings about stale local state.
When more than one installed pack covers the same lon/lat, the smallest bounding
box is preferred so a city pack wins over a broader regional pack.

Pack install must reject missing or corrupt assets. Every declared file is
checked against its manifest `bytes` and `sha256` before it is copied into the
local pack store.

Current asset kinds:

- `vector_tiles`
- `style_json`
- `valhalla_tiles`
- `search_index`
- `poi_index`
- `gtfs`

## Registry Contract

A pack registry is a JSON file with `schema`, `generated_at`, and a `packs`
array. Each pack entry declares `id`, `name`, `version`, `country`, `bbox`,
download `url`, archive `bytes`, archive `sha256`, and advertised `features`.

The app can use the registry to show downloadable regions. Installation must
download the selected bundle into a cache, verify its byte size and SHA-256, then
install the bundle through the normal pack installer.
Registry status combines downloadable pack metadata with the local store so the
app can show installed, update available, active, and not installed states.

Updates use the same registry verification path, unpack into a temporary store
directory, validate the bundle, confirm the manifest id matches the requested
installed pack, then replace the installed directory.

## Draft Layout

```text
region.mapperpack/
  manifest.json
  attribution.txt
  map/
    tiles.pmtiles
    style.json
    sprites.json
    sprites.png
  routing/
    valhalla_tiles.tar
    valhalla.json
  search/
    search.sqlite
  poi/
    poi.sqlite
  transit/
    gtfs.zip
  checksums.txt
```

## Manifest Draft

```json
{
  "schema": 1,
  "id": "region",
  "name": "Region",
  "region": {
    "country": "ZZ",
    "bbox": [1.0, 2.0, 3.0, 4.0]
  },
  "version": "2026.08.12",
  "generated_at": "2026-08-12T00:00:00Z",
  "sources": {
    "osm": {
      "extract": "region.osm.pbf",
      "license": "ODbL-1.0"
    }
  },
  "features": {
    "rendering": true,
    "routing": ["pedestrian", "bicycle", "car"],
    "search": true,
    "transit": false
  },
  "files": [
    {
      "path": "map/tiles.pmtiles",
      "kind": "vector_tiles",
      "bytes": 0,
      "sha256": ""
    }
  ]
}
```

## Open Questions

- Whether packs should be directories, zip archives, or a single SQLite-backed
  container.
- Whether PMTiles or MBTiles is the better first tile container.
- Whether Valhalla tiles should remain as a tar bundle or be packed into the
  same city-pack container.
- How much building geometry to include at each pack size.
