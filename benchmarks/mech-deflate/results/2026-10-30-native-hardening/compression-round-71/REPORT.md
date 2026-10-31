# R71: forward the already checked compute total

**Rejected.** Complete eight-byte level-12 compression: **12.674781× → 12.975584× Roc**, **3.612313 → 3.698041 ms**, matched elapsed change **+2.373244%**. Retired-instruction change **+3.265793%**. Allocation requests and requested bytes are unchanged.

Reject the unannotated checked-total factoring: all four matched executable pairs and 60/64 blocks are adverse; primary elapsed increases 2.3732442860356% and separate retired instructions increase from 40394030 to 41713215.5. The intended duplicate calculation is smaller than the observed extra executed work. Preserve the regression tests and full candidate evidence; restore accepted production before publication. Actual linked-code audit follows, so no specific inlining cause is yet assigned.

## Design and revisions

`charge_compute_with_limit` must check its caller ceiling before it rejects a foreign current account. It already calculates a checked next compute total for that ceiling. The accepted inner debit calculated that same total again. The compute-only boundary now forwards its checked candidate into a shared local-next validator and the same shared-worker commit. Generic compute/comparison callers retain their original checked arithmetic and use those same validation and commit functions. The candidate is immediate current arithmetic, never stored permission.

The only operations between the caller precheck and account borrow are a const-initialized TLS lookup and an account identity test; no kernel/user callback can mutate the same-thread account. Both reservation additions still evaluate before exceeded-limit selection. Caller overflow/ceiling precedes foreign-account rejection; local/reservation/relative checks precede shared-worker debit; the shared worker commits before infallible local assignment. Direct debits do not advance observation high-water state. All current caps, account identities, observers, reentrant/nested scope restoration and failure atomicity remain checked. Context lookup and helper dispatch are unchanged, so their costs are not claimed as savings.

- Accepted checkpoint/base: `eea505587ba9a18fb75fb72dc997529a7ca53848`.
- Actual baseline compiler: `3979d6e360ed6d5def3aea745df94d970c92dc4d`.
- New regression source, accepted runtime body: `7edff4a938be59cf7ef2b5a1809983d208cff462`.
- Production arithmetic/validation factoring: `1eeaac22619f34abbf68d1cc04180df6f0b5cd07`.
- Worker-only parameter naming for non-native warnings-as-errors: `c77e0e8ee2455280755e86cde6c8651a6e9920a7`.
- Actual candidate release: `c77e0e8ee2455280755e86cde6c8651a6e9920a7`.

Only five Engine-dependent project crates are rebuilt: Engine, compute, standard library, runtime and GPU. Core, syntax, bytecode, math, range, logic, compare and external inputs remain frozen. Exact commands, selected features, optimization profiles, warnings-as-errors and all library hashes are retained. Production changes are confined to `resident/budget.rs`; the additional regression file and module declaration are test-only. No descriptor representation, algorithm, compressor source, allocation site, timing boundary or resource allowance changes.

## Matched measurements

Fresh reference median **0.285000 ms**. Four independently prepared executable pairs, 16 alternating blocks/pair, 31 calls/block: **3,968 timed complete calls**. Improving pairs **0/4**, blocks **4/64**; median block ratio `1.024469467`, geometric ratio `1.023308980`. All contrary pairs and blocks remain in the raw journals.

| Metric | Roc | Before | Candidate | Change |
|---|---:|---:|---:|---:|
| Primary elapsed | 0.285000 ms | 3.612313 ms (12.674781×) | 3.698041 ms (12.975584×) | +2.373244% |
| Retired instructions | 1,364,372 | 40,394,030 (29.6063×) | 41,713,215.5 (30.5732×) | +3.265793% |
| CPU cycles | 929,571.5 | 11,358,539 (12.2191×) | 11,667,023 (12.5510×) | +2.715877% |
| Allocation requests incl. reallocations | 95 | 6,791 (71.4842×) | 6,791 (71.4842×) | +0.000000% |
| Requested bytes | 9,021,923 | 98,112,208 (10.8749×) | 98,112,208 (10.8749×) | +0.000000% |
| Reallocations, already included | 0 | 162 | 162 | +0.000000% |
| Peak live allocator bytes | 9,257,360 | 163,464,584 (17.6578×) | 163,464,584 (17.6578×) | +0.000000% |
| Incremental peak live allocator bytes | 9,257,328 | 20,482,072 (2.2125×) | 20,482,072 (2.2125×) | +0.000000% |
| Post-call RSS | 14,761,984 | 319,258,624 (21.6271×) | 319,012,864 (21.6104×) | -0.076978% |
| Post-call physical footprint | 14,402,432 | 229,017,344 (15.9013×) | 228,804,352 (15.8865×) | -0.093003% |

Timing, instructions/cycles, heap counters, complete symbolized allocation censuses, CPU-time samples, entry windows and mechanism counts are separate cohorts. Do not multiply their percentages. Post-call OS memory is not peak RSS or a standalone minimum. Requested bytes are not initialized/copied bytes. This arithmetic change claims no buffer-copy, input-streaming or memory improvement.
Separate counter-cohort elapsed change **+2.727371%** is excluded from primary timing. Contrary counter directions are retained explicitly in `scorecard.json`.

## Allocation and actual executable controls

Both full censuses reconcile exactly with their independent heap cohorts: **6,791 requests / 98,112,208 requested bytes / 162 reallocations** before and after. The previously removed owned observed-call clone remains separate from the initial-template and active-plan copies; disjoint control families and their measured deltas are archived. An unchanged total is not proof of identical execution cost.
The fresh generated Object/image/metadata remain byte-identical. Generated text **11,800,572 bytes**, entry stack **107,344 bytes**. Whole host text changes **-47,056 bytes**, worker file **-57,152 bytes**. Actual ARM64 function excerpts, own static instruction counts and calls are retained; they are not dynamic instruction counts.

| Own linked function | Before static instructions | Candidate |
|---|---:|---:|
| hot-budget-compute | 158, 291 | 148, 304 |
| checked-total | no standalone symbol | no standalone symbol |
| local-next | no standalone symbol | no standalone symbol |
| commit-debit | no standalone symbol | 89 |

Separate instruction windows put **95.998341%** of baseline instructions inside linked entry plus callbacks. There are two entry calls per compression, 32 diagnostic observations/variant, retained empty-window calibration and no loop-iteration counter reads. Disjoint means reconcile; medians need not sum.

Fresh fixed nonallocating counters in a staged GPU copy use the current candidate Engine/Core and identical linked compressor image. Four complete calls must have identical counts and exact output/inflate/work/capacity/cleanup. Instrumented timing, instruction and allocation observations are excluded from acceptance. The retained R70 counts are historical comparison evidence, not relabeled as fresh executions.

| Counted mechanism per complete call | Executions |
|---|---:|
| shared_compute_debit_attempts | 39,030 |
| admit_callback_attempts | 12,450 |
| helper_callback_attempts | 8,836 |
| helper_Library | 2,475 |
| helper_Move | 1,341 |
| helper_Collection | 505 |
| helper_Output | 1 |
| helper_ClosedTailBegin | 270 |
| helper_ClosedTailEnd | 270 |
| helper_StructuralObservation | 2,806 |
| helper_NumericWritePrepare | 168 |
| helper_NumericSegmentBegin | 500 |
| helper_NumericSegmentEnd | 500 |
| frame_Pack_selected | 847 |
| frame_Copy_selected | 1,139 |
| frame_Project_selected | 3,547 |
| frame_Project_composite_child | 10 |
| frame_Project_selected_scalar_word | 392 |
| frame_Project_selected_managed_child | 339 |

Frame-action counts are subsets of helper counts. Compute-debit attempts count the sole GPU call site of `charge_compute_with_limit`, not every Engine debit or TLS lookup. No per-function instruction saving is inferred by multiplying these counts by CPU samples.

| CPU-time ancestry (overlapping) | Before | Candidate |
|---|---:|---:|
| frame actions | 324/1312 (24.695%) | 379/1313 (28.865%) |
| projection accounting | 34/1312 (2.591%) | 38/1313 (2.894%) |
| thread-local lookup | 51/1312 (3.887%) | 48/1313 (3.656%) |
| memory move/copy | 28/1312 (2.134%) | 22/1313 (1.676%) |
| tail cleanup | 71/1312 (5.412%) | 60/1313 (4.570%) |

CPU-time ancestry is not a retired-instruction partition. Context-address reuse and helper routing remain separate possible experiments; this report does not claim they are implemented or forecast their gains.

## Qualification, failures and skipped coverage

**92 required local receipt checks pass.** Batches overlap; counts are not a sum of unique tests.

- Parent and candidate each execute **108 Budget tests**, including the same **4 new** arithmetic-equivalence, caller/foreign/reservation precedence, overflow, nested observation/live-limit and shared-worker denial/recovery tests. The parent runtime body is the accepted implementation; only test declaration/source are added for comparison.
- Parent and candidate each execute existing planner **22**, turn **7**, projection **12**, shape **6**, plus focused policy-order and overflow/retry selections.
- Selected runtime **329**, components **236**, connected **8**, linked acceptance **12**, work/memory/value **32**, preparation **2**, selected GPU CI **35**, workflow Python **132**, ten architecture/catalog/format checks and five type/memory source checks pass.
- Original/decoded scalar controls **12** retain the **1.05** ceiling and allocation constraints; frame controls **8** retain request/byte/work allowances. All expected values/limits are unchanged.
- Actual Engine Rust metadata compilation without the native-artifact cfg passes with warnings-as-errors and the exact retained operation cfgs/dependencies. This is an actual compiler consumer, not a complete Cargo feature matrix or linker claim.
- Core source/library are unchanged. Historical Core371 is inherited evidence, not a new test execution.
- Hosted/all-feature coverage, complete unchanged source-checker mutation suites, the inherited excluded complete-knucleotide runtime case, larger inputs and full-scale timing are not claimed. Hosted CI is cancelled as requested; the PR remains draft pending green CI and clean external review.

Initial-build disposition: inspection found extracted worker-only debit arguments would be unused without native-artifact. The first native-only build was deliberately stopped before any performance observation. Its actual signal/failed build logs and original frozen scope remain archived. Parameter names were corrected, a fresh build namespace was used, and the actual non-native warnings-as-errors consumer was added. No unexecuted non-native failure is claimed. Read `validation.json` for every additional failed attempt and disposition.

Restored production commit `9e65fbd3ab870ee6216be5dfd6c71894c6bc2f9a` is byte-identical to the accepted runtime body plus the new test declaration. The restored parent executable freshly reruns Budget108 and planner22; memory-runtime, formatting and whitespace checks pass after restoration. The rejected runtime patch is retained only in evidence.

## Pinned computation and replay

Complete eight-byte `abcdefgh` level-12 compression remains `[75,76,74,78,73,77,75,207,0,0]` with independent inflation. Tree **262,144 U16**, cache **1,500,513 U32**, path **305,257 U64**, depth **300**, nice length **258**, original source/tables/search/near-EOF/allocation behavior and output capacity **13** clear/reuse are unchanged. Input aliases, finite error prefixes, unchanged publication on rejection, cleanup and retry remain qualified. Timer `evaluate_into`, logical work **60,260,219,686**, comparison **47,612**, final managed bytes/nodes **0**. IR/work/ABI/bytecode **132/95/37/14**.

```text
source 331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9
bytecode a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7
allocator 38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874
```

M1 Mac mini, 8 GiB, macOS 15.6.1; Rust nightly 2026-03-03, mimalloc 2.1.7. `reconstruct.py /absolute/new/replay-directory` verifies all archived versions, overlays chronologically and rederives nine numerical outputs from committed raw observations with no compiler, compressor or external scratch dependency. It does not rerun executable qualification or performance.
