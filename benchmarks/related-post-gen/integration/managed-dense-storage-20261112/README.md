# IR340: dense checked worker ValueArena storage

IR340 reduces managed worker memory and confirms a latency gain for the U32 source against its same-source IR338 parent. It does **not** establish a gain over the selected IR334 scoped-frame checkpoint. IR334 remains selected for managed latency; the fastest eligible fresh-four-worker Rust target remains unmet. Cranelift remains the backend.

The primary run has 31 randomized rounds × 41 implementations = **1,271 exact outputs**. A separately randomized resource run has 7 × 39 = **273 exact outputs**. Independent orchestration on the same machine has 13 × 27 = **351 exact outputs**, including all 24 Rust controls, IR334, IR338 U32, and IR340 U32. Thus this packet retains **1,895 new timed/resource outputs**, plus **384 separate correctness qualification outputs**. No observation was removed, retried to select a time, or excluded for low worker overlap. These are single-machine observations, not a hardware replication.

## Latency evidence

The authority is one continuous post-JSON interval through input admission, intervening setup, output-slot allocation, native invocation, internal cleanup, index export and result materialization. JSON IO, descriptor restoration and final outer destruction remain visible in complete-process time. Whole-process time is reported and is **not an acceptance gate**. Both fresh-four-worker Rust controls allocate and start their query workers inside their processing interval. Preinitialized Rust variants remain retained diagnostic controls and cannot define the eligible post-JSON baseline.

| Cohort | IR340 U32 | Fastest eligible Rust | Mech / fastest Rust | IR334 control |
| --- | ---: | ---: | ---: | ---: |
| Primary, 31 rounds | 1.436250 ms | 1.237334 ms | **1.160762×** | 1.440834 ms |
| Independent orchestration, 13 rounds | 1.420375 ms | 1.189500 ms | **1.194094×** | 1.417041 ms |

Both fastest eligible Rust references are `rust-ordered-mask-tables256-fresh4`; the denominator is reselected from all 22 eligible Rust implementations in each cohort. The two other Rust controls initialize resources before their excluded processing clock and stay in the full-process comparison. A favorable comparison against serial Rust is secondary and does not establish parity with fresh-four-worker Rust.

Paired percentages use each round's candidate/control ratio. Negative means the candidate was faster; 95% percentile bootstrap intervals are diagnostic and retain all observations. Different bootstrap order-statistic conventions in the two original analysis scripts are preserved and independently reproduced by `verify.py`.

| Candidate / control | Median paired change | 95% interval | Faster pairs |
| --- | ---: | ---: | ---: |
| Primary IR340 U32 / IR338 U32 | −1.8614% | [−2.9228%, −1.1965%] | 28 / 31 |
| Confirmation IR340 U32 / IR338 U32 | −2.2091% | [−3.1312%, −1.2326%] | 11 / 13 |
| Primary IR340 U32 / IR334 | −0.6019% | [−1.5757%, +0.2677%] | 20 / 31 |
| Confirmation IR340 U32 / IR334 | −0.7512% | [−2.5995%, +0.7441%] | 8 / 13 |
| Primary IR340 wide / IR338 wide | +0.4824% | [−0.3595%, +1.2596%] | 12 / 31 |

Dense storage plus narrower keys recovers the same-source parent's regression, with confirmation. The interval against selected IR334 crosses zero in both cohorts, and the wide-source gain is inconclusive. The medians alone do not justify selecting a new managed latency checkpoint.

All Mech controls and the fastest serial and parallel Rust controls from the primary cohort follow. Full raw timing, process counters and all Rust variants remain in the records. Compact/unmetered controls have a separate contract and are not managed parity results; their arithmetic and indexing checks remain.

| Primary implementation | Post-JSON ms | × fastest eligible Rust | Complete process ms |
| --- | ---: | ---: | ---: |
| `rust-ordered-mask-tables256` | 1.541250 | 1.245622× | 30.154459 |
| `mech-managed-host-v2-4` | 1.614833 | 1.305091× | 54.223917 |
| `rust-ordered-mask-tables256-fresh4` | 1.237334 | 1.000000× | 31.601000 |
| `mech-managed-retire-overlap` | 1.451250 | 1.172885× | 55.112250 |
| `mech-ir320-compact-native-4` | 1.346292 | 1.088059× | 34.623083 |
| `mech-managed-scoped-frame` | 1.440834 | 1.164467× | 55.124583 |
| `mech-managed-typed-fixed-byte` | 1.536667 | 1.241918× | 61.213625 |
| `mech-managed-guarded-owner-wide` | 1.453334 | 1.174569× | 54.498500 |
| `mech-managed-guarded-owner-byte` | 1.474625 | 1.191776× | 60.910541 |
| `mech-managed-key-u32-probe` | 1.473709 | 1.191036× | 60.640875 |
| `mech-ir336-compact-maintained-wide-4` | 1.296291 | 1.047648× | 34.151459 |
| `mech-ir336-compact-direct-byte-4` | 1.280875 | 1.035189× | 34.245541 |
| `mech-ir336-compact-key-u32-4` | 1.262125 | 1.020036× | 33.258917 |
| `mech-managed-static-replay-byte` | 1.486334 | 1.201239× | 61.502250 |
| `mech-managed-static-replay-u32` | 1.454458 | 1.175477× | 60.686250 |
| `mech-managed-owned-capture-wide` | 1.448708 | 1.170830× | 54.372625 |
| `mech-managed-owned-capture-u32` | 1.471625 | 1.189351× | 60.956292 |
| `mech-managed-dense-worker-wide` | 1.449458 | 1.171436× | 54.466583 |
| `mech-managed-dense-worker-u32` | 1.436250 | 1.160762× | 61.696250 |

Complete-process comparisons are materially slower for Mech: the primary IR340 U32 median is 61.696250 ms, versus fresh-four-worker Rust's 31.601000 ms; confirmation is 46.435916 ms versus that cohort's fastest whole-process Rust median of 16.965292 ms. The confirmation control with the fastest full-process time is `rust-ordered-ordered`, distinct from the post-JSON winner. These scopes are not combined.

## Memory, instructions and size

The separate resource cohort is instrumented across all participating threads. Its times cannot select latency candidates. Requested heap bytes measure allocator requests, not resident memory or native payload mappings. Whole-process RSS is a separate high-water measurement; absolute before/after and lifetime hardware/allocator counters remain retained. Free-call counts are uninstrumented; freed bytes are retained. Native compact payload accounting must not be added blindly to tracked Rust heap or RSS.

| Resource implementation | Instructions | Cycles | Allocation requests | Requested bytes | Peak additional live requested bytes | Whole-process peak RSS bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `rust-ordered-mask-tables256-fresh4-instrumented` | 18,183,912 | 6,626,380 | 78 | 7,026,975 | 5,721,694 | 6,193,152 |
| `rust-ordered-mask-tables256-fresh4-apple-m3-instrumented` | 18,193,533 | 6,654,604 | 78 | 7,026,975 | 5,721,694 | 6,209,536 |
| `mech-managed-scoped-frame-instrumented` | 29,670,133 | 8,856,723 | 725 | 6,376,193 | 4,211,067 | 26,034,176 |
| `mech-managed-owned-capture-wide-instrumented` | 29,661,989 | 8,964,733 | 725 | 6,376,209 | 4,211,067 | 26,132,480 |
| `mech-managed-owned-capture-u32-instrumented` | 29,541,761 | 9,029,472 | 725 | 5,502,843 | 3,495,371 | 29,605,888 |
| `mech-managed-dense-worker-wide-instrumented` | 29,669,944 | 8,893,326 | 725 | 6,084,753 | 3,919,611 | 26,247,168 |
| `mech-managed-dense-worker-u32-instrumented` | 29,388,349 | 9,156,819 | 725 | 5,198,427 | 3,190,955 | 29,016,064 |

Wide requested and peak additional live bytes each fall **291,456 B** against IR338 wide. U32 each fall **304,416 B** against IR338 U32. Both differences exactly match the four workers' physical bank reduction. Request count remains **725**. U32 instructions fall by 153,412, while cycles **increase by 127,347** in this separate instrumented cohort. This is a retained counter regression, not evidence that every hardware metric improved. Wide instructions increase by 7,955 while cycles decrease by 71,407.

| IR340 ordinary executable | Native code bytes | Reported native stack bytes | Executable file bytes | Primary whole-process peak RSS bytes |
| --- | ---: | ---: | ---: | ---: |
| `mech-managed-dense-worker-wide` | 1,021,740 | 1,568 | 17,219,824 | 27,099,136 |
| `mech-managed-dense-worker-u32` | 1,149,636 | 1,680 | 18,194,656 | 30,015,488 |

Stack bytes are the compiler's reported native stack requirement, not total process or all-thread stack consumption. File size includes the host adapter and linked dependencies; native code bytes describe emitted benchmark code. Ordinary and metrics executable hashes and native Object audits are retained separately.

## Compiler change and physical storage proof

The existing checked all-path worker inventory is generalized into an immutable descriptor-owned global-slot-to-dense-slot map. Each worker borrows that map; its private `ValueArena` value and generation banks use dense indices. Original logical slot/type coordinates, handles and domains, generation semantics, scalar word coordinates, raw `BufferFrames` coordinates, ownership validation, logical admission claims and work/setup prices remain unchanged. The type table was already borrowed, so this change claims no type-copy saving.

The inventory unions actual slot sets, including captures, constants, the full lexical subtree, all source reads/definitions, branches and retirements, numeric/tree/composition metadata, cold aliases and fallback paths. All **50 canonical library outputs**, including **14 scalar outputs**, remain in the union: canonical calls publish scalar results into the arena too. Category counts overlap and are never summed to estimate the storage count. Unproved/unsupported storage or effect categories decline to full identity storage. Restore rederives the proof; serialized images cannot grant mapping authority.

| Source | Full funded logical slots | Prior physical prefix | Dense values / generations | Prior two-bank bytes / worker | Dense two-bank bytes / worker | Four-worker saving | Descriptor map bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Maintained wide | 2,420 | 1,341 | 329 / 329 | 96,552 | 23,688 | 291,456 | 5,364 |
| Direct-byte/U32 keys | 2,840 | 1,388 | 331 / 331 | 99,936 | 23,832 | 304,416 | 5,552 |

The measured stride is 64 bytes per optional Value plus 8 bytes per generation. Each map is allocated once per descriptor, outside the timed post-JSON interval; its map record is 24 B replacing the old 8 B prefix record. Each arena gains an 8 B borrowed pointer inline. Map storage is indexed by the prior global prefix, not the full logical type-table length. The preserved first audit included a conservative full-table map estimate; `PLAN-implemented.md` and positive native storage receipts give the actual final dimensions above. Savings exclude allocator internals and this descriptor/inline overhead.

Original and decoded source artifacts, each with original and restored descriptors, positively demonstrate all four combinations for both sources: 329 or 331 physical values and generations, the exact map bytes, and full 2,420 or 2,840 funded logical slots. The offline verifier also replays the retained read-only exact-set audit against its original descriptor JSON and binary metadata inputs. Unmapped in-range holes preserve the old empty value/generation-zero preview and retirement behavior. Invalid mutations fail closed while preserving existing validation/debit priority. Standalone arena API behavior remains the default full layout.

No host scoring implementation, compatibility bridge, LLVM dependency or backend switch is introduced. This is a general native descriptor/runtime storage improvement, exercised by the maintained source and the existing direct-byte/U32-key source; maintained benchmark source is unchanged.

## Correctness and retained failures

Twelve adapters are qualified: two sources × original/bytecode-decoded/descriptor-restored forms × ordinary/metrics adapters. Every adapter passes full 5,000-post output equality and 31 edge fixtures: **384 exact outputs** total, separate from timing. Every adapter passes exact Object-relative symbol and relocation-aware machine-code link auditing. Restored adapters use the existing checked image restoration API and retain the original Object identity; no compatibility API is added.

The final IR340 focused set has **33 distinct passing tests**: 26 arena contracts, five worker original/decoded/restored contracts, and two parallel numeric errors/aliases/retry contracts. Cases include exact finite admission/work denials, stale generations, wrong domains/work, unmapped slots, unknown storage categories, retirement and retry. The broad native suite ran once against final IR340 test fixture v3: **1,084 passed, 16 failed, 12 ignored, 35 filtered**. The failed-name set exactly equals the 16 inherited failures reproduced by unchanged IR338 (1,079 passed, 16 failed, 11 ignored, 35 filtered). The extra ignored test is the external positive-storage diagnostic. This is not an all-green broad suite.

Initial failures and corrections remain visible:

1. The first external diagnostic wrapper was launched before its copied test source existed, then failed while pinning that missing source. Its subprocess result was not checkpointed; stdout is unavailable. The wrapper failure receipt says so explicitly. Passing focused tests were preserved.
2. The first U32 positive-storage diagnostic hit its default JIT limit (4,676 blocks required; 4,096 allowed), before mapping or native execution. A test-only v4 fixture uses the same explicit `NoLimits.compilation()` path already used by the benchmark emitter; its positive U32 run passes. The original failed receipt remains retained. Wide's passing v3 result is reused without rerunning it.
3. The initial qualification helper required peak overlap exactly four and stopped after 321 exact outputs, including an exact U32 restored full-input output with peak three. The corrected policy requires four tasks, one batch and peak overlap 1–4; scheduling cannot guarantee overlap four. Only the remaining 63 cases ran. Two original/restore U32 ordinary full-input qualifiers observed peak three. Both are retained, as are all timing/resource overlap histograms. No already-completed case was repeated to improve timing.
4. The first primary paired-analysis attempt used the wrong result schema and failed; it was corrected offline, without repeating a benchmark. Its error receipt remains retained.
5. The packet verifier's first development attempt used the wrong wide-control name; it was corrected offline. `verifier-development.json` records that packaging error. It did not run or replace any benchmark.

All new qualified managed candidates preserve exact total and numeric work against their own compiler/source full-input qualification. The retained, unselected historical IR335 typed-fixed-byte control still has unattributed total-work variation while its numeric worker bill remains exact. All of its observations and work histograms are retained; this packet makes no total-work constancy claim for that control. Worker peak overlap histograms for every managed variant and each cohort remain in `report.json`.

## Published source authority and offline review

Required published Git objects:

- IR338 compiler-equivalent base: `b939e42a6cbbb6e9cd5a747d6c5ba1ba09567164`.
- Exact published IR340 source: `1ef5985a9c58fd2857bdf8eedc06a2b7b50ef622`.

Both are ancestors of the published branch. The old pre-rebase IR338 object `6f7e0458e088cf7e50c97ebfe2d15e7bca02b22f` is optional; a fresh checkout does **not** need it. When that historical object happens to exist locally, the verifier additionally checks it. Source application/publication receipts are retained under `IR340-publication/`.

The frozen IR338 base inventory contains 2,817 files. The exact ten-file overlay (including IR340 identity and one new arena-test file) reconstructs all **2,818** files of the measured production compiler, and each is checked against the published IR340 commit. All 164 literal includes are checked against the published base. Test-only fixture inventories reconstruct 2,819 files; their only extra changes are the diagnostic module include and diagnostic test source. The earlier identity-338 scratch production build is retained as a historical development receipt, separate from final IR340. Final library and test receipts pin their own frozen inventories. All eleven non-GPU crate stages preserve the base source/library/command/environment closure; the GPU library is rebuilt coherently. IR340 uses work policy 127, ABI 42, image 11.

`measured-source/IR338-to-IR340.patch` is the canonical offline reconstruction patch. The original scratch candidate patch is also retained with its original hash. The final source-publication receipt confirms the exact ten files. No giant duplicate full source trees, executables or libraries are stored: immutable inventories plus minimal override source bytes reconstruct the compiler, and omitted binaries keep exact source/build/link/hash receipts.

Run from a checkout containing the two required published commits:

```sh
python3 benchmarks/related-post-gen/integration/managed-dense-storage-20261112/verify.py
```

`verify.py` requires only Python's standard library, local Git objects and this packet. It does not use the original `/private/tmp` workspace, network, compiler builds, benchmark/native execution or third-party packages. It validates compressed and decompressed hashes/sizes, all output equality, seeded execution order, raw timer parsing, medians, fastest eligible Rust references, both paired bootstrap analyses, resource counters/windows, exact work and observed concurrency histograms, all twelve qualification bundles and link audits, source and literal-include closure, unchanged non-GPU stages, focused/broad tests, positive physical bank dimensions, and the exact-set audit replay. The `assemble.py` archival script records original paths for provenance and is not needed to verify a fresh checkout.

The archive contains **7,260 logical records** represented by **3,958 unique gzip blobs**, totaling **21,639,419 compressed bytes**. Repeated multi-megabyte exact outputs share one content-addressed blob. `records.json` maps each original logical record to its compressed and uncompressed digests and lengths. `report.json` is the comparison index; `verification.json` records the final offline verification and pins the verifier/report/manifest. The prior `managed-setup-reuse-20261112` packet remains unchanged.
