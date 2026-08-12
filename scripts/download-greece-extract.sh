#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out_dir="$root/data/sources"
url="https://download.geofabrik.de/europe/greece-latest.osm.pbf"
out="$out_dir/greece-latest.osm.pbf"

mkdir -p "$out_dir"

curl --fail --location --continue-at - --output "$out" "$url"
sha256sum "$out" > "$out.sha256"

printf 'downloaded %s\n' "$out"
printf 'wrote %s\n' "$out.sha256"
