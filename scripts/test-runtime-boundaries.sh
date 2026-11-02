#!/bin/sh
set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
audit="$repository_root/scripts/check-runtime-wildcard-imports.sh"
fixture=$(mktemp -d "${TMPDIR:-/tmp}/mech-runtime-boundary-tests.XXXXXX")
trap 'rm -rf "$fixture"' EXIT HUP INT TERM
runtime_root="$fixture/src/runtime/src/runtime"
output="$fixture/output"

reset_fixture() {
  rm -rf "$fixture/src"
  mkdir -p "$runtime_root"
}

expect_acceptance() {
  name=$1
  if ! "$audit" "$fixture" > "$output" 2>&1; then
    echo "runtime boundary fixture unexpectedly failed: $name" >&2
    cat "$output" >&2
    exit 1
  fi
}

expect_rejection() {
  name=$1
  expected=$2
  if "$audit" "$fixture" > "$output" 2>&1; then
    echo "runtime boundary fixture unexpectedly passed: $name" >&2
    exit 1
  fi
  if ! grep -Fq -- "$expected" "$output"; then
    echo "runtime boundary fixture failed for the wrong reason: $name" >&2
    cat "$output" >&2
    exit 1
  fi
}

reset_fixture
printf 'use crate::runtime::MechRuntime;\n' > "$runtime_root/explicit.rs"
expect_acceptance "explicit import"

reset_fixture
printf 'use super::*;\n' > "$runtime_root/super.rs"
expect_rejection "super wildcard" "super.rs:1"

reset_fixture
printf 'use super::super::*;\n' > "$runtime_root/repeated_parent.rs"
expect_rejection "repeated-parent wildcard" "repeated_parent.rs:1"

reset_fixture
printf 'use crate::*;\n' > "$runtime_root/crate.rs"
expect_rejection "crate wildcard" "crate.rs:1"

reset_fixture
mkdir -p "$runtime_root/tests"
printf 'use super::*;\n' > "$runtime_root/tests/allowed.rs"
expect_acceptance "tests wildcard"

reset_fixture
mkdir -p "$runtime_root/input_tests"
printf 'use super::*;\n' > "$runtime_root/input_tests/allowed.rs"
expect_acceptance "input_tests wildcard"

reset_fixture
mkdir -p "$runtime_root/test_support"
printf 'use crate::*;\n' > "$runtime_root/test_support/allowed.rs"
expect_acceptance "test_support wildcard"

echo "runtime boundary audit fixtures passed"
