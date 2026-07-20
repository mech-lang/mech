#!/usr/bin/env bash
set -uo pipefail

if [[ $# -lt 2 ]]; then
  echo "usage: $0 <summary label> <command> [args...]" >&2
  exit 2
fi

label="$1"
shift
start_seconds="$(date +%s)"

set +e
"$@"
status=$?
set -e

elapsed_seconds=$(( $(date +%s) - start_seconds ))
minutes=$(( elapsed_seconds / 60 ))
seconds=$(( elapsed_seconds % 60 ))

if [[ "$status" -eq 0 ]]; then
  result="passed"
else
  result="failed ($status)"
fi

printf 'CI timing: %s completed in %dm %02ds (%s)\n' \
  "$label" "$minutes" "$seconds" "$result"

if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
  printf '| `%s` | %dm %02ds | %s |\n' \
    "$label" "$minutes" "$seconds" "$result" >> "$GITHUB_STEP_SUMMARY"
fi

exit "$status"
