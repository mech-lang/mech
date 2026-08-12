#!/usr/bin/env bash
set -euo pipefail

EXAMPLE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$EXAMPLE_DIR/../.." && pwd)"
BUILD_DIR="${BUILD_DIR:-$ROOT/target/mech/metal-particle-field}"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This prototype uses AppKit and Metal and therefore requires macOS." >&2
  exit 1
fi

if [[ -n "${MLIR_BIN:-}" ]]; then
  MLIR_OPT="$MLIR_BIN/mlir-opt"
  MLIR_TRANSLATE="$MLIR_BIN/mlir-translate"
elif [[ -x /opt/homebrew/opt/llvm/bin/mlir-opt ]]; then
  MLIR_OPT=/opt/homebrew/opt/llvm/bin/mlir-opt
  MLIR_TRANSLATE=/opt/homebrew/opt/llvm/bin/mlir-translate
else
  MLIR_OPT="$(command -v mlir-opt || true)"
  MLIR_TRANSLATE="$(command -v mlir-translate || true)"
fi

if [[ ! -x "$MLIR_OPT" || ! -x "$MLIR_TRANSLATE" ]]; then
  echo "LLVM MLIR tools were not found. Set MLIR_BIN to an LLVM 22 bin directory." >&2
  exit 1
fi
if ! command -v spirv-cross >/dev/null 2>&1; then
  echo "spirv-cross was not found. Install it with: brew install spirv-cross" >&2
  exit 1
fi

mkdir -p "$BUILD_DIR"
cd "$ROOT"
cargo build --release -p mech --bin mech

"$ROOT/target/release/mech" build \
  --aot --emit mlir --target apple:metal-f32-host-init \
  --out "$BUILD_DIR/particles.spirv.mlir" \
  "$EXAMPLE_DIR"

"$ROOT/target/release/mech" build \
  --aot --emit initial-state --target cpu:f32 \
  --out "$BUILD_DIR/particles.initial.f32" \
  "$EXAMPLE_DIR"

STATE_ELEMENTS="$(sed -n 's|^// mech.state_len = ||p' "$BUILD_DIR/particles.spirv.mlir")"
LANES="$(sed -n 's|^// mech.batch_len = ||p' "$BUILD_DIR/particles.spirv.mlir")"
LANE_OFFSETS="$(sed -n 's|^// mech.lane_state_offsets = ||p' "$BUILD_DIR/particles.spirv.mlir")"
SCALAR_OFFSETS="$(sed -n 's|^// mech.scalar_state_offsets = ||p' "$BUILD_DIR/particles.spirv.mlir")"

if [[ ! "$STATE_ELEMENTS" =~ ^[0-9]+$ || ! "$LANES" =~ ^[0-9]+$ ]]; then
  echo "generated SPIR-V MLIR is missing numeric Mech state metadata" >&2
  exit 1
fi
if [[ "$(awk -F, '{print NF}' <<<"$LANE_OFFSETS")" -ne 4 ]]; then
  echo "expected x, y, vx, and vy lane-state offsets; got: $LANE_OFFSETS" >&2
  exit 1
fi
if [[ "$(awk -F, '{print NF}' <<<"$SCALAR_OFFSETS")" -ne 4 ]]; then
  echo "expected pointer-x, pointer-y, pointer-down, and dt scalar offsets; got: $SCALAR_OFFSETS" >&2
  exit 1
fi

"$MLIR_OPT" --no-implicit-module \
  "$BUILD_DIR/particles.spirv.mlir" \
  --spirv-update-vce \
  -o "$BUILD_DIR/particles.serializable.mlir"

"$MLIR_TRANSLATE" --no-implicit-module --serialize-spirv \
  "$BUILD_DIR/particles.serializable.mlir" \
  -o "$BUILD_DIR/particles.spv"

spirv-cross "$BUILD_DIR/particles.spv" \
  --entry mech_turn --stage comp --msl --msl-version 24000 \
  --output "$BUILD_DIR/particles.turn.metal"

/usr/bin/clang -O3 -fobjc-arc \
  -framework Cocoa -framework Metal -framework MetalKit -framework QuartzCore \
  "$EXAMPLE_DIR/particle_field.m" \
  -o "$BUILD_DIR/particle-field"

printf '%s\n' \
  "LANES=$LANES" \
  "STATE_ELEMENTS=$STATE_ELEMENTS" \
  "LANE_OFFSETS=$LANE_OFFSETS" \
  "SCALAR_OFFSETS=$SCALAR_OFFSETS" \
  > "$BUILD_DIR/layout.env"

echo "Built $LANES Mech particles ($((STATE_ELEMENTS * 4)) resident bytes)."
echo "Run: $EXAMPLE_DIR/run.sh"
