#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  printf 'usage: %s <tiles.pmtiles>\n' "$0" >&2
  exit 2
fi

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tiles="$(realpath "$1")"

if [[ ! -f "$tiles" ]]; then
  printf 'missing %s\n' "$tiles" >&2
  exit 1
fi

container_tiles="/data/input/$(basename "$tiles")"

if command -v pmtiles >/dev/null 2>&1; then
  pmtiles show "$tiles"
elif command -v docker >/dev/null 2>&1; then
  docker run --rm --pull missing \
    -v "$(dirname "$tiles"):/data/input:ro" \
    docker.io/protomaps/go-pmtiles:latest \
    show "$container_tiles"
elif command -v podman >/dev/null 2>&1; then
  podman run --rm --pull missing \
    -v "$(dirname "$tiles"):/data/input:ro,Z" \
    docker.io/protomaps/go-pmtiles:latest \
    show "$container_tiles"
else
  printf 'missing pmtiles and no docker/podman fallback found\n' >&2
  exit 1
fi
