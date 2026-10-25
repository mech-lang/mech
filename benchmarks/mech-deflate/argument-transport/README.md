# Ordinary function argument transport

This is a compiler allocation diagnostic, not a compressor or compression timing.
It calls an ordinary function that reads two buffer lengths and performs scalar
arithmetic. The checked variant also executes a function-local assertion.

The inputs are a 262144-entry U16 table (524288 payload bytes) and a
1500513-word U32 cache (6002052 payload bytes). The smaller control case has
16 U16 entries and 31 U32 words. Compilation, input construction, alias creation,
and result inspection are outside the allocator probe. Both original and decoded
artifacts execute twice. Every input alias survives, and managed retained memory
returns to zero after dropping inputs and outputs.

Run with the pinned toolchain:

```sh
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
CARGO_TARGET_DIR=/private/tmp/mech-live-control-target \
cargo +nightly-2026-03-03 run --locked --offline -j1 -p mech-gpu \
  --no-default-features --features canonical-native-program \
  --example argument_frame_transport
```

The allocator counts Rust heap requests and their requested bytes on the calling
thread. Requested allocation bytes are neither peak live memory nor measured
memcpy bytes. The native work report is accounting, not elapsed time.

Baseline compiler: `19505d931a7eea5612b81008079a147813de23c7`.
The executable diagnostic was added by
`97baab36922df4a4d537cf1bf96218238725f4db` without a compiler change.
Measurements used the same unoptimized build, source, inputs, and finite policy
before and after the argument-frame change.

| Case, repeated visit | Before requested bytes | After requested bytes | Result |
| --- | ---: | ---: | --- |
| Small, unchecked | 172777 | 172777 | Both succeed |
| Small, checked | 396589 | 173153 | Both succeed |
| Real extents, unchecked | 172777 | 172777 | Both succeed |
| Real extents, checked | 24067287 | 173153 | Before rejects; after succeeds |

Original and decoded artifacts agree. The first checked real-extent visit requests
24067543 bytes before the change and 173374 afterward. Before the change, frame
admission requires 192651225 temporary bytes against the unchanged 134217728-byte
limit. Rejection preserves retained memory. No budget was increased to obtain the
successful result.

Previously, supported argument kinds entered a checked function through a
`core/composite-pack` followed by structural parameter projections. The composite
constructor clones child `ValueData`, which owns typed buffer payloads. A
nonrecursive checked call now uses the existing lexical capture mechanism for
all argument kinds. It retains immutable input snapshots and every argument
dependency, including unused parameters, without constructing that composite.
Recursive calls retain their structural carrier and fresh per-visit bindings.

The regression acceptance test exercises valid, assertion-rejected, and valid
calls with these real buffer sizes. It checks resident/native agreement,
assertion-before-division ordering, unchanged prior publication, input aliases,
cleanup, and recovery. The entire native call must request fewer bytes than a
copy of the smaller table. Existing recursion, lexical assertion, linked execution,
and component qualification remain part of validation.

This improvement does not remove recursive carrier copies, copies needed to
publish modified buffers, or other component costs. Algorithms, search limits,
tables, buffer sizes, and compression timing boundaries are unchanged. A complete
compressor and its timing remain later work.
