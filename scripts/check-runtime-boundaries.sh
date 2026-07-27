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

# Runtime orchestration must use the program-owned finalization protocol and
# must never invoke lower-level interpreter journal coordination directly.
fail_if_found \
  "runtime invokes interpreter journal coordination directly" \
  '\.(advance_reactive_turn_with_journal(_and_services)?|step_with_reactive_turn_journal(_and_services)?)[[:space:]]*[(]' \
  src/runtime/src \
  --glob '*.rs'

fail_if_found \
  "production code declares a public journal-aware operation" \
  'pub[[:space:]]+fn[[:space:]]+[[:alnum:]_]*(with_journal|reactive_turn_journal)[[:alnum:]_]*[[:space:]]*[(]' \
  src \
  --glob 'src/*/src/**/*.rs'

fail_if_found \
  "a documentation-hidden public journal API remains" \
  '#\[doc\(hidden\)\]' \
  src/core/src/reactive_transaction.rs \
  src/core/src/functions.rs \
  src/interpreter/src/interpreter.rs \
  src/program/src/program.rs \
  --glob '*.rs'

fail_if_found \
  "runtime imports a lower-level reactive journal participant" \
  'ReactiveTurnJournal|ProgramReactiveTurnJournal|ReactiveJournalParticipant' \
  src/runtime/src \
  --glob '*.rs'

fail_if_found \
  "the physical reactive journal is publicly declared" \
  'pub[[:space:]]+struct[[:space:]]+ReactiveTurnJournal' \
  src/core/src \
  --glob '*.rs'

"$repository_root/scripts/check-unsafe-boundaries.sh"

echo "runtime boundary audit passed"
