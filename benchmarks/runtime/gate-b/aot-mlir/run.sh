#!/usr/bin/env sh
set -eu

TURNS=${1:-1000000}
SAMPLES=${2:-5}
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
BUILD_DIR=${BUILD_DIR:-"$("$HERE/build.sh" | tail -n 1)"}

rust_checksum=$("$BUILD_DIR/rust-aot" --turns 1000 --guarantees fast | tail -n 1 | awk -F, '{print $7}')
mlir_checksum=$("$BUILD_DIR/mlir-aot" 1000 | tail -n 1 | awk -F, '{print $6}')
if [ "$rust_checksum" != "$mlir_checksum" ]; then
  echo "checksum mismatch at 1000 turns: rust=$rust_checksum mlir=$mlir_checksum" >&2
  exit 1
fi

echo "sample,implementation,turns,seconds,ns_per_turn,turns_per_second,state_checksum"
run_rust() {
  rust_row=$("$BUILD_DIR/rust-aot" --turns "$TURNS" --guarantees fast | tail -n 1)
  echo "$rust_row" | awk -F, -v sample="$sample" \
    '{print sample ",mech-aot-rust-fast," $3 "," $4 "," $5 "," $6 "," $7}'
}
run_mlir() {
  mlir_row=$("$BUILD_DIR/mlir-aot" "$TURNS" | tail -n 1)
  echo "$mlir_row" | awk -F, -v sample="$sample" \
    '{print sample "," $1 "," $2 "," $3 "," $4 "," $5 "," $6}'
}
sample=1
while [ "$sample" -le "$SAMPLES" ]; do
  if [ $((sample % 2)) -eq 1 ]; then
    run_rust
    run_mlir
  else
    run_mlir
    run_rust
  fi
  sample=$((sample + 1))
done
