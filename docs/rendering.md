# Rendering

The visual identity is a pixelated city-builder map over real geographic data.

The renderer should simplify the city without falsifying it:

- roads remain connected to the real road graph
- buildings remain recognizable as buildings
- parks, water, rail, stations, and major landmarks are prominent
- navigation instructions remain conventional and readable

## Renderer Stack

- MapLibre Native for interactive rendering.
- Local vector tiles from the pack.
- A MapLibre style JSON generated or bundled with each pack.
- Pixel sprite atlas for POIs, transit stops, landmarks, user marker, and route
  events.

## Style Principles

- Route line is always the highest-priority visual object.
- Current position is exact, even if represented as a character or vehicle icon.
- Intersections and crossings can be visually emphasized.
- Roads and paths can be chunky, but should not obscure turn precision.
- Labels should be sparse and useful.
- Walking mode should emphasize crossings, stairs, alleys, plazas, parks, and
  station entrances.
- Cycling mode should emphasize bike lanes, low-stress streets, elevation, and
  crossings.
- Driving mode should stay calmer and more conventional.

## First Spike

Use one small OSM extract and generate a local tile set with a deliberately small
style surface:

- water
- parks
- buildings
- major roads
- minor roads
- footpaths
- rail
- POI symbols
- route overlay

