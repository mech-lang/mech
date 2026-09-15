#!/bin/sh
set -eu
cd "$(dirname "$0")/../../../.."
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
exec cargo +nightly-2026-03-03 test --locked -p mech-runtime \
  --no-default-features --features full_compiler,full_source,resident-routing-source,compute \
  --test s8_replacement_gap_audit "$@" -- --nocapture --test-threads=1
