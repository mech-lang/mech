# First optimized component reality check

The current implementation has substantial execution overhead on these prerequisites and still cannot complete the six-position collector witness under its unchanged policy. This is a component baseline, with no full Mech compressor or whole-file performance claim.

Measured 2026-10-26T20:22:17.221388+00:00 through 2026-10-26T20:23:00.572973+00:00 on the existing Apple M1 Mac mini (8 GB, macOS 15.6.1). This is a desktop measurement without CPU affinity or OS isolation. Background compilation finished before these observations.

| Computation | Mech median | Pinned reference median | Elapsed-time ratio |
| --- | ---: | ---: | ---: |
| initialization | 657.148 ms | 0.036 ms | 18,254× |
| search | 230.665 ms | 0.099 ms | 2,330× |
| skip | 68.674 ms | 0.011 ms | 6,243× |
| six-position | work rejection after 1,280.159 ms | 0.036 ms | no completed Mech result |

Each Mech row has 10 measured calls across two rounds; each reference row has 14. A warmup call per computation per round is excluded. All positive durations and repetition identities were checked. Reference/Rust run first in round 0 and Mech first in round 1. Source compilation, JIT compilation, process startup, input preparation, verification, printing and destruction of the returned output are outside the timed interval. The reference platform clock observations are quantized to 1 microsecond; short reference timings and derived ratios are approximate. Mech's normal runtime admission, accounting, execution and publication remain inside it. No compiler implementation or compressor fixture was changed.

## What was measured

- **Initialization:** one call to the existing `optimal-collect-prefix(input, 0)` initializes all 262144 U16 tree entries and 1500513 U32 cache words, then returns its complete state. The reference performs the same allocation sites and state return. Both complete buffers are verified as zero after timing.
- **Search:** one `bt-get-matches`/`get_matches` on a 320-byte all-97 input at position 1, with maximum/nice length 258 and depth 300. The tree starts with entries 127616 and 174848 equal to 32768; hashes are 21888 and 62080. A real 1500513-word zeroed cache starts at pointer 3. The entire resulting tree and cache are verified: table entries 127616/174848 become 32769, entry 174849 becomes 32768, cache words 3/4 are 65539/65794, and the pointer becomes 5. The unchanged input arrays are also verified, keeping prior aliases alive on both sides.
- **Skip:** the corresponding separate `bt-skip-byte`/`skip_byte`, with the same tree/input/hash/depth/nice settings. There is no cache argument or recording path in skip. The full table and original aliases are verified.
- **Six-position trace:** one compiled Mech computation initializes and carries its own full-sized state through the existing six-position prefix source. The pinned reference completes and checks every buffer entry plus final hashes, pointer and skip count. Mech instead reaches the same finite work denial on every attempt, publishes no output, and restores managed memory to its prior value. Completed positions are unknown; no throughput or successful timing is inferred from the rejection.

The reference trace is faster than a standalone search in this harness because the trace carries its owned state; the standalone search intentionally retains prior input aliases on both sides, requiring the reference to copy those buffers. Intermediate states of the trace are not retained by the harness.

The Rust control is **only allocation and zero initialization** of the same two typed buffers, with every element checked outside timing. It took a median 0.233 ms, 2 allocation requests and 6,526,340 requested bytes. It has no Mech schemas, resource accounting or returned state record, so it is a lower-level cost reference, not a Rust compressor comparison. Rust and Mech use the system allocator; the pinned reference platform directly links its original mimalloc 2.1.7. Allocation-strategy and allocator differences remain visible.

## Allocation evidence

Counts use the same thread-local allocator observer for Mech and the Rust control. These are cumulative allocation/reallocation requests and requested bytes during the timed computation, not peak resident memory or live memory. Reference allocation counts were not instrumented. Its execution uses the original platform and allocation behavior.

| Mech computation | Median requests | Median requested bytes |
| --- | ---: | ---: |
| initialization | 44,665 | 61,183,945 |
| search | 48,515 | 64,879,268 |
| skip | 39,497 | 9,873,930 |
| six-position | 294,030 | 285,477,792 |

The two raw buffers contain 6526340 payload bytes. The earlier initialization witness also reported 12338685 control visits; this identifies a generic buffer-construction path to investigate. These observations do not establish a full-program CPU profile or assign all elapsed time to a single helper.

## Revisions and configuration

- Mech compiler/source checkpoint: `fd790ca0fce74b8eb8563cc4442d3a0041672c70`. Local reconciliation base: `2fe73989472b85bae277efb608666f193860d5cc`.
- Pinned compression source: `671077457ff7ef3a3d44caf715a3fc595addf379`.
- Reference compiler: `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d` (`release-fast-d99c7b83`), application `--no-cache --opt=speed`.
- Rust: `rustc 1.96.0-nightly (ec818fda3 2026-03-02)`. Mech host: ordinary Cargo `release` (opt-level 3), warnings as errors; generated Mech program: Cranelift `speed`. Rust control: `rustc -O -D warnings`.
- Features: `mech-gpu/canonical-native-program`, no default features. The focused standalone driver uses the existing compiler crates and selects arithmetic, comparison, Boolean and range libraries; its full manifest, lockfile and selected feature inventory are retained under `focused/`. It excludes the broad integration-test dependency closure. All shared dependency versions match the original repository lockfile.
- Managed memory 256 MiB; temporary memory 128 MiB; native work, library compute and comparison work capped at 10000000000; maximum output elements 1500513. Existing allowances are unchanged.
- Artifact form timed: original. Original/decoded correctness is covered by the preceding 119-test qualification; decoded performance and linked/AOT performance were not measured here.
- A standalone measurement driver was added outside the checkout. The isolated checkout is clean at the stated revision. `git diff -- src hosts/gpu/src` was empty. Unfinished uniform-fill work in the other worktree was preserved and excluded.

The final focused build passed in 192.7 seconds. Its preceding matrix-only feature configuration spent 546.7 seconds compiling dependencies and then failed with 24 unused-macro warnings promoted to errors. Selecting the existing dynamic matrix/vector/row-vector shape family resolved that configuration failure; no compiler source or warning policy was changed. The initial overly broad integration-test build was cancelled after 1747.7 seconds; see `broad-build-cancellation.json` and `release-build.json`. A subsequent focused build attempt was also restarted before timing to preserve the original dependency versions. None of these setup attempts is a compiler correctness failure or an operation timing. This is one-time toolchain/dependency compilation and is excluded from every operation time. It is distinct from per-program source/JIT compilation, which is logged separately in [compilation observations](compilation-observations.json). All [build attempts and their dispositions](build-attempts.json) are retained. The matrix-only configuration failure was caused by the new driver feature selection against unchanged macro definitions; the successful configuration keeps warnings as errors.

The Rust control is archived byte-for-byte as `rust-init.rs.txt`; it is a measured source artifact, not a workspace Rust module. Its original source hash remains in the provenance record.

## Commands and receipts

Working tree: `/private/tmp/mech-deflate-reality-check`, detached at the checkpoint above.

```sh
RUSTUP_TOOLCHAIN=nightly-2026-03-03 CARGO_INCREMENTAL=0 \
CARGO_TARGET_DIR=/private/tmp/mech-live-control-target \
cargo build --release --locked --offline -j1 \
  --manifest-path /private/tmp/mech-stack-landscape-20261026/reality-check/focused/Cargo.toml \


ROC_CACHE_DIR=/private/tmp/mech-deflate-reference-cache \
/private/tmp/roc-deflate-repro-20261024/roc-install/bin/roc build \
  --no-cache --opt=speed \
  /private/tmp/roc-deflate-repro-20261024/roc-deflate/package/RealityCheck.roc \
  --output=/private/tmp/mech-stack-landscape-20261026/reality-check/roc-components

RUSTUP_TOOLCHAIN=nightly-2026-03-03 rustc -O -D warnings \
  /private/tmp/mech-stack-landscape-20261026/reality-check/rust-init.rs \
  -o /private/tmp/mech-stack-landscape-20261026/reality-check/rust-init

python3 /private/tmp/mech-stack-landscape-20261026/reality-check/run-measurements.py
```

The measurement runner invokes the standalone release executable `component-reality`, five measured repetitions per case plus warmup. It checks the exact existing denial and memory cleanup rather than calling a rejected computation successful. Every completed operation's whole buffers and relevant scalars are checked outside timing.

[Raw observations and summaries](measurements.json), [provenance and hashes](provenance.json), [host build receipt](focused-release-dynamic-shapes.json), [Mech harness](focused/src/measurement.rs), [reference harness](RealityCheck.roc), [Rust control](rust-init.rs.txt), [measurement runner](run-measurements.py).

Exact Mech rejection(s):

```text
Err(Value(Base(WorkLimit { phase: Setup, required: 10000306764, limit: 10000000000 })))
```

## Disposition

The six-position rejection is the existing collector blocker at the measured checkpoint; no new compiler change was introduced for this comparison. The 119 prior component tests establish behavior for their qualified pieces, and do not imply usable compressor performance. This packet does not rerun all of CI or add full DEFLATE functionality.

Preserve this baseline. The next bounded compiler step is to profile and improve the demonstrated generic construction/transport cost, then repeat these identical operations and the unchanged six-position witness. Completing the collector, reusable growing buffers, parser, writer and `compress_into` remains necessary before a full-file Mech compression benchmark. No whole-file time is extrapolated from these component samples.
