#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  printf 'usage: %s <input.osm.pbf> <output.pmtiles>\n' "$0" >&2
  exit 2
fi

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
input="$(realpath "$1")"
output="$(realpath -m "$2")"
output_dir="$(dirname "$output")"

mkdir -p "$output_dir"

if [[ ! -f "$input" ]]; then
  printf 'missing %s\n' "$input" >&2
  exit 1
fi

container_input="/data/input/$(basename "$input")"
container_output="/data/output/$(basename "$output")"

if command -v tilemaker >/dev/null 2>&1; then
  tilemaker "$input" --output "$output"
elif command -v podman >/dev/null 2>&1; then
  podman run --rm --pull missing \
    -v "$(dirname "$input"):/data/input:ro,Z" \
    -v "$output_dir:/data/output:Z" \
    ghcr.io/systemed/tilemaker:master \
    "$container_input" --output "$container_output"
elif command -v docker >/dev/null 2>&1; then
  docker run --rm --pull missing \
    -v "$(dirname "$input"):/data/input:ro,z" \
    -v "$output_dir:/data/output:z" \
    ghcr.io/systemed/tilemaker:master \
    "$container_input" --output "$container_output"
else
  printf 'missing tilemaker and no docker/podman fallback found\n' >&2
  exit 1
fi

if command -v pmtiles >/dev/null 2>&1; then
  pmtiles cluster "$output"
elif command -v podman >/dev/null 2>&1; then
  podman run --rm --pull missing \
    -v "$output_dir:/data/output:Z" \
    docker.io/protomaps/go-pmtiles:latest \
    cluster "$container_output"
elif command -v docker >/dev/null 2>&1; then
  docker run --rm --pull missing \
    -v "$output_dir:/data/output:z" \
    docker.io/protomaps/go-pmtiles:latest \
    cluster "$container_output"
else
  printf 'missing pmtiles and no docker/podman fallback found\n' >&2
  exit 1
fi

sha256sum "$output" > "$output.sha256"

printf 'wrote %s\n' "$output"
printf 'wrote %s\n' "$output.sha256"
