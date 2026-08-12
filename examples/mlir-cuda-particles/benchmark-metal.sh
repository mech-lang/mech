#!/usr/bin/env bash
set -euo pipefail

EXAMPLE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$EXAMPLE_DIR/../.." && pwd)"
BUILD_DIR="${BUILD_DIR:-$ROOT/target/mech/mlir-metal-particles-benchmark}"
TURNS="${TURNS:-10000}"
SAMPLES="${SAMPLES:-5}"

if ! [[ "$TURNS" =~ ^[1-9][0-9]*$ && "$SAMPLES" =~ ^[1-9][0-9]*$ ]]; then
  echo "TURNS and SAMPLES must be positive integers" >&2
  exit 2
fi

mkdir -p "$BUILD_DIR"
BUILD_DIR="$BUILD_DIR" TURNS=1 "$EXAMPLE_DIR/build-metal.sh"

CPU_ARGS=(
  build --aot --workspace-root "$ROOT"
  --out "$BUILD_DIR/particles-cpu-f64"
)
if [[ "${MECH_OFFLINE:-0}" == "1" ]]; then
  CPU_ARGS+=(--offline)
fi
CPU_ARGS+=("$EXAMPLE_DIR")
"$ROOT/target/release/mech" "${CPU_ARGS[@]}"

"$ROOT/target/release/mech" build \
  --aot --emit rust --target cpu:f32 \
  --out "$BUILD_DIR/native_numeric.rs" \
  "$EXAMPLE_DIR"

MECH_NUMERIC_SOURCE="$BUILD_DIR/native_numeric.rs" \
  rustc -O -C target-cpu=native --edition 2024 \
  "$EXAMPLE_DIR/cpu_f32_runner.rs" \
  -o "$BUILD_DIR/particles-cpu-f32"

LANES="$(sed -n 's|^// mech.batch_len = ||p' "$BUILD_DIR/particles.spirv.mlir")"
STATE_ELEMENTS="$(sed -n 's|^// mech.state_len = ||p' "$BUILD_DIR/particles.spirv.mlir")"
if [[ ! "$LANES" =~ ^[0-9]+$ || ! "$STATE_ELEMENTS" =~ ^[0-9]+$ ]]; then
  echo "generated GPU module is missing state metadata" >&2
  exit 1
fi

GPU_RESULTS="$BUILD_DIR/gpu.csv"
CPU_F32_RESULTS="$BUILD_DIR/cpu-f32.csv"
CPU_F64_RESULTS="$BUILD_DIR/cpu-f64.csv"
: >"$GPU_RESULTS"
: >"$CPU_F32_RESULTS"
: >"$CPU_F64_RESULTS"

for _ in $(seq 1 "$SAMPLES"); do
  output="$("$BUILD_DIR/particles-metal" \
    "$BUILD_DIR/particles.initialize.metal" \
    "$BUILD_DIR/particles.turn.metal" \
    "$LANES" "$STATE_ELEMENTS" "$TURNS")"
  echo "$output"
  echo "$output" | sed -n '/^benchmark_csv/p' >>"$GPU_RESULTS"

  output="$("$BUILD_DIR/particles-cpu-f32" "$TURNS" "$LANES")"
  echo "$output"
  echo "$output" | sed -n '/^benchmark_csv/p' >>"$CPU_F32_RESULTS"

  output="$("$BUILD_DIR/particles-cpu-f64" \
    --turns "$TURNS" --guarantees fast)"
  echo "$output"
  seconds="$(echo "$output" | awk -F, '$1 == "fast" { print $4 }')"
  throughput="$(awk -v lanes="$LANES" -v turns="$TURNS" -v seconds="$seconds" \
    'BEGIN { printf "%.3f", lanes * turns / seconds / 1000000 }')"
  echo "benchmark_csv,cpu,f64,$TURNS,$seconds,$throughput" >>"$CPU_F64_RESULTS"
done

median_seconds() {
  awk -F, '{ print $5 }' "$1" | sort -n | \
    awk '{ values[NR] = $1 } END {
      if (NR % 2) print values[(NR + 1) / 2];
      else print (values[NR / 2] + values[NR / 2 + 1]) / 2;
    }'
}

gpu="$(median_seconds "$GPU_RESULTS")"
cpu_f32="$(median_seconds "$CPU_F32_RESULTS")"
cpu_f64="$(median_seconds "$CPU_F64_RESULTS")"
awk -v gpu="$gpu" -v cpu_f32="$cpu_f32" -v cpu_f64="$cpu_f64" \
    -v lanes="$LANES" -v turns="$TURNS" 'BEGIN {
  printf "\nmedian over %s particle-turns per sample\n", lanes * turns;
  printf "GPU f32: %.3f ms, %.3f million particle-turns/s\n", gpu * 1000, lanes * turns / gpu / 1000000;
  printf "CPU f32: %.3f ms, %.3f million particle-turns/s\n", cpu_f32 * 1000, lanes * turns / cpu_f32 / 1000000;
  printf "CPU f64: %.3f ms, %.3f million particle-turns/s\n", cpu_f64 * 1000, lanes * turns / cpu_f64 / 1000000;
  printf "GPU speedup over CPU f32: %.2fx\n", cpu_f32 / gpu;
  printf "GPU speedup over CPU f64: %.2fx\n", cpu_f64 / gpu;
}'
