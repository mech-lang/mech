# Native compression: current status

**The native capability milestone through R140 is closed for review. R142 is a separate startup follow-up. R143 is closed: its private-store experiment was rejected, production restored exactly, and useful tests retained.**

| Current checkpoint | Value |
|---|---|
| Frozen milestone | [#973](https://github.com/mech-lang/mech/pull/973), draft at `23d152cbfc5d32e829f36d8000f0d61dd64c026a` |
| Reviewed follow-up / R143 base | [#976](https://github.com/mech-lang/mech/pull/976), draft at `f120b81e01cb868ace9ca7af474dd9081776fa32` |
| Retained compiler | `05102440a85c233069b3968f1e6a92c27461258c` |
| IR/work/ABI/bytecode | **168/107/39/14**, unchanged |
| Current increment | Two test files plus evidence/status; no production change from the R143 base |

## Work board and decisions

| State | Work and disposition |
|---|---|
| Closed: rejected | R141 early short-buffer refusal; useful regression test and evidence retained |
| Closed: retained for review | R142 exact header-query reuse; startup gain with adverse compression timings |
| Closed: rejected | R143 current-private direct stores; 20-case primary criterion failed, implementation removed |
| Closed | R130 recommendations reconciled; capability scope frozen; history archived |
| Next | Review/integrate the bounded increments; no active implementation task |

Known introduced costs remain explicit: R140’s 8B literals regressed **+1.322924%** in its original cohort. R141 saved four requests and0.1039% instructions but elapsed worsened0.200298%; it was rejected. Efficient ordinary fallback grouping needs a separate finite/error-prefix proof. R136 increased image admission from roughly17s to35–37s; R142 reduces its three-pair median35.081146958s→34.580817458s (−1.426206%), with byte-identical generated artifacts. Residual startup stays open.

R142 is a startup/runtime tradeoff:17/20 primary compression medians were adverse (8B+2.270195%). Its independent reverse-order guard repeat was also adverse at8B/1KiB/16KiB, within the unchanged1.05 guard. No compression improvement or neutrality is claimed. Its production and the frozen #973 head remain unchanged.

## Current retained compression measurements

All **20 retained compression inputs/sizes** pass exact bytes, independent inflate, aliases, prefixes, capacity/clear-reuse and zero final managed data. Below is the **retained control** from the fresh R143 matched cohort, not the rejected candidate. Source, algorithms, tables, search limits, allocations, allocator and complete `evaluate_into` timing stay pinned. Full multi-megabyte timing and the separate ten-workload suite stay excluded.

| Program/input | Retained elapsed | Mech / Roc |
|---|---:|---:|
|Periodic33,027B|34.828229ms|27.8849×|
|Repeated32,769B|23.351979ms|17.4074×|
|Random32,769B|7.995979ms|7.5648×|
|Reymont prefix32,769B|37.877813ms|8.7095×|
|Reymont prefix16KiB|19.571063ms|9.1840×|
|16KiB distribution change|6.322417ms|7.9929×|
|Reymont prefix1KiB|1.941604ms|5.7444×|
|8B literals|1.338126ms|4.6142×|

R143 tested private admission before an indexed old-cell load/comparison. The four-case screen suggested about0.4% target gains; final qualification failed: **geometric mean+0.369643%,14/20 slower**, including8B **4.6142×→4.6769× (+1.359140%)**. Repeated17.4074×→17.2733× and periodic27.8849×→27.8764× improve individually, but the predeclared whole-matrix criterion also required a geometric mean below1. The change was rejected without retuning or a repeat. All3,968 primary/4,048 total observations and adverse blocks remain.

Separate R143 counters are adverse on the targets: periodic679,667,220→690,025,266.5 instructions (+1.523988%); repeated474,163,319.5→484,442,172.5 (+2.167787%). All20 heap request/byte/reallocation/incremental-peak measurements are unchanged. Four candidate CPU-time profiles retain generated-exclusive shares60.09–89.59%; these do not partition retired instructions. Code+2,516B/629 static instructions and stack+112B are also rejected. No candidate number becomes the retained baseline. Reference0/1/7B medians are clock-limited; no ratio is invented or historical ratio compounded.

R140 retained its earlier matched distribution-change gain187.8584×→7.9902×, instructions2.479B→79.465M and requests67,321→5,679, with incremental peak18,702,352B. Exact all 20 results, failures, source pins and replay data remain in the [experiment index](../../benchmarks/mech-deflate/experiments.json), R141–R143.

## Reconcile the R130 recommendations

| Recommendation | Disposition |
|---|---|
| Required physical exit state; lexical freshness; genuine inputs | Completed through R132/R135/R136; preserve current frames, aliases, settlement and errors |
| Full physical word-bank compaction | Partial: transport/consumers shrink; full admitted lane capacity remains; further work deferred |
| Independent exit/representation reconstruction | Completed within documented domains; metadata grants no current authority |
| Address/arithmetic emission; broader inputs; bulk byte writing | Completed R137/R138/R139/R140;20-case matrix retained |
| Already-private stores | R143 rejected under the original elapsed criterion |
| Dynamically funded child trips; source pricing; constants/binding/arena; SIMD | Deferred; none enters this increment automatically |
| Terminal-error snapshot | R131 rejected; original evidence retained |

## Qualification and integration

The restored compiler passes **562 compatible GPU tests**, with the same 7 inherited profile exclusions; 137 guarded/listed/executed CI tests; 132 workflow tests; 15 architecture checks; current runtime-only metadata and configured formatting. The extended existing first-change test keeps original U32 coverage and adds U8/U16/U32/U64 equal canonical/private visits, two arrays, aliases, refusal, unchanged publication, cleanup and retry in original/decoded forms. Current IR168 is accepted; unsupported169/170 explicitly rejected. Warnings remain errors; no expected value, allowance or inventory changes.

The rejected candidate passed22 focused checks plus identity rejection before timing. Broad candidate component/scalar/frame qualification was intentionally skipped after primary failure. The unchanged retained production already has R142 qualification:236 components,8 composition,1 real-capacity path,12 scalar and8 frame controls. R143 does not claim those were rerun. Corrected fixture, identity and diagnostic include/selector failures remain archived; no unresolved introduced compiler failure. One read-only reviewer found no blocker in the final test-only diff.

Ordered follow-ups remain residual startup analysis and ordinary fallback grouping; further source, ownership and lowering campaigns stay deferred. Review follows the [current architecture](native-compiler-architecture.md). Historical evidence stays outside the source tree. Hosted ordinary CI remains cancelled, review jobs/#929 untouched. **All increments stay draft pending green CI and clean review; local qualification is distinct from merge readiness.**
