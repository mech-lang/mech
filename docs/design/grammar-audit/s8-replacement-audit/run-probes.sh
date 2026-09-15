#!/bin/sh
set -eu
if [ "$#" -gt 1 ]; then
  printf '%s\n' 'usage: run-probes.sh [exact-audit-test-name]' >&2
  exit 2
fi
if [ "$#" -eq 1 ]; then
  case "$1" in
    semantic_replacement_witnesses|compiler_entry_point_witnesses|ordered_transitive_explicit_root_witness|source_catalog_census|browser_document_payload_witness|source_visibility_witnesses) ;;
    *)
      printf 'unknown audit test selector: %s\n' "$1" >&2
      exit 2
      ;;
  esac
fi
cd "$(dirname "$0")/../../../.."
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
exec cargo +nightly-2026-03-03 test --locked -p mech-runtime \
  --no-default-features --features full_compiler,full_source,resident-routing-source,compute \
  --test s8_replacement_gap_audit "$@" -- --exact --nocapture --test-threads=1
