# R67: borrow the retained single-call plan

Production commit: `ae4a82d44eec478a936c518bf7b50359698173e2`. Candidate build source: `c9199ed6687c0125f64e9bf2abd0b0057d5cbf0d`. Baseline R66 build source: `739c8b3bf4a11dcfaa87800e6cb171390afc2ff1`. Accepted baseline checkpoint: `cc5cddaa356918620442626df5c0167cb2d516e4`.

The complete eight-byte level-12 call measured **12.724561× → 12.708040× Roc** in this matched cohort, 3.626500 → 3.621791 ms (-0.1298%). The fresh reference median was 0.285000 ms. Allocation requests fell by exactly 157 and retired instructions by 61,684. The elapsed difference is small and mixed; it does not establish a reliable latency improvement.

The native leaf adapter previously deep-cloned its retained CallMemoryPlan, retagged scratch owners, cloned allocations again, and constructed owned one-element node/call arrays solely to enter the general turn planner. A borrowed projection now supplies the same existing geometry to that same planner. Scratch owners acquire the actual source node only when selected output allocations are copied. The prepared call stays unchanged. There is no second planning algorithm, retained permission, or new execution cache.

Every current footprint, distinct published footprint, resolved region, checked demand, allocation placement and finite violation is still freshly derived. The original conservative work and temporary admission envelopes remain. The inner call-plan reconstruction and per-invocation binding have not been eliminated. This is a prerequisite toward keeping immutable preparation prepared, not completion of that milestone.

| Metric | Roc | R66 baseline | R67 candidate | Change |
|---|---:|---:|---:|
| Primary elapsed (ms) | 0.285000 | 3.626500 | 3.621791 | -0.1298% |
| Retired instructions | 1,364,372 | 40,593,136 | 40,531,452 | -0.1520% |
| CPU cycles | 941,227.5 | 11,426,539 | 11,428,649.5 | +0.0185% |
| Allocation requests | 95 | 7,097 | 6,940 | -2.2122% |
| Requested bytes | 9,021,923 | 98,146,732 | 98,127,494 | -0.0196% |
| Reallocations | 0 | 162 | 162 | +0.0000% |
| Peak allocator live usable bytes | 9,257,360 | 163,464,584 | 163,464,584 | +0.0000% |
| Incremental allocator peak | 9,257,328 | 20,482,072 | 20,482,072 | +0.0000% |
| RSS after call | 14,778,368 | 319,340,544 | 319,160,320 | -0.0564% |
| Physical footprint after call | 14,418,816 | 229,000,896 | 228,951,808 | -0.0214% |

Primary elapsed, CPU counters, heap counters and CPU-time samples are separate serial cohorts. Their percentages are not multiplied. Requested capacity is not initialized bytes copied. Allocator peak differs from post-call OS footprint; the latter includes retained compiler/catalog/image storage and is not peak RSS.

## Exact construction mechanism

| Family | Before requests / bytes | After requests / bytes |
|---|---:|---:|
| Owned adapter | 157 / 19,238 | 0 / 0 |
| Inner turn derivation | 414 / 52,969 | 414 / 52,969 |
| Current call resolution, nested within turn derivation | 386 / 42,006 | 386 / 42,006 |
| Current binding | 447 / 40,431 | 447 / 40,431 |

These are nested attribution groups and must not be summed. The removed adapter's complete size/kind histogram exactly equals the whole-call allocation delta. Every other size/kind count is unchanged, including the retained inner-planning and binding groups. Both fresh traces independently reconcile with all eight observations in their separate heap cohorts. `allocation-mechanism.json` contains the exact histogram proof and `allocation-comparison.json` retains both census pins.

## Timing variation and adverse observations

| Pair | Baseline ms | Candidate ms | Candidate/baseline |
|---|---:|---:|---:|
| 0 | 3.643583 | 3.614167 | 0.99192663× |
| 1 | 3.624229 | 3.616083 | 0.99775235× |
| 2 | 3.616355 | 3.648417 | 1.00886583× |
| 3 | 3.623375 | 3.620917 | 0.99932149× |

3/4 independently prepared pairs and 39/64 alternating blocks improved. The geometric mean block ratio was **1.00163205×**, an adverse direction. Four pairs ×16 blocks ×31 calls ×2 variants provide 3,968 primary observations. Blocks within a pair are descriptive, not independent repetitions.

The separate CPU-counter diagnostic elapsed was 3.564813 → 3.564520 ms (-0.008191%). Those observations are excluded from primary elapsed. Other observed median increases:

```json
{
  "cycles": 0.018470159687034027
}
```

## Actual binary and profiles

| Metric | Baseline | Candidate |
|---|---:|---:|
| Generated text bytes | 11,800,572 | 11,800,572 |
| Machine entry stack bytes | 107,344 | 107,344 |
| Typed IR version | 132 | 132 |
| ABI version | 37 | 37 |
| Linked host text bytes | 60,835,452 | 60,846,264 |
| Linked executable bytes | 170,060,616 | 170,080,328 |

Fresh compressor Object/image/metadata remain byte-identical. The changed host adapter and shared turn-planner functions are disassembled separately in `host-assembly/`; `host-assembly-comparison.json` records actual static function sizes/call edges. Static instruction counts and CPU sample weights are not dynamic instruction attribution. Whole-call retired instructions are measured independently.

Baseline is the completed retained R66 graph and its actual build source, with complete-axis frame guards. Historical build identity is not replaced with a later report/checkpoint SHA.

R66 fresh emission matched the retained R62 Object/image/metadata byte-for-byte. R62 previously corrected stale-offset pointer-role attribution; that correction changed no whole opcode totals or hardware measurements. R67 performs its own fresh emission identity check.

Fresh CPU-time samples:

| Executable | Exclusive sample weights | Frame-action ancestry | Thread-local ancestry | Tail-cleanup ancestry |
|---|---:|---:|---:|---:|
| baseline | 1,297 | 23.67% | 2.93% | 4.01% |
| candidate | 1,222 | 26.43% | 4.91% | 4.34% |
| reference | 1,517 | 0.00% | 0.13% | 0.00% |

These ancestry groups overlap. Samples include the diagnostic harness and are excluded from primary elapsed. They do not partition retired instructions. The complete top leaves and overlapping categories remain in `profile-numbers.json`.

## Qualification and scope

Status: **passed**; 76 required receipt checks. Counts below are actual executions; inventory rows remain in the JSON. Batches overlap.

| Receipt | Expected | Observed | Exit / expected | Passed |
|---|---:|---:|---:|---|
| `engine-parent/build.json` | None | None | 0 / 0 | True |
| `engine-parent/projection.json` | 12 | 12 | 0 / 0 | True |
| `engine-parent/policy-order.json` | 1 | 1 | 0 / 0 | True |
| `engine-parent/overflow-retry.json` | 1 | 1 | 0 / 0 | True |
| `engine-parent/shape.json` | 6 | 6 | 0 / 0 | True |
| `engine-parent/borrowed-turn.json` | 4 | 4 | 0 / 0 | True |
| `engine-parent/full-planner.json` | 19 | 19 | 0 / 0 | True |
| `engine-after/build.json` | None | None | 0 / 0 | True |
| `engine-after/projection.json` | 12 | 12 | 0 / 0 | True |
| `engine-after/policy-order.json` | 1 | 1 | 0 / 0 | True |
| `engine-after/overflow-retry.json` | 1 | 1 | 0 / 0 | True |
| `engine-after/shape.json` | 6 | 6 | 0 / 0 | True |
| `engine-after/borrowed-turn.json` | 4 | 4 | 0 / 0 | True |
| `engine-after/full-planner.json` | 19 | 19 | 0 / 0 | True |
| `engine-preflight/build.json` | None | None | 0 / 0 | True |
| `engine-preflight/borrowed-turn.json` | 4 | 4 | 0 / 0 | True |
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

Both parent and candidate projection tests must pass. The new borrowed adapter is compared with the former owned single-call projection on both parent and candidate test sources. Parent adds only a test shim; no parent production behavior is changed.

Both parent and candidate execute the complete 19-test Engine planner inventory, including both new borrowed-versus-owned cases. Coverage includes source nodes 0/37, actual scratch owners, current payload sizes 0/13/511, unrelated-node facts, distinct published footprints, finite violations, missing-witness versus overflow precedence, unchanged original templates and retry. Parent uses exact R66 production plus a test-only shim exposing the former owned adapter. Candidate uses the actual borrowed adapter.

Engine and its Compute/Stdlib/Runtime/GPU dependents were rebuilt coherently at opt-level 3 with warnings-as-errors and their retained feature sets. Core/math/syntax/external libraries remain frozen by hash. The fresh Engine tests use their archived broader feature list at opt-level 0; GPU units use the retained opt-level 1 configuration. Scalar release controls retain the unchanged 1.05 ceiling. Exact commands, flags and dependencies are in each build receipt. These are local executions, not a claim of minimal-profile or completed hosted CI. The ordinary Engine owner profile schedules the new tests through its existing unfiltered `--lib` selection.

Failures and corrections:
- `findings-packet/build-script-provenance.json` — introduced harness-source provenance mismatch; corrected: Restored the exact executed script; retain both versions and evidence that the executed helper produced separate copies. Numerical/measurement receipts and compiler artifacts require no rerun. Future edits to a running receipt producer are prohibited.

No compiler or qualification failure remains unless explicitly listed above. The build-helper provenance correction preserves both the actual executed source and the unused transient storage edit; explicit rustc commands and all compiler/library hashes were unchanged.

Skipped coverage:
- complete_unchanged_knucleotide_body_uses_the_owned_emitted_map: Existing runtime selection exclusion retained; unrelated complete knucleotide body is not requalified here.
- full remote CI: Not awaited as requested. Local selected tests with their actual retained feature configurations and architecture checks are reported separately.
- larger compression inputs/full-scale timing: Not run; this optimization measures the complete pinned eight-byte level-12 call only.

## Preserved workload and remaining structural work

Full eight-byte level-12 compression of `abcdefgh`, exact raw output `[75,76,74,78,73,77,75,207,0,0]` and independent inflate. Output capacity 13 is cleared/reused. Real buffers remain 262,144 U16 tree entries, 1,500,513 U32 cache words and 305,257 U64 path capacity; depth 300, nice length 258. The pinned source, bytecode, source allocation sites, allocator, input aliases and evaluate_into timing are unchanged. Logical work 60,260,219,686 and comparison work 47,612 are accounting, not hardware instructions. Final managed bytes/nodes are zero.

R66 already removed the first-change immutable-to-private round trip and demonstrated a zero allocation slope within the real reserved buffer capacities. R67 does not relabel that historical evidence as a fresh test. Further work remains to stage invariant binding/plan structure, extend private ownership across genuinely unobserved handoffs, reuse sequential owner workspace and identify the required consumer of every remaining canonical materialization. Current-account authority and finite checks must stay invocation-local.

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

Decision: **retained**. Retain the simpler borrowed projection: it removes exactly 157 allocation requests /19238 requested bytes and 61684 measured whole-call instructions while preserving the maintained planner and all qualification. Primary pooled elapsed improves only 0.1298359%; one pair and the geometric mean block ratio are adverse, so no reliable latency win is established. The positive cycle delta and linked host growth are retained. This is not completion of prepared binding or ownership lifetime work.

## Standalone replay

Run `python3 reconstruct.py /new/empty/output/path` from this packet. Replay verifies every archived version and reproduces scorecard JSON/CSV, census comparison, allocation mechanism, host-assembly comparison, profiles and validation from raw receipts, without compiling or executing a compressor. No scratch directory is needed.

This result does not establish larger-input throughput, allocation parity or standalone minimum memory. No earlier-round gains are multiplied into these measurements.
