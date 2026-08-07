#!/bin/sh
set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repository_root"
scratch=$(mktemp -d "${TMPDIR:-/tmp}/mech-engine-catalog-slice.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

core_patch="patch.crates-io.mech-core.path=\"$repository_root/src/core\""
math_target="$scratch/math-target"

fail() {
  echo "engine catalog boundary failed: $*" >&2
  exit 1
}

[ ! -e "$repository_root/src/program" ] || fail "obsolete src/program still exists"
[ -f "$repository_root/src/engine/Cargo.toml" ] || fail "src/engine/Cargo.toml is missing"

cargo +nightly-2026-03-03 metadata \
  --manifest-path "$repository_root/Cargo.toml" \
  --format-version 1 \
  --no-deps |
  python3 -c '
import json
import sys

metadata = json.load(sys.stdin)
members = set(metadata["workspace_members"])
names = {
    package["name"]
    for package in metadata["packages"]
    if package["id"] in members
}
if "mech-engine" not in names:
    raise SystemExit("engine catalog boundary failed: mech-engine is not a workspace member")
if "mech-program" in names:
    raise SystemExit("engine catalog boundary failed: mech-program remains a workspace member")
'

# Audit only active Rust and manifest inputs. Frozen architecture corpora are
# data contracts and are deliberately excluded. HTML formatter strings retain
# `mech-program*` CSS classes as presentation semantics, not crate identities.
active_name_hits=$(
  rg -n -H 'mech-program|mech_program' "$repository_root" \
    --glob 'Cargo.toml' \
    --glob '*.rs' \
    --glob '!target/**' \
    --glob '!tests/architecture/function-system/**' \
    --glob '!tests/architecture/legacy-bytecode/**' || true
)
unexpected_name_hits=$(
  printf '%s\n' "$active_name_hits" |
    awk '
      $0 == "" { next }
      index($0, "class=\\\"") && index($0, "mech-program") { next }
      { print }
    '
)
if [ -n "$unexpected_name_hits" ]; then
  printf '%s\n' "$unexpected_name_hits" >&2
  fail "active Rust or manifest input still names mech-program"
fi

if rg -n '"math/add"' \
  "$repository_root/src/interpreter/src/expressions/functions.rs"
then
  fail "generic named dispatch contains an operation-specific math/add literal"
fi

if rg -n 'migrated_runtime_function_ids' \
  "$repository_root/src/interpreter/src"
then
  fail "interpreter still contains hidden default-derived migration ownership"
fi

# Complete package suites run in their dedicated CI jobs. Keep this boundary
# focused on the catalog slice so it does not replay the PR0 compatibility suite.
cargo +nightly-2026-03-03 test \
  -p mech-core \
  --lib \
  function_catalog

cargo +nightly-2026-03-03 test \
  --manifest-path "$repository_root/machines/math/Cargo.toml" \
  --target-dir "$math_target" \
  --config "$core_patch" \
  --no-default-features \
  --features "program,compiler,functions,f64,i32,matrixd,row_vectord,vector2,add" \
  --lib \
  ops::add::tests

cargo +nightly-2026-03-03 test \
  -p mech-interpreter \
  --lib \
  catalog

cargo +nightly-2026-03-03 test \
  -p mech-engine \
  --lib \
  program_uses_and_retains_an_explicit_function_system

cargo +nightly-2026-03-03 test \
  -p mech-engine \
  --lib \
  program_checkpoint_restore_rolls_back_the_function_environment

cargo +nightly-2026-03-03 test \
  -p mech-runtime \
  --lib \
  custom_function_system_reaches_retained_and_runtime_created_programs

echo "engine catalog boundary passed"
