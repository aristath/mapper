#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 3 ]]; then
  printf 'usage: %s <pack-dir> [runtime-config.json] [concurrency]\n' "$0" >&2
  exit 2
fi

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
pack="$(realpath -m "$1")"
runtime_config="$(realpath -m "${2:-$root/target/valhalla-runtime/$(basename "$pack").json}")"
concurrency="${3:-1}"

if [[ ! -d "$pack" ]]; then
  printf 'missing pack directory: %s\n' "$pack" >&2
  exit 1
fi

if ! command -v valhalla_service >/dev/null 2>&1; then
  printf 'missing valhalla_service\n' >&2
  printf 'install Valhalla command-line tools, then rerun this script\n' >&2
  exit 1
fi

if command -v mapper-pack >/dev/null 2>&1; then
  mapper-pack valhalla-runtime-config --pack "$pack" --out "$runtime_config"
else
  cargo run -q -p mapper-pack -- valhalla-runtime-config --pack "$pack" --out "$runtime_config"
fi

printf 'starting valhalla_service with %s\n' "$runtime_config" >&2
exec valhalla_service "$runtime_config" "$concurrency"
