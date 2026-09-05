#!/usr/bin/env bash
set -euo pipefail
cd "$1"
export CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
cargo +nightly-2026-03-03 fmt --all -- --check
python3 scripts/check-r5-memory-planner.py
python3 -B -m unittest scripts.tests.test_check_r5_memory_planner
cargo +nightly-2026-03-03 test --locked -p mech-core --all-features --test r5_memory_plan
cargo +nightly-2026-03-03 test --locked -p mech-engine --no-default-features --features full_compiler,resident-artifact --test r5_memory_plan
cargo +nightly-2026-03-03 test --locked -p mech-engine --no-default-features --features full_compiler,resident-artifact --lib
cargo +nightly-2026-03-03 test --locked -p mech-engine --no-default-features --features full_compiler,resident-artifact --test program_artifact_contract
python3 scripts/check-r1-compatibility-closure.py
python3 scripts/check-r2-type-memory-boundary.py
python3 scripts/check-r3-type-system.py
python3 scripts/check-r4-type-cutover.py
python3 scripts/check-bytecode-v1-format.py
git diff --check
git diff --exit-code "$BASE" -- Cargo.lock ':(glob)**/Cargo.toml'
