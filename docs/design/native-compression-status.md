# Native compression: current status

**Milestone: stabilize the native capabilities retained through R140. Capability scope is frozen.** Performance parity is a later objective, not this PR's completion condition.

| Current baseline | Value |
|---|---|
| PR | [#973](https://github.com/mech-lang/mech/pull/973), draft |
| Stack base | `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b` |
| R140 published checkpoint | `b511c718c5c30cca5ac35812429c3e22198be7db` |
| Retained compiler | `58f75827e5c44187731df5253b3a058d2e9cd417` |
| IR/work/ABI/bytecode | **168/107/39/14** |
| Scope of final source change | Short-buffer semantic test and guarded CI inventory137; production is identical to R140 |

## Work board

| State | Work and finish condition |
|---|---|
| Active | Milestone closure/review: compact documents, responsibility review, evidence publication and integration boundaries |
| Closed: rejected | R141 early-refusal candidate `80dab41c28`: no established elapsed benefit; implementation removed, evidence and useful correctness test retained |
| Closed: dispositioned | R136 cold-validation investigation: repeated reachability/liveness work localized; remedy deferred as the first follow-up compiler task |
| Next | Address review blockers and decide the integration boundary; no new capability work in this increment |
| Next | Bound immutable analysis reuse within one image admission, with identical reconstructed proofs and malformed-image rejection |

**Known introduced performance findings remain open:** R140's8B literal regression is **+1.322924%** in its retained matched cohort; R136 image validation rose from roughly17s to35–37s. They are explicit tradeoffs for review, not resolved failures or merge-ready claims. The next queue is limited to the two items above.

Deferred backlog order after startup: ordinary fallback grouping; already-private stores; bounded-child trips; Huffman/header pricing source work; word-bank/constants/binding/arena changes; further instruction/SIMD work. None is active.

## Current measurements

All **20 retained compression inputs/sizes** have exact raw-output/reference agreement, independent inflation, aliases, output-prefix/capacity/clear-reuse checks and zero final managed data. The fresh R141 **before** side executes the exact retained R140 binary; its after side is rejected. These are complete `evaluate_into` timings. Full multi-megabyte timing and the separate ten-workload suite are excluded.

| Retained program/input | Elapsed | Mech / Roc |
|---|---:|---:|
|Periodic 33,027B|34.753979ms|27.8032×|
|Repeated 32,769B|23.108604ms|17.3098×|
|Random 32,769B|8.035166ms|7.6489×|
|Reymont prefix 32,769B|37.562645ms|8.6272×|
|Reymont prefix 16KiB|19.419875ms|9.1280×|
|16KiB distribution change|6.349562ms|7.9969×|
|Reymont prefix 1KiB|1.944208ms|5.8560×|
|8B literals|1.341750ms|4.5950×|

The full20-case measurements and adverse observations are in the [experiment index](../../benchmarks/mech-deflate/experiments.json), R140/R141. Reference medians for0/1/7B are below the retained clock's resolution; their ratios are unavailable. Ratios from different cohorts must not be compounded.

R140's matched16KiB distribution-change result improved **187.8584×→7.9902×**, with instructions2.479B→79.465M and requests67,321→5,679. Its incremental heap peak stayed18,702,352B. The retained executable's CPU samples are60.88% generated for periodic33,027B and90.01% for repeated32,769B; samples are separate from instruction counts.

## Decisions and limitations

- **Short buffers:** R141 removed four literal-case allocations and0.1039% of instructions, but elapsed was0.200298% worse, with four improving/four adverse blocks. It failed the elapsed-overhead objective and was rejected. Restoring efficient ordinary grouping requires a separate proof distinguishing current prepaid visits from ordinary fallback; the residual R140 regression is deferred. Source/output capacity13 and the conservative16-byte writer guard stay fixed.
- **Startup:** a current diagnostic measures36.110197s in image validation; historical R135 records17.022061s and R13634.88–35.42s. Sampling localizes expensive reachability/liveness and ordered-set work during independent proof reconstruction. These are diagnostic/historical observations, not a new controlled speedup claim. No validation is skipped. Remedy is deferred; this remains an integration/readiness finding.
- **Architecture review:** one read-only reviewer found no blocking correctness/architecture issue in R130→R140, no temporary production tracing, and no safely removable duplicate. Required ordinary and distinct collection fallbacks remain. This is separate from pending hosted review.

## Reconcile the R130 recommendations

| Recommendation | Disposition |
|---|---|
| Exact required state at native exits | Completed through R132/R136; preserve live frames, aliases, settlement and original errors |
| Remove historical temporary dependencies; initialize genuine inputs | Completed in R135 and retained exit lowering |
| Full physical word-bank lifetime compaction | Partial: transports/consumers shrink; full admitted lane capacity remains. Further compaction deferred |
| Independently reconstruct exit/representation recipes | Completed; serialized metadata grants no runtime authority |
| Better address/arithmetic emission | Completed in R137/R138 |
| Broader inputs and native bulk byte writing | Completed in R139/R140;20-case regression matrix retained |
| Direct stores after backing becomes private | Deferred; the old-element load/comparison still precedes the private flag |
| Bounded-child proofs in dynamically funded phases | Deferred; those phases still select empty trip proofs |
| Huffman lengths-only planning, header pricing and histogram source rewrites | Deferred; compressor source remains pinned |
| Further constants/binding/arena work, SIMD and instruction tuning | Ordered research backlog; no active implementation |
| Terminal-error snapshot experiment | R131 rejected; original evidence retained |

## Qualification and integration

Retained production plus the new test passes **562 compatible GPU tests** (same7 inherited profile exclusions), **7 focused bulk tests**, and **137 guarded/executed CI tests**. Stable R140 qualification supplies236 components,8 composition,1 real-capacity path,132 workflow tests,15 architecture checks, runtime-only metadata,12 scalar controls and8 frame controls. The R141 candidate also passed those broader cohorts before rejection. The final formatting-edition correction is retained with the failed receipt; configured formatting and diff checks pass. No allowance or expected component value increased.

Review by responsibility: Engine semantics/proofs → checked preparation → shared emission → current ownership/accounting/cleanup → independent image restoration. Keep coupled proof/emitter/runtime/identity changes atomic. Historical checkpoints permit separate liveness/freshness, scalar/address, and byte-writer reviews; CI/test-only repairs already have separate commits. No additional PRs or history rewrites are needed for this closure.

Hosted CI is cancelled by standing instruction; **the PR remains draft pending green CI and clean review**. Local qualification does not imply either. Closure does not authorize another optimization campaign. Full historical narratives, commands, raw data, failures and replay scripts are archived through the existing experiment index; [architecture](native-compiler-architecture.md) documents current responsibilities only.
