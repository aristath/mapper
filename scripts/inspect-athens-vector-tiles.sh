#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tiles="$root/data/work/athens-metro.pmtiles"

if [[ ! -f "$tiles" ]]; then
  printf 'missing %s\n' "$tiles" >&2
  printf 'run scripts/build-athens-vector-tiles.sh first\n' >&2
  exit 1
fi

if command -v pmtiles >/dev/null 2>&1; then
  pmtiles show "$tiles"
elif command -v docker >/dev/null 2>&1; then
  docker run --rm --pull missing \
    -v "$root/data:/data" \
    docker.io/protomaps/go-pmtiles:latest \
    show /data/work/athens-metro.pmtiles
elif command -v podman >/dev/null 2>&1; then
  podman run --rm --pull missing \
    -v "$root/data:/data:Z" \
    docker.io/protomaps/go-pmtiles:latest \
    show /data/work/athens-metro.pmtiles
else
  printf 'missing pmtiles and no docker/podman fallback found\n' >&2
  exit 1
fi
