#!/bin/sh
set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
baseline_manifest="$repository_root/tests/fixtures/function-system-baseline/Cargo.toml"
consumer_manifest="$repository_root/tests/fixtures/function-system-bytecode-consumer/Cargo.toml"

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

cargo +nightly-2026-03-03 run \
  --manifest-path "$baseline_manifest" \
  -- \
  --check "$repository_root/tests/architecture/function-system"

bash "$repository_root/scripts/check-standard-machine-baseline.sh"

cargo +nightly-2026-03-03 test \
  --manifest-path "$repository_root/Cargo.toml" \
  -p mech-runtime \
  --features linked_stdlib \
  --test function_system_source_contract \
  -- \
  --nocapture

consumer_tree=$(cargo +nightly-2026-03-03 tree \
  --manifest-path "$consumer_manifest" \
  -e features)

for package in mech-core mech-interpreter mech-program mech-math mech-range mech-string
do
  consumer_tree="$consumer_tree
$(cargo +nightly-2026-03-03 tree \
    --manifest-path "$consumer_manifest" \
    -e features \
    -i "$package")"
done

reject_tree_entry "$consumer_tree" "mech-bytecode v" "mech-bytecode in the consumer graph"
reject_tree_entry "$consumer_tree" 'mech-core feature "compiler"' "mech-core/compiler in the consumer graph"
reject_tree_entry "$consumer_tree" 'mech-interpreter feature "compiler"' "mech-interpreter/compiler in the consumer graph"
reject_tree_entry "$consumer_tree" 'mech-program feature "compiler"' "mech-program/compiler in the consumer graph"
reject_tree_entry "$consumer_tree" 'mech-math feature "compiler"' "mech-math/compiler in the consumer graph"
reject_tree_entry "$consumer_tree" 'mech-range feature "compiler"' "mech-range/compiler in the consumer graph"
reject_tree_entry "$consumer_tree" 'mech-string feature "compiler"' "mech-string/compiler in the consumer graph"

cargo +nightly-2026-03-03 run \
  --manifest-path "$consumer_manifest"

echo "function-system compatibility contracts passed"
