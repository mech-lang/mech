#!/bin/sh
set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repository_root"

"$repository_root/scripts/check-runtime-wildcard-imports.sh" \
  "$repository_root"

production_source_roots=$(find src \
  -mindepth 2 \
  -maxdepth 2 \
  -type d \
  -name src \
  -print)

fail_if_found() {
  description=$1
  pattern=$2
  shift 2
  matches=$(grep -REn \
    --include='*.rs' \
    -- "$pattern" \
    "$@" \
    2>/dev/null || true)
  if [ -n "$matches" ]; then
    echo "runtime boundary audit failed: $description" >&2
    echo "$matches" >&2
    exit 1
  fi
}

require_found() {
  description=$1
  pattern=$2
  shift 2
  if ! grep -Eq -- "$pattern" "$@"; then
    echo "runtime boundary audit failed: $description" >&2
    exit 1
  fi
}

fail_if_cloneable() {
  file=$1
  type_name=$2
  matches=$(awk -v type_name="$type_name" '
    function check_derive(line_number) {
      if (derive_text ~ /(^|[^[:alnum:]_])(Clone|Copy)([^[:alnum:]_]|$)/) {
        print line_number ":" derive_text
      }
      derive_text = ""
    }
    function check_impl(line_number) {
      if (impl_text ~ /(^|[^[:alnum:]_])(Clone|Copy)([^[:alnum:]_]|$)/ && impl_text ~ type_name) {
        print line_number ":" impl_text
      }
      impl_text = ""
      in_impl = 0
    }
    /^[[:space:]]*#\[derive/ {
      derive_text = $0
      in_derive = ($0 !~ /\]/)
      next
    }
    in_derive {
      derive_text = derive_text " " $0
      if ($0 ~ /\]/) {
        in_derive = 0
      }
      next
    }
    $0 ~ ("struct[[:space:]]+" type_name "([^[:alnum:]_]|$)") {
      check_derive(NR)
    }
    /^[[:space:]]*impl/ {
      impl_text = $0
      in_impl = ($0 !~ /\{/)
      if (!in_impl) {
        check_impl(NR)
      }
      next
    }
    in_impl {
      impl_text = impl_text " " $0
      if ($0 ~ /\{/) {
        check_impl(NR)
      }
      next
    }
    /^[[:space:]]*$/ {
      next
    }
    !/^[[:space:]]*#\[/ {
      derive_text = ""
    }
  ' "$file")
  if [ -n "$matches" ]; then
    echo "runtime boundary audit failed: $type_name must not implement Clone or Copy" >&2
    echo "$matches" >&2
    exit 1
  fi
}

fail_if_found \
  "removed runtime execution bridges remain in production code" \
  'ACTIVE_RUNTIME_PROGRAM_HOST|RuntimeProgramHostTarget|ActiveRuntimeProgramHostGuard' \
  $production_source_roots

fail_if_found \
  "a public mutable runtime component accessor remains" \
  'pub[[:space:]]+fn[[:space:]]+(program_mut|take_program|store_mut|capability_kernel_mut|source_resolver_mut|host_registry_mut|host_policy_mut|scheduler_mut|scheduler_policy_mut|actor_behavior_driver_mut)[[:space:]]*[(<]' \
  $production_source_roots

fail_if_found \
  "a removed capability or host compatibility type remains" \
  'RuntimeCapabilityGrantRegistry|HostFunctionTransactionMode|ImmediateOnly' \
  $production_source_roots

# Runtime orchestration must use the program-owned finalization protocol and
# must never invoke lower-level interpreter journal coordination directly.
fail_if_found \
  "runtime invokes interpreter journal coordination directly" \
  '\.(advance_reactive_turn_with_journal(_and_services)?|step_with_reactive_turn_journal(_and_services)?)[[:space:]]*[(]' \
  src/runtime/src

fail_if_found \
  "production code declares a public journal-aware operation" \
  'pub[[:space:]]+fn[[:space:]]+[[:alnum:]_]*(with_journal|reactive_turn_journal)[[:alnum:]_]*[[:space:]]*[(]' \
  $production_source_roots

fail_if_found \
  "a documentation-hidden public journal API remains" \
  '#\[doc\(hidden\)\]' \
  src/core/src/reactive_transaction.rs \
  src/core/src/functions.rs \
  src/interpreter/src/interpreter.rs \
  src/engine/src/program.rs

fail_if_found \
  "runtime imports a lower-level reactive journal participant" \
  'ReactiveTurnJournal|ProgramReactiveTurnJournal|ReactiveJournalParticipant' \
  src/runtime/src

fail_if_found \
  "the physical reactive journal is publicly declared" \
  'pub[[:space:]]+struct[[:space:]]+ReactiveTurnJournal' \
  src/core/src

fail_if_found \
  "reactive journal finalization remains non-affine" \
  'finalized:[[:space:]]*bool|pub[[:space:]]+fn[[:space:]]+commit[[:space:]]*\(&mut[[:space:]]+self\)|pub[[:space:]]+fn[[:space:]]+apply_restore_before[[:space:]]*\(&mut[[:space:]]+self\)' \
  src/core/src/reactive_transaction.rs

require_found \
  "reactive journal commit must consume its participant" \
  'pub[[:space:]]+fn[[:space:]]+commit[[:space:]]*\(self\)' \
  src/core/src/reactive_transaction.rs

require_found \
  "reactive journal rollback must consume its participant" \
  'pub[[:space:]]+fn[[:space:]]+apply_restore_before[[:space:]]*\(self\)' \
  src/core/src/reactive_transaction.rs

fail_if_cloneable \
  src/core/src/reactive_transaction.rs \
  ReactiveJournalParticipant

fail_if_cloneable \
  src/engine/src/program.rs \
  ProgramReactiveTurnJournal

"$repository_root/scripts/check-unsafe-boundaries.sh"

echo "runtime boundary audit passed"
