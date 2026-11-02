#!/usr/bin/env sh
set -eu

needle='integrity_invalid_host_input_aborts_staged_receiver_before_commit'

test_list="$(cargo test -p mech-runtime --lib -- --list)"

matches="$(
  printf '%s\n' "$test_list" |
    grep "${needle}: test$" || true
)"

count="$(
  printf '%s\n' "$matches" |
    sed '/^[[:space:]]*$/d' |
    wc -l |
    tr -d ' '
)"

if [ "$count" -ne 1 ]; then
  printf '%s\n' "$test_list"
  printf >&2 "Expected exactly one '%s' test; found %s.\n" "$needle" "$count"
  exit 1
fi

qualified_name="$(
  printf '%s\n' "$matches" |
    sed 's/: test$//'
)"

printf 'Running exactly one test: %s\n' "$qualified_name"

cargo test -p mech-runtime --lib "$qualified_name" -- --exact --nocapture
