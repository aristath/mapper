#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
input="$root/data/sources/greece-latest.osm.pbf"
output_dir="$root/data/work"
output="$output_dir/athens-metro.osm.pbf"
bbox="23.45,37.75,24.15,38.25"

mkdir -p "$output_dir"

if [[ ! -f "$input" ]]; then
  printf 'missing %s\n' "$input" >&2
  printf 'run scripts/download-greece-extract.sh first\n' >&2
  exit 1
fi

if command -v osmium >/dev/null 2>&1; then
  osmium extract --bbox "$bbox" --overwrite --output "$output" "$input"
elif command -v docker >/dev/null 2>&1; then
  docker run --rm \
    -v "$root/data:/data" \
    docker.io/iboates/osmium:latest \
    extract --bbox "$bbox" --overwrite --output /data/work/athens-metro.osm.pbf /data/sources/greece-latest.osm.pbf
elif command -v podman >/dev/null 2>&1; then
  podman run --rm \
    -v "$root/data:/data:Z" \
    docker.io/iboates/osmium:latest \
    extract --bbox "$bbox" --overwrite --output /data/work/athens-metro.osm.pbf /data/sources/greece-latest.osm.pbf
else
  printf 'missing osmium and no docker/podman fallback found\n' >&2
  exit 1
fi

sha256sum "$output" > "$output.sha256"

printf 'wrote %s\n' "$output"
printf 'wrote %s\n' "$output.sha256"
