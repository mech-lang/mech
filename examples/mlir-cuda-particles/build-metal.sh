#!/usr/bin/env bash
set -euo pipefail

EXAMPLE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$EXAMPLE_DIR/../.." && pwd)"
BUILD_DIR="${BUILD_DIR:-$ROOT/target/mech/mlir-metal-particles}"
TURNS="${TURNS:-10000}"

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
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "The Metal runner requires macOS." >&2
  exit 1
fi

mkdir -p "$BUILD_DIR"
cd "$ROOT"
CARGO_ARGS=(build --release -p mech --bin mech)
if [[ "${MECH_OFFLINE:-0}" == "1" ]]; then
  CARGO_ARGS+=(--offline)
fi
cargo "${CARGO_ARGS[@]}"

"$ROOT/target/release/mech" build \
  --aot \
  --emit mlir \
  --target apple:metal-f32 \
  --out "$BUILD_DIR/particles.spirv.mlir" \
  "$EXAMPLE_DIR"

STATE_ELEMENTS="$(sed -n 's|^// mech.state_len = ||p' "$BUILD_DIR/particles.spirv.mlir")"
LANES="$(sed -n 's|^// mech.batch_len = ||p' "$BUILD_DIR/particles.spirv.mlir")"
if [[ ! "$STATE_ELEMENTS" =~ ^[0-9]+$ || ! "$LANES" =~ ^[0-9]+$ ]]; then
  echo "generated SPIR-V MLIR is missing numeric Mech state metadata" >&2
  exit 1
fi

"$MLIR_OPT" \
  --no-implicit-module \
  "$BUILD_DIR/particles.spirv.mlir" \
  --spirv-update-vce \
  -o "$BUILD_DIR/particles.serializable.mlir"

"$MLIR_TRANSLATE" \
  --no-implicit-module \
  --serialize-spirv \
  "$BUILD_DIR/particles.serializable.mlir" \
  -o "$BUILD_DIR/particles.spv"

spirv-cross "$BUILD_DIR/particles.spv" \
  --entry mech_initialize \
  --stage comp \
  --msl \
  --msl-version 24000 \
  --output "$BUILD_DIR/particles.initialize.metal"

spirv-cross "$BUILD_DIR/particles.spv" \
  --entry mech_turn \
  --stage comp \
  --msl \
  --msl-version 24000 \
  --output "$BUILD_DIR/particles.turn.metal"

/usr/bin/clang \
  -O3 \
  -fobjc-arc \
  -framework Foundation \
  -framework Metal \
  "$EXAMPLE_DIR/metal_runner.m" \
  -o "$BUILD_DIR/particles-metal"

"$BUILD_DIR/particles-metal" \
  "$BUILD_DIR/particles.initialize.metal" \
  "$BUILD_DIR/particles.turn.metal" \
  "$LANES" \
  "$STATE_ELEMENTS" \
  "$TURNS"

echo "[Output] SPIR-V MLIR: $BUILD_DIR/particles.spirv.mlir"
echo "[Output] SPIR-V binary: $BUILD_DIR/particles.spv"
echo "[Output] Metal source: $BUILD_DIR/particles.turn.metal"
