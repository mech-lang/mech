#!/bin/sh
set -eu
base=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
mkdir -p "$base/.build/apl-support"
clang++ -O3 -std=c++11 -dynamiclib -Wl,-undefined,dynamic_lookup -I"$base/.build/toolchains/apl-2.0" -I"$base/.build/toolchains/apl-2.0/src" -I/opt/homebrew/opt/gmp/include "$base/support/apl/gmp-native.cpp" -L/opt/homebrew/opt/gmp/lib -lgmp -o "$base/.build/apl-support/libgmp-apl.dylib"
