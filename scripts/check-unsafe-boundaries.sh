#!/bin/sh
set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repository_root"

fail_if_found() {
  description=$1
  pattern=$2
  shift 2
  if matches=$(rg -n "$pattern" "$@" 2>/dev/null); then
    echo "unsafe boundary audit failed: $description" >&2
    echo "$matches" >&2
    exit 1
  fi
}

# Approved unsafe-code allowlists are deliberately exact-file lists:
#
# - src/interpreter/src/modules.rs
#   Reason: dynamic-module ABI loading and calls.
#   Owner: mech-interpreter dynamic module boundary.
#   Safety invariant: imported symbols and views are validated before use and
#   remain alive for every call.
#   Justification: native FFI is required to load separately built modules.
#
# - src/core/src/value.rs
#   Reason: type-erased numerical matrix dispatch.
#   Owner: mech-core value representation.
#   Safety invariant: every transmute arm is selected only after matching the
#   concrete Value variant and its corresponding matrix element type.
#   Justification: preserves zero-copy numerical-kernel dispatch; replacing it
#   requires its own representation and benchmark work.
#
# There are currently no approved manual `unsafe impl Send` or
# `unsafe impl Sync` declarations.

unsafe_syntax='unsafe[[:space:]]*(\{|fn|impl|trait|extern)|#\[unsafe'

fail_if_found \
  "unsafe code exists in the runtime crate" \
  "$unsafe_syntax" \
  src/runtime --glob '*.rs'

fail_if_found \
  "unsafe code exists in the program crate" \
  "$unsafe_syntax" \
  src/program --glob '*.rs'

fail_if_found \
  "manual Send or Sync promises exist outside the empty allowlist" \
  'unsafe[[:space:]]+impl[[:space:]]+(Send|Sync)' \
  . --glob '*.rs' --glob '!target/**'

fail_if_found \
  "unsafe functions exist in orchestration, providers, CLI, or interpreter coordination" \
  'unsafe[[:space:]]+(extern[[:space:]]+"[^"]+"[[:space:]]+)?fn' \
  src/runtime \
  src/program \
  src/cli \
  src/lib.rs \
  hosts \
  src/interpreter/src/activation.rs \
  src/interpreter/src/builtins.rs \
  src/interpreter/src/functions.rs \
  src/interpreter/src/interpreter.rs \
  src/interpreter/src/mechdown.rs \
  src/interpreter/src/patterns.rs \
  src/interpreter/src/state_machines.rs \
  src/interpreter/src/statements.rs \
  src/interpreter/src/tracing.rs \
  --glob '*.rs'

transmute_matches=$(
  rg -n '(^|[^[:alnum:]_])((std|core)::mem::)?transmute[[:space:]]*[:<(]' \
    . \
    --glob '*.rs' \
    --glob '!target/**' \
    --glob '!src/core/src/value.rs' \
    2>/dev/null || true
)
if [ -n "$transmute_matches" ]; then
  echo "unsafe boundary audit failed: transmute exists outside the exact numerical allowlist" >&2
  echo "$transmute_matches" >&2
  exit 1
fi

fail_if_found \
  "a raw pointer is captured directly by a closure" \
  '(move[[:space:]]*\|[^|]*\|[^;\n]*\*(mut|const)|\|[^|]*:[[:space:]]*\*(mut|const)[^|]*\|)' \
  src/runtime \
  src/program \
  src/cli \
  hosts \
  src/interpreter/src \
  --glob '*.rs'

fail_if_found \
  "the removed runtime TLS/raw-pointer execution bridge was reintroduced" \
  '(ACTIVE_RUNTIME_PROGRAM_HOST|RuntimeProgramHostTarget|ActiveRuntimeProgramHostGuard|CURRENT_RUNTIME_(PTR|TARGET)|RUNTIME_(PTR|TARGET)_TLS)' \
  src --glob '*.rs'

fail_if_found \
  "unchecked journal coordination was reintroduced" \
  '(journal[^[:space:](]*_unchecked|_unchecked[^[:space:](]*journal)' \
  src/runtime \
  src/program \
  src/interpreter/src \
  --glob '*.rs'

fail_if_found \
  "runtime or program orchestration uses Value::as_unchecked" \
  'Value::as_unchecked|\.as_unchecked[[:space:]]*[(<]' \
  src/runtime \
  src/program \
  --glob '*.rs'

echo "unsafe boundary audit passed"
