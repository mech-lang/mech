# Reuse already computed guarded reads — R234

Retain the eight-file standalone compiler experiment over R233. Both predeclared complete cohorts and all 48 pooled balanced blocks pass all 25 unchanged 1.05 point/block gates. Pooled geometric elapsed improves **0.813%**. Natural32 improves **2.536%**, with **1.984% fewer retired instructions**. Instructions decrease on 22 cases and tie on three; cycles decrease on 24 and rise on one. Preserve the five adverse pooled block means and the second cohort's **0.0249% adverse geometric mean**. This is a measured incremental gain; parity remains open.

## Complete-call measurements

| Case | R233 / Roc | R234 / Roc | Pooled blocks | Instructions | Cycles |
|---|---:|---:|---:|---:|---:|
| reymont-prefix-32769 | 2.306883× | 2.248385× | -2.536% | -1.984% | -2.049% |
| 33027-periodic-window | 1.688313× | 1.662209× | -1.546% | -0.772% | -0.920% |
| repeated-32769 | 1.591807× | 1.567436× | -1.531% | -0.774% | -1.561% |
| random-32769 | 2.794810× | 2.766237× | -1.022% | -0.991% | -0.888% |
| 16384-distribution-change | 2.773834× | 2.761977× | -0.427% | -1.404% | -0.915% |
| eight-literals | 0.514124× | 0.505476× | -0.524% | -0.028% | -1.839% |
| reymont-prefix-16384 | 2.247177× | 2.199797× | -2.108% | -2.064% | -2.378% |
| empty | 1.976190× | 1.976190× | -1.362% | +0.000% | -6.827% |
| one-stored | 1.976190× | 1.976190× | -0.323% | +0.000% | -4.517% |
| seven-stored-prefix | 2.000000× | 2.000000× | +0.593% | +0.000% | +4.238% |
| eight-overlap | 0.498921× | 0.496112× | -0.056% | -0.026% | -5.029% |
| 320-overlap | 0.559965× | 0.555879× | -1.052% | -0.394% | -3.055% |
| eight-literals-prefix | 0.500071× | 0.499067× | +0.984% | -0.023% | -0.814% |
| 128-biased-dynamic | 0.504412× | 0.504846× | +0.237% | -0.175% | -4.291% |
| 32-high-entropy-stored | 0.528626× | 0.533029× | +0.890% | -0.054% | -2.446% |
| random-64 | 0.561063× | 0.564615× | +1.162% | -0.092% | -4.862% |
| random-256 | 0.658970× | 0.656744× | -0.277% | -0.242% | -4.682% |
| random-1024 | 0.745581× | 0.742756× | -0.553% | -0.515% | -7.145% |
| repeated-1024 | 0.671363× | 0.664890× | -1.132% | -0.760% | -4.753% |
| reymont-prefix-1024 | 0.938722× | 0.936865× | -0.217% | -1.001% | -4.143% |
| heldout-window-32767 | 2.299683× | 2.248854× | -2.210% | -1.985% | -2.496% |
| heldout-window-32768 | 2.303619× | 2.249963× | -2.329% | -1.989% | -2.334% |
| heldout-slides-65537 | 2.381857× | 2.328957× | -2.221% | -1.801% | -1.618% |
| heldout-random-65537 | 3.126098× | 3.099287× | -0.858% | -1.003% | -1.176% |
| heldout-mixed-131073 | 2.569527× | 2.524480× | -1.753% | -1.706% | -1.705% |

Both columns use fresh matched reference denominators from these cohorts. They are point-median multiples; the separate percentage column compares medians of balanced block means. A changed historical denominator is not a compiler effect. Eight literals measures 148.625→146.125 µs against 289.084 µs reference; its balanced-block improvement is only 0.5245% and instruction reduction 0.0275%, rather than a new large tiny-input algorithmic gain. No additional confirmation, altered expected value or increased allowance.

324,444 complete primary calls /324,144 retained observations,3,900 separate CPU/heap calls and 50 full-stream smokes. One executable per variant accepts all 25 runtime inputs:20 original and five held-outs through 131,073 B. Both sides use fixed level 12, equal 200 ms warmup, the same Rust clock and validation library, balanced order, pinned mimalloc, and live borrowed input/output aliases. Compression, real scratch construction, output transfer and cleanup stay timed. Preparation, validation/logging and old output drop remain outside. All streams match reference exactly and independently inflate. Final nonreturned private payload/owners are zero. Builds/tests/profiles never overlap primary or counter collection.

## Transformation and proof

Use existing numeric traversal expression/read-version recipes to reuse a scalar value already computed on every current path. Availability intersects across partitions and control joins and starts empty for every traversal/invocation; copied versions have separate SSA snapshots. Array view identity is provided by the existing read-only numeric provenance proof, which validates every definition and next-state carrier and rejects swapping/unknown retargeting. Growth and unknown actions terminate the supported prefix. No pointer, funding authority, invocation permission, or latch snapshot is retained.

A raw typed load may reuse a previous scalar only for the same array and logical index at the current conservative content epoch. Every possible write invalidates that memory fact across aliases. Two already evaluated getter results may also be reused if their recipe DAGs agree: equal literal bits, identical values, successful word-preserving unsigned conversions, the same versioned read, and either identical joins or a currently proved selected branch. Old scalar equality can survive a later write; a newly read epoch cannot match it. Other arithmetic, unknown/ambiguous recipes and inconsistent join proofs decline.

The existing later bounds, checked arithmetic, assertion and lexical failure sites still execute. A forwarded scalar replaces only an already computed load/result; no operation is hoisted across a source check. Ordinary exact-resource entries receive no new certificate. No program name, field name, phase number, size, compression path or source selector is part of eligibility. The source and strict MBC14 remain byte-identical; the R225 source bindings and separate nontrapping ABI-packet idea are uncombined.

Real requirement: OptimalPathMatches and optimal-path-position extract two fields from repeated reads of each packed cache word/header. The pinned CompressOptimal reference source repeats those reads as well; the compiler should eliminate repeated work generically. In the complete emitted image only four existing path bodies change; root and 132 other numeric/map bodies remain byte-identical. Private reports may record additional equivalent operand bindings whose code was already optimized to the same instructions. Report actual machine-code and dynamic counts separately.

Qualification includes six proof tests, three new checked native programs (same index, distinct index, write/live alias) with changing recursive visits and 279 fresh resident comparisons, original/decoded parent/candidate forms, assertion/bounds/overflow order, unchanged previous publication, cleanup/retry, plus the unchanged real path component and matching controls. Old path/matching resident observations retain their provenance and default work denials; they are not counted as newly executed resident calls. All new source cases execute current native code and an independent oracle. No allowances or expected values change.


There are eight static raw-read substitutions in four real path bodies, alongside 13 reported operand substitutions in each. Reports in other bodies merely identify equivalent bindings; their actual machine code is identical and no instruction saving is claimed there. The four bodies shrink 2,096→2,048 B each, 192 B total, with 288 B frames unchanged. Actual disassembly of each representative path body loses four load instructions and two conditional branches; load counts include spills and are not source-read counts. The complete root Object and 132 other numeric/map bodies remain byte-identical. Root text 5,649,872 B, root stack 55,088 B and full worker 6,623,288 B remain unchanged.

The intermediate typed-load-only version and source patch are preserved in the packet. It was compiled/inspected, not primary-timed. The final version also reuses the already evaluated getter result, letting the backend retire its unused join. No benchmark source change is combined.

## Validation and profiles

692 maintained tests pass with the same single six-feature configuration (`aot`, `jit`, `canonical-native`, `canonical-native-runtime`, `canonical-native-program`, `canonical-native-program-runtime`) and `-D warnings`, with the same seven inherited exclusions. Six new proof tests execute focused and in the full suite. The existing 1,057,536 admitted/declined native value controls run twice with unchanged expectations. All 7,064 native/oracle source controls pass:1,116 new checked-read cases,160 unchanged path cases,5,788 unchanged matching cases across parent/candidate and original/decoded. 279 resident calls are fresh; other forms reuse those current-round results. Old path/matching resident observations are explicitly pinned and not recounted. The path's 34 inherited default-work denials remain. No assertion or load error is replaced by a raised allowance.

Fresh 25,529 wall-profile calls and 104,666 qualified actual-PC-profile calls use the exact current executable, Object and symbol map. Natural32 wall samples are 97.855% generated exclusive and 0.525% host-under-generated, with the rest outside generated execution. Current PC samples assign 2,511/5,334 to the path scan, 1,285 to matching and 1,182 to the generated root. These are CPU-time samples, not per-function retired-instruction counts. The separate whole-call counters establish the instruction deltas.

Every case retains identical payload requests/requested bytes/initialized copies/zeroing/private peak and global request/byte/reallocation/incremental usable-peak fields. Eight literals still has 230 allocation requests including 6 reallocations and 9,030,708 requested bytes; the matched reference has 96 requests / 9,021,944 bytes. The older 95-request reference is a different retained harness observation, not a minimum. Incremental usable peaks are 9,192,488 B versus 9,257,360 B. Whole-worker RSS/physical peaks differ and remain in the report; no whole-worker memory or streaming improvement is claimed. Completed test client/executable files totalling 2,996,955,064 logical bytes are verified before deletion; source, hashes, receipts and measured workers/Objects remain.

## Remaining failures and scope

The maintained module-layout and native-build-analysis checks pass; all eight changed Rust files pass formatting. An additional audit exposes an inherited experimental-only module collision: compact/owner_flow.rs coexists with compact/owner_flow/. Parent R233 and candidate scans both fail, and every involved file is hash-identical. This failure is not relabelled green. Assign its separate layout repair before ordinary integration; do not mix unrelated reorganization into this read optimization. Exact paths/hashes and both failing receipts are in review-checks.json/failure-disposition.json.

Initial diagnostic-client builds exposed a boxed Matrix construction mismatch and an unused shared publication gate under -Dwarnings. Both are corrected in the harness; the latter is now actually exercised. One dependent launch after a failed build and read-only inspection errors remain recorded. There are no introduced compiler build/test/semantic failures or new runtime/program resource denials. No completed primary/counter/profile observation is discarded or repeated. CI is cancelled under the standing instruction, not green; this PR remains draft with a fresh review requested.

Normal compiler R215 at `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5` remains IR/work/ABI/MBC215/107/39/14. The standalone comparison omits its additional exact resource/admission/accounting/publication/recovery contract; no ordinary full-compressor timing claim. Source/MBC pins, algorithm, tables, depth 300, nice 258, near-EOF limits, allocation sites,262,144 U16 tree,1,500,513 U32 cache,305,257 U64 path capacity and output cap 13 clear/reuse remain. No LLVM, new operation class, new branch/PR or R225 source combination. Small original/decoded pairs rerun; the strict complete image emits once. Multi-MB inputs, a second complete-image emit and historical packet/write censuses are not claimed newly run.

Next investigate unused argument-packet loads in these exact native prologues. Prove only the compiler-owned packet's current span/lifetime before enabling nontrapping access or eliminating dead lanes. Keep source-array guards, alias/failure/retry controls and prefetch hints separate; no blanket readonly/movable-load flags. The remaining hot path is already vectorized, so do not propose that completed operation as unexplored work.

## Pins and verified evidence

- base: `768936add16b8f4e87588fc96edf7116329289a8`
- normal_compiler: `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`
- compiler_source_manifest_sha256: `d38d8de7f410fd50db57234122b9ace76ad4a4b11e1a6c2dbb03aa51357ccc4a`
- compiler_patch_sha256: `d564505ca3a2b1692d722b000ed301d8ad11cea5ebc868783edfb7d48f3dd006`
- source_sha256: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`
- bytecode_sha256: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`
- object_sha256: `1561d3a0d98b1db3e8176fb23d5725f543e9accf0b3b383d765c0d80855573fc`
- parent_object_sha256: `1c2796ec92aa28a3865cdd7679b14b21a1fccadfeb0f3e563fe30b5c60f8cd7f`
- worker_sha256: `abada0410847dcb96a17f7425c5c2f003d000b7b5a56425d25e75f990ceb7b5c`
- reference_worker_sha256: `ed1ab9175cd175d26d453afc26ba1fa7adb4827ec705aa1cf9bcd231ac84860c`
- allocator_sha256: `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`

[Full compiler source, minimal patch, both cohorts, counters, profiles and tests](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r234-read-reuse-f6ce37405bf8.tar.gz). SHA-256 `f6ce37405bf8bda68d989b96f39ce2cda90c730bfec021239a6b389335645724`;6,949,660 B. All 1,677 public file hashes and numerical replay verify. Native binaries/Objects, MBC, full executable descriptors/metadata and raw traces/stack samples are excluded. The immutable packet includes the full 25-case reports, allocation/physical-memory cohorts and inherited failure evidence.
