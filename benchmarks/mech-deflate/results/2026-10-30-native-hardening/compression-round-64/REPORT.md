# R64: first-only central projection budget checks

Candidate build: `a27ea040845b41a63fe29f6837773860bea8607c`. Baseline R63 build: `4bf38cf4f75e30d5d14ca13dd5a7149a232da8fe`. Accepted baseline checkpoint: `50794460eeffabb156e513f2dde01c08a23cb9f4`.

The complete eight-byte level-12 call measured **13.954588× the reference**, from **14.104766×** in this matched baseline cohort. Candidate/baseline elapsed: **0.98935267×** (-1.0647%). The freshly measured reference median was 0.278000 ms.

Two central projection-meter queries use the existing first-violation evaluator instead of constructing the full list of violations. Both APIs traverse the same ordered Core checks. Preview still checks arithmetic, worker-scope restrictions, scope authorization and enclosing-work funding before call policy. Charge retains its current enclosing-work prefix before policy rejection, leaves accumulated meter state unchanged on failure, and commits only after all checks pass. The new tests exercise competing policy and funding failures, compute/node overflow, unchanged state and retry on both parent and candidate.

Engine, Compute, Stdlib, Runtime and GPU were rebuilt coherently with their retained flags/features and warnings-as-errors. Core, math, syntax and external artifacts remain frozen. The candidate emits its Object, descriptor and metadata afresh; all three must match the R63 baseline byte-for-byte. No compressor source, algorithm, source allocation site or timing boundary changes.

| Metric | Reference | R63 baseline | R64 candidate | Change |
|---|---:|---:|---:|---:|
| Primary elapsed (ms) | 0.278000 | 3.921125 | 3.879375 | -1.0647% |
| Retired instructions | 1,364,372 | 42,051,992.5 | 41,630,111 | -1.0032% |
| CPU cycles | 901,081.5 | 12,902,412 | 11,776,216 | -8.7286% |
| Allocation requests | 95 | 8,105 | 8,105 | +0.0000% |
| Requested bytes | 9,021,923 | 98,227,372 | 98,227,372 | +0.0000% |
| Reallocations | 0 | 162 | 162 | +0.0000% |
| Peak allocator live usable bytes | 9,257,360 | 163,464,584 | 163,464,584 | +0.0000% |
| Incremental allocator peak | 9,257,328 | 20,482,072 | 20,482,072 | +0.0000% |
| RSS after call | 14,778,368 | 319,143,936 | 319,193,088 | +0.0154% |
| Physical footprint after call | 14,418,816 | 228,902,656 | 228,919,040 | +0.0072% |

Primary elapsed, CPU counters, heap counters and CPU-time samples are separate serial cohorts. Their percentages are not multiplied or combined. Requested bytes represent allocation requests, not initialized bytes copied. Allocator peak and post-call OS footprint are different measures; the latter is not peak RSS and includes retained compiler/catalog/image storage.

## Allocation and instruction mechanism

The successful full-budget evaluator already returns an allocation-free empty vector. This change predicts zero successful-call allocation savings; it targets the work around the existing call boundary. Failure-only savings cannot explain successful primary elapsed. Hardware instruction changes are measured independently of allocator counts and CPU-time samples.

| Complete census | R63 baseline | R64 candidate | Delta |
|---|---:|---:|---:|
| Allocation requests | 8,105 | 8,105 | +0 |
| Requested bytes | 98,227,372 | 98,227,372 | +0 |
| Reallocations | 162 | 162 | +0 |

Size/kind request histogram unchanged: **True**. Both full censuses are fresh R64 measurements, individually reconciled to their separate eight-observation heap cohorts. The archived trace pins and per-size counts are in `allocation-comparison.json`. Equal totals do not establish identical work or execution counts.

A retained R62 CPU sample attributed 19 of 1,320 exclusive sample weights to the full budget evaluator. That was a lead for this experiment, not a predicted elapsed or instruction saving and not an R64 observation. Fresh whole-call counters and samples are the result evidence.

## Matched pairs and adverse observations

| Pair | Baseline ms | Candidate ms | Candidate/baseline |
|---|---:|---:|---:|
| 0 | 4.040708 | 4.024167 | 0.99590629× |
| 1 | 3.972416 | 3.944751 | 0.99303560× |
| 2 | 3.759709 | 3.720021 | 0.98944400× |
| 3 | 3.833584 | 3.807479 | 0.99319071× |

4/4 independently prepared pairs and 52/64 alternating blocks improved. Four pairs ×16 blocks ×31 calls ×2 variants produce 3,968 timed observations. Blocks within one pair are descriptive, not independent repetitions.
Adverse or unchanged primary pairs: none.

The separate CPU-counter cohort elapsed was 4.025896 → 3.676417 ms (-8.680764%). These 32 observations per variant are excluded from primary elapsed and retain their independent direction.

Other observed median increases:

```json
{
  "rss_after": 0.0154012012937077,
  "physical_after": 0.007157627738485495
}
```

## Binary and baseline provenance

| Metric | R63 baseline | R64 candidate |
|---|---:|---:|
| Generated text bytes | 11,800,572 | 11,800,572 |
| Machine entry stack bytes | 107,344 | 107,344 |
| Typed IR version | 132 | 132 |
| ABI version | 37 | 37 |

Object/image/metadata equality: True. The unchanged generated bytes have unchanged static instruction content. Hardware retired instructions can still change because this patch changes host admission work. `binary-audit.json` records each actual image pin.

Baseline is the completed retained R63 graph and its actual build source, with complete-axis frame guards. Historical build identity is not replaced with a later report/checkpoint SHA.

R63 fresh emission matched the retained R62 Object/image/metadata byte-for-byte. R62 previously corrected stale-offset pointer-role attribution; that correction changed no whole opcode totals or hardware measurements. R64 performs its own fresh emission identity check.

These historical notes preserve provenance; no R63 measurement or test execution is counted as an R64 observation.

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
| `engine-parent/build.json` | None | None | 0 / 0 | True |
| `engine-parent/projection.json` | 12 | 12 | 0 / 0 | True |
| `engine-parent/policy-order.json` | 1 | 1 | 0 / 0 | True |
| `engine-parent/overflow-retry.json` | 1 | 1 | 0 / 0 | True |
| `engine-parent/shape.json` | 6 | 6 | 0 / 0 | True |
| `engine-after/build.json` | None | None | 0 / 0 | True |
| `engine-after/projection.json` | 12 | 12 | 0 / 0 | True |
| `engine-after/policy-order.json` | 1 | 1 | 0 / 0 | True |
| `engine-after/overflow-retry.json` | 1 | 1 | 0 / 0 | True |
| `engine-after/shape.json` | 6 | 6 | 0 / 0 | True |
| `core-focused/build.json` | None | None | 0 / 0 | True |
| `core-focused/selected.json` | 2 | 2 | 0 / 0 | True |
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
| `ci/engine-shape-selected.json` | 6 | 6 | 0 / 0 | True |
| `ci/engine-projection-selected.json` | 12 | 12 | 0 / 0 | True |
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

Both parent and candidate projection tests must pass. R63 expected-failure coverage is not inherited; the six existing shape tests pass on both R64 variants.

Fresh full R64 runtime execution is required. Historical R63 batches are not counted. Projection12, shape6, Core2 and GPU CI selections are additional overlapping batches, not a sum of unique tests.

The local `ci.py` Engine batches execute all 12 projection-meter tests and all six inherited frame-shape tests using the retained broader Engine test feature configuration. Core also executes its two existing first-violation equivalence/order tests. The complete commands are archived; this is not a claim that the local Engine build used exactly a minimal CI feature set or that remote CI completed.

Earlier failed attempts and dispositions:
None recorded.

Skipped coverage:
- complete_unchanged_knucleotide_body_uses_the_owned_emitted_map: Existing runtime selection exclusion retained; unrelated complete knucleotide body is not requalified here.
- full remote CI: Not awaited as requested. Local selected tests with their actual retained feature configurations and architecture checks are reported separately.
- larger compression inputs/full-scale timing: Not run; this optimization measures the complete pinned eight-byte level-12 call only.

Decision: **retained**. Matched primary elapsed decreases1.064733% with all four pairs improving. Separate retired instructions decrease421,881.5 (1.003238%). Successful allocation requests/bytes, reallocations and allocator peak are unchanged, as predicted. Larger cycle/diagnostic elapsed reductions are separate observations, not primary claims. Retain adverse post-call RSS +49,152B and physical footprint +16,384B. Existing error ordering, paid prefixes, overflow and recovery are preserved; complete qualification remains a finalizer gate.

## Replay

Run `python3 reconstruct.py /new/empty/output/path` from the published packet. Replay verifies every archived file version and reproduces scorecard JSON/CSV, allocation-comparison JSON, profile JSON and validation JSON without compilation, compressor execution or external scratch inputs.

This complete small-input result does not establish larger-input throughput or a standalone minimum-memory footprint. Each reported direction and decision comes from R64 receipts; no earlier-round gains are multiplied into this result.
