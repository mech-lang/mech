# R65: inline the maintained checked-debit helper

Candidate build: `78209b2aab2175f1fad92ee4daae3d6b38669d36`. Baseline R64 build: `a27ea040845b41a63fe29f6837773860bea8607c`. Accepted baseline checkpoint: `78614f175a932b37a73e9b8a12be1fd4e390c9f3`.

The complete eight-byte level-12 call measured **12.676222× the reference**, from **12.709267×** in this matched baseline cohort. Candidate/baseline elapsed: **0.99739994×** (-0.2600%). The freshly measured reference median was 0.290000 ms.

The sole production change replaces `#[inline]` with `#[inline(always)]` on `NativeLibraryWorkBudget::charge_in_context`. It exposes the existing maintained checked-debit body to its callers, including a constant zero comparison delta in the compute-only path. Accounting arithmetic, current account/TLS access, finite limits, error precedence and debit order remain source-identical. Existing parent and candidate tests cover finite ceiling/overflow before foreign-account rejection, tighter library caps, unchanged work on denial and successful retry. No new semantic implementation or source test is introduced.

Engine, Compute, Stdlib, Runtime and GPU were rebuilt coherently with their retained flags/features and warnings-as-errors. Core, math, syntax and external artifacts remain frozen. The candidate emits its Object, descriptor and metadata afresh; all three must match the R64 baseline byte-for-byte. No compressor source, algorithm, source allocation site or timing boundary changes.

| Metric | Reference | R64 baseline | R65 candidate | Change |
|---|---:|---:|---:|---:|
| Primary elapsed (ms) | 0.290000 | 3.685688 | 3.676105 | -0.2600% |
| Retired instructions | 1,364,372 | 41,623,860 | 41,081,843 | -1.3022% |
| CPU cycles | 913,799 | 11,629,355 | 11,597,526.5 | -0.2737% |
| Allocation requests | 95 | 8,105 | 8,105 | +0.0000% |
| Requested bytes | 9,021,923 | 98,227,372 | 98,227,372 | +0.0000% |
| Reallocations | 0 | 162 | 162 | +0.0000% |
| Peak allocator live usable bytes | 9,257,360 | 163,464,584 | 163,464,584 | +0.0000% |
| Incremental allocator peak | 9,257,328 | 20,482,072 | 20,482,072 | +0.0000% |
| RSS after call | 14,761,984 | 319,291,392 | 318,963,712 | -0.1026% |
| Physical footprint after call | 14,402,432 | 229,017,344 | 228,574,912 | -0.1932% |

Primary elapsed, CPU counters, heap counters and CPU-time samples are separate serial cohorts. Their percentages are not multiplied or combined. Requested bytes represent allocation requests, not initialized bytes copied. Allocator peak and post-call OS footprint are different measures; the latter is not peak RSS and includes retained compiler/catalog/image storage.

## Allocation and instruction mechanism

This attribute-only change predicts zero successful-call allocation savings. It targets checked-debit call/return, constant-argument and result transport costs. The actual emitted host code determines what disappears; the directive itself is not evidence. Hardware instruction changes are measured independently of allocator counts and CPU-time samples.

| Complete census | R64 baseline | R65 candidate | Delta |
|---|---:|---:|---:|
| Allocation requests | 8,105 | 8,105 | +0 |
| Requested bytes | 98,227,372 | 98,227,372 | +0 |
| Reallocations | 162 | 162 | +0 |

Size/kind request histogram unchanged: **True**. Both full censuses are fresh R65 measurements, individually reconciled to their separate eight-observation heap cohorts. The archived trace pins and per-size counts are in `allocation-comparison.json`. Equal totals do not establish identical work or execution counts.

Earlier sample and host-code evidence identified the checked-debit caller/callee boundary. R64 already addressed the separate full-policy-evaluator query. No R64 gain is reproposed or included here. TLS source is unchanged; earlier TLS sample weights are not treated as removable by this attribute.

## Matched pairs and adverse observations

| Pair | Baseline ms | Candidate ms | Candidate/baseline |
|---|---:|---:|---:|
| 0 | 3.682063 | 3.682520 | 1.00012425× |
| 1 | 3.695312 | 3.671896 | 0.99366319× |
| 2 | 3.672375 | 3.667355 | 0.99863290× |
| 3 | 3.690271 | 3.678021 | 0.99668060× |

3/4 independently prepared pairs and 44/64 alternating blocks improved. Four pairs ×16 blocks ×31 calls ×2 variants produce 3,968 timed observations. Blocks within one pair are descriptive, not independent repetitions.
Adverse or unchanged primary pairs: 0 (+0.0124%)

The separate CPU-counter cohort elapsed was 3.628583 → 3.618417 ms (-0.280178%). These 32 observations per variant are excluded from primary elapsed and retain their independent direction.

Other observed median increases:

```json
{}
```

## Binary and baseline provenance

| Metric | R64 baseline | R65 candidate |
|---|---:|---:|
| Generated text bytes | 11,800,572 | 11,800,572 |
| Machine entry stack bytes | 107,344 | 107,344 |
| Typed IR version | 132 | 132 |
| ABI version | 37 | 37 |

### Actual linked host code

The generated Object is identical; the host code in the linked worker is audited separately. Full-function static counts include failure/panic paths and are not dynamic or successful-path instruction counts. Missing callee symbols alone do not establish cost removal.

| Host metric | Baseline | Candidate | Change |
|---|---:|---:|---:|
| Linked __text bytes | 60,829,048 | 60,823,288 | -5,760 |
| Linked worker file bytes | 170,088,056 | 170,087,800 | -256 |

| Variant / function | Static instructions | Calls to outlined helper |
|---|---:|---:|
| before / `charge_compute_with_limit` | 73 | 1 |
| before / `charge_in_context` | 99 | 0 |
| before / `NativeLibraryWorkBudget6charge` | 49 | 1 |
| after / `charge_compute_with_limit` | 158 | 0 |
| after / `NativeLibraryWorkBudget6charge` | 133 | 0 |

Raw caller/callee assembly, instruction records, TLS calls and stack adjustments are retained in `host-assembly-comparison.json` and its raw archive. Do not sum individual stack adjustments into an invented simultaneous stack depth. A later path interpretation must retain the exact branches and current TLS/account assumptions.


Object/image/metadata equality: True. The unchanged generated bytes have unchanged static instruction content. Hardware retired instructions can still change because this patch changes the host checked-debit code. `binary-audit.json` records each actual image pin.

Baseline is the completed retained R64 graph and its actual build source, including the completed first-only central projection-budget change. Historical build identity is not replaced with a later report/checkpoint SHA.

R64 fresh emission matched its retained baseline Object/image/metadata byte-for-byte. R65 separately checks fresh emitted-image identity and audits the linked host caller/callee assembly; these are distinct code layers.

These historical notes preserve provenance; no R64 measurement or test execution is counted as an R65 observation.

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
| `engine-parent/reuse.json` | None | None | 0 / 0 | True |
| `engine-parent/projection.json` | 12 | 12 | 0 / 0 | True |
| `engine-parent/policy-order.json` | 1 | 1 | 0 / 0 | True |
| `engine-parent/overflow-retry.json` | 1 | 1 | 0 / 0 | True |
| `engine-parent/shape.json` | 6 | 6 | 0 / 0 | True |
| `engine-after/build.json` | None | None | 0 / 0 | True |
| `engine-after/projection.json` | 12 | 12 | 0 / 0 | True |
| `engine-after/policy-order.json` | 1 | 1 | 0 / 0 | True |
| `engine-after/overflow-retry.json` | 1 | 1 | 0 / 0 | True |
| `engine-after/shape.json` | 6 | 6 | 0 / 0 | True |
| `core-focused/reuse.json` | None | None | 0 / 0 | True |
| `core-focused/selected.json` | 2 | 2 | 0 / 0 | True |
| `gpu-parent-work/reuse.json` | None | None | 0 / 0 | True |
| `gpu-parent-work/work.json` | 12 | 12 | 0 / 0 | True |
| `gpu-parent-work/ceiling-order.json` | 1 | 1 | 0 / 0 | True |
| `units-after/work.json` | 12 | 12 | 0 / 0 | True |
| `units-after/ceiling-order.json` | 1 | 1 | 0 / 0 | True |
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

Fresh execution of the pinned unchanged R64 parent Engine binary and freshly built candidate Engine must each pass projection 12, shape 6, GPU work 12 and exact error-order 1 tests must pass. No new source tests or expected-failure controls; unchanged Core equivalence 2 is executed afresh using the pinned R64 test binary.

Fresh full R65 runtime execution is required. Historical R64 batches are not counted. Projection 12, shape 6, GPU work 12, Core 2 and GPU CI selections are additional overlapping batches, not a sum of unique tests.

Parent Engine qualification freshly executes the pinned unchanged R64 test binary; candidate Engine tests are newly compiled against the candidate graph. The local `ci.py` Engine batches execute all 12 projection-meter tests and all six inherited frame-shape tests using the retained broader Engine test feature configuration. Core also executes its two existing first-violation equivalence/order tests afresh using the pinned unchanged R64 Core test binary. Both variants freshly execute the 12 GPU work tests and the exact ceiling/foreign-scope-order case. The complete commands are archived; this is not a claim that the local Engine build used exactly a minimal CI feature set or that remote CI completed.

Earlier failed attempts and dispositions:
- `parent-runner-initial/receipt.json` — introduced harness argument mismatch: Prepared README/root runner used gpu-parent but helper accepts gpu. Earlier parent Engine12/shape6 and Core2 executions passed. Fix only orchestration argument and execute GPU witnesses; no production failure or benchmark rerun.
- `report-initial/receipt.json` — introduced report block-count assertion: Round-number substitution changed the raw primary block-count assertion from64 to65. The runner actually contains the unchanged4pairs×16blocks=64blocks and3968timed observations. Restore the assertion to64; retain all raw timings and qualification thresholds unchanged. All74qualification checks already pass.
- `replay-initial/receipt.json` — introduced replay packet input omission: New host-assembly replay consumes linked-primary/before/binding.json; baseline binding was archived at baseline/binding.json only. Add the exact same verified baseline binding at the required replay path. No measurement, test, source or pin change.

Skipped coverage:
- complete_unchanged_knucleotide_body_uses_the_owned_emitted_map: Existing runtime selection exclusion retained; unrelated complete knucleotide body is not requalified here.
- full remote CI: Not awaited as requested. Local selected tests with their actual retained feature configurations and architecture checks are reported separately.
- larger compression inputs/full-scale timing: Not run; this optimization measures the complete pinned eight-byte level-12 call only.

Decision: **retained**. Retained for 542,017 fewer whole-call retired instructions (1.302179%) and 5,760 fewer linked text bytes. Pooled primary elapsed decreases 0.260006%; three of four pairs improve, with the first pair adverse by 0.012425%. This small elapsed result is retained with its contrary observation; it does not establish a universal or larger-input speedup. Allocation counts/bytes, reallocations and allocator peak are unchanged. The actual caller loses its separate helper call and post-helper 40-byte Result copy, while TLS and duplicate compute checks remain. Individual callers grow and the compute caller frame grows from 96 to 128 bytes; newly materialized immutable-borrow bookkeeping is explicit in binary-review.md. All scalar controls pass the unchanged 1.05 ceiling; their largest adverse observed ratio is 1.016217325 for the decoded 64-iteration loop. Complete qualification remains a finalizer gate.

## Replay

Run `python3 reconstruct.py /new/empty/output/path` from the published packet. Replay verifies every archived file version and reproduces scorecard JSON/CSV, allocation-comparison JSON, profile JSON, validation JSON and host-assembly-comparison JSON without compilation, compressor execution or external scratch inputs.

This complete small-input result does not establish larger-input throughput or a standalone minimum-memory footprint. Each reported direction and decision comes from R65 receipts; no earlier-round gains are multiplied into this result.
