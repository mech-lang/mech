#!/bin/sh
# General interpreter launcher: fd 3 is application output; interpreter messages use stderr.
exec 3>&1
exec 1>&2
base=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec "$base/.build/toolchains/gnu-apl-local/bin/apl" --silent --noCIN --noCONT --noColor --noSV "$@"
