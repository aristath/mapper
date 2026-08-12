# Offline Packs

An offline pack is the unit the user downloads.

Examples:

- `athens-core`
- `athens-metro`
- `attica`
- `berlin-core`

## Pack Goals

- Install and remove cleanly.
- Work without network after download.
- Support incremental updates later.
- Keep enough metadata to explain freshness and attribution.
- Keep rendering, routing, and search versions in sync.

## Draft Layout

```text
athens-metro.mapperpack/
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
  "id": "athens-metro",
  "name": "Athens Metro",
  "region": {
    "country": "GR",
    "bbox": [23.45, 37.75, 24.15, 38.25]
  },
  "version": "2026.08.12",
  "generated_at": "2026-08-12T00:00:00Z",
  "sources": {
    "osm": {
      "extract": "greece-latest.osm.pbf",
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

