#!/usr/bin/env bash
set -euo pipefail

EXAMPLE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$EXAMPLE_DIR/../.." && pwd)"
BUILD_DIR="${BUILD_DIR:-$ROOT/target/mech/metal-particle-field}"

if [[ ! -x "$BUILD_DIR/particle-field" ]]; then
  "$EXAMPLE_DIR/build.sh"
fi

source "$BUILD_DIR/layout.env"
exec "$BUILD_DIR/particle-field" \
  "$BUILD_DIR/particles.turn.metal" \
  "$EXAMPLE_DIR/render.metal" \
  "$BUILD_DIR/particles.initial.f32" \
  "$LANES" "$STATE_ELEMENTS" "$LANE_OFFSETS" "$SCALAR_OFFSETS"
