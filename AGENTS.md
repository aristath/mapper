# Project Instructions

This project is about building a real offline-first maps and navigation app.
Work must meaningfully move that product forward.

## Priorities

- Prefer concrete implementation, prototypes, tests, and usable artifacts over
  ritualistic process.
- Do not add corporate scaffolding, status theater, templates, dashboards,
  observability layers, or process documents unless they directly unblock or
  improve the app.
- Observability is not a default priority for this project.
- Keep decisions grounded in the product: offline maps, offline routing,
  downloadable city packs, open-source infrastructure, and the pixel-city
  navigation experience.
- Do not reinvent infrastructure that mature open-source map projects already
  provide.
- When choosing work, favor the shortest path to a working vertical slice.

## Engineering Direction

- Use OpenStreetMap-derived data as the base.
- Prefer open-source components such as MapLibre, Valhalla, PMTiles/MBTiles,
  SQLite, and Rust tooling.
- Keep custom code focused on Mapper-specific value:
  - offline pack format and generation
  - pixel-city rendering style
  - app UX
  - local search integration
  - on-device routing integration
  - city-pack install/update flow

## Agent Behavior

- Do the work; do not substitute plans for progress when implementation is
  possible.
- Keep updates brief and practical.
- Avoid bureaucratic language.
- Preserve the literal scope the user requested.

