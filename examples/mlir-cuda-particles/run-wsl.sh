#!/usr/bin/env bash
set -euo pipefail

EXAMPLE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$EXAMPLE_DIR/../.." && pwd)"
BUILD_DIR="${BUILD_DIR:-$ROOT/target/mech/mlir-cuda-particles}"
CUDA_HOME="${CUDA_HOME:-/usr/local/cuda}"

"$EXAMPLE_DIR/build-ptx.sh"

if ! command -v nvidia-smi >/dev/null 2>&1; then
  echo "nvidia-smi was not found; run this inside WSL with NVIDIA GPU support enabled." >&2
  exit 1
fi
if [[ ! -f "$CUDA_HOME/include/cuda.h" ]]; then
  echo "cuda.h was not found under $CUDA_HOME; set CUDA_HOME to the CUDA toolkit root." >&2
  exit 1
fi

if [[ -n "${MLIR_BIN:-}" ]]; then
  CLANG="$MLIR_BIN/clang"
elif [[ -x /opt/homebrew/opt/llvm/bin/clang ]]; then
  CLANG=/opt/homebrew/opt/llvm/bin/clang
else
  CLANG="$(command -v clang || true)"
fi
if [[ ! -x "$CLANG" ]]; then
  echo "clang was not found. Set MLIR_BIN to the same LLVM 22 bin directory used for MLIR." >&2
  exit 1
fi

"$CLANG" -O3 \
  "$BUILD_DIR/particles.host.ll" \
  "$EXAMPLE_DIR/runner.c" \
  "$EXAMPLE_DIR/cuda_driver_runtime.c" \
  -I"$CUDA_HOME/include" \
  -L"$CUDA_HOME/lib64" \
  -lcuda -ldl -lm -pthread \
  -o "$BUILD_DIR/particles-gpu"

"$BUILD_DIR/particles-gpu"
