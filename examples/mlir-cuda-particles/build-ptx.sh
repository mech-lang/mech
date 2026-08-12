#!/usr/bin/env bash
set -euo pipefail

EXAMPLE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$EXAMPLE_DIR/../.." && pwd)"
BUILD_DIR="${BUILD_DIR:-$ROOT/target/mech/mlir-cuda-particles}"
GPU_CHIP="${GPU_CHIP:-sm_86}"
GPU_FEATURES="${GPU_FEATURES:-+ptx80}"

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
  --target "nvidia:$GPU_CHIP" \
  --out "$BUILD_DIR/particles.gpu.mlir" \
  "$EXAMPLE_DIR"

"$MLIR_OPT" "$BUILD_DIR/particles.gpu.mlir" \
  --gpu-lower-to-nvvm-pipeline="cubin-chip=$GPU_CHIP cubin-features=$GPU_FEATURES cubin-format=assembly opt-level=3 kernel-bare-ptr-calling-convention=true host-bare-ptr-calling-convention=false" \
  -o "$BUILD_DIR/particles.lowered.mlir"

"$MLIR_TRANSLATE" "$BUILD_DIR/particles.lowered.mlir" \
  --mlir-to-llvmir \
  -o "$BUILD_DIR/particles.host.ll"

echo "[Output] GPU MLIR: $BUILD_DIR/particles.gpu.mlir"
echo "[Output] Embedded PTX MLIR: $BUILD_DIR/particles.lowered.mlir"
echo "[Output] Host LLVM IR: $BUILD_DIR/particles.host.ll"
