#!/bin/sh
# Futhark 0.27 emits unused f64 erf/erfc declarations that conflict with the
# declarations in ISPC 1.31's standard library. Remove only those declarations
# while leaving the generated kernels and all used math functions unchanged.
set -eu

source=
for argument in "$@"; do
    case "$argument" in
        *.kernels.ispc) source="$argument" ;;
    esac
done

if [ -z "$source" ]; then
    exec /opt/homebrew/bin/ispc "$@"
fi

patched="${TMPDIR:-/tmp}/futhark-ispc-compat-$$.ispc"
trap 'rm -f "$patched"' EXIT HUP INT TERM
sed \
    -e '/^[[:space:]]*extern "C" unmasked uniform double erf(/d' \
    -e '/^[[:space:]]*extern "C" unmasked uniform double erfc(/d' \
    "$source" > "$patched"

arguments=
for argument in "$@"; do
    if [ "$argument" = "$source" ]; then
        arguments="$arguments \"$patched\""
    else
        arguments="$arguments \"$argument\""
    fi
done
eval "exec /opt/homebrew/bin/ispc $arguments"
