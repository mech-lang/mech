# R69: immutable call-port array experiment — rejected

The candidate removed **388 allocation requests** but increased paired elapsed by **2.4299%** and retired instructions by **4.4757%**. It is rejected and the exact accepted production implementation is restored. **6,552 is a rejected candidate count; accepted production remains 6,940 requests.**

The change retained immutable input/output descriptor arrays in `BoundCall` using `Arc<[ResolvedValueDescriptor]>`, preserving each descriptor’s original inline type/schema/shape layout and accessors. Current-descriptor refresh still validates arity, type and dimension lifetime before creating replacement arrays. The experiment changes neither current argument/account authority nor the compressor.

## Exact revisions

- Accepted base/checkpoint: `164f1d1e403cee3c37b63d0bb6df2116dd56b10a`.
- Actual accepted compiler build used in fresh before observations: `c9199ed6687c0125f64e9bf2abd0b0057d5cbf0d`.
- Rejected production patch: `f95e2a8f3b3f5da8a8623d4c2da84ada4842b222`.
- Actual coherent candidate build: `ca9045f075d21cd627fc2b353fbdd5f565a135ba`.
- Restored production source/checkpoint: `e385fb7b0948c5f6b8d3458eddc2ec5a5b3aeb85`; actual confirmation executable remains `c9199ed6687c0125f64e9bf2abd0b0057d5cbf0d`.
- Two independent call-refresh tests remain. The rejected-only allocation witness is archived in `findings-packet/rejected-call-port-allocation-test.rs`; the maintained allocation target and strict CI guard return to four.

## Complete eight-byte results

Fresh reference median: **0.286000 ms**. Four independently prepared executable pairs, 16 alternating blocks per pair, 31 measured complete compressions per block: **3,968 timed calls**. Warm/profile observations are separately retained and excluded. All four pairs are adverse; only **1/64** blocks improve. Median block ratio `1.025247180`; geometric block ratio `1.024933247`.

| Metric | Roc | Accepted before | Rejected candidate | Candidate change |
|---|---:|---:|---:|---:|
| Primary elapsed | 0.286000 ms | 3.618917 ms (12.653556×) | 3.706854 ms (12.961030×) | +2.4299% |
| Retired instructions | 1,364,372 | 40,530,533.5 (29.7064×) | 42,344,551 (31.0359×) | +4.4757% |
| CPU cycles | 941,763 | 11,382,057 (12.0859×) | 11,708,762 (12.4328×) | +2.8704% |
| Allocation requests, including reallocations | 95 | 6,940 (73.0526×) | 6,552 (68.9684×) | -5.5908% |
| Requested bytes | 9,021,923 | 98,127,494 (10.8766×) | 98,092,872 (10.8727×) | -0.0353% |
| Reallocations, already included | 0 | 162 | 162 | +0.0000% |
| Peak live allocator bytes | 9,257,360 | 163,464,584 (17.6578×) | 163,464,584 (17.6578×) | +0.0000% |
| Incremental peak live allocator bytes | 9,257,328 | 20,482,072 (2.2125×) | 20,482,072 (2.2125×) | +0.0000% |
| Post-call RSS | 14,761,984 | 319,406,080 (21.6371×) | 319,193,088 (21.6226×) | -0.0667% |
| Post-call physical footprint | 14,418,880 | 229,197,568 (15.8957×) | 228,902,592 (15.8752×) | -0.1287% |

Primary elapsed, CPU counters, heap counters, full allocation traces, and CPU samples are separate serialized cohorts. Do not combine their percentages. Post-call OS memory is not peak RSS or standalone minimum memory. Requested capacity is not initialized/copied bytes. No input streaming or copy-volume reduction is established here.

| Pair | Before ms | Candidate ms | Ratio |
|---|---:|---:|---:|
| 0 | 3.625042 | 3.696146 | 1.019615 |
| 1 | 3.630812 | 3.708063 | 1.021276 |
| 2 | 3.607458 | 3.723688 | 1.032219 |
| 3 | 3.606771 | 3.699646 | 1.025750 |

Separate CPU diagnostic elapsed changed **+2.8057%**. Its 32 observations per compiler are excluded from primary. All contrary pairs/counters remain in `scorecard.json`.

## Allocation mechanism

Complete original traces reconcile exactly to the independent heap counters. Ordered stack families below are disjoint; the counts describe observed allocation ancestry, not a complete object/call ledger. Symbol deduplication/inlining can move a request between labels.

| Observed family | Before requests | Candidate requests | Delta | Requested-byte delta |
|---|---:|---:|---:|---:|
| bound_call_clone_descriptor | 336 | 0 | -336 | -30,636 |
| bound_call_clone_other | 96 | 112 | +16 | +298 |
| call_construction | 6 | 10 | +4 | +1,120 |
| other_descriptor_clone | 368 | 312 | -56 | -5,106 |
| current_shape | 52 | 52 | +0 | +0 |
| other_binding | 193 | 193 | +0 | +0 |
| other_turn_planning | 166 | 166 | +0 | +0 |
| remaining | 5723 | 5707 | -16 | -298 |

The preselected 336-request descriptor subtree under `BoundCall::clone` disappears. Another 56 descriptor-clone requests disappear; the visible call-construction family adds four requests. A 16-request shift between other-clone and remaining labels cancels and must not be called an additional saving or newly constructed ownership class. The net is **−388 requests / −34,622 requested bytes**, with reallocations unchanged. Large-buffer allocations and initialized copy work are unchanged.

The schema-size-sensitive clone regression fails on the exact accepted Core library: one short-name clone requests `(27, 4,656 bytes)` and the equivalent long-name clone `(27, 397,866 bytes)`. The candidate passes the same witness at 1, 64 and 4,096 clones, then all five experimental constructor tests. This validates the construction mechanism, not its speed. The failed parent control is expected; it is not an unresolved compiler failure.

## Actual instruction windows and linked code

Fresh diagnostic windows place **96.011568%** of before instructions inside linked entry plus its callbacks. The candidate’s window means add **1,775,722.8125** inside instructions and **44,131.1250** outside instructions; **97.575018%** of the additional instructions occur inside execution.
Two entry calls occur per compression. The nonallocating probe surrounds whole entries, never individual loop iterations. Empty begin/end calibration and all raw rows remain retained without subtraction. Disjoint means add exactly; medians need not. This is not an instruction partition among callbacks, and it does not label all entry work overhead.

The freshly emitted Object, descriptor and metadata are byte-identical. Generated text remains **11,800,572 bytes** and machine-entry stack **107,344 bytes**. Actual host text changes **-15,988 bytes**; worker size **-6,368 bytes**. Static own-function counts do not establish executed instruction counts.

Actual release geometry is identical: `BoundCall` 160 bytes/alignment 8; `ResolvedValueDescriptor` 176/16; `CallMemoryPlan` 1,000/8. This rules out an outer-size change, not all access/ownership/code-generation effects.

One concrete consuming-code difference deserves follow-up: the accepted `NativeLibraryWorkBudget::charge_compute_with_limit` has 158 own static instructions with the maintained account debit inlined; the rejected build has 113 own instructions and a call to `charge_control_account_at_limits`. Both are real linked ARM64 code, and normalized Engine/GPU release flags match. The generic resident compute meter remains 304 own instructions. This establishes an inlining difference in a frequently used budget path; it does **not** establish its dynamic count or prove it explains all additional instructions. No specific blame is assigned to atomics or pointer chasing.

The initial call-port-only host audit completed before the audit was extended. Its raw output and producer-provenance note are preserved under `host-assembly-initial`; the extended audit ran fresh with producer hashes. No compiler or timed producer was modified for that extension.

## Fresh profiling and next priority

| Sample ancestry (overlapping) | Accepted | Rejected |
|---|---:|---:|
| frame actions | 313 / 1243 (25.181%) | 358 / 1311 (27.307%) |
| projection accounting | 41 / 1243 (3.298%) | 37 / 1311 (2.822%) |
| thread-local lookup | 41 / 1243 (3.298%) | 39 / 1311 (2.975%) |
| memory move/copy | 25 / 1243 (2.011%) | 37 / 1311 (2.822%) |
| tail cleanup | 41 / 1243 (3.298%) | 46 / 1311 (3.509%) |

These are CPU-time samples, not retired-instruction samples. Neither these overlapping ancestry percentages nor static instruction counts are multiplied by whole-call counters.

The next bounded patch removes `plan.call.clone()` in `apply_observed_turn_demand`, which already owns its `TurnMemoryPlan`. The reviewer attributes 149 requests to that specific clone; fresh before/after traces must establish the actual delta. The main instruction project is bounded diagnostic counts for debit/helper/context work, then reuse of the checked next-compute value and scoped live-context access as separate experiments. No new global descriptor representation retry is proposed.

Unread-constant imports, remaining planner geometry, region-exit transfer, shape temporaries and workspace reuse remain explicit follow-ups. They require current-reader/lifetime/accounting proofs; no forecast assumes all such allocations are removable or that request-count savings imply elapsed gains.

## Validation, restoration and failures

**82 required receipt checks pass.** Counts below overlap and are not a total of unique tests.

- Same fresh Core tests on exact original and candidate implementations: **371 each**, including the two new call-refresh retention/type/arity/lifetime/retry cases.
- Candidate experimental allocation target **5**; original negative control executes **1 failing test as expected**. Restored maintained target and strict CI inventory: **4 passing Rust tests**.
- Unchanged Engine planner **19**, projection **12**, shape **6**, plus their focused ordering/overflow cases.
- Full selected runtime **329**, work/memory/value **32**, preparation **2**, components **236**, connected cases **8**, linked acceptance **12 cases**.
- Original/decoded scalar controls **12** preserve the **1.05** ceiling, request/byte constraints and zero allocation slope; frame allocation controls **8** preserve requests, bytes and work.
- Selected GPU CI inventory/execution **35**; workflow Python **132**; ten architecture/catalog/format/warnings checks and all five type/memory source contracts pass.
- All twelve selected Core-dependent project crates rebuilt coherently with retained release flags; external artifact hashes frozen. Exact commands, environment, actual counts and outcomes are in the archived JSON receipts.
- Full hosted CI and all-feature coverage are not awaited/run. Complete unchanged source-checker mutation suites, the previously excluded complete knucleotide runtime case, larger compression inputs and full-scale timing are not claimed.

Environmental failure: the first worker link reports `write() failed, errno=28`. The already successful fresh emission and all twelve libraries are unchanged. Completed historical build/test outputs were removed; the exact link command then passed. The failed log/command, artifact hashes, cleanup inventory and retry are retained. This is environmental, not a new language/runtime rejection. No resource allowance, feature configuration or expected result was relaxed.

Restoration verifies production equality with `164f1d1e403cee3c37b63d0bb6df2116dd56b10a`, executes the current four allocation tests and CI guard, and runs **32** complete-call correctness confirmations using the pinned accepted executable. Their unpaired median is 3.578854 ms, excluded from all improvement claims. The retained semantic-test source matches the freshly passed parent Core suite. Final managed bytes/nodes are zero. No unresolved introduced or inherited qualification failure remains in the selected coverage.

## Pins and reproduction

Workload remains full eight-byte `abcdefgh`, level 12; tree 262,144 U16 entries, cache 1,500,513 U32 words, path capacity 305,257 U64 words, depth 300, nice length 258, original tables/search/near-EOF rules/allocation sites, output capacity 13 with clear/reuse, unchanged input aliases and `evaluate_into` timing. Exact output `[75,76,74,78,73,77,75,207,0,0]` and independent inflate pass. Logical work **60,260,219,686**, comparison work **47,612**; neither is a hardware instruction count. IR/work/ABI/bytecode **132/95/37/14**.

```text
source 331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9
bytecode a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7
allocator 38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874
```

M1 Mac mini, 8 GiB, macOS 15.6.1; pinned Rust nightly 2026-03-03 and mimalloc 2.1.7. Source/allocator/reference hashes, emitted artifact pins, all CPU/heap rows, both full symbolized traces and current samples are committed. `reconstruct.py /absolute/new/replay-directory` verifies every archived version and reproduces all eight numerical outputs without a compiler or compressor. It does not rerun performance or executable tests.
