#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'EOF'
usage: assemble-pack.sh --out <pack-dir> --id <id> --name <name> --country <code> --bbox <min_lon,min_lat,max_lon,max_lat> --version <version> --generated-at <iso-date> --osm-extract <file> --vector-tiles <tiles.pmtiles> [--valhalla-tiles <valhalla_tiles.tar>] [--routing-modes <mode[,mode...]>] [--bundle <pack.tar>]
EOF
}

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

out=""
id=""
name=""
country=""
bbox=""
version=""
generated_at=""
osm_extract=""
vector_tiles=""
valhalla_tiles=""
routing_modes="pedestrian,bicycle,auto"
bundle=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out) out="${2:-}"; shift 2 ;;
    --id) id="${2:-}"; shift 2 ;;
    --name) name="${2:-}"; shift 2 ;;
    --country) country="${2:-}"; shift 2 ;;
    --bbox) bbox="${2:-}"; shift 2 ;;
    --version) version="${2:-}"; shift 2 ;;
    --generated-at) generated_at="${2:-}"; shift 2 ;;
    --osm-extract) osm_extract="${2:-}"; shift 2 ;;
    --vector-tiles) vector_tiles="${2:-}"; shift 2 ;;
    --valhalla-tiles) valhalla_tiles="${2:-}"; shift 2 ;;
    --routing-modes) routing_modes="${2:-}"; shift 2 ;;
    --bundle) bundle="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *)
      printf 'unknown option: %s\n' "$1" >&2
      usage
      exit 2
      ;;
  esac
done

for required in out id name country bbox version generated_at osm_extract vector_tiles; do
  if [[ -z "${!required}" ]]; then
    printf 'missing --%s\n' "${required//_/-}" >&2
    usage
    exit 2
  fi
done

out="$(realpath -m "$out")"
vector_tiles="$(realpath "$vector_tiles")"
if [[ -n "$valhalla_tiles" ]]; then
  valhalla_tiles="$(realpath "$valhalla_tiles")"
fi
if [[ -n "$bundle" ]]; then
  bundle="$(realpath -m "$bundle")"
fi

if [[ -e "$out" && -n "$(find "$out" -mindepth 1 -maxdepth 1 -print -quit)" ]]; then
  printf 'refusing to assemble into non-empty directory: %s\n' "$out" >&2
  exit 1
fi

if [[ ! -f "$vector_tiles" ]]; then
  printf 'missing vector tiles: %s\n' "$vector_tiles" >&2
  exit 1
fi

if [[ -n "$valhalla_tiles" && ! -f "$valhalla_tiles" ]]; then
  printf 'missing Valhalla tiles: %s\n' "$valhalla_tiles" >&2
  exit 1
fi

if command -v mapper-pack >/dev/null 2>&1; then
  mapper_pack=(mapper-pack)
else
  mapper_pack=(cargo run -q -p mapper-pack --)
fi

"${mapper_pack[@]}" init \
  --out "$out" \
  --id "$id" \
  --name "$name" \
  --country "$country" \
  --bbox "$bbox" \
  --version "$version" \
  --generated-at "$generated_at" \
  --osm-extract "$osm_extract"

"${mapper_pack[@]}" add-file \
  --pack "$out" \
  --source "$vector_tiles" \
  --pack-path map/tiles.pmtiles \
  --kind vector_tiles \
  --feature rendering

"${mapper_pack[@]}" add-default-style --pack "$out"

if [[ -n "$valhalla_tiles" ]]; then
  IFS=',' read -r -a modes <<< "$routing_modes"
  first_mode="${modes[0]:-pedestrian}"
  "${mapper_pack[@]}" add-file \
    --pack "$out" \
    --source "$valhalla_tiles" \
    --pack-path routing/valhalla_tiles.tar \
    --kind valhalla_tiles \
    --feature "routing:$first_mode"

  for mode in "${modes[@]:1}"; do
    "${mapper_pack[@]}" enable-feature \
      --pack "$out" \
      --feature "routing:$mode"
  done

  "${mapper_pack[@]}" add-default-valhalla-config --pack "$out"
fi

"${mapper_pack[@]}" inspect "$out"

if [[ -n "$bundle" ]]; then
  "${mapper_pack[@]}" bundle --pack "$out" --out "$bundle"
fi

printf 'assembled %s\n' "$out"
if [[ -n "$bundle" ]]; then
  printf 'bundled %s\n' "$bundle"
fi
