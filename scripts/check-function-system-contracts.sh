#!/bin/sh
set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/mech-function-system-contracts.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

mode=${1:-all}
case "$mode" in
  all | surface | machines | source | consumer) ;;
  *)
    echo "usage: $0 [all|surface|machines|source|consumer]" >&2
    exit 2
    ;;
esac

baseline_manifest="$repository_root/tests/fixtures/function-system-baseline/Cargo.toml"
consumer_manifest="$repository_root/tests/fixtures/function-system-bytecode-consumer/Cargo.toml"
baseline_target="$scratch/baseline-target"
consumer_target="$scratch/consumer-target"

reject_tree_entry() {
  tree=$1
  entry=$2
  description=$3
  case $tree in
    *"$entry"*)
      echo "function-system contract failed: found $description" >&2
      exit 1
      ;;
    *) ;;
  esac
}

check_surface() {
  CARGO_PROFILE_DEV_DEBUG=0 cargo +nightly-2026-03-03 run \
    --manifest-path "$baseline_manifest" \
    --target-dir "$baseline_target" \
    -- \
    --check "$repository_root/tests/architecture/function-system"

  CARGO_PROFILE_DEV_DEBUG=0 cargo +nightly-2026-03-03 run \
    --manifest-path "$baseline_manifest" \
    --target-dir "$baseline_target" \
    --no-default-features \
    -- \
    --check-runtime "$repository_root/tests/architecture/function-system"
}

check_machines() {
  bash "$repository_root/scripts/check-standard-machine-baseline.sh"
}

check_source() {
  cargo +nightly-2026-03-03 test \
    --manifest-path "$repository_root/Cargo.toml" \
    -p mech-runtime \
    --features linked_stdlib \
    --test function_system_source_contract \
    -- \
    --nocapture
}

check_consumer() {
  consumer_tree=$(cargo +nightly-2026-03-03 tree \
    --manifest-path "$consumer_manifest" \
    -e features)

  for package in mech-core mech-engine mech-math mech-range mech-string
  do
    consumer_tree="$consumer_tree
$(cargo +nightly-2026-03-03 tree \
      --manifest-path "$consumer_manifest" \
      -e features \
      -i "$package")"
  done

  reject_tree_entry "$consumer_tree" "mech-bytecode v" "mech-bytecode in the consumer graph"
  reject_tree_entry "$consumer_tree" 'mech-core feature "compiler"' "mech-core/compiler in the consumer graph"
  reject_tree_entry "$consumer_tree" 'mech-engine feature "compiler"' "mech-engine/compiler in the consumer graph"
  reject_tree_entry "$consumer_tree" 'mech-math feature "compiler"' "mech-math/compiler in the consumer graph"
  reject_tree_entry "$consumer_tree" 'mech-range feature "compiler"' "mech-range/compiler in the consumer graph"
  reject_tree_entry "$consumer_tree" 'mech-string feature "compiler"' "mech-string/compiler in the consumer graph"

  CARGO_PROFILE_DEV_DEBUG=0 cargo +nightly-2026-03-03 run \
    --manifest-path "$consumer_manifest" \
    --target-dir "$consumer_target"
}

case "$mode" in
  all)
    check_surface
    rm -rf "$baseline_target"
    check_machines
    check_source
    check_consumer
    ;;
  surface) check_surface ;;
  machines) check_machines ;;
  source) check_source ;;
  consumer) check_consumer ;;
esac

case "$mode" in
  all) echo "function-system compatibility contracts passed" ;;
  *) echo "function-system compatibility contract slice passed ($mode)" ;;
esac
