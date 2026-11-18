# Matched compression deployment size

The interface reads one input file and writes one raw level-12 DEFLATE stream
to standard output. The three executables link the same C file driver and
mimalloc object on Apple M1 with macOS 15.6.1. Apple Clang uses the same arm64
target, macOS 15 deployment target and dead stripping. The stripped copies
use the same Apple strip flags, `-S -x`. The codec objects come from the
matched build comparison. [Exact input hashes](input-pins.json)
· [Build commands and binary hashes](measurements/build.json)

| Bytes | Mech | Roc | libdeflate |
| --- | ---: | ---: | ---: |
| Generated encoder text | 71,768 | 57,696 | 37,080 |
| Original benchmark executable | 956,056 | 230,600 | 210,168 |
| Deployment executable before symbol stripping | 668,392 | 199,680 | 161,400 |
| Deployment executable after symbol stripping | **575,632** | **191,920** | **155,760** |
| Deployment linked machine code | 341,012 | 122,416 | 98,848 |

The stripped Mech executable is **3.00× Roc** and **3.70× libdeflate**.
Mech's original executable includes the benchmark adapter, JSON
serialization, an independent inflater and the Rust standard libraries.
The file also carries link metadata, constants and unwind tables.
The minimal link removes the benchmark harness and leaves **199,224 B**
of linked Rust standard-library machine code, including **31,364 B**
from the backtrace symbolization libraries. The fixed compact runtime
contributes **4,508 B** of linked machine code. The generated encoder is
byte-identical to the matched benchmark encoder.
[Deployment file and symbol census](measurements/census.json)
· [Original benchmark census](../linked-size-census/report.json)
· [Matched benchmark builds](../refused-general-ssa/README.md)

All six deployed binaries produce the exact raw stream on all 81 named
fixtures, using each peer's frozen oracle. A separate zlib decoder consumes
each complete stream and reproduces the input. The common CLI has an
untimed one-stream interface. The historical timing workers and the
source-through-final-link timings are under the benchmark contract.
[Verification with output and binary hashes](measurements/verify.json)

## Reproduce

From this directory on Apple arm64, use Apple Clang and the preinstalled
`nightly-2026-03-03` Rust toolchain. The input kit restores prebuilt objects
under their verified hashes. The Mech compiler's checked source and complete
encoder reproduction are in the [maintained rebuild](../refused-general-ssa/README.md).
The exact fixed compact runtime source is in [fixed-runtime-source](fixed-runtime-source).
The peer inputs use Roc compiler `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`,
roc-deflate `671077457ff7ef3a3d44caf715a3fc595addf379`, libdeflate
`92e6a0db9fa848d742f9eb286c92afc60f2c3dda` and mimalloc
`8c532c32c3c96e5ba1f2283e032f69ead8add00f`. Linked hashes identify the
measured executables. Absolute paths and toolchain revisions can affect
the final bytes.

~~~sh
work=$(mktemp -d /tmp/mech-deploy.XXXXXX)
python3 -B prepare-inputs.py "$work/inputs"
python3 -B source/build.py "$work/inputs/manifest.json" "$work/build"
python3 -B source/verify.py "$work/verify.json" \
  --inventory ../../fixtures.json --libdeflate-oracle libdeflate-output-oracle.json \
  --mech-unstripped "$work/build/mech" --mech-stripped "$work/build/mech-stripped" \
  --roc-unstripped "$work/build/roc" --roc-stripped "$work/build/roc-stripped" \
  --libdeflate-unstripped "$work/build/libdeflate" \
  --libdeflate-stripped "$work/build/libdeflate-stripped"
python3 -B census.py "$work/build" "$work/census.json"
~~~
