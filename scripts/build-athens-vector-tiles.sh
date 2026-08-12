#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
input="$root/data/work/athens-metro.osm.pbf"
output_dir="$root/data/work"
output="$output_dir/athens-metro.pmtiles"

mkdir -p "$output_dir"

if [[ ! -f "$input" ]]; then
  printf 'missing %s\n' "$input" >&2
  printf 'run scripts/clip-athens-extract.sh first\n' >&2
  exit 1
fi

if command -v tilemaker >/dev/null 2>&1; then
  tilemaker "$input" --output "$output"
elif command -v docker >/dev/null 2>&1; then
  docker run --rm --pull missing \
    -v "$root/data:/data" \
    ghcr.io/systemed/tilemaker:master \
    /data/work/athens-metro.osm.pbf --output /data/work/athens-metro.pmtiles
elif command -v podman >/dev/null 2>&1; then
  podman run --rm --pull missing \
    -v "$root/data:/data:Z" \
    ghcr.io/systemed/tilemaker:master \
    /data/work/athens-metro.osm.pbf --output /data/work/athens-metro.pmtiles
else
  printf 'missing tilemaker and no docker/podman fallback found\n' >&2
  exit 1
fi

sha256sum "$output" > "$output.sha256"

printf 'wrote %s\n' "$output"
printf 'wrote %s\n' "$output.sha256"
