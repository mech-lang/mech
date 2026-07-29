#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
generated="$repo_root/fuzz/corpus/generated"

mkdir -p "$generated/parse_document"

find "$repo_root/src/syntax/tests/fixtures/document" \
  -type f -name '*.mec' -print0 |
  while IFS= read -r -d '' source; do
    name="$(basename "$source")"
    cp "$source" "$generated/parse_document/fixture-$name"
  done

find "$repo_root/docs" "$repo_root/examples" \
  -type f -name '*.mec' -print0 |
  while IFS= read -r -d '' source; do
    relative="${source#"$repo_root"/}"
    name="$(printf '%s' "$relative" | tr '/ ' '__')"
    cp "$source" "$generated/parse_document/repository-$name"
  done

printf 'Refreshed local corpus in %s\n' "$generated"
