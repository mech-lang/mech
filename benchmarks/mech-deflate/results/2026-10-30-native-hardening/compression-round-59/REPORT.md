# R59: reuse the interned schema key during frame admission

The measured complete eight-byte level-12 call moves from **13.181703× to 13.122997× the shared reference**, a **0.445361% reduction in pooled elapsed time**. This change removes exactly **600 allocation requests**, from 8,773 to 8,173, and 139,789 retired instructions in the separate counter cohort. It does not reduce peak heap or the large scratch allocations. One of four primary pairs is slower, and the separate CPU-cycle cohort is substantially worse; both are retained below.

The production change reads `entry.key()` from the existing immutable schema-table entry instead of encoding and hashing `entry.schema()` again. It keeps the same schema ID lookup and current key, account, value, shape and footprint checks, with the same admission charges and error order. There is no new cache, retained invocation permission or benchmark-specific path. R57 and R58 were rejected and restored before this experiment; the comparison uses accepted R56 production.

| Metric | Shared reference | R56 baseline, rerun | R59 candidate | Candidate / baseline change |
|---|---:|---:|---:|---:|
| Primary elapsed, ms | 0.291000 | 3.8358755 | 3.8187920 | −0.445361% |
| Multiple of reference elapsed | 1× | 13.181703× | 13.122997× | |
| Retired instructions | 1,364,372 | 42,989,744 | 42,849,955 | −0.325168% |
| CPU cycles | 935,292 | 12,172,185.5 | 13,511,916.5 | **+11.006495%** |
| Allocation requests, including reallocations | 95 | 8,773 | 8,173 | −6.839166% |
| Requested bytes | 9,021,923 | 98,243,843 | 98,228,396 | −0.015723% |
| Reallocations, included above | 0 | 540 | 162 | −70.000000% |
| Incremental peak usable heap, bytes | 9,257,328 | 20,482,072 | 20,482,072 | unchanged |

The primary cohort contains four independently prepared executable pairs, 16 alternating blocks per pair, 31 observations per variant per block: 3,968 timed observations, plus 16 excluded warmup/profile observations. The fresh shared reference contains 20,000 calls. Pair changes are −0.640727%, **+0.276494%**, −0.147087%, and −1.108054%; 45 of 64 blocks improve. Blocks within a prepared pair are descriptive, not independent repetitions. The CPU cohort contains 32 observations per executable; heap contains eight per Mech variant and 32 reference observations. These cohorts were run separately and serially. The adverse cycle result is not replaced by primary elapsed or explained away as established noise. This is a measured small elapsed improvement and exact construction reduction, not an across-the-board hardware-counter improvement.

The full censuses reconcile to the independent heap counters. Exactly 222 eight-byte allocations and 378 reallocations disappear. Their total requested bytes are 15,447. Every other size/kind count is unchanged:

| Mutually exclusive stack family | Before requests / bytes | After requests / bytes |
|---|---:|---:|
| Composition canonical schema encoding | 600 / 15,447 | 0 / 0 |
| Other composition validation | 36 / 528 | 36 / 528 |
| Other canonical schema encoding | 148 / 4,893 | 148 / 4,893 |
| Remaining complete-call requests | 7,989 / 98,222,975 | 7,989 / 98,222,975 |

The 50 requests of at least 64 KiB still request 97,086,052 bytes. Path capacity is still requested 35 times, the U16 tree five times, and the U32 cache once. Requested capacity is not initialized bytes copied. Total peak usable heap remains 163,464,584 bytes including retained compiler/catalog/image state; post-call RSS is 319,782,912 → 319,766,528 bytes. That RSS measurement is not peak RSS or minimum standalone execution memory. All CPU observations have zero page-ins.

Fresh CPU samples contain 1,312 baseline, 1,320 candidate and 1,516 reference weights. Generated-entry exclusive weights are 173 → 172; frame-action ancestry is 318 → 328. The reference's largest opaque symbol has 1,424 weights. `profile-numbers.json` preserves symbol names and overlapping ancestry groups. These are CPU-time samples, not instruction counts, exact timer fractions or an explanation of the cycle regression.

Provenance is exact:

| Item | Revision or SHA-256 |
|---|---|
| PR base | `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b` |
| Published accepted R56 | `46617b61f3ccf0e4a6ddf27fe37f9f4003016ac4` |
| Baseline release source | `198b7818a0e0d0aac518678445a4bd31c08f3267` |
| Production change | `7658ace6877bc48bc7445ce967ca48994149c5d3` |
| Separate CI inventory change | `32f4d59da54f063969f1aff73e67325804177b84` |
| Candidate release source | `09b53c3d57f52fef8d9a067b9f110f48c0ddbafc` |
| Compressor source | `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9` |
| Bytecode | `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7` |
| Object, unchanged | `1a8bb46e9f78fec2147b3242fab5651e470f86ba0347f4f11c7f1aedd5008777` |
| Descriptor, unchanged | `1b4dbdd2fb0c1f9f51ae7e9dd0bc931bbc2804e6cb949653461d6348c31615c9` |
| Metadata, unchanged | `a1d3f2710f3c2bb40eeb7cd10529b87ad6eae1d3786b3811823ee07ea3c3cac9` |
| mimalloc 2.1.7 object | `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874` |
| Reference executable | `3941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324` |

IR/work/ABI/bytecode versions remain 132/95/37/14; generated code is 13,046,668 bytes and entry stack is 108,368 bytes. The Object and two JSON artifacts remain byte-identical. The host-side Engine change required serial Engine/Compute/Stdlib/Runtime/GPU rebuilds with retained opt-level 3, feature selections and warnings as errors. Their build times were 190.715 / 1.909 / 0.323 / 13.394 / 21.564 seconds. Core, math, syntax and external dependencies were reused unchanged. Archived build commands, environment, per-library features/profiles/hashes and worker bindings identify the actual linked graphs. No compilation ran in the measurement windows.

The host was an Apple M1 Mac mini with 8 GiB, macOS 15.6.1 (24G90), Rust nightly-2026-03-03 (`ec818fda361ca216eb186f5cf45131bd9c776bb4`), LLVM 22.1.0. The reference compiler is `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`, package `671077457ff7ef3a3d44caf715a3fc595addf379`.

All required local qualification passes after a whitespace-only formatting correction (`3341dea859f498f3c636cb8021360c891cfc1ee2`); the measured release and binary bindings remain those listed above. The focused test passes on parent and candidate; each executes original and decoded artifacts. Their exact visit receipts agree: success uses 12,014,128 logical compute work; wrong key, nonzero managed words and unadmitted owner reject with `InvalidInput` after 12,013,616; zero funding rejects before the wrong key with required work 12,013,104 and zero charged work. Retained aliases and memory stay unchanged through failure and retry, and final managed bytes/nodes are zero. The executed command groups pass: cleanup preparation 2; GPU runtime 327; work/memory/values 32; existing components 234; connected composition 8; path mechanism 1; linked acceptance 12 cases; scalar controls 12 (maximum ratio 1.015847, unchanged 1.05 ceiling); frame allocation/work controls 8; Python CI contract/inventory 132; native CI selection 32 listed and 32 executed. These groups overlap and are not a unique-test total. Nine of ten initial architecture/style commands pass; the introduced formatting failure is retained, corrected, and its exact formatter command passes on recheck. Routing, catalog, layout, factory safety, warning policy, unsafe boundaries and diff checks pass. Exact commands and profiles are retained in `validation.json` inside the analysis-input archive and embedded in `scorecard.json`. The inherited reduced-catalog knucleotide test exclusion remains explicit; unchanged Core suites were not rerun. No hosted CI or review approval is claimed.

Two initial test-construction compilation failures (wrong record draft representation, then wrong exported path) are retained in `unit-before-build` and `unit-before-fixed-build`; both were corrected before parent/candidate measurement.

All complete-call observations retain exact raw output `[75,76,74,78,73,77,75,207,0,0]`, independent inflate, input aliases/content, output capacity 13 with clear/reuse, logical work 60,260,219,686 and final managed bytes/nodes zero. Comparison work remains 47,612. Source algorithms, tables, search limits, allocation sites, real buffer capacities and `evaluate_into` timing boundaries are unchanged. Logical work is not a hardware instruction count. No larger-input performance claim is made.

Run `python3 reconstruct.py /private/tmp/replay-r59-new` from this report's directory to verify every archived file version, overlay receipts in creation order, independently recompute the primary summary from raw observations, and reproduce `scorecard.json`, `scorecard.csv`, `profile-numbers.json` and `attribution-comparison.json` byte-for-byte. Replay runs only Python analysis; it does not build or execute a compressor. The script rejects a nonempty existing replay directory. The completed replay receipt is `reconstruction.json`; all original failed receipts remain alongside their successful corrections.
