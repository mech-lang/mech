#!/bin/sh
set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/mech-machine-lib-tests.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

core_patch="patch.crates-io.mech-core.path=\"$repository_root/src/core\""
abi_patch="patch.crates-io.mech-abi.path=\"$repository_root/src/abi\""

check_machine() {
  machine=$1

  echo "testing $machine default library configuration"
  case "$machine" in
    combinatorics)
      cargo +nightly-2026-03-03 test \
        --manifest-path "$repository_root/machines/$machine/Cargo.toml" \
        --target-dir "$scratch/$machine" \
        --config "$core_patch" \
        --config "$abi_patch" \
        --lib
      ;;
    *)
      cargo +nightly-2026-03-03 test \
        --manifest-path "$repository_root/machines/$machine/Cargo.toml" \
        --target-dir "$scratch/$machine" \
        --config "$core_patch" \
        --lib
      ;;
  esac
}

for machine in \
  math \
  compare \
  logic \
  range \
  matrix \
  set \
  string \
  stats \
  combinatorics
do
  check_machine "$machine"
done

echo "standard machine library tests passed"
