#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
  printf 'usage: %s <input.osm.pbf> <output.valhalla_tiles.tar> [work-dir]\n' "$0" >&2
  exit 2
fi

input="$(realpath "$1")"
output="$(realpath -m "$2")"
output_dir="$(dirname "$output")"
work_dir="$(realpath -m "${3:-$output_dir/valhalla-work}")"
tile_dir="$work_dir/tiles"
config="$work_dir/valhalla.json"
timezone_db="$work_dir/timezones.sqlite"
admin_db="$work_dir/admins.sqlite"

missing=()
for tool in valhalla_build_config valhalla_build_tiles; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    missing+=("$tool")
  fi
done

if [[ ${#missing[@]} -gt 0 ]]; then
  printf 'missing required Valhalla tool(s): %s\n' "${missing[*]}" >&2
  printf 'install Valhalla command-line tools, then rerun this script\n' >&2
  exit 1
fi

if [[ ! -f "$input" ]]; then
  printf 'missing %s\n' "$input" >&2
  exit 1
fi

mkdir -p "$output_dir" "$tile_dir"

valhalla_build_config \
  --mjolnir-tile-dir "$tile_dir" \
  --mjolnir-tile-extract "$output" \
  --mjolnir-timezone "$timezone_db" \
  --mjolnir-admin "$admin_db" \
  > "$config"

if command -v valhalla_build_timezones >/dev/null 2>&1; then
  valhalla_build_timezones > "$timezone_db"
else
  printf 'warning: valhalla_build_timezones not found; continuing without timezone db\n' >&2
fi

if command -v valhalla_build_admins >/dev/null 2>&1; then
  valhalla_build_admins -c "$config" "$input"
else
  printf 'warning: valhalla_build_admins not found; continuing without admin db\n' >&2
fi

valhalla_build_tiles -c "$config" "$input"

if command -v valhalla_build_extract >/dev/null 2>&1; then
  valhalla_build_extract -c "$config" -v
else
  find "$tile_dir" | sort -n | tar cf "$output" --no-recursion -T -
fi

sha256sum "$output" > "$output.sha256"

printf 'wrote %s\n' "$output"
printf 'wrote %s\n' "$output.sha256"
printf 'wrote %s\n' "$config"
