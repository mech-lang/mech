#!/usr/bin/env bash
# Keep the event's comparison base even if GitHub merged a newer base tip.
set -euo pipefail

base=${1:?expected the PR event base SHA}
head=${2:?expected the exact checked-out merge SHA}
for revision in "$base" "$head"
do
  if ! [[ "$revision" =~ ^[0-9a-fA-F]{40}$ || "$revision" =~ ^[0-9a-fA-F]{64}$ ]]
  then
    echo "impact history requires exact commit SHAs" >&2
    exit 64
  fi
done
test "$(git rev-parse HEAD)" = "$head"

has_merge_base() {
  GIT_NO_LAZY_FETCH=1 git merge-base "$base" "$head" >/dev/null 2>&1
}

if has_merge_base
then
  exit 0
fi

# Fetch only the two compared histories. Blob filtering keeps the benchmark
# archives out of this metadata-only job; no other branch or tag is needed.
for depth in 64 256 1024 4096
do
  git fetch --no-tags --filter=blob:none --depth="$depth" origin "$head" "$base"
  test "$(git rev-parse HEAD)" = "$head"
  if has_merge_base
  then
    exit 0
  fi
done

echo "impact comparison has no merge base within the fetched histories" >&2
exit 1
