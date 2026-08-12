#!/usr/bin/env bash
set -euo pipefail

turns="${1:-1000000}"
samples="${2:-5}"
root="$(cd "$(dirname "$0")/../../../.." && pwd)"
suite="$root/benchmarks/runtime/gate-b/nbody-languages"
build="$root/target/benchmarks/nbody"
mech_binary="$root/target/mech/aot-n-body"
rust_binary="$build/nbody-rust"
python_numpy="$build/venv/bin/python"

mkdir -p "$build" "$(dirname "$mech_binary")"

for tool in rustc lua luajit julia python3; do
  command -v "$tool" >/dev/null || {
    printf 'missing required runtime: %s\n' "$tool" >&2
    exit 1
  }
done
[[ -x "$python_numpy" ]] || {
  printf 'missing NumPy environment: %s\n' "$python_numpy" >&2
  exit 1
}

cargo build --release --manifest-path "$root/Cargo.toml" --offline
"$root/target/release/mech" build --aot "$root/examples/aot-n-body" \
  --workspace-root "$root" --offline --out "$mech_binary" >&2
rustc -C opt-level=3 -C target-cpu=native -C codegen-units=1 \
  "$suite/sources/nbody.rs" -o "$rust_binary"

verify_checkpoint() {
  local row="$1"
  local implementation
  implementation="${row%%,*}"
  if ! awk -F, '
    function abs(value) { return value < 0 ? -value : value }
    {
      if (abs($6 - -0.169075164) > 1e-8 ||
          abs($7 - -0.169087605) > 1e-8) {
        exit 1
      }
    }
  ' <<<"$row"; then
    printf 'failed N=1000 energy checkpoint: %s\n' "$implementation" >&2
    exit 1
  fi
}

verify_checkpoint "$($rust_binary 1000)"
verify_checkpoint "$(lua "$suite/sources/nbody.lua" 1000 lua-game-2)"
verify_checkpoint "$(luajit "$suite/sources/nbody.lua" 1000 luajit-game-2)"
verify_checkpoint "$(JULIA_LLVM_ARGS='-unroll-threshold=500' julia -O3 --startup-file=no \
  "$suite/sources/nbody.jl" 1000)"
verify_checkpoint "$(python3 -OO "$suite/sources/nbody.py" 1000)"
verify_checkpoint "$(OPENBLAS_NUM_THREADS=1 "$python_numpy" -OO \
  "$suite/sources/nbody_numpy.py" 1000)"
printf 'validated official N=1000 energy checkpoint\n' >&2

printf 'sample,implementation,turns,seconds,ns_per_turn,turns_per_second,initial_energy,final_energy,gc_seconds_or_collections,allocated_bytes_or_heap_delta,state_checksum\n'

run_one() {
  local sample="$1"
  local implementation="$2"
  local row
  case "$implementation" in
    mech-aot-fast)
      row="$($mech_binary --turns "$turns" --guarantees fast | tail -n 1)"
      IFS=, read -r _ _ measured_turns seconds ns_per_turn turns_per_second state_checksum _ <<<"$row"
      printf '%s,mech-aot-fast,%s,%s,%s,%s,,,0,0,%s\n' \
        "$sample" "$measured_turns" "$seconds" "$ns_per_turn" \
        "$turns_per_second" "$state_checksum"
      ;;
    rust-game-3)
      printf '%s,%s,\n' "$sample" "$($rust_binary "$turns")"
      ;;
    julia-game-5)
      row="$(JULIA_LLVM_ARGS='-unroll-threshold=500' julia -O3 --startup-file=no \
        "$suite/sources/nbody.jl" "$turns")"
      printf '%s,%s,\n' "$sample" "$row"
      ;;
    luajit-game-2)
      printf '%s,%s,\n' "$sample" "$(luajit "$suite/sources/nbody.lua" "$turns" luajit-game-2)"
      ;;
    lua-game-2)
      printf '%s,%s,\n' "$sample" "$(lua "$suite/sources/nbody.lua" "$turns" lua-game-2)"
      ;;
    python-game)
      printf '%s,%s,\n' "$sample" "$(python3 -OO "$suite/sources/nbody.py" "$turns")"
      ;;
    numpy-matrix)
      row="$(OPENBLAS_NUM_THREADS=1 "$python_numpy" -OO "$suite/sources/nbody_numpy.py" "$turns")"
      printf '%s,%s,\n' "$sample" "$row"
      ;;
  esac
}

implementations=(
  mech-aot-fast rust-game-3 julia-game-5 luajit-game-2
  lua-game-2 python-game numpy-matrix
)
count="${#implementations[@]}"
for sample in $(seq 1 "$samples"); do
  offset=$(((sample - 1) % count))
  for ordinal in $(seq 0 $((count - 1))); do
    index=$(((offset + ordinal) % count))
    run_one "$sample" "${implementations[$index]}"
  done
done
