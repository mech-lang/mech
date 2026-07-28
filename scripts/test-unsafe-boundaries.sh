#!/bin/sh
set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
audit="$repository_root/scripts/check-unsafe-boundaries.sh"
fixture=$(mktemp -d "${TMPDIR:-/tmp}/mech-unsafe-audit-tests.XXXXXX")
trap 'rm -rf "$fixture"' EXIT HUP INT TERM
allowlist="$fixture/allowlist"
output="$fixture/output"

reset_fixture() {
  rm -rf "$fixture/src"
  mkdir -p "$fixture/src"
  : > "$allowlist"
}

expect_rejection() {
  name=$1
  expected=$2
  if UNSAFE_BOUNDARY_ROOT="$fixture" \
    UNSAFE_BOUNDARY_ALLOWLIST="$allowlist" \
    "$audit" > "$output" 2>&1
  then
    echo "unsafe boundary fixture unexpectedly passed: $name" >&2
    exit 1
  fi
  if ! grep -Fq -- "$expected" "$output"; then
    echo "unsafe boundary fixture failed for the wrong reason: $name" >&2
    cat "$output" >&2
    exit 1
  fi
}

reset_fixture
printf 'fn main() { unsafe {} }\n' > "$fixture/src/unsafe_block.rs"
expect_rejection "unsafe block" "outside the exact allowlist"

reset_fixture
printf 'unsafe fn escape() {}\n' > "$fixture/src/unsafe_fn.rs"
expect_rejection "unsafe fn" "outside the exact allowlist"

reset_fixture
printf 'struct Marker;\nunsafe impl Sync for Marker {}\n' > "$fixture/src/unsafe_sync.rs"
expect_rejection "unsafe impl Sync" "outside the exact allowlist"

reset_fixture
printf 'fn cast(x: u32) -> f32 { transmute(x) }\n' > "$fixture/src/transmute.rs"
expect_rejection "transmute" "outside the exact allowlist"

reset_fixture
printf 'fn read(xs: &[u8]) -> u8 { *xs.get_unchecked(0) }\n' > "$fixture/src/get_unchecked.rs"
expect_rejection "get_unchecked" "outside the exact allowlist"

reset_fixture
printf 'fn pointer(value: &u8) -> *const u8 { value as *const u8 }\n' > "$fixture/src/raw_pointer.rs"
expect_rejection "raw pointer" "outside the exact allowlist"

reset_fixture
printf 'src/example.rs|ffi|owner|missing justification\n' > "$allowlist"
printf 'fn main() {}\n' > "$fixture/src/example.rs"
expect_rejection "malformed allowlist" "expected five fields"

reset_fixture
printf 'src/stale.rs|ffi|owner|pointer validity|test fixture\n' > "$allowlist"
printf 'fn main() {}\n' > "$fixture/src/stale.rs"
expect_rejection "stale allowlist" "contains no audited unsafe syntax"

reset_fixture
mkdir -p "$fixture/src/directory"
printf 'src/directory|ffi|owner|pointer validity|test fixture\n' > "$allowlist"
expect_rejection "directory allowlist" "names a directory"

reset_fixture
printf 'fn main() { unsafe {} }\n' > "$fixture/src/allowed.rs"
printf 'src/allowed.rs|ffi|owner|pointer validity|documented fixture\n' > "$allowlist"
UNSAFE_BOUNDARY_ROOT="$fixture" \
  UNSAFE_BOUNDARY_ALLOWLIST="$allowlist" \
  "$audit" > "$output" 2>&1
grep -Fq "unsafe boundary audit passed" "$output"

echo "unsafe boundary audit fixtures passed"
