#!/usr/bin/env bash
set -euo pipefail

turns="${1:-1000000}"
samples="${2:-5}"
root="$(cd "$(dirname "$0")/../../../.." && pwd)"
binary="$root/target/mech/aot-ekf"

cargo run --release --manifest-path "$root/Cargo.toml" -- build \
  --aot "$root/examples/aot-ekf" \
  --workspace-root "$root" \
  --offline \
  --out "$binary"

printf 'sample,mode,guarantees,turns,seconds,ns_per_turn,turns_per_second,checksum\n'
for mode in fast atomic checked transactional; do
  for sample in $(seq 1 "$samples"); do
    row="$($binary --turns "$turns" --guarantees "$mode" | tail -n 1)"
    printf '%s,%s\n' "$sample" "$row"
  done
done
