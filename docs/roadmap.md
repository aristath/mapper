# Roadmap

## Milestone 1: Technical Spine

- Create repository structure.
- Define offline pack shape.
- Add pack inspection, install, update, and selection CLI.
- Support generic downloadable region packs.
- Build one tiny local map tile experiment from any OSM extract.

## Milestone 2: Offline Map Spike

- Generate local vector tiles from one city extract.
- Render them in a simple MapLibre view.
- Apply a first pixel-city style.
- Confirm Linux desktop rendering first.

Current local-map scripts:

- `scripts/download-osm-extract.sh`
- `scripts/build-vector-tiles.sh`
- `scripts/inspect-vector-tiles.sh`

## Milestone 3: Offline Routing Spike

- Generate Valhalla graph tiles for the same city.
- Calculate routes locally.
- Render a route overlay on the local map.
- Validate walking and cycling routes.

## Milestone 4: App Shell

- Add Flutter app. Started with `apps/mapper_app`.
- Integrate MapLibre Native.
- Add pack list, pack install status, active-pack selection, and map screen.
- Bridge route requests to the local routing layer.

## Milestone 5: City Pack Builder

- Build repeatable pack generation.
- Add manifest validation.
- Add checksums.
- Add local search index.
- Add update metadata.
- Add coordinate coverage lookup for installed packs.
