#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

targets=(
  "$repo_root/fuzz/corpus/parse_document"
  "$repo_root/fuzz/corpus/incremental_equivalence"
  "$repo_root/fuzz/corpus/recovery_progress"
)

for target in "${targets[@]}"; do
  mkdir -p "$target"
done

seed_paths=(
  "$repo_root/src/syntax/tests/fixtures/grammar/accepted"
  "$repo_root/src/syntax/tests/fixtures/grammar/rejected"
  "$repo_root/src/syntax/tests/fixtures/document/accepted"
  "$repo_root/src/syntax/tests/fixtures/document/malformed"
  "$repo_root/src/syntax/tests/fixtures/document/fuzz-regressions"
  "$repo_root/docs"
  "$repo_root/examples"
  "$repo_root/mika"
)

find "${seed_paths[@]}" -type f -name '*.mec' -print0 |
  while IFS= read -r -d '' source; do
    relative="${source#"$repo_root"/}"
    name="$(printf '%s' "$relative" | tr '/ ' '__')"
    for target in "${targets[@]}"; do
      cp "$source" "$target/generated-$name"
    done
  done

printf 'Refreshed target corpora under %s\n' "$repo_root/fuzz/corpus"
