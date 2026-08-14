#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

(
  cd "$root/apps/mapper_app"
  dart run bin/check_geofabrik_viewport.dart \
    18.0,36.0,26.5,42.0 \
    greece
)
