#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'EOF'
usage: run-valhalla-route-service.sh --store <pack-store> --from-lon <lon> --from-lat <lat> --to-lon <lon> --to-lat <lat> --mode <mode> [--runtime-config <valhalla.json>] [--concurrency <n>]
EOF
}

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

store=""
from_lon=""
from_lat=""
to_lon=""
to_lat=""
mode=""
runtime_config=""
concurrency="1"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --store) store="${2:-}"; shift 2 ;;
    --from-lon) from_lon="${2:-}"; shift 2 ;;
    --from-lat) from_lat="${2:-}"; shift 2 ;;
    --to-lon) to_lon="${2:-}"; shift 2 ;;
    --to-lat) to_lat="${2:-}"; shift 2 ;;
    --mode) mode="${2:-}"; shift 2 ;;
    --runtime-config) runtime_config="${2:-}"; shift 2 ;;
    --concurrency) concurrency="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *)
      printf 'unknown option: %s\n' "$1" >&2
      usage
      exit 2
      ;;
  esac
done

for required in store from_lon from_lat to_lon to_lat mode; do
  if [[ -z "${!required}" ]]; then
    printf 'missing --%s\n' "${required//_/-}" >&2
    usage
    exit 2
  fi
done

store="$(realpath -m "$store")"
if [[ -z "$runtime_config" ]]; then
  runtime_config="$root/target/valhalla-runtime/route-${mode}.json"
fi
runtime_config="$(realpath -m "$runtime_config")"

if [[ ! -d "$store" ]]; then
  printf 'missing pack store: %s\n' "$store" >&2
  exit 1
fi

if ! command -v valhalla_service >/dev/null 2>&1; then
  printf 'missing valhalla_service\n' >&2
  printf 'install Valhalla command-line tools, then rerun this script\n' >&2
  exit 1
fi

if command -v mapper-pack >/dev/null 2>&1; then
  mapper_pack=(mapper-pack)
else
  mapper_pack=(cargo run -q -p mapper-pack --)
fi

"${mapper_pack[@]}" valhalla-runtime-config-at \
  --store "$store" \
  --out "$runtime_config" \
  --from-lon "$from_lon" \
  --from-lat "$from_lat" \
  --to-lon "$to_lon" \
  --to-lat "$to_lat" \
  --mode "$mode"

printf 'starting valhalla_service with %s\n' "$runtime_config" >&2
exec valhalla_service "$runtime_config" "$concurrency"
