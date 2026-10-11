#!/bin/sh
# GNU APL 2.0 from GNU's release archive; native arm64, ordinary strict floating point.
set -eu
base=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
tools="$base/.build/toolchains"
mkdir -p "$tools"
if [ ! -f "$tools/apl-2.0.tar.gz" ]; then
  curl -L --fail https://ftp.gnu.org/gnu/apl/apl-2.0.tar.gz -o "$tools/apl-2.0.tar.gz"
fi
printf '%s  %s\n' '24bbb744fce47e62837234a053bdeecee51b9ea61c82c79f7cc191bc6a54c0a1' "$tools/apl-2.0.tar.gz" | shasum -a 256 -c -
if [ ! -d "$tools/apl-2.0" ]; then
  tar -xf "$tools/apl-2.0.tar.gz" -C "$tools"
fi
cd "$tools/apl-2.0"
./configure --prefix="$tools/gnu-apl-local" --without-x --without-gtk3 --without-sqlite3 --without-postgresql --with-pcre=/opt/homebrew/opt/pcre2 CORE_COUNT_WANTED=argv CXXFLAGS='-O3 -DNDEBUG -std=c++11' LDFLAGS=-L/opt/homebrew/opt/pcre2/lib CPPFLAGS=-I/opt/homebrew/opt/pcre2/include > configure.log 2>&1
make -j4 > build.log 2>&1
make install > install.log 2>&1
"$base/build-apl-support.sh"
