# Native compression: current status

**The native capability milestone through R140 is closed for review. Scope is frozen; performance parity and further optimizations are subsequent assignments.** R142 is one bounded startup follow-up, with no new execution capability.

| Current checkpoint | Value |
|---|---|
| Frozen milestone | [#973](https://github.com/mech-lang/mech/pull/973), draft at `23d152cbfc5d32e829f36d8000f0d61dd64c026a` |
| Stack base | `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b` |
| R142 integration base | Reviewed `23d152cbfc5d32e829f36d8000f0d61dd64c026a`; separate `codex/native-header-liveness-reuse` branch |
| Retained compiler | `05102440a85c233069b3968f1e6a92c27461258c` |
| IR/work/ABI/bytecode | **168/107/39/14**, unchanged |
| Current increment | Reuse the exact header-liveness set within one immutable preparation; two production files, 15 insertions/8 deletions |

## Work board and decisions

| State | Work and disposition |
|---|---|
| Closed: rejected | R141 short-buffer early refusal: no established elapsed benefit; implementation removed, evidence and useful regression test retained |
| Closed: retained | R142 local header-query reuse: all three startup pairs improve; guards pass with adverse compression timings; separate review increment |
| Closed | R130 recommendations reconciled, capability scope frozen, current documents consolidated, historical evidence archived |
| Next | Review the bounded follow-up and decide integration; no active implementation task |

**Known introduced costs remain explicit:** R140's 8B literal regression is **+1.322924%** in its original matched cohort. R141 removed four allocations and 0.1039% of instructions but elapsed was0.200298% worse, with four improving/four adverse blocks; its hypothesis was rejected. Efficient ordinary fallback grouping needs a separate finite/error-prefix proof. R136 raised cold image validation from roughly 17 s to 35–37 s; R142 only partly reduces it.

R142's predeclared primary outcome is image admission, outside compression timing. Three balanced fresh-application pairs with warm file caches give a pooled median **35.081146958s→34.580817458s (−1.426206%)**; each pair improves. Fresh Object, descriptor, metadata, source and bytecode are byte-identical. The already-owned header set moves within one derivation with the same plan/root/facts/empty-predicate/full-query context. Independent image reconstruction, candidate/exit queries, errors and runtime authority remain intact. No compression speedup is inferred.

Ordered backlog: residual startup analysis; ordinary fallback grouping; already-private stores; bounded-child trips; Huffman/header pricing source work; word-bank/constants/binding/arena changes; further instruction/SIMD work. None is active or enters this increment automatically.

## Current compression measurements

All **20 retained compression inputs/sizes** pass exact raw-reference agreement, independent inflation, aliases, prefixes, output capacity/clear-reuse and zero final managed data. R142 has 3,968 primary observations in eight balanced blocks per case. Compressor source, algorithms, tables, search limits, allocations, allocator and complete `evaluate_into` timing boundary remain pinned. Full multi-megabyte timing and the separate ten-workload suite are excluded.

| Program/input | Current elapsed | Mech / Roc |
|---|---:|---:|
|Periodic 33,027 B|34.759562ms|27.7411×|
|Repeated 32,769 B|23.183229ms|17.3527×|
|Random 32,769 B|7.997001ms|7.5945×|
|Reymont prefix 32,769 B|37.865354ms|8.6648×|
|Reymont prefix 16 KiB|19.617292ms|9.2100×|
|16KiB distribution change|6.360084ms|8.0253×|
|Reymont prefix 1 KiB|1.978855ms|5.9965×|
|8B literals|1.364604ms|4.7547×|

The final current/control changes range **−1.222688% to +2.764377%**; 8B is **+2.270195%**. Seventeen of 20 medians are slower; every adverse result is retained, with no claim of compression improvement or neutrality. All 20 satisfy the predeclared 1.05 guard and equal logical work. Separate CPU cohorts change −0.003181% to +0.137973% in instructions; all 20 heap cohorts have exactly unchanged request counts, requested bytes and incremental peaks. Scalar controls retain their 1.05 gate. Reference medians for 0/1/7 B remain clock-limited; no ratio is invented. Cohorts and historical ratios must not be compounded.

A single independent guard repeat reverses initialization/block order: 8B **+0.726250%**, random 1KiB **+0.957393%**, Reymont 16KiB **+0.665400%**. All three are adverse and inside the unchanged 1.05 guard. Both cohorts remain separate. R142 is retained for review as a startup/runtime tradeoff; no further experiment is active.

R140's earlier matched 16 KiB distribution-change gain remains 187.8584×→7.9902×, instructions 2.479B→79.465M and requests 67,321→5,679, with incremental peak 18,702,352 B. Current exclusive CPU samples are 60.99% generated for periodic 33,027 B and 89.43% for repeated 32,769 B; these samples do not partition retired instructions. Full 20-case tables and all failures are in the [experiment index](../../benchmarks/mech-deflate/experiments.json), R142.

## Reconcile the R130 recommendations

| Recommendation | Disposition |
|---|---|
| Required physical state at native exits | Completed through R132/R136; preserve live frames, aliases, settlement and original errors |
| Historical temporary dependencies; genuine inputs | Completed in R135 and retained exit lowering |
| Full physical word-bank lifetime compaction | Partial: transport/consumers shrink; full admitted lane capacity remains; further compaction deferred |
| Independent exit/representation reconstruction | Completed within documented domains; serialized metadata grants no current authority |
| Address/arithmetic emission | Completed in R137/R138 |
| Broader inputs and native bulk byte writing | Completed in R139/R140; 20-case matrix retained |
| Already-private direct stores; dynamically funded child-trip proofs | Deferred; load/comparison and empty-trip limitations remain |
| Huffman/header source work; constants/binding/arena; further SIMD/instructions | Deferred in the ordered backlog |
| Terminal-error snapshot | R131 rejected; original evidence retained |

## Qualification and integration

Current compiler passes **562 compatible GPU tests** with the same 7 inherited profile exclusions; 19 focused checks overlap that suite. Also pass: 236 component, 8 composition and 1 real-capacity path tests; 137 guarded/listed/executed CI tests; 132 workflow tests; 15 architecture checks; current runtime-only metadata; 12 scalar and 8 frame controls. Warnings remain errors. No expected values, allowances or inventory guards changed. Two introduced harness omissions—an old absolute source route and a misnamed baseline pin—were corrected narrowly; original receipts remain. Neither is an unresolved compiler failure.

One read-only reviewer found no correctness/architecture blocker in R142; the frozen #973 head has a [clean external review](https://github.com/mech-lang/mech/pull/973#issuecomment-6649009367) and no unresolved threads. Review follows Engine semantics/proofs → preparation → shared emission → ownership/accounting → independent restoration. Coupled proof/emitter/runtime/identity changes remain atomic; the local analysis reuse is independently reviewable. No temporary production instrumentation or safely removable duplicate was found; required ordinary and distinct collection fallbacks remain.

Hosted ordinary CI stays cancelled by standing instruction, with review jobs and #929 untouched. **Both increments stay draft pending green CI and clean review.** Local qualification does not imply hosted readiness. Current architecture is [organized by responsibility](native-compiler-architecture.md); raw evidence and historical prose remain archived. This checkpoint does not authorize another optimization campaign.
