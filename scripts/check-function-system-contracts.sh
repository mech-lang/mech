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

check_surface() {
  bash "$repository_root/scripts/check-static-distribution-profiles.sh" static
}

check_machines() {
  bash "$repository_root/scripts/check-standard-machine-baseline.sh"
}

check_source() {
  bash "$repository_root/scripts/check-static-distribution-profiles.sh" full-source
  cargo +nightly-2026-03-03 test --locked \
    --manifest-path "$repository_root/src/stdlib/Cargo.toml" \
    --no-default-features \
    --features full_compiler,matrix2,vector2 \
    --test specialization_contract
}

check_consumer() {
  bash "$repository_root/scripts/check-static-distribution-profiles.sh" full-runtime
}

case "$mode" in
  all)
    check_surface
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
