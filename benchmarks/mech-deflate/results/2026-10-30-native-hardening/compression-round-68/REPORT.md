# R68: rejected descriptor-sharing experiment and instruction windows

Rejected candidate change: `e015670b7be26d3cc07d26640d1f538a0594f71b`. Candidate build source: `f175488334a2c75e027de455f59d636bd2ffc502`. Baseline R67 build source: `c9199ed6687c0125f64e9bf2abd0b0057d5cbf0d`. Accepted baseline checkpoint: `5dff1496b523749e5f10f5085e2206be0c930a7e`.

The rejected candidate complete eight-byte level-12 call measured **12.454038× → 12.561067× Roc** in this matched cohort, 3.624125 → 3.655271 ms (+0.8594%). The fresh reference median was 0.291000 ms. This is a fresh comparison, not a compounded earlier-round gain.

This candidate was reverted in `ae1113085`; production retains the accepted R67 representation. The independent shape/lifetime tests and source-contract repairs remain. Candidate qualifications/measurements below are kept distinct from restored production.

A resolved value descriptor previously copied its immutable type and schema trees on each clone or current-shape update. The candidate retained that semantic definition through one shared owner. Each current shape remained independently owned and was checked by the original schema instantiator before sharing. A rejected shape cannot change earlier descriptors. The owner carried no value, account, reservation, factory state or permission.

| Metric | Roc | R67 baseline | R68 candidate | Change |
|---|---:|---:|---:|
| Primary elapsed (ms) | 0.291000 | 3.624125 | 3.655271 | +0.8594% |
| Retired instructions | 1,364,372 | 40,536,406.5 | 41,517,149.5 | +2.4194% |
| CPU cycles | 930,342.5 | 11,350,422 | 11,589,705 | +2.1081% |
| Allocation requests | 95 | 6,940 | 6,212 | -10.4899% |
| Requested bytes | 9,021,923 | 98,127,494 | 98,056,810 | -0.0720% |
| Reallocations | 0 | 162 | 162 | +0.0000% |
| Peak allocator live usable bytes | 9,257,360 | 163,464,584 | 160,924,648 | -1.5538% |
| Incremental allocator peak | 9,257,328 | 20,482,072 | 20,482,072 | +0.0000% |
| RSS after call | 14,778,368 | 318,963,712 | 306,462,720 | -3.9193% |
| Physical footprint after call | 14,418,816 | 228,738,752 | 216,286,976 | -5.4437% |

Primary elapsed, CPU counters, heap counters and CPU-time samples are separate serial cohorts. Their percentages are not multiplied. Requested capacity is not initialized bytes copied. Allocator peak differs from post-call OS footprint; the latter includes retained compiler/catalog/image storage and is not peak RSS.

## Construction and lifetime evidence

The new release witness fails against the exact parent Core library: one closed record descriptor clone makes 11 allocation requests. The rejected candidate clones and retires descriptors with **zero requests** at 1, 64 and 4096 repetitions, using both short and 64 KiB labels. Definition construction is outside that clone interval; it remains a real construction cost and is included wherever it occurs in the complete-call census. This diagnostic uses System symmetrically, separate from pinned-mimalloc compression measurements.

The complete fresh traces are classified using mutually exclusive first-match rules. The first two rows replace the former deep semantic construction; residual binding/turn rows exclude those earlier families. These rows may be summed. They must not be added to overlapping full binding/turn ancestry from earlier reports.

| Disjoint census family | Before requests / bytes | After requests / bytes | Request delta |
|---|---:|---:|---:|
| descriptor_clone | 704 / 60,042 | 28 / 1,008 | -676 |
| current_shape | 52 / 4,050 | 0 / 0 | -52 |
| identified_descriptor_construction | 0 / 0 | 0 / 0 | +0 |
| other_binding | 231 / 22,119 | 231 / 19,079 | +0 |
| other_turn_planning | 198 / 34,657 | 198 / 32,833 | +0 |
| remaining | 5,755 / 98,006,626 | 5,755 / 98,003,890 | +0 |

Whole-call delta: -728 requests / -70,684 requested bytes / +0 reallocations. Both traces reconcile independently with all eight observations in their separate heap cohorts. `allocation-mechanism.json` retains the exact disjoint attribution and `allocation-comparison.json` records both census hashes/histograms. New immutable owners or residual shape/container changes must be included in the net delta; eliminating one old family is not itself the whole saving.

## Timing variation and adverse measurements

| Pair | Baseline ms | Candidate ms | Candidate/baseline |
|---|---:|---:|---:|
| 0 | 3.619312 | 3.654104 | 1.00961288× |
| 1 | 3.596334 | 3.650729 | 1.01512540× |
| 2 | 3.623833 | 3.654188 | 1.00837649× |
| 3 | 3.675063 | 3.663542 | 0.99686509× |

1/4 independently prepared pairs and 10/64 alternating blocks improve. Geometric mean block ratio: 1.00555284×. Four pairs ×16 blocks ×31 calls ×2 variants provide 3,968 primary observations; blocks within a pair are descriptive, not independent repetitions.

The separate CPU-counter diagnostic elapsed was 3.538875 → 3.614958 ms (+2.149921%). These observations are excluded from primary elapsed. Retained median increases:

```json
{
  "primary_seconds": 0.8593936467423235,
  "instructions": 2.4194127814462396,
  "cycles": 2.108141882301817
}
```

## Actual binary and profiles

| Metric | Baseline | Candidate |
|---|---:|---:|
| Generated text bytes | 11,800,572 | 11,800,572 |
| Machine entry stack bytes | 107,344 | 107,344 |
| Typed IR version | 132 | 132 |
| ABI version | 37 | 37 |
| Linked host text bytes | 60,846,264 | 60,845,240 |
| Linked executable bytes | 170,080,328 | 170,120,616 |

Fresh compressor Object/image/metadata are byte-identical. Actual changed ARM64 host functions are disassembled separately in `host-assembly/`; `host-assembly-comparison.json` retains own static function sizes and call edges. Missing standalone symbols can reflect inlining or deduplication. Static counts and sample weights are not dynamic instruction attribution.

| Executable | Exclusive sample weights | Frame-action ancestry | Thread-local ancestry | Tail-cleanup ancestry |
|---|---:|---:|---:|---:|
| baseline | 1,236 | 24.43% | 2.27% | 4.37% |
| candidate | 1,318 | 24.58% | 2.58% | 4.40% |
| reference | 1,521 | 0.00% | 0.07% | 0.00% |

These ancestry groups overlap; samples include the diagnostic harness and are excluded from primary elapsed. Complete leaves/categories remain in `profile-numbers.json`.

## Semantic qualification and build configuration

Status: **passed**; 82 required receipt checks. Rows below are actual executions; inventories remain in JSON. Batches overlap.

| Receipt | Expected | Observed | Exit / expected | Passed |
|---|---:|---:|---:|---|
| `core-before/build.json` | None | None | 0 / 0 | True |
| `core-before/selected.json` | 369 | 369 | 0 / 0 | True |
| `core-before/descriptor.json` | 2 | 2 | 0 / 0 | True |
| `allocation-before/build.json` | None | None | 0 / 0 | True |
| `allocation-before/descriptor.json` | 1 | 1 | 101 / 101 | True |
| `core-after/build.json` | None | None | 0 / 0 | True |
| `core-after/selected.json` | 369 | 369 | 0 / 0 | True |
| `core-after/descriptor.json` | 2 | 2 | 0 / 0 | True |
| `allocation-after/build.json` | None | None | 0 / 0 | True |
| `allocation-after/descriptor.json` | 1 | 1 | 0 / 0 | True |
| `allocation-after/complete.json` | 5 | 5 | 0 / 0 | True |
| `engine-after/build.json` | None | None | 0 / 0 | True |
| `engine-after/projection.json` | 12 | 12 | 0 / 0 | True |
| `engine-after/policy-order.json` | 1 | 1 | 0 / 0 | True |
| `engine-after/overflow-retry.json` | 1 | 1 | 0 / 0 | True |
| `engine-after/shape.json` | 6 | 6 | 0 / 0 | True |
| `engine-after/borrowed-turn.json` | 4 | 4 | 0 / 0 | True |
| `engine-after/full-planner.json` | 19 | 19 | 0 / 0 | True |
| `source-contracts-focused/planner.json` | None | None | 0 / 0 | True |
| `source-contracts-focused/runtime.json` | None | None | 0 / 0 | True |
| `source-contracts-focused/mutations.json` | 16 | 16 | 0 / 0 | True |
| `constructor-ci.json` | None | None | 0 / 0 | True |
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
| `type-architecture/results.json` | — | — | 0 / 0 | True |
| `type-architecture/results.json` | — | — | 0 / 0 | True |
| `type-architecture/results.json` | — | — | 0 / 0 | True |
| `restoration/summary.json` | — | 4 | — / 0 | True |

Both exact parent and candidate Core implementations execute the same 369 tests, including current shape isolation/bounds/retention/retry. The new release allocation witness must fail on the pinned parent and pass on the candidate. The complete unchanged Engine planner inventory executes against coherent rebuilt dependencies.

The same 369 Core tests run against exact parent/candidate implementations. New cases compare fresh shape updates with independent descriptor derivation, retain earlier shapes, reject bounds, retry, and distinguish turn from activation-lifetime type contracts. Each unchanged Engine planner/current-value test runs against the coherent candidate graph. Original/decoded component and linked cases retain finite errors, aliases, publication and recovery. Scalar controls keep the unchanged 1.05 ceiling.

All twelve selected Core-dependent crates are rebuilt coherently at their retained optimized profiles/features, with warnings-as-errors and frozen external dependencies. Actual release commands include Core, Syntax, Bytecode, Engine, Compute, Math, Range, Logic, Compare, Stdlib, Runtime and GPU; no mixed old Core-dependent project libraries are searched. Core/Engine unit roots and GPU opt-level-1 roots use their separately archived retained flags. This is local coverage, not a claim of completed hosted/all-features CI.

Rejected-candidate Rust API detail: `ResolvedValueDescriptor::schema` and `resolved_type` become ordinary read-only methods because their storage is shared; their return types/content are unchanged. `shape` remains a constant accessor. The diagnostic field format is retained explicitly. Schema/shape codecs serialize semantic content, and the freshly emitted artifact bytes remain unchanged. Production restores both original constant accessors. Candidate design/source are archived in `findings-packet/candidate-design.md` and the source patches; `docs/design/prepared-descriptor-semantics.md` records the rejection.

## Separately reviewed source-contract repairs

The clean published R67 parent reproduces two stale source-checker diagnostics. One expected budget closure directly inside the old turn-planning entry. The updated check verifies both current-limit projections into the common derivation and its finite recheck. The other expected an old token-cache format and shared-return inventory. It now requires the existing schema/shape prefix/token pair, actual shared cache owner, checked hit/recomputed miss, and the admitted target shape for an equivalent primitive matrix rebind. Unknown return forms still fail closed.

The repairs are separate from descriptor semantics in `ffbb5df7f` and `41fbab23a`; focused positive/bypass mutations cover both entries, missing closure, wrong cache owner, stale prefix/token, skipped target-shape admission, unequal geometry and old-shape reuse. The constructor inventory update is separately reviewable in `e6c027f57`: the actual expanded list contains five tests; the old four-count guard rejects it and the five-count guard passes. Rust execution of all five candidate tests is retained. After rejection, the candidate-only allocation witness is archived and production returns to its original four-test inventory; the restored four-test list, execution and strict guard all pass.

Every observed failure and disposition:
- `baseline-contracts/memory-planner.stderr` — Inherited in R68: reproduced on clean published R67 head 5dff1496b523749e5f10f5085e2206be0c930a7e: Repair the source checker to follow both current-limit projections into the shared turn derivation; retain checks of its final finite-budget closure and add bypass mutants.
- `baseline-contracts/memory-runtime.stderr` — Inherited in R68: reproduced on clean published R67 head 5dff1496b523749e5f10f5085e2206be0c930a7e: Repair the checker and old mutation anchors for the existing prefix/token cache pair. Require the current schema/shape prefix, the actual shared data owner, checked hit and recalculated miss; add stale-prefix/token mutants.
- `source-contracts-focused-initial/runtime.stderr` — Introduced incomplete source-checker migration; resolved separately: The first cache guard repair still rejected the existing equivalent primitive matrix return with its admitted target shape. Preserve that failed attempt; the revised closed inventory accepts only its complete geometry/shape-admission prelude and rejects all three bypass mutants. No runtime or compressor code changed.

The first source-checker repair attempt is preserved. It missed the existing admitted primitive-matrix return; the subsequent closed inventory and three bypass mutants correct it. No compiler or compressor runtime code changed for those repairs.

Skipped coverage:
- complete source-checker mutation suites: The focused 16 ownership/budget/cache mutation tests execute. Other mutation families are unchanged and are not rerun here; this does not claim a complete hosted architecture-shard execution.
- complete_unchanged_knucleotide_body_uses_the_owned_emitted_map: Existing runtime selection exclusion retained; unrelated complete knucleotide body is not requalified here.
- full remote CI: Not awaited as requested. Local selected tests with their actual retained feature configurations and architecture checks are reported separately.
- larger compression inputs/full-scale timing: Not run; this optimization measures the complete pinned eight-byte level-12 call only.

## Disjoint diagnostic instruction windows

Before: **96.013157%** of instructions are within the linked entry and its callbacks. Candidate: **96.082146%**. The entry/callback window adds **973,716.78125** mean instructions; outside setup/cleanup adds **10,587.28125**. Their sum is **984,304.06250**; **98.924389%** of the diagnostic increase is inside entry/callbacks.

Each variant executes 32 measured diagnostic calls. Two linked-entry calls occur per compression. The trampoline adds only two counter reads per entry, never per loop; its empty-entry median is 5,000 instructions per call in each variant. Raw windows and overhead are retained without subtraction. Means of disjoint windows reconcile exactly; medians need not sum. These diagnostics are excluded from primary elapsed and do not identify an individual callback or establish a pointer/refcount explanation.

## Restored production checkpoint

Production is byte-identical to accepted R67 for all Rust implementation files; only the two independent Core semantic tests differ. Their exact current sources already pass with the restored parent implementation in the 369-test parent run. The current four-test allocation suite and four-count guard execute successfully. A separate 32-call confirmation using the pinned accepted compiler completes at median **3.592437 ms**, with exact output, independent inflate, aliases, capacity/work and cleanup checks. This unpaired confirmation is excluded from primary/improvement claims. All five type/memory source-contract checks pass after restoration.

## Workload and remaining materialization boundaries

Full eight-byte level-12 compression of `abcdefgh`: raw bytes `[75,76,74,78,73,77,75,207,0,0]` with independent inflate. Output capacity 13 is cleared/reused. Real capacities remain 262,144 U16 tree entries, 1,500,513 U32 cache words and 305,257 U64 path words; depth 300 and nice length 258. Source, bytecode, source allocation sites, allocator, input aliases and evaluate_into timer are unchanged. Logical work 60,260,219,686 and comparison work 47,612 are accounting, not hardware instructions. Final managed bytes/nodes are zero.

The private first-change round trip and within-capacity update slopes qualified in R66 stay removed; they are not new R68 observations. This patch does not make all binding/plan geometry prepared or explain away every canonical owner. The remaining inventory must distinguish these consumers:

| Boundary | Present consumer / remaining obligation |
|---|---|
| Caller input import | Current invocation admission and retained caller aliases; authority cannot be reused from preparation. |
| Constant imports | Canonical constant slots are still initialized each invocation. Inventory all fallback/canonical consumers before changing their ownership. |
| Seed record into singleton matrix | The ordinary array constructor still consumes canonical element values; that representation boundary remains. |
| Ordinary collection/update results | Existing canonical consumers and their retained versions remain; each eligibility/refusal must be traced before extending private storage. |
| Private numeric continuation | R66 retains one final owner inside an eligible region. Further handoffs require proof that no canonical observer or identity needs the intermediate. |
| Public result | Escaping output requires a durable published value. Publication/abort/retry remain atomic. |

This is a partial architectural inventory, not a claim that every remaining materialization is necessary. Next work should use the new census/profile to identify one remaining internal ownership or preparation boundary with a concrete consumer and acceptance witness.

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

Decision: **rejected**. Reject and revert descriptor-wide semantic sharing. It removes 728 requests (10.4899%) and lowers observed memory, but matched elapsed rises 0.859394%, 3/4 pairs and 54/64 blocks are adverse, and independent counters add 980743 instructions (2.4194%) and 239283 cycles. Preserve the resource finding, all contrary measurements and exact candidate code. Restore R67 production byte-for-byte, retain independent semantic regressions and source-contract repairs. The next experiment should place sharing at immutable call-plan boundaries rather than assuming a global representation change is free.

## Standalone replay

Run `python3 reconstruct.py /new/empty/output/path` from this packet. Every archived version is hash-verified before chronological overlay. Eight numerical outputs are reconstructed without compilation, execution or external scratch inputs. Actual compiler/test commands, raw failures, source patches, cohorts, samples and host excerpts remain archived.

This small complete-call result does not establish larger-input throughput, allocation parity or standalone minimum memory. No independent or earlier-round gains are multiplied.
