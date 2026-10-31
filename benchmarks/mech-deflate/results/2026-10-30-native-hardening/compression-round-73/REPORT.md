# R73: inline the original ordinary debit across the Engine/runtime boundary

**Rejected.** Complete eight-byte level-12 compression: **12.370933× → 12.518550× Roc**, **3.612313 → 3.655416 ms**, matched elapsed change **+1.193252%**. Retired-instruction change **+2.498901%**. Allocation requests and requested bytes are unchanged.

Reject:all4 prepared pairs and60/64 blocks are slower; paired elapsed+1.1932522449%. Retired instructions increase from40401626 to41411222.5. Allocation count/bytes remain unchanged. Actual code re-outlines TLS access with the debit closure,so forcing source-level inlining does not remove the executed fallible-result boundary. Preserve evidence and restore all three attributes.

## Design and revisions

The runtime invokes the ordinary shared compute debit39,030 times per complete call. Its Engine boundary returns a fallible result to the GPU runtime. R73 makes the existing public compute-only boundary and its two maintained private arithmetic/commit helpers available for cross-crate inlining. The patch adds exactly three `#[inline(always)]` attributes. Function bodies,signatures,algorithmic arithmetic and checks remain byte-for-byte unchanged.

Caller overflow/ceiling checks still precede foreign-account rejection. The existing account/relative caps,both reservation additions and their precedence,shared-worker debit,observers and final local commit are unchanged. No checked-total forwarding or extracted commit helper from the rejected R71/R72 candidates is present. No permission,live limit or current account is cached. The shared-worker method is unannotated.

Actual linked code and whole-call instructions determine whether cross-crate result/argument transport is removed. Static own-function size and missing symbols alone are not a dynamic instruction ledger. Allocation,code size,stack behavior and matched elapsed are evaluated separately.

- Accepted checkpoint/base: `ec0f21efbd4ce4c102a33263a6de44a708bae9d3`.
- Actual baseline compiler: `3979d6e360ed6d5def3aea745df94d970c92dc4d`.
- Retained regression source and exact accepted runtime body: `7edff4a938be59cf7ef2b5a1809983d208cff462`.
- Three-attribute production patch and actual candidate release: `636945c270484b15334dbc54af9226afd1163c37`.
- Prior R71/R72 checked-total experiments are rejected and their runtime code restored. Their reports preserve all adverse results.

Only five Engine-dependent project crates are rebuilt: Engine, compute, standard library, runtime and GPU. Core, syntax, bytecode, math, range, logic, compare and external inputs remain frozen. Exact commands, selected features, optimization profiles, warnings-as-errors and all library hashes are retained. Production changes are confined to `resident/budget.rs`; the R71 regression file/module declaration are retained unchanged on both sides. No descriptor representation, algorithm, compressor source, allocation site, timing boundary or resource allowance changes.

## Matched measurements

Fresh reference median **0.292000 ms**. Four independently prepared executable pairs, 16 alternating blocks/pair, 31 calls/block: **3,968 timed complete calls**. Improving pairs **0/4**, blocks **4/64**; median block ratio `1.010454520`, geometric ratio `1.010965668`. All contrary pairs and blocks remain in the raw journals.

| Metric | Roc | Before | Candidate | Change |
|---|---:|---:|---:|---:|
| Primary elapsed | 0.292000 ms | 3.612313 ms (12.370933×) | 3.655416 ms (12.518550×) | +1.193252% |
| Retired instructions | 1,364,372 | 40,401,626 (29.6119×) | 41,411,222.5 (30.3519×) | +2.498901% |
| CPU cycles | 925,485 | 11,345,133.5 (12.2586×) | 11,614,042.5 (12.5491×) | +2.370259% |
| Allocation requests incl. reallocations | 95 | 6,791 (71.4842×) | 6,791 (71.4842×) | +0.000000% |
| Requested bytes | 9,021,923 | 98,112,208 (10.8749×) | 98,112,208 (10.8749×) | +0.000000% |
| Reallocations, already included | 0 | 162 | 162 | +0.000000% |
| Peak live allocator bytes | 9,257,360 | 163,464,584 (17.6578×) | 163,464,584 (17.6578×) | +0.000000% |
| Incremental peak live allocator bytes | 9,257,328 | 20,482,072 (2.2125×) | 20,482,072 (2.2125×) | +0.000000% |
| Post-call RSS | 14,745,600 | 319,242,240 (21.6500×) | 319,127,552 (21.6422×) | -0.035925% |
| Post-call physical footprint | 14,386,048 | 228,984,512 (15.9171×) | 228,837,056 (15.9069×) | -0.064396% |

Timing, instructions/cycles, heap counters, complete symbolized allocation censuses, CPU-time samples, entry windows and mechanism counts are separate cohorts. Do not multiply their percentages. Post-call OS memory is not peak RSS or a standalone minimum. Requested bytes are not initialized/copied bytes. This arithmetic change claims no buffer-copy, input-streaming or memory improvement.
Separate counter-cohort elapsed change **+2.405610%** is excluded from primary timing. Contrary counter directions are retained explicitly in `scorecard.json`.

## Allocation and actual executable controls

Both full censuses reconcile exactly with their independent heap cohorts: **6,791 requests / 98,112,208 requested bytes / 162 reallocations** before and after. The previously removed owned observed-call clone remains separate from the initial-template and active-plan copies; disjoint control families and their measured deltas are archived. An unchanged total is not proof of identical execution cost.
The fresh generated Object/image/metadata remain byte-identical. Generated text **11,800,572 bytes**, entry stack **107,344 bytes**. Whole host text changes **+7,404 bytes**, worker file **+23,104 bytes**. Actual ARM64 function excerpts, own static instruction counts and calls are retained; they are not dynamic instruction counts.

| Own linked function | Before static instructions | Candidate |
|---|---:|---:|
| hot-budget-compute | 158, 291 | 118, 304 |
| checked-total | no standalone symbol | no standalone symbol |
| local-next | no standalone symbol | no standalone symbol |
| commit-debit | no standalone symbol | no standalone symbol |

Separate instruction windows put **95.996818%** of baseline instructions inside linked entry plus callbacks. There are two entry calls per compression, 32 diagnostic observations/variant, retained empty-window calibration and no loop-iteration counter reads. Disjoint means reconcile; medians need not sum.

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
| frame actions | 331/1316 (25.152%) | 351/1321 (26.571%) |
| projection accounting | 45/1316 (3.419%) | 36/1321 (2.725%) |
| thread-local lookup | 40/1316 (3.040%) | 47/1321 (3.558%) |
| memory move/copy | 31/1316 (2.356%) | 27/1321 (2.044%) |
| tail cleanup | 48/1316 (3.647%) | 64/1321 (4.845%) |

CPU-time ancestry is not a retired-instruction partition. Context-address reuse and helper routing remain separate possible experiments; this report does not claim they are implemented or forecast their gains.

## Qualification, failures and skipped coverage

**43 required local receipt checks pass.** Batches overlap; counts are not a sum of unique tests.

- Parent and candidate each execute **108 Budget tests**, including the same **4 retained** arithmetic-equivalence, caller/foreign/reservation precedence, overflow, nested observation/live-limit and shared-worker denial/recovery tests. The parent runtime body is the accepted implementation and both sides retain the same test source.
- Parent and candidate each execute existing planner **22**, turn **7**, projection **12**, shape **6**, plus focused policy-order and overflow/retry selections.
- The actual non-native Engine Rust metadata consumer passes with warnings-as-errors and the retained operation cfgs/dependencies.
- Full runtime/component/connected/linked/scalar/frame/CI/architecture/type qualification is deferred and not started after diagnostic rejection. R71 and selected R72 executions are inherited only. The1.05 scalar ceiling and resource allowances are unchanged; no fresh R73 scalar pass is claimed.
- Hosted/all-feature coverage,unchanged mutation suites,the inherited complete-knucleotide exclusion and larger inputs/full-scale timing are not claimed. The PR remains draft.

Parent executable reuse is explicit:the entire staged Engine tree and every recorded library hash match the retained R72 parent Rust binary before reuse. Parent test processes run freshly; the candidate unit root compiles freshly. Raw input/executable hashes and the historical Rust command are archived in the reuse proof.

Actual ARM64 reveals that the public debit boundary was inlined,but the compiler outlined `std::thread::LocalKey<ControlWorkContext>::with` with the debit closure. This118-own-instruction function still carries the fallible result across a call. Source annotations therefore moved an executed boundary; they did not eliminate it. No complete per-function hardware instruction attribution is inferred. A scoped borrowed context is the next mechanism to investigate separately.

Restoration commit `0d0953a0fdb1a76fb25072a6b95c895bc247dc19` removes all three attributes and retains the R71 regression tests. Its Budget source is byte-identical to accepted7edff4a. The exact parent executable freshly reruns Budget108/planner22,and memory-runtime/formatting/whitespace checks pass. `validation.json` distinguishes actual execution from omitted broad coverage.

## Pinned computation and replay

Complete eight-byte `abcdefgh` level-12 compression remains `[75,76,74,78,73,77,75,207,0,0]` with independent inflation. Tree **262,144 U16**, cache **1,500,513 U32**, path **305,257 U64**, depth **300**, nice length **258**, original source/tables/search/near-EOF/allocation behavior and output capacity **13** clear/reuse are unchanged. Budget/error order and retry are covered by the listed controls; broader alias/publication/component coverage is inherited from R71,not relabeled as fresh R73 execution. Timer `evaluate_into`, logical work **60,260,219,686**, comparison **47,612**, final managed bytes/nodes **0**. IR/work/ABI/bytecode **132/95/37/14**.

```text
source 331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9
bytecode a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7
allocator 38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874
```

M1 Mac mini, 8 GiB, macOS 15.6.1; Rust nightly 2026-03-03, mimalloc 2.1.7. `reconstruct.py /absolute/new/replay-directory` verifies all archived versions, overlays chronologically and rederives nine numerical outputs from committed raw observations with no compiler, compressor or external scratch dependency. It does not rerun executable qualification or performance.
