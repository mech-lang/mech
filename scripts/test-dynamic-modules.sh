#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${REPO_ROOT}"

TARGET_DIR="${REPO_ROOT}/target/dynamic-modules"
MODULE_DIR="${REPO_ROOT}/target/mech-modules"
PROFILE_DIR="${TARGET_DIR}/debug"
EXTERNAL_LOCK_DIR="${TARGET_DIR}/locks"
MATH_LOCKFILE="${EXTERNAL_LOCK_DIR}/math/Cargo.lock"
COMBINATORICS_LOCKFILE="${EXTERNAL_LOCK_DIR}/combinatorics/Cargo.lock"
STATUS_LOCKFILE="${EXTERNAL_LOCK_DIR}/status/Cargo.lock"

external_cargo() {
local lockfile="$1"
shift
cargo +nightly-2026-03-03 \
  -Z lockfile-path \
  --config "resolver.lockfile-path=\"${lockfile}\"" \
  "$@"
}

export CARGO_HTTP_MULTIPLEXING=false

case "$(uname -s)" in
Darwin*)
DYLIB_PREFIX="lib"
DYLIB_EXT="dylib"
;;
MINGW*|MSYS*|CYGWIN*)
DYLIB_PREFIX=""
DYLIB_EXT="dll"
;;
*)
DYLIB_PREFIX="lib"
DYLIB_EXT="so"
;;
esac

stage_module() {
local source_name="$1"
local staged_name="$2"

local source_path="${PROFILE_DIR}/${DYLIB_PREFIX}${source_name}.${DYLIB_EXT}"
local staged_path="${MODULE_DIR}/${DYLIB_PREFIX}${staged_name}.${DYLIB_EXT}"

if [[ ! -f "${source_path}" ]]; then
echo "missing dynamic module artifact: ${source_path}" >&2
exit 1
fi

cp "${source_path}" "${staged_path}"
}

ROOT_FEATURES="distribution-standard dynamic-modules"

mkdir -p \
  "${EXTERNAL_LOCK_DIR}/math" \
  "${EXTERNAL_LOCK_DIR}/combinatorics" \
  "${EXTERNAL_LOCK_DIR}/status"

echo "checking the supported distribution with dynamic modules enabled"
cargo +nightly-2026-03-03 check --locked --no-default-features --features "${ROOT_FEATURES}"

echo "testing math dynamic provider"
external_cargo "${MATH_LOCKFILE}" generate-lockfile --offline --manifest-path machines/math/Cargo.toml
external_cargo "${MATH_LOCKFILE}" test --locked --offline --manifest-path machines/math/Cargo.toml --no-default-features --features "dynamic-module"

echo "building math dynamic provider"
external_cargo "${MATH_LOCKFILE}" build --locked --offline --manifest-path machines/math/Cargo.toml --no-default-features --features "dynamic-module" --target-dir "${TARGET_DIR}"

echo "testing combinatorics dynamic provider"
external_cargo "${COMBINATORICS_LOCKFILE}" generate-lockfile --offline --manifest-path machines/combinatorics/Cargo.toml
external_cargo "${COMBINATORICS_LOCKFILE}" test --locked --offline --manifest-path machines/combinatorics/Cargo.toml --no-default-features --features "dynamic-module"

echo "building combinatorics dynamic provider"
external_cargo "${COMBINATORICS_LOCKFILE}" build --locked --offline --manifest-path machines/combinatorics/Cargo.toml --no-default-features --features "dynamic-module" --target-dir "${TARGET_DIR}"

echo "building dynamic status test provider"
external_cargo "${STATUS_LOCKFILE}" generate-lockfile --offline \
  --manifest-path tests/fixtures/dynamic-status-module/Cargo.toml
external_cargo "${STATUS_LOCKFILE}" build --locked --offline \
  --manifest-path tests/fixtures/dynamic-status-module/Cargo.toml \
  --target-dir "${TARGET_DIR}"

echo "staging dynamic modules"
rm -rf "${MODULE_DIR}"
mkdir -p "${MODULE_DIR}"

stage_module "mech_math" "mech_module_math"
stage_module "mech_combinatorics" "mech_module_combinatorics"
stage_module "mech_dynamic_status_test" "mech_module_status_test"

echo "running dynamic math integration tests"
MECH_MODULE_PATH="${MODULE_DIR}" cargo +nightly-2026-03-03 test --locked --test dynamic_math --no-default-features --features "${ROOT_FEATURES}"

echo "running dynamic combinatorics integration tests"
MECH_MODULE_PATH="${MODULE_DIR}" cargo +nightly-2026-03-03 test --locked --test dynamic_combinatorics --no-default-features --features "${ROOT_FEATURES}"

echo "running dynamic status failure tests"
MECH_MODULE_PATH="${MODULE_DIR}" \
  cargo +nightly-2026-03-03 test --locked \
    --test dynamic_status_failures \
    --no-default-features \
    --features "${ROOT_FEATURES}"

echo "running dynamic module smoke tests"
MECH_MODULE_PATH="${MODULE_DIR}" cargo +nightly-2026-03-03 test --locked --test dynamic_modules --no-default-features --features "${ROOT_FEATURES}"

echo "dynamic module smoke path passed"
