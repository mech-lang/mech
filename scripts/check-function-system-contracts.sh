#!/bin/sh
set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/mech-function-system-contracts.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

mode=${1:-all}
case "$mode" in
  all | surface | machines | source | consumer) ;;
  *)
    echo "usage: $0 [all|surface|machines|source|consumer]" >&2
    exit 2
    ;;
esac

baseline_manifest="$repository_root/tests/fixtures/function-system-baseline/Cargo.toml"
baseline_target="$scratch/baseline-target"

check_surface() {
  CARGO_PROFILE_DEV_DEBUG=0 cargo +nightly-2026-03-03 run \
    --manifest-path "$baseline_manifest" \
    --target-dir "$baseline_target" \
    -- \
    --check "$repository_root/tests/architecture/function-system"

  CARGO_PROFILE_DEV_DEBUG=0 cargo +nightly-2026-03-03 run \
    --manifest-path "$baseline_manifest" \
    --target-dir "$baseline_target" \
    --no-default-features \
    -- \
    --check-runtime "$repository_root/tests/architecture/function-system"
}

check_machines() {
  bash "$repository_root/scripts/check-standard-machine-baseline.sh"
}

check_source() {
  bash "$repository_root/scripts/check-static-distribution-profiles.sh" full-source
}

check_consumer() {
  bash "$repository_root/scripts/check-static-distribution-profiles.sh" full-runtime
}

case "$mode" in
  all)
    check_surface
    rm -rf "$baseline_target"
    check_machines
    check_source
    check_consumer
    ;;
  surface) check_surface ;;
  machines) check_machines ;;
  source) check_source ;;
  consumer) check_consumer ;;
esac

case "$mode" in
  all) echo "function-system compatibility contracts passed" ;;
  *) echo "function-system compatibility contract slice passed ($mode)" ;;
esac
