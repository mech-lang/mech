#!/bin/sh
set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/mech-distribution-sizes.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

report_path=${1:-"$repository_root/target/distribution-sizes.tsv"}
build_target="$scratch/release-target"

fail() {
  echo "distribution size report failed: $*" >&2
  exit 1
}

for utility in cargo python3 rustc rustup sed sort tr wc
do
  command -v "$utility" >/dev/null 2>&1 || fail "required utility '$utility' is unavailable"
done

rustup target list --installed | python3 -c \
  'import sys; raise SystemExit(0 if "wasm32-unknown-unknown" in sys.stdin.read().split() else 1)' \
  || fail "the wasm32-unknown-unknown Rust target is not installed"

cargo_nightly() {
  cargo +nightly-2026-03-03 "$@"
}

package_count() {
  tree_path="$scratch/package-count.tree"
  cargo_nightly tree \
    "$@" \
    -e normal,build \
    --no-dedupe \
    --prefix none \
    --format '{p}' > "$tree_path"
  sort -u "$tree_path" \
    | wc -l \
    | tr -d ' '
}

catalog_counts() {
  profile=$1
  features=$2
  output="$scratch/$profile.catalog-counts"
  CARGO_PROFILE_RELEASE_DEBUG=0 cargo_nightly test \
    --manifest-path "$repository_root/Cargo.toml" \
    -p mech-stdlib \
    --release \
    --no-default-features \
    --features "$features" \
    --test profile_contracts \
    --target-dir "$build_target" \
    distribution_size_report_catalog_counts \
    -- \
    --exact \
    --nocapture > "$output"
  python3 - "$output" <<'PY'
from pathlib import Path
import re
import sys

match = re.search(r"MECH_CATALOG_COUNTS\s+(\d+)\s+(\d+)", Path(sys.argv[1]).read_text())
if match is None:
    raise SystemExit("distribution size report failed: catalog count probe produced no result")
print("\t".join(match.groups()))
PY
}

selected_manifest="$repository_root/tests/fixtures/bytecode-runtime-consumer/Cargo.toml"
runtime_manifest="$repository_root/tests/fixtures/function-system-bytecode-consumer/Cargo.toml"
source_manifest="$repository_root/tests/fixtures/standard-source-runtime/Cargo.toml"
compiler_manifest="$repository_root/tests/fixtures/bytecode-compiler-producer/Cargo.toml"
host_target=$(rustc +nightly-2026-03-03 -vV | sed -n 's/^host: //p')
test -n "$host_target" || fail "could not determine the nightly toolchain host target"

CARGO_PROFILE_RELEASE_DEBUG=0 cargo_nightly build \
  --release \
  --manifest-path "$selected_manifest" \
  --target-dir "$build_target"
CARGO_PROFILE_RELEASE_DEBUG=0 cargo_nightly build \
  --release \
  --manifest-path "$runtime_manifest" \
  --target-dir "$build_target"
CARGO_PROFILE_RELEASE_DEBUG=0 cargo_nightly build \
  --release \
  --manifest-path "$source_manifest" \
  --target-dir "$build_target"
CARGO_PROFILE_RELEASE_DEBUG=0 cargo_nightly build \
  --release \
  --manifest-path "$compiler_manifest" \
  --target-dir "$build_target"
CARGO_PROFILE_RELEASE_DEBUG=0 cargo_nightly build \
  --release \
  --manifest-path "$repository_root/Cargo.toml" \
  -p mech-wasm \
  --target wasm32-unknown-unknown \
  --no-default-features \
  --features browser_project \
  --target-dir "$build_target"

selected_artifact="$build_target/release/bytecode-runtime-consumer"
runtime_artifact="$build_target/release/function-system-bytecode-consumer"
source_artifact="$build_target/release/standard-source-runtime"
compiler_artifact="$build_target/release/bytecode-compiler-producer"
wasm_artifact="$build_target/wasm32-unknown-unknown/release/mech_wasm.wasm"

for artifact in "$selected_artifact" "$runtime_artifact" "$source_artifact" "$compiler_artifact" "$wasm_artifact"
do
  test -f "$artifact" || fail "expected release artifact is missing: $artifact"
done

selected_packages=$(package_count \
  --manifest-path "$selected_manifest" \
  --target "$host_target")
runtime_packages=$(package_count \
  --manifest-path "$runtime_manifest" \
  --target "$host_target")
source_packages=$(package_count \
  --manifest-path "$source_manifest" \
  --target "$host_target")
compiler_packages=$(package_count \
  --manifest-path "$compiler_manifest" \
  --target "$host_target")
wasm_packages=$(package_count \
  --manifest-path "$repository_root/src/wasm/Cargo.toml" \
  --target wasm32-unknown-unknown \
  --no-default-features \
  --features browser_project)

cargo_nightly tree \
  --manifest-path "$repository_root/Cargo.toml" \
  -p mech-wasm \
  --target wasm32-unknown-unknown \
  --no-default-features \
  --features browser_project \
  -e features \
  -i mech-stdlib > "$scratch/wasm-stdlib-features.tree"
browser_stdlib_features=$(python3 - "$scratch/wasm-stdlib-features.tree" <<'PY'
from pathlib import Path
import re
import sys

features = sorted(set(re.findall(
    r'mech-stdlib feature "([^"]+)"',
    Path(sys.argv[1]).read_text(encoding="utf-8"),
)))
if not features:
    raise SystemExit("distribution size report failed: browser profile selected no stdlib features")
print(",".join(features))
PY
)

selected_counts=$(catalog_counts selected-runtime "runtime,f64,math_add")
runtime_counts=$(catalog_counts standard-runtime standard_runtime)
source_counts=$(catalog_counts standard-source standard_source)
compiler_counts=$(catalog_counts standard-compiler standard_compiler)
wasm_counts=$(catalog_counts wasm-browser-project "$browser_stdlib_features")

mkdir -p "$(dirname -- "$report_path")"
{
  printf 'profile\tartifact_path\tartifact_size_bytes\tresolved_package_count\truntime_factory_count\tsource_specializer_count\n'
  printf 'selected-bytecode-runtime\t%s\t%s\t%s\t%s\n' \
    "$selected_artifact" "$(wc -c < "$selected_artifact" | tr -d ' ')" "$selected_packages" "$selected_counts"
  printf 'standard-bytecode-runtime\t%s\t%s\t%s\t%s\n' \
    "$runtime_artifact" "$(wc -c < "$runtime_artifact" | tr -d ' ')" "$runtime_packages" "$runtime_counts"
  printf 'standard-source-runtime\t%s\t%s\t%s\t%s\n' \
    "$source_artifact" "$(wc -c < "$source_artifact" | tr -d ' ')" "$source_packages" "$source_counts"
  printf 'standard-compiler-tooling\t%s\t%s\t%s\t%s\n' \
    "$compiler_artifact" "$(wc -c < "$compiler_artifact" | tr -d ' ')" "$compiler_packages" "$compiler_counts"
  printf 'wasm-browser-project\t%s\t%s\t%s\t%s\n' \
    "$wasm_artifact" "$(wc -c < "$wasm_artifact" | tr -d ' ')" "$wasm_packages" "$wasm_counts"
} > "$report_path"

cat "$report_path"
