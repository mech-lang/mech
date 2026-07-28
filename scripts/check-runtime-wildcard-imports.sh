#!/bin/sh
set -eu

if [ "$#" -gt 1 ]; then
  echo "usage: $0 [repository-root]" >&2
  exit 2
fi

if [ "$#" -eq 1 ]; then
  repository_root=$1
else
  repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
fi

runtime_root="$repository_root/src/runtime/src/runtime"
pattern='^[[:space:]]*use[[:space:]]+(crate|super(::super)*)::\*;'

matches=$(find "$runtime_root" \
  -type f \
  -name '*.rs' \
  ! -path '*/tests/*' \
  ! -path '*/input_tests/*' \
  ! -path '*/test_support/*' \
  -exec grep -HEn -- "$pattern" {} + || true)

if [ -n "$matches" ]; then
  echo "runtime wildcard import audit failed:" >&2
  echo "$matches" >&2
  exit 1
fi

echo "runtime wildcard import audit passed"
