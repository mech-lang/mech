# Shared M1 and M3 compiler integration

The shared `codex/pr993-m3-optimization` branch contains the M1 size and
compilation work through `0e101e9a12`, the M3 allocation-setup pass at
`5e91c17df7`, and their merge at `e981b748cf`. Source commit `5f8ef0d201`
adds evidence without changing that compiler. The Mech program and its
canonical bytecode are shared.

The combined compiler reproduces the latest named M1
`m1-shared-heap-layout` encoder, complete metadata and bytecode exactly.
The encoder contains **64,652 B** of text. This check runs on M3; it is an
artifact identity check, not a new timing measurement.
[Exact hashes](m1-object-replay.json) · [Compiler build receipt](compiler-build.json) ·
[Compiler replay log](m1-replay.log)

That exact native recipe has also been linked on M3 with the same pinned
runtime sources and allocator. It passes 162 exact-output, independent
inflation and borrowed-alias checks, six checked failure classes, cleanup
checks and the 81-case stack probe. Peak native stack is **4,784 B**, below
the unchanged 5,280 B limit. Its M3 timing run is pending.
[Common reference](common-reference-ready.json)

The existing M1 results and M3 results still describe different build
configurations. The newest enabled M3 settings have not been measured on
M1, and a single final configuration has not been qualified on both
machines. Five combinations of the M1 heap-setup pass and M3 options are
prepared and correctness-checked for M3. Their timings are pending.
[Source pins, candidate identities and stack bounds](status.json)

The latest completed M3 pair remains **81/81 Roc in both orders**, with
**58/81 libdeflate level 12 forward and 60/81 reverse**. The 61/81 target
has not been requalified. M1 peer scores are not assigned to M3 builds.

## Reproduce the artifact identity check

Build `compile_profile` from the integrated source with the recorded
toolchain and flags in `compiler-build.json`. The compiler build receipt
pins its source revision and binary hash. Then run:

~~~sh
python3 -B reproduce.py \
  --source-root /path/to/mech \
  --compiler /path/to/compile_profile \
  --build-receipt /path/to/compiler-build.json \
  --output /tmp/mech-integrated-m1-replay \
  --campaign-lock /tmp/mech-benchmark.lock
~~~

The output directory must be new. The command reads the checked-in M1
recipe and layout profiles, verifies their hashes, invokes the supplied
compiler and compares all three artifacts with the named M1 reference.
It makes no compilation-time or execution-time claim.

An initial harness omitted the frame-copy recipe's fresh-owner-count
setting and read root code size as complete encoder size. The corrected
harness reproduces all three artifacts without a compiler change.
[Retained harness failure](initial-harness-failure.json)
