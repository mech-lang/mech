#!/bin/sh
set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repository_root"

fail_if_found() {
  description=$1
  pattern=$2
  shift 2
  if matches=$(rg -n "$pattern" "$@" 2>/dev/null); then
    echo "runtime boundary audit failed: $description" >&2
    echo "$matches" >&2
    exit 1
  fi
}

fail_if_found \
  "removed runtime execution bridges remain in production code" \
  'ACTIVE_RUNTIME_PROGRAM_HOST|RuntimeProgramHostTarget|ActiveRuntimeProgramHostGuard' \
  src --glob 'src/*/src/**/*.rs'

fail_if_found \
  "a public mutable runtime component accessor remains" \
  'pub[[:space:]]+fn[[:space:]]+(program_mut|take_program|store_mut|capability_kernel_mut|source_resolver_mut|host_registry_mut|host_policy_mut|scheduler_mut|scheduler_policy_mut|actor_behavior_driver_mut)[[:space:]]*[(<]' \
  src --glob 'src/*/src/**/*.rs'

fail_if_found \
  "a removed capability or host compatibility type remains" \
  'RuntimeCapabilityGrantRegistry|HostFunctionTransactionMode|ImmediateOnly' \
  src --glob 'src/*/src/**/*.rs'

# The interpreter defines the unsafe primitives and uses one primitive to
# implement another. Outside that implementation, only mech-program may invoke
# journal-aware unchecked interpreter entry points.
fail_if_found \
  "unchecked interpreter journal APIs are invoked outside mech-program" \
  '\.(advance_reactive_turn_with_journal(_and_services)?_unchecked|step_with_reactive_turn_journal(_and_services)?_unchecked)[[:space:]]*[(]' \
  src \
  --glob '*.rs' \
  --glob '!src/program/src/**' \
  --glob '!src/interpreter/src/interpreter.rs'

echo "runtime boundary audit passed"
