#!/bin/sh
# General standard-I/O launcher: allocate one private scratch path, then clean it up.
set -eu
base=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/gnu-apl-scratch.XXXXXXXX")
cleanup() {
  rm -f -- "$scratch/data"
  rmdir -- "$scratch"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP
"$base/../run-apl.sh" "$@" -- "$scratch/data"
