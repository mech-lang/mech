#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${REPO_ROOT}"

TARGET_DIR="${REPO_ROOT}/target/dynamic-modules"
MODULE_DIR="${REPO_ROOT}/target/mech-modules"
PROFILE_DIR="${TARGET_DIR}/debug"

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

echo "checking interpreter with dynamic modules enabled"
cargo check -p mech-interpreter --no-default-features --features "base dynamic-modules f64"

echo "testing math dynamic provider"
cargo test --manifest-path machines/math/Cargo.toml --no-default-features --features "dynamic-module"

echo "building math dynamic provider"
cargo build --manifest-path machines/math/Cargo.toml --no-default-features --features "dynamic-module" --target-dir "${TARGET_DIR}"

echo "testing combinatorics dynamic provider"
cargo test --manifest-path machines/combinatorics/Cargo.toml --no-default-features --features "dynamic-module"

echo "building combinatorics dynamic provider"
cargo build --manifest-path machines/combinatorics/Cargo.toml --no-default-features --features "dynamic-module" --target-dir "${TARGET_DIR}"

echo "staging dynamic modules"
rm -rf "${MODULE_DIR}"
mkdir -p "${MODULE_DIR}"

stage_module "mech_math" "mech_module_math"
stage_module "mech_combinatorics" "mech_module_combinatorics"

echo "running dynamic math integration tests"
MECH_MODULE_PATH="${MODULE_DIR}" cargo test --test dynamic_math --no-default-features --features "base dynamic-modules f64"

echo "running dynamic combinatorics integration tests"
MECH_MODULE_PATH="${MODULE_DIR}" cargo test --test dynamic_combinatorics --no-default-features --features "base dynamic-modules f64"

echo "running dynamic module smoke tests"
MECH_MODULE_PATH="${MODULE_DIR}" cargo test --test dynamic_modules --no-default-features --features "base dynamic-modules f64"

echo "dynamic module smoke path passed"
