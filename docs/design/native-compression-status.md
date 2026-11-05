# Native compression: current status

**R144 is closed, retained for review. No further optimization is active. The earlier capability milestone and published #973/#976/#977 stay frozen.**

| Checkpoint | Value |
|---|---|
| Frozen milestone | [#973](https://github.com/mech-lang/mech/pull/973), draft `23d152cbfc5d32e829f36d8000f0d61dd64c026a` |
| Frozen startup increment | [#976](https://github.com/mech-lang/mech/pull/976), draft `f120b81e01cb868ace9ca7af474dd9081776fa32` |
| Reviewed R144 base | [#977](https://github.com/mech-lang/mech/pull/977), draft `d441153985d861d5fe6a4a838fb4b04e1e7c367d` |
| Retained compiler | `0ae8926bfa5bccb6e36f5cb92bebae20b4c8adfc` |
| IR/work/ABI/bytecode | **171/107/39/14**; rejected IR169/170 remain reserved |
| Bounded change | Shared numeric addressing uses current unit dimensions directly; general matrix formula and bounds remain |

## Work board

| State | Disposition |
|---|---|
| Closed | R130 recommendations reconciled; native capabilities through R140 frozen |
| Closed: rejected | R141 early refusal; R143 private direct stores; tests and contrary evidence retained |
| Closed: retained for review | R142 exact header-query reuse; R144 current vector geometry |
| Next | Review this bounded increment before further semantic or performance changes |
| Deferred | Residual startup, ordinary fallback grouping; source/ownership/allocation/wider-load work stays in backlog |

Known costs stay explicit: R140 introduced **8B+1.322924%**; R141 saved four requests/0.1039% instructions but elapsed worsened0.200298%, so it was rejected. Ordinary fallback grouping needs a separate finite/error-prefix proof. R136 image admission rose from roughly17s to35–37s; R142 reduced its three-pair median35.081146958s→34.580817458s (−1.426206%) with byte-identical generated artifacts. R142 also had17/20 adverse compression medians and an adverse guard repeat; no runtime neutrality is claimed. Residual startup remains open, not freshly measured in R144.

## Current complete-compression measurements

R144 primary **20-case geometric after/before0.987968789 (−1.203121%)**,14 improving/six adverse medians; both predeclared targets improve. All3,968 primary/4,048 total calls preserve exact raw bytes, independent inflate, aliases, prefixes, capacities/clear-reuse and zero final managed data. Algorithms, tables, limits, allocation sites, allocator, source/MBC and complete `evaluate_into` boundary stay pinned. Times below are one matched cohort, not independent replications or compounded historical gains.

| Input | Before→after ms | Mech / Roc before→after | Change |
|---|---:|---:|---:|
|eight-literals|1.733271→1.789771|5.9055×→6.0980×|+3.2597%|
|16384-distribution-change|7.140438→7.077063|8.6656×→8.5887×|-0.8876%|
|33027-periodic-window|36.814313→33.939979|28.9421×→26.6824×|-7.8077%|
|random-32769|10.176167→10.134833|9.3876×→9.3495×|-0.4062%|
|repeated-32769|25.425917→22.416125|18.4781×→16.2908×|-11.8375%|
|reymont-prefix-32769|43.775500→42.925875|9.3100×→9.1293×|-1.9409%|
|reymont-prefix-16384|24.677083→24.314063|10.7525×→10.5944×|-1.4711%|
|random-1024|2.185583→2.175563|6.8729×→6.8414×|-0.4585%|
|repeated-1024|2.187874→2.127187|6.4922×→6.3121×|-2.7738%|
|reymont-prefix-1024|2.320667→2.314917|6.7071×→6.6905×|-0.2478%|
|320-overlap|1.768812→1.765542|5.7429×→5.7323×|-0.1849%|
|random-256|1.656272→1.657021|5.3256×→5.3280×|+0.0453%|
|128-biased-dynamic|2.083208→2.069459|7.0379×→6.9914×|-0.6600%|
|random-64|2.203625→2.261125|7.2968×→7.4872×|+2.6094%|
|32-high-entropy-stored|2.130500→2.134105|7.0546×→7.0666×|+0.1692%|
|eight-overlap|2.338854→2.297104|7.6936×→7.5563×|-1.7851%|
|eight-literals-prefix|2.523333→2.538791|8.3004×→8.3513×|+0.6126%|
|seven-stored-prefix|0.136688→0.139459|clock-limited reference|+2.0269%|
|one-stored|0.143291→0.142209|clock-limited reference|-0.7558%|
|empty|0.136854→0.136270|clock-limited reference|-0.4264%|

One separately declared reversed-order guard repeat retains the primary unchanged: periodic27.7494×→25.7581× (−7.1762%), repeated17.3333×→15.5362× (−10.3684%),16KiB text9.1870×→8.9788× (−2.2656%);8B4.6853×→4.6895× (+0.0883%) and random64B−0.5152%. All five medians pass1.05. The development screen is diagnostic only. Higher primary absolute controls did not recur with the same binaries; no cause is established and no8B neutrality/improvement is claimed.

Separate counters: periodic679.647M→659.476M instructions (−2.9678%); repeated474.164M→454.064M (−4.2392%). Many other counts rise, including8B+0.00146% and16KiB text+0.5326%. All20 heap requests/bytes/deallocations/reallocations/copy upper bounds/incremental peaks and logical work are equal, including raw probe samples. Text7,870,320→7,884,856B (+14,536); stack7,744→7,584B. Static source2864 confirms vector bypass of UDIV; dynamic address visits are unmeasured. Current four-case CPU-time profiles show57.80–88.55% generated-exclusive samples; these are not retired-instruction shares. No allocation or universal runtime improvement is claimed.

## Qualification and integration

Old-emitter semantic-first6pass; candidate29 focused;562 compatible GPU tests/7 inherited exclusions;137 CI tests actually listed/guarded/executed;132 workflow tests;15 architecture checks; runtime-only metadata and formatting pass. Source-derived236 components/8 composition/1 real-capacity path pass unchanged. Address oracle covers1,180 evaluations/all four unsigned widths; current geometry/empty/failure/publication/alias/retry witness covers61cases in each original/decoded form. Scalar12 pass unchanged1.05 gate, **all12 medians adverse, maximum1.017231291**; frame8 pass request/byte/work controls. Warnings stay errors. Core/Engine/math/dependencies are frozen; only GPU/required roots rebuilt.

The first component harness build failed on a stale removed-worktree include, identically reproduced on the clean parent. Its exit1 stays archived. Only the external include route changed;82 support files verify byte-identical to the reviewed parent; affected236/8/1 rerun passes. Earlier formatting/diagnostic preparation mistakes were corrected before timing and retained. No unresolved introduced functional failure. Same7 profile exclusions, fresh startup, all-features/dependency rebuild, multi-MB timing and the separate ten-workload suite remain excluded. Hosted ordinary CI is cancelled under standing instruction; **draft until green CI and clean external review**.

R130 reconciliation remains closed: required physical exits/fresh lexical state/independent reconstruction implemented; full admitted lane-bank compaction partial and deferred; address/arithmetic/bulk writer and all20 matrix implemented; R131 error snapshots and R143 stores rejected; dynamic child funding/source pricing/constants/binding/arena/SIMD deferred. R143 failed its whole-matrix elapsed criterion (+0.369643%,14adverse); its production was restored before R144. No rejected gain enters this baseline.

Exact commands, feature profiles, all observations/failures/adverse blocks, pins and replay reports are in the [experiment index](../../benchmarks/mech-deflate/experiments.json), R144; large evidence stays external. Review follows the [current architecture](native-compiler-architecture.md). Completion of this increment does not imply performance parity.
