#!/bin/sh
set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/mech-bytecode-boundaries.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

runtime_machine_target="$scratch/runtime-machine-target"
compiler_machine_target="$scratch/compiler-machine-target"
runtime_program_target="$scratch/runtime-program-target"
compiler_program_target="$scratch/compiler-program-target"
runtime_runtime_target="$scratch/runtime-runtime-target"
producer_target="$scratch/producer-target"
consumer_target="$scratch/consumer-target"
bytecode_path="$scratch/add.mecb"
machine_manifest="$repository_root/machines/math/Cargo.toml"
producer_manifest="$repository_root/tests/fixtures/bytecode-compiler-producer/Cargo.toml"
consumer_manifest="$repository_root/tests/fixtures/bytecode-runtime-consumer/Cargo.toml"
core_patch="patch.crates-io.mech-core.path=\"$repository_root/src/core\""

require_tree_entry() {
  tree=$1
  entry=$2
  description=$3
  case $tree in
    *"$entry"*) ;;
    *)
      echo "bytecode compiler boundary failed: missing $description" >&2
      exit 1
      ;;
  esac
}

reject_tree_entry() {
  tree=$1
  entry=$2
  description=$3
  case $tree in
    *"$entry"*)
      echo "bytecode compiler boundary failed: found $description" >&2
      exit 1
      ;;
    *) ;;
  esac
}

cargo +nightly-2026-03-03 check \
  --manifest-path "$machine_manifest" \
  --target-dir "$runtime_machine_target" \
  --config "$core_patch" \
  --no-default-features \
  --features "program functions f64 add"

cargo +nightly-2026-03-03 check \
  --manifest-path "$machine_manifest" \
  --target-dir "$compiler_machine_target" \
  --config "$core_patch" \
  --no-default-features \
  --features "program compiler functions f64 add"

cargo +nightly-2026-03-03 check \
  --manifest-path "$repository_root/Cargo.toml" \
  --target-dir "$runtime_program_target" \
  -p mech-program \
  --no-default-features \
  --features "program functions symbol_table f64"

runtime_program_tree=$(cargo +nightly-2026-03-03 tree \
  --manifest-path "$repository_root/Cargo.toml" \
  -p mech-program \
  --no-default-features \
  --features "program functions symbol_table f64" \
  -e features)
runtime_program_tree="$runtime_program_tree
$(cargo +nightly-2026-03-03 tree \
  --manifest-path "$repository_root/Cargo.toml" \
  -p mech-program \
  --no-default-features \
  --features "program functions symbol_table f64" \
  -e features \
  -i mech-core)
$(cargo +nightly-2026-03-03 tree \
  --manifest-path "$repository_root/Cargo.toml" \
  -p mech-program \
  --no-default-features \
  --features "program functions symbol_table f64" \
  -e features \
  -i mech-interpreter)"
reject_tree_entry "$runtime_program_tree" "mech-bytecode v" "mech-bytecode in the runtime-only program graph"
reject_tree_entry "$runtime_program_tree" 'mech-core feature "compiler"' "mech-core/compiler in the runtime-only program graph"
reject_tree_entry "$runtime_program_tree" 'mech-interpreter feature "compiler"' "mech-interpreter/compiler in the runtime-only program graph"

cargo +nightly-2026-03-03 check \
  --manifest-path "$repository_root/Cargo.toml" \
  --target-dir "$compiler_program_target" \
  -p mech-program \
  --no-default-features \
  --features "compiler functions symbol_table f64"

compiler_program_tree=$(cargo +nightly-2026-03-03 tree \
  --manifest-path "$repository_root/Cargo.toml" \
  -p mech-program \
  --no-default-features \
  --features "compiler functions symbol_table f64" \
  -e features)
compiler_program_tree="$compiler_program_tree
$(cargo +nightly-2026-03-03 tree \
  --manifest-path "$repository_root/Cargo.toml" \
  -p mech-program \
  --no-default-features \
  --features "compiler functions symbol_table f64" \
  -e features \
  -i mech-core)
$(cargo +nightly-2026-03-03 tree \
  --manifest-path "$repository_root/Cargo.toml" \
  -p mech-program \
  --no-default-features \
  --features "compiler functions symbol_table f64" \
  -e features \
  -i mech-interpreter)"
require_tree_entry "$compiler_program_tree" "mech-bytecode v" "mech-bytecode in the compiler-enabled program graph"
require_tree_entry "$compiler_program_tree" 'mech-core feature "compiler"' "mech-core/compiler in the compiler-enabled program graph"
require_tree_entry "$compiler_program_tree" 'mech-interpreter feature "compiler"' "mech-interpreter/compiler in the compiler-enabled program graph"

cargo +nightly-2026-03-03 check \
  --manifest-path "$repository_root/Cargo.toml" \
  --target-dir "$runtime_runtime_target" \
  -p mech-runtime \
  --no-default-features \
  --features "program functions symbol_table f64"

runtime_runtime_tree=$(cargo +nightly-2026-03-03 tree \
  --manifest-path "$repository_root/Cargo.toml" \
  -p mech-runtime \
  --no-default-features \
  --features "program functions symbol_table f64" \
  -e features)
reject_tree_entry "$runtime_runtime_tree" "mech-bytecode v" "mech-bytecode in the runtime-only runtime graph"

producer_tree=$(cargo +nightly-2026-03-03 tree \
  --manifest-path "$producer_manifest" \
  -e features)
require_tree_entry "$producer_tree" "mech-bytecode v" "mech-bytecode in the producer graph"

consumer_tree=$(cargo +nightly-2026-03-03 tree \
  --manifest-path "$consumer_manifest" \
  -e features)
consumer_tree="$consumer_tree
$(cargo +nightly-2026-03-03 tree \
  --manifest-path "$consumer_manifest" \
  -e features \
  -i mech-core)
$(cargo +nightly-2026-03-03 tree \
  --manifest-path "$consumer_manifest" \
  -e features \
  -i mech-interpreter)"
reject_tree_entry "$consumer_tree" "mech-bytecode v" "mech-bytecode in the consumer graph"
reject_tree_entry "$consumer_tree" 'mech-core feature "compiler"' "mech-core/compiler in the consumer graph"
reject_tree_entry "$consumer_tree" 'mech-interpreter feature "compiler"' "mech-interpreter/compiler in the consumer graph"

cargo +nightly-2026-03-03 run \
  --manifest-path "$producer_manifest" \
  --target-dir "$producer_target" \
  -- "$bytecode_path"

cargo +nightly-2026-03-03 run \
  --manifest-path "$consumer_manifest" \
  --target-dir "$consumer_target" \
  -- "$bytecode_path"

echo "bytecode compiler boundaries passed"
