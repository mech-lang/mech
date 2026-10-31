# R72: keep the shared checked-compute commit inline

**Rejected.** Complete eight-byte level-12 compression: **12.584498× → 12.674825× Roc**, **3.599166 → 3.625000 ms**, matched elapsed change **+0.717763%**. Retired-instruction change **-0.085416%**. Allocation requests and requested bytes are unchanged.

Reject:all4 matched pairs and57/64 blocks are slower. Elapsed+0.7177634%,cycles+1.2865%,while retired instructions fall only34509.5/0.0854%. Allocation requests/bytes remain unchanged. Keep the full adverse evidence; restore the accepted runtime and preserve the R71 tests.

## Design and revisions

`charge_compute_with_limit` must check its caller ceiling before it rejects a foreign current account. It already calculates a checked next compute total for that ceiling. The accepted inner debit calculated that same total again. The compute-only boundary now forwards its checked candidate into a shared local-next validator and the same shared-worker commit. Generic compute/comparison callers retain their original checked arithmetic and use those same validation and commit functions. The candidate is immediate current arithmetic, never stored permission.

The only operations between the caller precheck and account borrow are a const-initialized TLS lookup and an account identity test; no kernel/user callback can mutate the same-thread account. Both reservation additions still evaluate before exceeded-limit selection. Caller overflow/ceiling precedes foreign-account rejection; local/reservation/relative checks precede shared-worker debit; the shared worker commits before infallible local assignment. Direct debits do not advance observation high-water state. All current caps, account identities, observers, reentrant/nested scope restoration and failure atomicity remain checked. Context lookup and helper dispatch are unchanged, so their costs are not claimed as savings.

- Accepted checkpoint/base: `298accda93644920618465d17db34f1b81f9d336`.
- Actual baseline compiler: `3979d6e360ed6d5def3aea745df94d970c92dc4d`.
- Retained R71 regression source, accepted runtime body: `7edff4a938be59cf7ef2b5a1809983d208cff462`.
- The R71 unannotated factoring is rejected and restored in `9e65fbd3ab870ee6216be5dfd6c71894c6bc2f9a`.
- R72 production patch/build: `3dd0f3b739caac5b5166649a2c1dc96b9dbe8c62`.
- Actual candidate release: `3dd0f3b739caac5b5166649a2c1dc96b9dbe8c62`.

Only five Engine-dependent project crates are rebuilt: Engine, compute, standard library, runtime and GPU. Core, syntax, bytecode, math, range, logic, compare and external inputs remain frozen. Exact commands, selected features, optimization profiles, warnings-as-errors and all library hashes are retained. Production changes are confined to `resident/budget.rs`; the additional regression file and module declaration are test-only. No descriptor representation, algorithm, compressor source, allocation site, timing boundary or resource allowance changes.

## Matched measurements

Fresh reference median **0.286000 ms**. Four independently prepared executable pairs, 16 alternating blocks/pair, 31 calls/block: **3,968 timed complete calls**. Improving pairs **0/4**, blocks **7/64**; median block ratio `1.007626313`, geometric ratio `1.007614106`. All contrary pairs and blocks remain in the raw journals.

| Metric | Roc | Before | Candidate | Change |
|---|---:|---:|---:|---:|
| Primary elapsed | 0.286000 ms | 3.599166 ms (12.584498×) | 3.625000 ms (12.674825×) | +0.717763% |
| Retired instructions | 1,364,372 | 40,401,542.5 (29.6118×) | 40,367,033 (29.5865×) | -0.085416% |
| CPU cycles | 943,322.5 | 11,333,724 (12.0147×) | 11,479,534 (12.1693×) | +1.286514% |
| Allocation requests incl. reallocations | 95 | 6,791 (71.4842×) | 6,791 (71.4842×) | +0.000000% |
| Requested bytes | 9,021,923 | 98,112,208 (10.8749×) | 98,112,208 (10.8749×) | +0.000000% |
| Reallocations, already included | 0 | 162 | 162 | +0.000000% |
| Peak live allocator bytes | 9,257,360 | 163,464,584 (17.6578×) | 163,464,584 (17.6578×) | +0.000000% |
| Incremental peak live allocator bytes | 9,257,328 | 20,482,072 (2.2125×) | 20,482,072 (2.2125×) | +0.000000% |
| Post-call RSS | 14,761,984 | 319,012,864 (21.6104×) | 318,881,792 (21.6016×) | -0.041087% |
| Post-call physical footprint | 14,402,432 | 228,771,584 (15.8842×) | 228,673,280 (15.8774×) | -0.042970% |

Timing, instructions/cycles, heap counters, complete symbolized allocation censuses, CPU-time samples, entry windows and mechanism counts are separate cohorts. Do not multiply their percentages. Post-call OS memory is not peak RSS or a standalone minimum. Requested bytes are not initialized/copied bytes. This arithmetic change claims no buffer-copy, input-streaming or memory improvement.
Separate counter-cohort elapsed change **+1.246647%** is excluded from primary timing. Contrary counter directions are retained explicitly in `scorecard.json`.

## Allocation and actual executable controls

Both full censuses reconcile exactly with their independent heap cohorts: **6,791 requests / 98,112,208 requested bytes / 162 reallocations** before and after. The previously removed owned observed-call clone remains separate from the initial-template and active-plan copies; disjoint control families and their measured deltas are archived. An unchanged total is not proof of identical execution cost.
The fresh generated Object/image/metadata remain byte-identical. Generated text **11,800,572 bytes**, entry stack **107,344 bytes**. Whole host text changes **-45,776 bytes**, worker file **-56,768 bytes**. Actual ARM64 function excerpts, own static instruction counts and calls are retained; they are not dynamic instruction counts.

| Own linked function | Before static instructions | Candidate |
|---|---:|---:|
| hot-budget-compute | 158, 291 | 149, 304 |
| checked-total | no standalone symbol | no standalone symbol |
| local-next | no standalone symbol | no standalone symbol |
| commit-debit | no standalone symbol | no standalone symbol |

Separate instruction windows put **95.997745%** of baseline instructions inside linked entry plus callbacks. There are two entry calls per compression, 32 diagnostic observations/variant, retained empty-window calibration and no loop-iteration counter reads. Disjoint means reconcile; medians need not sum.

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
| frame actions | 348/1308 (26.606%) | 321/1315 (24.411%) |
| projection accounting | 27/1308 (2.064%) | 38/1315 (2.890%) |
| thread-local lookup | 59/1308 (4.511%) | 49/1315 (3.726%) |
| memory move/copy | 31/1308 (2.370%) | 22/1315 (1.673%) |
| tail cleanup | 54/1308 (4.128%) | 50/1315 (3.802%) |

CPU-time ancestry is not a retired-instruction partition. Context-address reuse and helper routing remain separate possible experiments; this report does not claim they are implemented or forecast their gains.

## Qualification, failures and skipped coverage

**60 required local receipt checks pass.** Batches overlap; counts are not a sum of unique tests.

- Parent and candidate each execute **108 Budget tests**, including the same **4 retained** arithmetic-equivalence, caller/foreign/reservation precedence, overflow, nested observation/live-limit and shared-worker denial/recovery tests. The parent runtime body is the accepted implementation; the R71 test declaration/source are retained unchanged on both sides.
- Parent and candidate each execute existing planner **22**, turn **7**, projection **12**, shape **6**, plus focused policy-order and overflow/retry selections.
- Actual selected GPU runtime **329**,work/memory/value**32**,preparation**2**,three focusedGPU cases and the real path-mechanism control execute successfully.
- The component selection lists**236**,but its execution is deliberately stopped after rejection. ItsSIGTERM/partial output is preserved; it is not reported as passed. Connected/linked/scalar/frame/CI/architecture packages are not continued. Their complete R71 receipts remain inherited evidence,not fresh R72 coverage. The1.05 scalar ceiling is unchanged; no R72 scalar pass is claimed.
- Actual Engine Rust metadata consumption without native-artifact passes with warnings-as-errors and the retained operation cfgs/dependencies. Core is unchanged; historicalCore371 is not a fresh run.
- Hosted/all-feature coverage,unchanged complete checker mutation suites,the inherited complete-knucleotide exclusion and larger input/full-scale timing are not claimed. The PR remains draft.

R71 identified an outlined commit with144B frame and24 own static instructions on its no-worker path. R72 forces only that shared commit inline; the already inlined validator is unannotated. Arithmetic/policy semantics are unchanged. The measured small instruction decrease does not offset the consistent paired elapsed/cycle regression,so the candidate is rejected.

Restoration commit `50093425798a31df49daf2444dae4d4fee9ce6e7` retains the new Budget tests and restores the exact accepted runtime body. The restored parent executable freshly reruns Budget108/planner22,and memory-runtime/formatting/whitespace checks pass. Read `validation.json` for complete stop/failure disposition.

## Pinned computation and replay

The complete eight-byte `abcdefgh` level-12 measured compression remains `[75,76,74,78,73,77,75,207,0,0]` with independent inflation. Tree **262,144 U16**, cache **1,500,513 U32**, path **305,257 U64**, depth **300**, nice length **258**, original source/tables/search/near-EOF/allocation behavior and output capacity **13** clear/reuse are unchanged. Input aliases, finite budget/error order and retry are covered by the listed Budget/runtime controls and inherited complete R71 qualification; broader omitted R72 coverage is explicit above. Timer `evaluate_into`, logical work **60,260,219,686**, comparison **47,612**, final managed bytes/nodes **0**. IR/work/ABI/bytecode **132/95/37/14**.

```text
source 331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9
bytecode a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7
allocator 38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874
```

M1 Mac mini, 8 GiB, macOS 15.6.1; Rust nightly 2026-03-03, mimalloc 2.1.7. `reconstruct.py /absolute/new/replay-directory` verifies all archived versions, overlays chronologically and rederives nine numerical outputs from committed raw observations with no compiler, compressor or external scratch dependency. It does not rerun executable qualification or performance.
