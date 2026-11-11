The compact/unmetered compiler meets the related-post processing target: **1.315 ms / 0.867× fastest eligible Rust**. Independent orchestration confirms **1.327 ms / 0.878×** on the same machine. The maintained Mech source is unchanged.

| First matched cohort, 31 rounds | Complete post-JSON median | × fastest eligible Rust |
|---|---:|---:|
| Rust ordered mask tables256 | 1.517 ms | 1.000× |
| Managed IR317, four workers | 1.682 ms | 1.109× |
| Compact IR320, four native workers | 1.315 ms | 0.867× |

This interval includes input preparation, allocations, worker creation/run/join, native cleanup and result materialization. JSON decoding and serialization are outside. Whole-process time remains visible: compact 26.532 ms versus 23.663 ms for the post-JSON Rust reference. This is a processing win; whole-process performance is not an acceptance gate.

The compiler uses native String classes to avoid repeating equivalent lookups, native canonical Map indices, shared UTF8 hash instructions, current Boolean ownership proofs, optimized numeric phases and compiled worker bodies. The worker runtime only manages generic threads and allocations. It has no post/tag/scoring algorithm. Cranelift remains the backend; no LLVM dependency or compatibility bridge is added. The pinned #973 base is `fd47444b440f6422cdefe957f2b2cd6a251ab44c`, retained R314, at the recorded integration cutoff.

The compact entry retains arithmetic, indexing and assertion checks. It omits exact managed resource/admission/recovery semantics and remains a distinct diagnostic contract. The ordinary managed entry stays selected. The native class proof accepts direct immutable invocation-input strings; owned or unproved sources retain their existing lowering. Separate Maps compute their own values. Missing-key errors keep the original source site and leave output untouched.

Current post-JSON medians are 29.0M instructions, 7.96M cycles, 71 allocator requests, 3.93MB requested allocator heap and 3.00MB peak additional allocator heap. Native mappings are tracked separately: 23 payload requests / 4.17MB requested backing. These counters overlap and must not be added. Native Object text is 182,604 bytes; the linked executable is 914,816 bytes.

`latest.json` contains current timings, resource boundaries, compiler pins, qualification and limitations. `records-ir320.json` names losslessly compressed, content-hashed records, including all raw samples and execution order, adverse observations, configs, adapters, exact sources, native Objects, profiles and current regression receipts. Identical full JSON outputs are stored once and indexed per sample. The original `report.json` / `records.json` remain the historical initial compact baseline.

All 744 first-cohort, 175 separately instrumented resource and 312 confirmation outputs match Rust. Both artifact forms pass the complete workload and 31 edge cases. The final compiler passes 589 selected contract checks, retaining all 583 prior test names. Earlier native Map/class qualification and scalar/DEFLATE results retain their explicit compiler scope in `latest.json`; no full hosted-CI pass is claimed.

Run the offline evidence verifier from the repository root:

```sh
python3 benchmarks/related-post-gen/integration/native-compact-20261111/verify.py
```

It checks all record hashes, current compiler sources and measured Objects, then reconstructs medians, reference selection and resource counters from raw observations. It does not rerun executables. The retained measurement scripts, build commands and exact source pins support new machine-specific runs; their absolute temporary paths document the original environment and need relocation for a new build.
