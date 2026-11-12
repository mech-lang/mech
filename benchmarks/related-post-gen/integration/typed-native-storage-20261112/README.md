# Typed native storage: IR335/336 checkpoint

The compact U32-source version improves on the current compact wide-source version in both matched cohorts. **It is still 1.023× the fastest eligible Rust in the primary cohort and 1.039× in independent confirmation.** Managed IR334 remains the selected managed implementation: IR336 wide is inconclusive, while the managed byte and U32 sources regress by about 3% against IR334. Fastest-Rust parity remains unmet.

The measured compiler source is published as [`4b6133f85ae85f858bfe29f8cc871aada58a77bb`](https://github.com/mech-lang/mech/commit/4b6133f85ae85f858bfe29f8cc871aada58a77bb), with a minimal 23-file overlay over IR334 [`6cd626c3dfb5cdac2efc3c6978949a717ada27a8`](https://github.com/mech-lang/mech/commit/6cd626c3dfb5cdac2efc3c6978949a717ada27a8). The overlay includes two test-only version corrections made after the broad run. The original build inventories, their exact test/source overrides and complete 12-crate library receipts distinguish each actually compiled snapshot. Production code generation remains Cranelift; no backend dependency or compatibility bridge is added.

The compiler now recognizes checked U8/U16/U32 yields in native UTF8 dictionary lookup loops and emits matching packed storage in both managed and compact programs. It can preserve a private dictionary owner across a small, proved total scalar guard corridor, choosing the existing canonical publication path on bypasses. The guard prediction's physical work is funded. A new finite-budget test exposed a real omitted binding/cache metadata bound in optional native replay; the engine-owned bound now includes those costs before attempting the optional path. Unsupported or overflowing proofs decline to the original source execution. Arithmetic/index checks and original error order remain tested.

Three source variants retain the same query algorithm, ties and fallback:

| Source | SHA256 | Storage change |
|---|---|---|
| Maintained wide | `d70a001620977aff9529884d0207fbf7952a359388988431b4102ae0465f1905` | Existing U64 IDs and combinadic keys |
| Direct byte IDs | `46ad6f23a4a3387a38056fcd4956a9f31b9792c674ebc820896bdd586143706f` | Collect checked U8 tag IDs directly under the existing vocabulary guard |
| U32 keys | `5a2e9525cc38628687841a8c9fbcbea655fb7156a60e5e217ed75c90dfc95abf` | Also store bounded triple/quartet keys as U32; retain checked U64 arithmetic and index widening |

All primary timings are medians of 31 seeded randomized fresh-process rounds. The continuous post-JSON interval includes input preparation/admission and creation, execution and joining of four native query workers. Compact also destroys its preparation buffers and native result backing inside its interval; final JSON output and decoded Post/result destruction follow the documented adapter boundary. JSON IO and managed artifact restoration are tracked in whole-process diagnostics. Two preinitialized-pool Rust controls are retained but excluded from the reference. Current Rust is the fastest of 22 eligible controls, including two fresh-four-worker controls. Lower multiples are faster. Whole-process time is informational, not an acceptance gate.

| Implementation | Primary post-JSON | × fastest eligible Rust | Whole process |
|---|---:|---:|---:|
| Fastest fresh-four Rust | 1.249375 ms | 1.000 | 31.048 ms |
| Fastest original 20 serial Rust | 1.563333 ms | 1.251 | 30.536 ms |
| Managed host-v2 | 1.638167 ms | 1.311 | 55.394 ms |
| Managed IR331 | 1.483125 ms | 1.187 | 55.830 ms |
| **Selected managed IR334** | **1.448625 ms** | **1.159** | **54.353 ms** |
| Managed IR335 direct byte | 1.563084 ms | 1.251 | 61.392 ms |
| Managed IR336 wide | 1.461208 ms | 1.170 | 55.242 ms |
| Managed IR336 byte | 1.487375 ms | 1.190 | 61.973 ms |
| Managed IR336 U32 | 1.489584 ms | 1.192 | 60.751 ms |
| Historical compact IR320 | 1.349459 ms | 1.080 | 33.354 ms |
| Current compact IR336 wide | 1.312250 ms | 1.050 | 34.228 ms |
| Current compact IR336 byte | 1.288958 ms | 1.032 | 34.318 ms |
| **Current compact IR336 U32** | **1.278459 ms** | **1.023** | **33.857 ms** |

These are distinct managed and compact contracts. Compact removes managed metering, while arithmetic and indexing remain checked. Its native worker entry is compiled into the binary. No host scoring loop is used.

Paired changes use the median per-round percentage, which differs from the ratio of marginal medians:

| Comparison | Median paired change | 95% bootstrap interval | Faster pairs |
|---|---:|---:|---:|
| Managed IR336 wide / IR334 | +0.211% | [−0.280%, +0.902%] | 15/31 |
| Managed IR336 byte / IR334 | +3.033% | [+1.477%, +3.584%] | 7/31 |
| Managed IR336 U32 / IR334 | +2.912% | [+1.519%, +3.230%] | 7/31 |
| Compact IR336 U32 / IR336 wide | **−2.369%** | **[−3.370%, −1.180%]** | **29/31** |
| Compact IR336 U32 / IR336 byte | −1.181% | [−1.702%, −0.007%] | 21/31 |
| Compact IR336 U32 / IR336 wide, independent | **−3.455%** | **[−3.646%, −1.248%]** | **11/13** |

Independent orchestration on the same machine measured compact U32 at **1.262833 ms**, compact wide at **1.287958 ms**, and the fastest fresh-four Rust at **1.215958 ms**. This confirms the source-storage gain, not Rust parity or separate-machine replication. All initial observations are retained, including slow and fast pairs. No unchanged adverse timing was rerun.

A separate seven-round managed resource cohort records all-thread post-JSON counters. Instrumented latency does not select the winner:

| Implementation | Instructions | Cycles | Allocation requests | Requested bytes | Additional requested-heap peak |
|---|---:|---:|---:|---:|---:|
| Fresh-four Rust | 18,181,257 | 6,626,228 | 78 | 7,026,975 | 5,721,694 |
| Managed IR334 | 29,709,692 | 9,031,714 | 725 | 6,376,193 | 4,211,067 |
| Managed IR336 wide | 29,693,957 | 8,965,175 | 725 | 6,376,209 | 4,211,067 |
| Managed IR336 byte | 29,837,466 | 9,382,613 | 725 | 6,404,707 | 4,397,235 |
| Managed IR336 U32 | 29,589,832 | 9,138,908 | 725 | 5,502,843 | 3,495,371 |

U32 storage reduces requested bytes and peak requested heap while failing to improve managed latency. This is useful negative evidence. Requested heap excludes allocator internals, OS stacks and mmap. All windows retain freed bytes, allocator underflows, RSS/footprint endpoints, executable sections, native code/frame sizes and whole-process peak RSS. Free-call count is unavailable. The three current compact resource adapters have separately passed full 5,000-post and 31-edge qualification with unchanged native objects and runtime/library closure; their qualification counters are diagnostic and are not a matched resource comparison in this packet.

Two complete repeated-path profiles each perform 5,001 fresh JSON decodes, input admissions, four-worker native evaluations, result materializations and input/result cleanups. They restore the program once and write only one final exact JSON result. Raw native samples, stage clocks, drivers and link receipts are retained. These profiles are diagnostics, not fresh-process latency observations; phase clocks and worker scheduling differ from the primary cohort. The first profile driver wrongly required peak overlap to equal four and failed; it is retained. The corrected driver requires four tasks and accepts observed overlap from one to four. The v2 prelaunch schema error and later offline field-name finalization are also retained. No IR334 profile was rerun for the finalizer correction.

Qualification retains four new managed adapters and six compact original/decoded adapters, each with full-output and 31-edge exactness, native linkage and machine/lowering audits. Six typed-lookup contract tests, six guarded-owner tests, the compact typed-storage JIT test and 131 engine collection-call regression tests pass. Filter overlaps are not counted as extra distinct tests. The broad native run initially passed **1,067**, failed **18**, and ignored **11**. Sixteen failures match the inherited baseline. Two new failures were stale work-policy literals (`126` versus `127`); a test-only correction passes both focused tests with every historical exact work bill and other assertion preserved. The broad suite was not rerun after that correction, and no full-broad-green claim is made. Earlier compile/fixture failures, the legitimate paid-fee relative-limit translation and the real global-budget displacement diagnostic remain archived.

[report.json](report.json) gives concise machine-readable results. [records.json](records.json) maps historical paths to deterministic SHA256-checked gzip blobs. Large identical outputs are stored once. The source closure includes all 2,814 file hashes, 164 literal-include hashes, the 23-file source overlay, exact snapshot overrides and coherent build receipts; it does not duplicate full compiler trees or libraries. Historical absolute paths are provenance, not verification dependencies.

Run from a checkout containing the published source commit:

```sh
python3 benchmarks/related-post-gen/integration/typed-native-storage-20261112/verify.py
```

The offline verifier launches no compiler, benchmark or profiler. It checks **1,660 completed timed/resource outputs** (1,085 primary, 351 confirmation, 224 resources), original orders and timer sources, medians and bootstrap intervals, resource windows, qualification/edge equality, source/build/link pins and both diagnostic profile outputs. Qualification and profile executions are excluded from the 1,660 total.
