#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../../.." && pwd)
BUILD_DIR=${BUILD_DIR:-"$ROOT/target/mech/aot-mlir"}
PROGRAM=${PROGRAM:-"$ROOT/examples/aot-n-body"}
MLIR_BIN=${MLIR_BIN:-/opt/homebrew/opt/llvm/bin}
CC=${CC:-"$MLIR_BIN/clang"}

MLIR_OPT=${MLIR_OPT:-"$MLIR_BIN/mlir-opt"}
MLIR_TRANSLATE=${MLIR_TRANSLATE:-"$MLIR_BIN/mlir-translate"}

for tool in "$MLIR_OPT" "$MLIR_TRANSLATE" "$CC"; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "required tool not found: $tool" >&2
    exit 2
  fi
done

mkdir -p "$BUILD_DIR"
cargo build --manifest-path "$ROOT/Cargo.toml" --release --offline -p mech
"$ROOT/target/release/mech" build --aot "$PROGRAM" \
  --workspace-root "$ROOT" --offline --out "$BUILD_DIR/rust-aot"
"$ROOT/target/release/mech" build --aot --emit mlir "$PROGRAM" \
  --out "$BUILD_DIR/kernel.mlir"

"$MLIR_OPT" "$BUILD_DIR/kernel.mlir" \
  --canonicalize \
  --cse \
  --convert-scf-to-cf \
  --convert-cf-to-llvm \
  --convert-math-to-llvm \
  --convert-vector-to-llvm \
  --convert-arith-to-llvm \
  --finalize-memref-to-llvm \
  --convert-func-to-llvm \
  --reconcile-unrealized-casts \
  -o "$BUILD_DIR/kernel.lowered.mlir"
"$MLIR_TRANSLATE" --mlir-to-llvmir "$BUILD_DIR/kernel.lowered.mlir" \
  -o "$BUILD_DIR/kernel.ll"
if command -v xcrun >/dev/null 2>&1; then
  "$CC" -isysroot "$(xcrun --show-sdk-path)" -O3 -DNDEBUG \
    "$BUILD_DIR/kernel.ll" "$(dirname -- "$0")/runner.c" -o "$BUILD_DIR/mlir-aot"
else
  "$CC" -O3 -DNDEBUG "$BUILD_DIR/kernel.ll" "$(dirname -- "$0")/runner.c" \
    -o "$BUILD_DIR/mlir-aot"
fi

printf '%s\n' "$BUILD_DIR"
