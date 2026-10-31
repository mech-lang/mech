# R70: borrow the owned call in observed-demand refinement

**Retained.** The complete eight-byte level-12 call improves from **12.573895× to 12.528890× Roc** (3.608708 → 3.595791 ms), a **0.357926%** paired elapsed reduction. Retired instructions fall **129,960.5**, and allocation requests fall exactly **149**, from **6,940 to 6,791**.

`apply_observed_turn_demand` already owns its `TurnMemoryPlan`. It now borrows `plan.call.as_mut()` instead of cloning the call before refinement. The maintained current-call derivation, published/candidate footprint handling, allocation growth/placement and finite-budget evaluation remain the same. The unselected branch retains the original call. Failure consumes the local plan as before; no external caller-visible original needs a copy.

No descriptor representation change, cached invocation authority, alternate planner or compressor-source edit is introduced. R68 and R69 descriptor-sharing candidates remain rejected. Their lower candidate allocation counts are not production counts.

## Revisions and source identity

- Accepted checkpoint/base: `a4d6d54e098e17b7be4a08273d3733f9f0dfa682`.
- Actual baseline compiler build: `c9199ed6687c0125f64e9bf2abd0b0057d5cbf0d`.
- Production ownership change: `d4c69dd9a84f372b301b5286ffda001d44e2d42d`.
- Actual candidate release build: `3979d6e360ed6d5def3aea745df94d970c92dc4d`.
- Corrected test-source checkpoint: `86bf89365877a99ed9c0aec0b678fcccb319d017`.
- Release source scope and original full patch remain archived. A later test-only fixture correction is separately pinned by `test-correction.json/.patch` and `validation-source-scope.json`; the production-patch hash is unchanged. The test file is behind `cfg(test)` and is not compiled into the frozen release. Both unit roots execute the same corrected test source.

Only the five Engine-dependent project crates are rebuilt: Engine, compute, standard library, runtime and GPU. Core, syntax, bytecode, math, range, logic, compare and external inputs stay pinned. Exact selected features, optimized configurations, warnings-as-errors, source/library hashes and commands are retained. No full-math-library rebuild or feature expansion is used.

## Measurements

Fresh reference median **0.287000 ms**. Four independently prepared executable pairs, 16 alternating blocks/pair, 31 compressions/block: **3,968 timed complete calls**. All **4/4** pairs and **47/64** blocks improve. Median block ratio `0.995480123`, geometric block ratio `0.995018122`. Contrary blocks remain in the raw observations.

| Metric | Roc | Before | Candidate | Change |
|---|---:|---:|---:|---:|
| Primary elapsed | 0.287000 ms | 3.608708 ms (12.573895×) | 3.595791 ms (12.528890×) | -0.35793% |
| Retired instructions | 1,364,372 | 40,531,488.5 (29.7071×) | 40,401,528 (29.6118×) | -0.32064% |
| CPU cycles | 925,745.5 | 11,489,712 (12.4113×) | 11,436,150 (12.3534×) | -0.46617% |
| Allocation requests incl. reallocations | 95 | 6,940 (73.0526×) | 6,791 (71.4842×) | -2.14697% |
| Requested bytes | 9,021,923 | 98,127,494 (10.8766×) | 98,112,208 (10.8749×) | -0.01558% |
| Reallocations, already included | 0 | 162 | 162 | +0.00000% |
| Peak live allocator bytes | 9,257,360 | 163,464,584 (17.6578×) | 163,464,584 (17.6578×) | +0.00000% |
| Incremental peak live allocator bytes | 9,257,328 | 20,482,072 (2.2125×) | 20,482,072 (2.2125×) | +0.00000% |
| Post-call RSS | 14,761,984 | 318,996,480 (21.6093×) | 319,176,704 (21.6215×) | +0.05650% |
| Post-call physical footprint | 14,402,432 | 228,787,968 (15.8854×) | 228,935,424 (15.8956×) | +0.06445% |

Timing, CPU counters, heap counters, stack censuses, CPU samples and diagnostic event counts are separate cohorts; do not multiply or combine their percentages. Post-call OS memory is not peak RSS or a standalone minimum. Requested bytes are not initialized/copied bytes. No buffer-copy, input-streaming or peak-live-memory improvement is inferred from the metadata reduction.

Separate CPU-diagnostic elapsed changes **-0.51845%**, excluded from primary timing. All contrary counter directions remain explicit in `scorecard.json`.

## Exact allocation mechanism

| Disjoint observed stack family | Before requests | Candidate | Delta | Requested-byte delta |
|---|---:|---:|---:|---:|
| observed_owned_call_clone | 149 | 0 | -149 | -15,286 |
| initial_call_clone | 151 | 151 | +0 | +0 |
| active_turn_plan_clone | 165 | 165 | +0 | +0 |
| other_observed_refinement | 233 | 233 | +0 | +0 |
| other_initial_turn | 263 | 263 | +0 | +0 |
| remaining | 5979 | 5979 | +0 | +0 |

Both complete symbolized traces reconcile exactly with independent heap observations. The preselected owned-call clone family must disappear, and the net **149-request / 15,286-byte** reduction must match it. Initial call-template and active turn-plan copies have different lifetimes and remain separate. Stack ancestry labels are not semantic object counts or retired-instruction counts. Large tree/cache/path and output allocation behavior is unchanged.

## Instructions, actual code and event counts

Fresh whole-entry diagnostic windows place **96.009705%** of baseline instructions inside the compiled entry plus its callbacks. These are separate 32-call cohorts, two entries/call, with retained empty-window calibration and no per-loop counter reads. Disjoint means reconcile exactly; medians need not sum.
The generated Object/image/metadata stay byte-identical: text **11,800,572 bytes**, machine-entry stack **107,344 bytes**. Actual host text changes **-2,516 bytes** and linked worker size **-768 bytes**. Own static function counts/assembly are retained; static code size is not executed instruction count.

The isolated baseline mechanism census adds fixed nonallocating atomic counters in a staged GPU source copy, using the exact accepted Engine/Core and unchanged linked compressor image. Four complete calls have identical counts and exact output/inflate/work/capacity/cleanup. Diagnostic source, patch, release flags, library pins and executable hash are archived. Its elapsed/instructions/allocator observations are excluded from acceptance.

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

Selected frame counts are subsets of helper counts and must not be added to them. Compute-debit attempts count the sole production call site of `charge_compute_with_limit`; they are not a count of every Engine debit or TLS lookup. No exact per-function instruction saving is inferred. These counts guide the next bounded instruction experiment.

| CPU-sample ancestry (overlapping) | Before | Candidate |
|---|---:|---:|
| frame actions | 326/1242 (26.248%) | 334/1318 (25.341%) |
| projection accounting | 40/1242 (3.221%) | 64/1318 (4.856%) |
| thread-local lookup | 46/1242 (3.704%) | 58/1318 (4.401%) |
| memory move/copy | 18/1242 (1.449%) | 32/1318 (2.428%) |
| tail cleanup | 62/1242 (4.992%) | 59/1318 (4.476%) |

CPU-time sample ancestry is not an instruction partition. The main next target is repeated budget/debit/context/helper work inside execution, beginning with the already-checked next-compute value through the same maintained debit implementation. Scope/address reuse is a separate experiment. Any inlining change is inspected in the actual linked host, rather than blamed on a descriptor representation without a dynamic witness.

## Qualification and failure disposition

**78 required local receipt checks pass.** The batches overlap; their counts are not a total of unique tests.

- Exact parent and candidate Engine planner suites: **22 each**, including three new output/published-footprint/finite-limit, unselected/no-call branch, overflow-before-resolution and recovery cases. The focused turn selection executes **7 each**. Projection **12**, shape **6**, plus focused policy/overflow tests execute on both.
- Current selected runtime **329**, components **236**, connected **8**, linked acceptance **12**, work/memory/value **32**, preparation **2**, selected GPU CI **35**, workflow Python **132**, ten architecture/catalog/format checks and five type/memory source checks pass.
- Original/decoded scalar controls **12** retain the **1.05** ceiling and allocation constraints; frame controls **8** retain request/byte/work allowances. No expected values or limits increase.
- Core implementation/library are unchanged. R69’s fresh **371-test Core** parent execution is inherited evidence, not relabeled as a new R70 execution.
- Full hosted/all-feature coverage, complete unchanged source-checker mutation suites, the inherited excluded complete-knucleotide runtime case and larger compression inputs/full-scale timing are not claimed. Hosted CI is cancelled by user direction; the PR remains draft pending green CI and clean review.

All failed setup attempts are preserved and resolved: (1) the first new negative fixture omitted a redundant fact from an already-resolved call; use of a genuinely deferred template preserves the intended missing-witness/error-precedence check; (2) the corrected parent build succeeded but its archive label collided with the immutable first attempt, so the successful binary was archived under a unique label and the previously unrun tests executed; (3) the unit source guard detected the declared test-only correction after release freeze, so a separate strict validation-source scope was captured while proving the production hash unchanged. (4) the packet publisher tried to recreate the existing diagnostic-receipt directory; its unchanged summary is copied into that directory before numerical replay. These are introduced test/receipt harness issues, not inherited compiler failures. No runtime failure or resource denial remains unresolved in the selected coverage.

## Pinned computation and reproduction

The complete eight-byte `abcdefgh` level-12 computation still produces `[75,76,74,78,73,77,75,207,0,0]` and independently inflates. Tree **262,144 U16**, cache **1,500,513 U32**, path capacity **305,257 U64**, depth **300**, nice length **258**, original tables/search/near-EOF rules/source allocations and output capacity **13** with clear/reuse remain unchanged. Input aliases and failure publication/recovery are preserved. The timer remains `evaluate_into`; logical work **60,260,219,686**, comparison work **47,612**, final managed bytes/nodes **0**. IR/work/ABI/bytecode **132/95/37/14**.

```text
source 331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9
bytecode a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7
allocator 38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874
```

M1 Mac mini, 8 GiB, macOS 15.6.1; Rust nightly 2026-03-03, pinned mimalloc 2.1.7. `reconstruct.py /absolute/new/replay-directory` verifies every archived version, overlays chronologically and rederives nine numerical outputs from committed raw observations without a compiler, compressor or external scratch dependency. It does not rerun performance/executable qualification.
