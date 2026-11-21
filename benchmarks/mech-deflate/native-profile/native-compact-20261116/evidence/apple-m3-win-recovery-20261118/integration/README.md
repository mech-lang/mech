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
the unchanged 5,280 B limit. Both complete M3 timing orders now record
**81/81 Roc and 61/81 libdeflate level 12 wins**.
[Complete qualification and all timings](confirmation/README.md) ·
[Common reference](common-reference-ready.json)

The eight-case M3 screen completed in both orders: **7/8 Roc and 6/8
libdeflate forward**, **8/8 Roc and 7/8 libdeflate reverse**. Raw-record
replay verifies all **199,680 timed calls**. These focused results do not
qualify the full 81-case configuration. The first full run stopped after
17 completed cases when the activity guard observed an external Rust
compiler at 94.1% CPU. A second interrupted attempt and the first clean,
complete comparison are retained. Only the clean complete pair qualifies
the shared recipe.
[Focused results](m3-focus-result.json) · [Seconds and comparator multiples](m3-focus-timings.csv) ·
[Raw-record replay](m3-focus-replay.json) · [Full-run rejection](m3-first-full-rejection.json)

The source snapshots, compiler and worker identities, prepared combined
builds, original and corrected replay harnesses, focused observations,
interrupted observations and guarded retry commands are preserved in the
[verified 553-payload-member integration archive](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/pr995-deflate-shared-integration-20261118-8908ed6b1349.tar.gz).
Its SHA-256 is `8908ed6b1349acc41a3718a21d96e0d87e7046fde7dad64a7b4bde65f1f020fc`.
It is a delta over the preceding recovery archive; unchanged payload is
retained there. [Archive receipt and parent hash](archive.json)

The named shared recipe is now qualified on both M1 and M3. Additional
M3-only settings are not selected by this recipe and have not been
measured on M1. Five combinations of the M1 heap-setup pass and M3 options
remain prepared and correctness-checked; their timings are pending.
[Source pins, candidate identities and stack bounds](status.json)

The latest completed M3 pair records **81/81 Roc and 61/81 libdeflate
level 12 in both orders**. M1 peer scores are not assigned to M3 builds.

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
