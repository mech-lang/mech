# R63: current frame-shape guards

Candidate build: `4bf38cf4f75e30d5d14ca13dd5a7149a232da8fe`. Baseline R62 build: `ded89a9c0014fe66734be22ee6a3170e7ec10448`. Accepted baseline checkpoint: `a9a97af9861e9fa26532c8dc9d6b7ddd975a54f1`.

The complete eight-byte level-12 call measured **14.387878× the reference**, from **14.517425×** in this matched baseline cohort. Candidate/baseline elapsed: **0.99107643×** (-0.8924%). The freshly measured reference median was 0.275000 ms.

The two frame-admission paths reuse their prepared current-shape predicate after current typed fields have been checked. The predicate additionally requires a complete-axis proof: nested parameter relations or output-size checks that its direct axes cannot establish retain ordinary constructor validation. This separates footprint-recipe eligibility from shape-proof eligibility while preserving the fallback and its error order. The nested-output-overflow counterexample and actual caller/constructor recovery are retained as regressions.

Engine, Compute, Stdlib, Runtime and GPU were rebuilt coherently with their retained flags/features and warnings-as-errors. Core, math, syntax and external artifacts remain frozen. The candidate emits its Object, descriptor and metadata afresh; all three must match the R62 baseline byte-for-byte. No compressor source, algorithm, source allocation site or timing boundary changes.

| Metric | Reference | R62 baseline | R63 candidate | Change |
|---|---:|---:|---:|---:|
| Primary elapsed (ms) | 0.275000 | 3.992292 | 3.956666 | -0.8924% |
| Retired instructions | 1,364,372 | 42,117,044.5 | 42,050,588 | -0.1578% |
| CPU cycles | 895,905 | 12,659,449 | 12,075,457 | -4.6131% |
| Allocation requests | 95 | 8,173 | 8,105 | -0.8320% |
| Requested bytes | 9,021,923 | 98,228,396 | 98,227,372 | -0.0010% |
| Reallocations | 0 | 162 | 162 | +0.0000% |
| Peak allocator live usable bytes | 9,257,360 | 163,464,584 | 163,464,584 | +0.0000% |
| Incremental allocator peak | 9,257,328 | 20,482,072 | 20,482,072 | +0.0000% |
| RSS after call | 14,778,368 | 319,094,784 | 318,947,328 | -0.0462% |
| Physical footprint after call | 14,418,816 | 228,837,056 | 228,706,048 | -0.0572% |

Primary elapsed, CPU counters, heap counters and CPU-time samples are separate serial cohorts. Their percentages are not multiplied or combined. Requested bytes represent allocation requests, not initialized bytes copied. Allocator peak and post-call OS footprint are different measures; the latter is not peak RSS and includes retained compiler/catalog/image storage.

## Allocation mechanism

| Family | Baseline requests / bytes | Candidate requests / bytes | Request / byte delta |
|---|---:|---:|---:|
| flat_frame_shape_validation | 32 / 496 | 0 / 0 | -32 / -496 |
| composed_frame_admission | 36 / 528 | 0 / 0 | -36 / -528 |
| combined_candidate | 68 / 1,024 | 0 / 0 | -68 / -1,024 |
| separate_resolved_extent_solver | 806 / 11,730 | 806 / 11,730 | +0 / +0 |
| outside_candidate | 8,105 / 98,227,372 | 8,105 / 98,227,372 | +0 / +0 |

The pre-change ceiling was 68 requests / 1,024 requested bytes, predicting 8,105 requests / 98,227,372 bytes only if all attributed requests disappeared. The table reports actual complete-census results. Combined-candidate is the union of the first two rows; do not add it to them. The extent solver and outside-candidate totals are separate, disjoint families.

Flat stacks explicitly name FrameShapeRecipe::validate. Composed stacks retain complete composition-admission ancestry but their immediate allocation callee is deduplicated; attribution to shape validation uses source and allocation-pattern evidence. The full family comparison establishes request/byte effects, not all branch visits or per-branch eligibility counts.

Before-family attribution uses the retained R62 complete census, pinned by SHA-256 `ae2452719ce2d535ab0528d207d1cf9109da2d5b9b96f0ae3bbe449f5e3a1cee`. It is historical trace evidence; its totals reconcile with the fresh R63 before heap cohort. The after census is freshly collected for R63 and reconciles with fresh after heap counters. Both timing, hardware-counter and sample variants are also freshly measured. Raw symbols, per-size counts and generated-entry return groups remain in `allocation-families.json`. Matching heap totals do not manufacture a fresh baseline stack trace, and no per-branch or nonallocating-visit count is claimed.

## Matched pairs and adverse observations

| Pair | Baseline ms | Candidate ms | Candidate/baseline |
|---|---:|---:|---:|
| 0 | 4.027625 | 3.977979 | 0.98767363× |
| 1 | 4.020063 | 3.990771 | 0.99271355× |
| 2 | 3.831167 | 3.798583 | 0.99149528× |
| 3 | 4.030291 | 3.998104 | 0.99201360× |

4/4 independently prepared pairs and 49/64 alternating blocks improved. Four pairs ×16 blocks ×31 calls ×2 variants produce 3,968 timed observations. Blocks within one pair are descriptive, not independent repetitions.
Adverse or unchanged primary pairs: none.

The separate CPU-counter cohort elapsed was 3.955105 → 3.767605 ms (-4.740709%). These 32 observations per variant are excluded from primary elapsed and retain their independent direction.

Other observed median increases:

```json
{}
```

## Binary and baseline provenance

| Metric | R62 baseline | R63 candidate |
|---|---:|---:|
| Generated text bytes | 11,800,572 | 11,800,572 |
| Machine entry stack bytes | 107,344 | 107,344 |
| Typed IR version | 132 | 132 |
| ABI version | 37 | 37 |

Object/image/metadata equality: True. The unchanged generated bytes have unchanged static instruction content. Hardware retired instructions can still change because this patch changes host admission work. `binary-audit.json` records each actual image pin.

The accepted checkpoint includes rustfmt-only producer normalization and the callback-pointer snapshot regression after the original R62 measurements; R62 verified normalized production bytes before the later regression. Historical build source remains ded89a9, not relabeled as checkpoint a9a97af.

R62 corrected earlier pointer-role attribution: immediate updates to x16 were previously missed, assigning some stack loads to stale offsets. Corrected role counts are retained in baseline/binary-audit.json. Whole opcode/stack totals and hardware timing/counter observations were unchanged by that analysis correction. R63 does not recompute a new dynamic result from these static counts.

These historical notes preserve provenance; no R62 measurement or test execution is counted as an R63 observation.

## Workload and correctness

Input: eight bytes `abcdefgh`. Exact raw output: `[75,76,74,78,73,77,75,207,0,0]`, independently inflated. Output capacity 13 is cleared/reused. The tree has 262,144 U16 entries; the cache has 1,500,513 U32 words; depth is 300 and nice length 258. Original source allocation sites, input aliases and allocator are retained. Logical work is 60,260,219,686 and comparison work is 47,612; neither is a hardware instruction count. Successful invocations finish with zero managed bytes/nodes.

```json
{
  "source_sha256": "331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9",
  "bytecode_sha256": "a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7",
  "allocator_sha256": "38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874",
  "logical_work": 60260219686,
  "comparison_work": 47612,
  "input_bytes": 8,
  "expected_output": [
    75,
    76,
    74,
    78,
    73,
    77,
    75,
    207,
    0,
    0
  ],
  "output_capacity": 13
}
```

## Qualification

Status: **passed**. Counts below are actual executed receipts; inventory-only rows are omitted from this view and remain in `validation.json`. Batches overlap.

| Receipt | Expected tests | Observed | Exit / expected | Passed |
|---|---:|---:|---:|---|
| `engine-parent-constructor/build.json` | None | None | 0 / 0 | True |
| `engine-parent-constructor/caller.json` | 1 | 1 | 0 / 0 | True |
| `engine-parent-constructor/opaque.json` | 1 | 1 | 0 / 0 | True |
| `engine-parent-constructor/coverage.json` | 1 | 1 | 101 / 101 | True |
| `engine-after-overlay/build.json` | None | None | 0 / 0 | True |
| `engine-after-overlay/caller.json` | 1 | 1 | 0 / 0 | True |
| `engine-after-overlay/opaque.json` | 1 | 1 | 0 / 0 | True |
| `engine-after-overlay/coverage.json` | 1 | 1 | 0 / 0 | True |
| `engine-after-overlay/all.json` | 6 | 6 | 0 / 0 | True |
| `units-after/build.json` | None | None | 0 / 0 | True |
| `units-after/vector-address.json` | 1 | 1 | 0 / 0 | True |
| `units-after/first-change.json` | 1 | 1 | 0 / 0 | True |
| `units-after/locals-snapshot.json` | 1 | 1 | 0 / 0 | True |
| `units-after/preparation.json` | 2 | 2 | 0 / 0 | True |
| `units-after/runtime.json` | 329 | 329 | 0 / 0 | True |
| `units-after/work-values.json` | 32 | 32 | 0 / 0 | True |
| `qualification/component-build.json` | None | None | 0 / 0 | True |
| `qualification/path-mechanism.json` | 1 | 1 | 0 / 0 | True |
| `qualification/components.json` | 236 | 236 | 0 / 0 | True |
| `qualification/composition.json` | 8 | 8 | 0 / 0 | True |
| `ci/workflow-python.json` | 132 | 132 | 0 / 0 | True |
| `ci/engine-selected.json` | 6 | 6 | 0 / 0 | True |
| `linked-acceptance/emit-build.json` | None | None | 0 / 0 | True |
| `linked-acceptance/emit.json` | None | None | 0 / 0 | True |
| `linked-acceptance/worker-build.json` | None | None | 0 / 0 | True |
| `linked-acceptance/run.json` | None | None | 0 / 0 | True |
| `ci/results.json` | 35 | 35 | 0 / 0 | True |
| `architecture/results.json` | — | — | 0 / 0 | True |
| `architecture/results.json` | — | — | 0 / 0 | True |
| `architecture/results.json` | — | — | 0 / 0 | True |
| `architecture/results.json` | — | — | 0 / 0 | True |
| `architecture/results.json` | — | — | 0 / 0 | True |
| `architecture/results.json` | — | — | 0 / 0 | True |
| `architecture/results.json` | — | — | 0 / 0 | True |
| `architecture/results.json` | — | — | 0 / 0 | True |
| `architecture/results.json` | — | — | 0 / 0 | True |
| `architecture/results.json` | — | — | 0 / 0 | True |
| `primary/scalar-results.json` | — | 12 | — / 0 | True |
| `frame-allocation/results.json` | — | 8 | — / 0 | True |
| `linked-acceptance/run.stdout` | — | 12 | — / 0 | True |

The parent complete-axis negative control fails first at `assertion failed: !recipe.is_identity_forwarding()`, before the later current-shape assertion. The parent caller/ordinary-constructor and opaque-shape tests must actually pass; this expected assertion failure does not substitute for those executions. A malformed-fixture setup failure does not count as this expected rejection.

Fresh full R63 runtime execution is required. Historical R62 batches are not counted. The six Engine shape tests and GPU CI selections are additional overlapping qualification batches, not a sum of unique tests.

Six Engine `frame_shape::tests::` tests are explicitly scheduled in CI with `full_compiler,native-artifact,sequence_buffers`. The local `ci.py` Engine batch actually executes them using the retained broader Engine test feature configuration, which includes those required flags among its other enabled cfg features. This is not a claim that the local build used exactly the minimal CI feature set or that remote CI completed; the complete actual command is archived.

Earlier failed attempts and dispositions:
- `engine-parent/build.json` — environmental harness setup: Retained frozen transitive external dependencies were missing from the isolated test search path. Corrected dependency overlay; no production failure. Corrected parent-constructor build succeeds with warnings-as-errors.
- `engine-parent-overlay/caller.json` — introduced test fixture construction: Fixture appended noncanonical schema IDs; strict bytecode decoding rejected expected2/found5 before the assertion path. Rebuilt fixture in a canonical table and reran parent/candidate including ordinary constructor output, original/decoded behavior, funding order, aliases, cleanup and retry. Rejection behavior retained.
- `analysis-initial/receipt.json` — introduced report metadata assembly: Decision record omitted required round/build-source fields. Added exact implementation pins; final analysis and replay are required. No benchmark or test rerun warranted.
- `packaging-initial/receipt.json` — introduced receipt packaging: Root-created host metadata symlink was archived as a symlink rather than regular content. Preserve original archive and failure, replace that member with the exact manifest-hash-verified bytes, retain regular metadata, and dereference future archived inputs. No compiler, test or performance change.

Skipped coverage:
- complete_unchanged_knucleotide_body_uses_the_owned_emitted_map: Existing runtime selection exclusion retained; unrelated complete knucleotide body is not requalified here.
- full remote CI: Not awaited as requested. Local exact profile selections and architecture checks are reported separately.
- larger compression inputs/full-scale timing: Not run; this optimization measures the complete pinned eight-byte level-12 call only.

Decision: **retained**. Exact full census removes the predicted 68 shape-validation requests and 1,024 requested bytes. All four matched primary pairs improve; pooled elapsed is 0.892357% lower. Separate retired instructions decrease0.157790%; cycles and diagnostic elapsed are kept separate. Generated Object, descriptor and metadata are byte-identical. Existing charges, fallback, nested-output error precedence and recovery are preserved. Complete qualification remains a closure gate rather than assumed by this performance decision.

## Replay

Run `python3 reconstruct.py /new/empty/output/path` from the published packet. Replay verifies every archived file version and reproduces scorecard JSON/CSV, allocation-family JSON, profile JSON and validation JSON without compilation, compressor execution or external scratch inputs.

This complete small-input result does not establish larger-input throughput or a standalone minimum-memory footprint. Each reported direction and decision comes from R63 receipts; no earlier-round gains are multiplied into this result.
