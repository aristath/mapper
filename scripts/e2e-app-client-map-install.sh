#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

(
  cd "$root/apps/mapper_app"
  dart run bin/e2e_install_geofabrik_region.dart
)
