#!/bin/sh
set -eu

slice=${1:-all}

run_runtime_example() {
  cargo +nightly-2026-03-03 run -p mech-runtime \
    --no-default-features \
    --features source_default \
    --example "$1"
}

run_runtime_bin() {
  cargo +nightly-2026-03-03 run -p mech-runtime \
    --no-default-features \
    --features source_default \
    --bin "$1"
}

run_slice() {
  case "$1" in
    basic)
      run_runtime_example basic_runtime
      run_runtime_example runtime_dep_src
      run_runtime_example source_import_dependency
      ;;
    hosts)
      run_runtime_example arbitrary_rust_host_function
      run_runtime_example arbitrary_rust_host_functions2
      run_runtime_example host_value_roundtrip
      run_runtime_example runtime_services_host
      ;;
    actors)
      run_runtime_example actor_context_host
      run_runtime_example actor_state_host
      run_runtime_example scheduled_actor_native_functions
      run_runtime_example scheduled_actor_state_host
      run_runtime_example scheduled_runtime
      ;;
    modules)
      run_runtime_example dep_cycle
      run_runtime_example dep_diamond
      run_runtime_bin module_smoke
      ;;
    diagnostics)
      run_runtime_bin address_target_diagnostics
      ;;
    standard)
      cargo +nightly-2026-03-03 run \
        --no-default-features \
        --features "full_source,project" \
        --example matrix_multiply
      ;;
    *)
      echo "usage: $0 [basic|hosts|actors|modules|diagnostics|standard|all]" >&2
      exit 2
      ;;
  esac
}

case "$slice" in
  all)
    for group in basic hosts actors modules diagnostics standard
    do
      run_slice "$group"
    done
    ;;
  basic | hosts | actors | modules | diagnostics | standard)
    run_slice "$slice"
    ;;
  *)
    echo "usage: $0 [basic|hosts|actors|modules|diagnostics|standard|all]" >&2
    exit 2
    ;;
esac
