#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

targets=(
  "$repo_root/fuzz/corpus/parse_document"
  "$repo_root/fuzz/corpus/incremental_equivalence"
  "$repo_root/fuzz/corpus/recovery_progress"
  "$repo_root/fuzz/corpus/edit_piece_table"
)

for target in "${targets[@]}"; do
  mkdir -p "$target"
  git -C "$repo_root" clean -fdqX -- "${target#"$repo_root"/}"
  mkdir -p "$target"
done

find "$repo_root" \
  \( \
    -path "$repo_root/.git" -o \
    -path "$repo_root/target" -o \
    -path "$repo_root/fuzz/target" -o \
    -path "$repo_root/fuzz/corpus" \
  \) -prune -o \
  -type f -name '*.mec' -print0 |
  while IFS= read -r -d '' source; do
    relative="${source#"$repo_root"/}"
    name="$(printf '%s' "$relative" | tr '/ ' '__')"
    for target in "${targets[@]}"; do
      cp "$source" "$target/generated-$name"
    done
  done

printf 'Refreshed target corpora under %s\n' "$repo_root/fuzz/corpus"
