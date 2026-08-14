#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source_ref="${1:-europe/monaco-latest.osm.pbf}"
id="${2:-monaco}"
name="${3:-Monaco}"
country="${4:-MC}"
bbox="${5:-7.409205,43.724759,7.439939,43.751931}"
version="$(date -u +%Y.%m.%d)"
generated_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
work="$root/target/e2e-map-download/$id"
store="$root/target/e2e-installed-packs"
pbf="$work/$id.osm.pbf"
pmtiles="$work/$id.pmtiles"
pack="$work/$id.mapperpack"

rm -rf "$work" "$store"
mkdir -p "$work" "$store"

"$root/scripts/download-osm-extract.sh" "$source_ref" "$pbf"
"$root/scripts/build-vector-tiles.sh" "$pbf" "$pmtiles"
"$root/scripts/assemble-pack.sh" \
  --out "$pack" \
  --id "$id" \
  --name "$name" \
  --country "$country" \
  --bbox "$bbox" \
  --version "$version" \
  --generated-at "$generated_at" \
  --osm-extract "$pbf" \
  --vector-tiles "$pmtiles"

cargo run -q -p mapper-pack -- install --pack "$pack" --store "$store"
cargo run -q -p mapper-pack -- active-set --store "$store" --id "$id"
cargo run -q -p mapper-pack -- active-runtime-config --store "$store" > "$work/runtime.json"

test -s "$store/$id/map/tiles.pmtiles"
grep -q '"vector_tiles"' "$work/runtime.json"
grep -q "$store/$id/map/tiles.pmtiles" "$work/runtime.json"

printf 'e2e map download completed: %s\n' "$store/$id"
