# Packed numeric liveness on Apple M3

Over #973 at `cb8dfa790dd6e1eb043f76885f7d65c9c9abe00e`, source-to-Object compilation falls from **171.986 s to 143.598 s** without sampling, **16.5% less elapsed time**. The independent profiled pair improves 14.4%. All four compressor Objects, canonical bytecode and metadata are byte-identical.

## What changed

[The pure numeric liveness solver](../../hosts/gpu/src/native_artifact/program/prepare/numeric_liveness.rs) still seeds local reads and propagates only new live facts backwards. Definitions kill successor facts, while a read at the defining block stays live. The graph, root latch/finish stops, immutable predicate pruning, requested-value projection and fact validation in [numeric_exits.rs](../../hosts/gpu/src/native_artifact/program/prepare/numeric_exits.rs) are unchanged.

The solver indexes graph rows once and groups up to 64 value facts in each sparse word. The pending queue uses row indices. Only the requested entry result becomes a BTreeSet; other rows remain packed. Sparse words retain storage proportional to occupied words, including sparse block labels and high u32 values. There is no dense block/value matrix and no cached context shared across queries or compilations. No generated-code version, runtime permission, resource pricing, compressor source or algorithm changes.

Production source is local commit `84b6c076e2a0635df6fb101fa760b8b507d7dab5` over the pinned public baseline. The evidence includes its complete patch and exact source snapshots.

## Boundary and measurements

Apple M3 MacBook Air, 8 cores, 16 GB, macOS 26.2; nightly-2026-03-03 Rust 1.96.0 and the repository's vendored backend. The same focused release manifest and compact Object API as [the initial compile-time study](native-compile-time-profile.md) are used. Each process freshly parses and compiles the 160,857-byte level-12 source, creates its 4,138,627-byte canonical artifact, and emits its Object. Building the Rust executable, linking a generated host and running compression are separate.

| Run | Source → Object | Native | Preparation | Guard / owner interval | Peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| Baseline, sampled | 167.853 s | 133.575 s | 50.570 s | 45.956 s | 7.541 GB |
| Packed, sampled | 143.704 s | 108.927 s | 24.572 s | 47.232 s | 7.534 GB |
| Packed, sampling off | 143.598 s | 109.602 s | 24.760 s | 47.045 s | 7.681 GB |
| Baseline, sampling off | 171.986 s | 137.147 s | 52.495 s | 46.455 s | 7.720 GB |

These are two individual pairs, not a statistical timing campaign. The diagnostic pair uses consecutive 20-second samples at 5 ms; the unsampled confirmation reverses the execution order. No Rust builds or tests overlap source compilation. Routine source inspection and receipt preparation overlap diagnostic captures. Terminal sampler attach failures after process exit are retained; all four compilers and timing wrappers exit successfully.

Whole-process retired instructions fall 8.67% in the sampled pair and 8.55% in the clean pair. These are compiler process counters. Peak RSS remains high. Preparation intervals include surrounding work and are not exclusive timers for the pure solver. Guard/ownership remains about 46–47 s; `State::close` is prominent in the subsequent samples.

## Qualification and replay

All **18 focused tests pass** with warnings as errors: 10 guard-state tests and 8 numeric-liveness tests. The four existing liveness tests remain. Independent read-before-definition traversal covers branches and cycles, and 256 seeded sparse graphs compare all live sets and scheduling/fact counters against the entire pre-change solver. High IDs, word boundaries, local read/definition ordering, sparse storage and empty graphs are explicit.

Seven fresh source controls cover Bool/U64/I64/F64, tail recursion, guarded ownership and Boolean relations. Every source, canonical bytecode, Object and metadata file agrees between the two immutable compiler executables. The compressor Object remains `2a5cf7742a71959b288b40a7f1e5dfc3ca1a2b7502dee4304329fce1508896ff`; metadata remains `a63abffb005718010b043783b345c607fe7d0de4c47d89057526d58a8b41ab19`. Full repository CI and the 81-case runtime timing campaign were not rerun. No runtime speedup is claimed.

The [compact comparison](../../benchmarks/mech-deflate/native-profile/compile-liveness-apple-m3-20261113.json) and [archive receipt](../../benchmarks/mech-deflate/native-profile/compile-liveness-apple-m3-20261113.archive.json) pin [all raw evidence](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/level12-liveness-apple-m3-20261113.tar.gz): 205 files, archive SHA-256 `405f72872051ed62c8e57101326ab5f5a0d8d3ca58038fbf50dfaf1a4be24190`. Compiler binaries are omitted; their immutable hashes are retained. Fetch and verify using `experiment.py`, then run the extracted `replay.py` without a compiler or compressor.

```sh
python3 benchmarks/mech-deflate/experiment.py fetch --receipt benchmarks/mech-deflate/native-profile/compile-liveness-apple-m3-20261113.archive.json --output /tmp/level12-liveness-download
python3 benchmarks/mech-deflate/experiment.py verify /tmp/level12-liveness-download/level12-liveness-apple-m3-20261113.tar.gz --receipt benchmarks/mech-deflate/native-profile/compile-liveness-apple-m3-20261113.archive.json --extract /tmp/level12-liveness-replay
python3 /tmp/level12-liveness-replay/replay.py
```
