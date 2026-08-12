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
- Support incremental updates later.
- Keep enough metadata to explain freshness and attribution.
- Keep rendering, routing, and search versions in sync.

## App Runtime Contract

The app should treat an installed pack as a local directory with a validated
`manifest.json`. It should resolve assets by declared `kind`, not by guessing
filenames.

Pack install must reject missing or corrupt assets. Every declared file is
checked against its manifest `bytes` and `sha256` before it is copied into the
local pack store.

Current asset kinds:

- `vector_tiles`
- `valhalla_tiles`
- `search_index`
- `poi_index`
- `gtfs`

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
