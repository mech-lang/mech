# IR303: borrowed Map keys through native consumers

**The complete lowering is working and retained as the next compiler baseline.** Confirmation processing is **1.878375 ms / 1.205535×** the fastest eligible Rust **1.558125 ms**. The remaining gap is **320.250 µs (17.05% of current Mech time)**. Cranelift remains production; this adds no LLVM dependency. IR295 and IR302 remain comparison controls.

The isolated lowering wins against the same new Engine/runtime with producer emission disabled in both independently randomized cohorts: median paired processing changes −1.590% (95% [−2.957%, −0.142%],22/31 favorable) and −1.870% ([−2.809%, −0.984%],24/31). Against IR302 and IR295, the first intervals cross zero; confirmation intervals are strictly negative. Those first observations are retained. Whole-process performance is not an acceptance gate.

| Implementation | First processing | × fastest Rust | Confirmation processing | × fastest Rust |
|---|---:|---:|---:|---:|
| Fastest eligible Rust | 1.546542 ms | 1.000000× | 1.558125 ms | 1.000000× |
| IR295 retained processing control | 1.934458 ms | 1.250828× | 1.917250 ms | 1.230485× |
| IR297 retained constructor control | 1.927500 ms | 1.246329× | 1.925584 ms | 1.235834× |
| IR302 immediate parent | 1.921875 ms | 1.242692× | 1.914625 ms | 1.228801× |
| IR303 emission disabled; same new Engine/runtime | 1.928250 ms | 1.246814× | 1.923333 ms | 1.234389× |
| IR303 borrowed key view | 1.899208 ms | 1.228035× | 1.878375 ms | 1.205535× |

Both Rust references are `rust-ordered-mask-tables256`; all22 Rust controls are measured,20 eligible for processing. Input decoding and restoration are excluded from the continuous post-JSON processing interval. Input preparation/admission, allocations, native execution, worker creation/run/join, internal cleanup and result materialization are included. There are31 randomized samples per implementation in each cohort, with separate7-sample resource runs. All1,674 latency and392 resource outputs match Rust exactly.

| Confirmation resources | Rust reference | IR302 parent | IR303 |
|---|---:|---:|---:|
| Processing allocation requests | 58 | 1,751 | 1,291 |
| Requested bytes during processing | 6,825,713 | 7,861,191 | 7,807,730 |
| Additional peak live requested heap bytes | 5,648,124 | 5,763,966 | 5,765,741 |
| Total peak live requested heap bytes | 7,703,209 | 14,107,494 | 14,109,947 |
| Processing instructions, all workers | 17,664,479 | 34,317,435 | 33,879,919 |
| Processing cycles, all workers | 5,898,704 | 10,976,155 | 10,830,541 |

IR303 saves460 processing requests (26.27% vs IR302;698/35.09% vs IR295),53,461 requested bytes and437,516 instructions vs IR302 in confirmation (first instruction reduction417,283). Additional peak requested heap rises1,775B; total peak rises2,453B. Requested bytes and peak heap are different measures. The generated native program is1,031,248B with1,584B maximum stack, versus IR302 1,031,144B/1,584B. The latency executable is17,097,728B.

Whole-process medians are37.626ms for IR303 and36.376ms for IR302; the paired change is+3.16%,95% [−1.44%,+11.28%]. Restoration rises from8.372750ms to8.600458ms (+2.36% paired,95% [+1.90%,+2.86%]). Both costs stay visible and are retained under the user's instruction that whole-process regression is not a gate. Process peak RSS is27,721,728B for IR303 versus27,787,264B for IR302; peak physical footprint is16,990,568B versus16,974,208B.

## Implemented compiler/Engine change

A complete-use proof replaces a private `map/keys` String matrix with a canonical-order view that owns the genuine immutable Map snapshot. Generated length, private-frame and adjacent retained MapInsert routes consume the sealed view. It survives legitimate retirement of the original histogram handle and insertion-owner publication when later source uses remain. A private typed virtual arena generation rejects stale definitions without constructing a fake canonical Value. Current dictionary values remain independent of borrowed key-set identity.

Late owner refusal consumes the view once and invokes the ordinary canonical factory under the original source operation's remaining compute/comparison allowance. Its already-spent setup and global intervening work remain charged. Metadata is released before ordinary materialization. An enclosing external relative cap retains immediate canonical execution. Logical shape, output-cardinality/bytes, clone-payload, physical metadata, node, identity/account and cleanup obligations remain checked.

Qualification:135 Engine collection/footprint contracts (all129 parents retained),19 projection/frame/relative-budget regressions,451 native composition CI contracts (all446 parent names retained), retained map/image/caller/UTF8/protocol contracts,128 scalar and5 DEFLATE cases with every nontiming field exactly equal to IR297, and32 exact same-Object linked outputs for each of default and emission control. Full5000 original/restored observation verifies one owned view, zero canonical key matrices, zero per-key String Values,28 borrowed insertions, original histogram-handle retirement and exact results. Finite one-less caps, late fallback, foreign factories/accounts, generation exhaustion/staleness, long/Unicode/NUL/empty keys, bounds, aliases, atomic failed publication and retry are covered. Old Stage-A matrix fault tests explicitly disable the new view so their original canonical-generation contract stays exercised; separate new tests cover the virtual representation.

## Fresh profiles and next compiler target

The unchanged production native Objects were profiled with fresh invocation/loan/worker/cleanup state; the same-Object audit and final5000 output pass. A separate64-output diagnostic capture aligns phases with the currently fastest Rust control. Probes affect timing; phase medians and inclusive sample stacks are not additive production savings.

| Aligned phase | IR303 diagnostic median | Rust diagnostic median | Median paired Mech − Rust |
|---|---:|---:|---:|
| Preparation, interning, eligibility | 874.229 µs | 616.021 µs | +254.875 µs |
| Triple index and coverage | 352.708 µs | 187.188 µs | +166.730 µs |
| Quartet index and masks | 266.417 µs | 199.125 µs | +66.230 µs |
| Queries, cleanup, materialization | 485.833 µs | 583.438 µs | −100.584 µs |

Preparation wins all8 diagnostic pairs against the same-runtime emission control (median paired −32.854µs), and its requests fall1,393→933 (Rust38). Actual key extraction falls462→4 requests. Detailed dictionary/lookup prelude still contains590 requests (about63% of preparation) and a262.312µs diagnostic envelope, including useful work and probes; it is not promised removable time. The native sample still shows private-frame/footprint admission plus dictionary publication and lookup preparation alongside generated numeric kernels.

Next: Stage C, retaining the nonempty generic UTF8 seed and completed dictionary through typed immutable lookup. Prove full nested/private-frame uses and exact current value generations; preserve canonical fallback and finite/logical obligations. Investigate a dictionary with an immutable key set and a mutable numeric value buffer indexed by borrowed key ordinal when source/seed identity is proved, supporting arbitrary UTF8 and sizes. Existing empty-seed/short-packed-key FrozenPackedMap is not a substitute. Completed-producer key-range applicability into posting scatter remains separately attributable; a key range does not prove posting-ID or destination-position bounds. Keep input-admission/private-frame overhead in the current ledger.

The historical prototype packet remains `borrowed-map-key-view-prototype-20261108`. Its separately recorded dynamic-to-exact Map-cardinality conversion lowering gap remains open. The measured benchmark source was not changed around it.

## Evidence and reproduction

Source commit: [`61c5a6e0a5008fbecbab31ddcb84bab96afad7ba`](https://github.com/mech-lang/mech/commit/61c5a6e0a5008fbecbab31ddcb84bab96afad7ba), branch `codex/related-borrowed-map-key-views-20261108`. PR990 continues targeting the PR973 branch; this research is not silently integrated into its IR242 head.

`measurements/` contains the four complete cohort reports. `evidence/first` and `evidence/confirmation` preserve raw runs, outputs, schedules and paired analyses. `evidence/phase-diagnostics/profile` preserves all64 diagnostic runs and traces. `evidence/native-profile*` preserves the fresh executable profile. `builds/`, `regressions/`, `runners/`, and `tools/` retain qualification, link/source inventories and recipes. The first fixture-order error, visibility build error, old-test representation failures and pre-measurement disk refusal are retained. Completed executables were losslessly archived only after use; archive manifests record hashes/modes, and every measurement runner remains intact. Named hardlinked output aliases are stored as `.json.ref.json` with an exact hash/byte count and a pointer to a shared byte-for-byte copy in `evidence/output-objects`; this preserves every output without expanding identical files. Rust/source-toolchain dependency paths in raw receipts are original absolute paths. Files named `.rs.txt` preserve source verbatim without adding new Rust inputs to the frozen compiler inventory.

Run `evidence/run-cohort.py` for the first cohort and append `confirmation` for the independent cohort only after recreating their pinned paths and verified dependencies. Existing output paths are rejected. `evidence/prospective-rule.json` predates both runs. Bootstrap seeds/resamples and every first/adverse observation remain available. The goal of meeting or beating Rust is still active.
