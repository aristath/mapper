#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  printf 'usage: %s <geofabrik-relative-path-or-url> [output-name.osm.pbf]\n' "$0" >&2
  printf 'example: %s europe/monaco-latest.osm.pbf monaco-latest.osm.pbf\n' "$0" >&2
  exit 2
fi

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out_dir="$root/data/sources"
source_ref="$1"
output_name="${2:-$(basename "$source_ref")}"

case "$source_ref" in
  http://*|https://*) url="$source_ref" ;;
  *) url="https://download.geofabrik.de/$source_ref" ;;
esac

out="$out_dir/$output_name"

mkdir -p "$out_dir"

curl --fail --location --continue-at - --output "$out" "$url"
sha256sum "$out" > "$out.sha256"

printf 'downloaded %s\n' "$out"
printf 'wrote %s\n' "$out.sha256"
