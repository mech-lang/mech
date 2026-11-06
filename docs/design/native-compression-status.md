# Native compression: current status

**R145 is qualified and retained for a separate draft review. No implementation task is active. Published #973/#976/#977/#978 remain frozen.**

| Checkpoint | Value |
|---|---|
| Reviewed base | `65c8f37c58a19d1e39eed833395b98ab87d7bf65`, [#978](https://github.com/mech-lang/mech/pull/978), clean external review |
| Before / retained compiler | `0ae8926bfa5bccb6e36f5cb92bebae20b4c8adfc` / `c03bdfc7ffca8ffdf4f2207729536d8b4a5d8dbd` |
| IR/work/ABI/bytecode | **172/107/39/14**; rejected IR169/170 remain reserved |
| Bounded change | Confined native byte packs use a guarded current-window word load; original source fallback remains |
| Frozen chain | [#973](https://github.com/mech-lang/mech/pull/973) → [#976](https://github.com/mech-lang/mech/pull/976) → [#977](https://github.com/mech-lang/mech/pull/977) → [#978](https://github.com/mech-lang/mech/pull/978) |

## Work board

| State | Disposition |
|---|---|
| Closed | R130 recommendations reconciled; native capabilities through R140 frozen |
| Closed: rejected | R141 early refusal; R143 private stores; tests and contrary evidence retained |
| Closed: retained for review | R142 exact header-query reuse; R144 current vector geometry; R145 confined byte packs |
| Active | None; R145 awaits external review as a separate draft |
| Next | Review/integrate the bounded compiler increment; disposition residual startup before another finite task |
| Deferred | Ordinary fallback grouping, source/ownership/allocation/SIMD/further lowering work stays in backlog |

Known costs remain explicit: R140 introduced **8B+1.322924%**; R141 saved four requests/0.1039% instructions but worsened elapsed0.200298%, so was rejected. R136 image admission rose roughly17s→35–37s; R142 reduced its three-pair median35.081146958s→34.580817458s (−1.426206%) with identical generated artifacts, but had17/20 adverse compression medians and an adverse guard repeat. Residual startup remains open, not freshly measured in R145.

## Current complete-compression measurements

R145 primary **20-case geometric after/before0.922481482122 (-7.751852%)**,17 improving/three adverse medians. Both target medians and every unchanged1.05 guard pass. All3,968 primary/4,048 total calls preserve exact raw bytes, independent inflate, aliases, prefixes, capacities/clear-reuse and zero final managed data. Algorithms, tables, search limits, EOF, allocation sites, scratch sizes, allocator, source/MBC and complete `evaluate_into` boundary stay pinned. This is one matched cohort; blocks are descriptive, not independent replications or compounded historical gains.

| Input | Before→after ms | Mech / Roc before→after | Change |
|---|---:|---:|---:|
|eight-literals|1.349479→1.339146|4.7185×→4.6823×|-0.7657%|
|16384-distribution-change|6.359646→6.231438|8.0096×→7.8482×|-2.0160%|
|33027-periodic-window|32.313667→21.264312|25.8509×→17.0114×|-34.1941%|
|random-32769|8.032104→7.870563|7.6278×→7.4744×|-2.0112%|
|repeated-32769|20.704542→9.523687|15.4742×→7.1179×|-54.0019%|
|reymont-prefix-32769|37.029084→35.269604|8.5027×→8.0986×|-4.7516%|
|reymont-prefix-16384|19.108875→18.403667|8.9524×→8.6220×|-3.6905%|
|random-1024|1.282521→1.337042|4.2050×→4.3837×|+4.2511%|
|repeated-1024|1.869834→1.543292|5.7357×→4.7340×|-17.4637%|
|reymont-prefix-1024|1.955437→1.906792|5.9436×→5.7957×|-2.4877%|
|320-overlap|1.419709→1.347521|4.8289×→4.5834×|-5.0847%|
|random-256|1.325520→1.312750|4.3318×→4.2900×|-0.9635%|
|128-biased-dynamic|1.147146→1.132562|3.9694×→3.9189×|-1.2713%|
|random-64|1.260563→1.255958|4.3618×→4.3459×|-0.3653%|
|32-high-entropy-stored|1.237896→1.229812|4.3588×→4.3303×|-0.6530%|
|eight-overlap|1.317187→1.303001|4.5109×→4.4623×|-1.0771%|
|eight-literals-prefix|1.356791→1.340583|4.7943×→4.7370×|-1.1946%|
|seven-stored-prefix|0.120292→0.120083|clock-limited reference|-0.1737%|
|one-stored|0.120125→0.120209|clock-limited reference|+0.0699%|
|empty|0.116604→0.116625|clock-limited reference|+0.0176%|

The four-case development screen agrees on both targets but is separate from qualification. No guard repeat was needed. Random1KiB+4.251080%, one-byte+0.069927% and empty+0.017581% remain adverse; no universal improvement is claimed. Absolute times differ from the previous round; no cause is established and only matched ratios support this change.

Separate counters: periodic659.473916M→380.141994M instructions (-42.356781%); repeated454.072956M→176.080872M (-61.221898%). All20 heap requests/bytes/deallocations/reallocations/copy upper bounds/incremental peaks and logical work match, including all raw probe samples. Text7,884,856→7,888,192B (+3,336); stack7,584→7,504B. The Object selects27static packs; dynamic visits are unmeasured. Current four-case profiles show33.29–83.18% generated-exclusive CPU-time weights; these are not retired-instruction shares. No allocation or memory reduction is claimed.

## Qualification and integration

Semantic-first parent2pass; candidate3focused tests; all304 work/error-origin rows match before/after across original/decoded/direct/restored forms. Covers2/4/8-byte packs, nine negative controls, current geometry/windows, overflow/error order, finite budgets, aliases/publication/cleanup/retry. CompatibleGPU565pass/7same exclusions; CI140listed/guarded/executed;132workflow tests;15architecture checks; runtime-only and changed-file formatting pass. Source-derived236components/8composition/1real-capacity path pass unchanged;82supportfiles match the reviewed parent. Scalar12pass unchanged1.05 gate (two adverse, maximum1.000695226446); frame8pass request/byte/work controls. Warnings stay errors. Core/Engine/math/dependencies frozen; onlyGPU/required roots rebuilt.

Preparation-only test inference/selection/transcript and inherited unused-symlink staging errors were corrected before final qualification; diagnostic assembly/Git-metadata receipt corrections are archived. The reviewer-requested length-predicate excerpts are complete. No unresolved introduced functional failure. Exact commands/configurations/exits and all contrary blocks are in the archive. Same7profile exclusions, full dependency/all-features graph, startup remeasurement, multi-MB timing and the separate ten-workload suite remain skipped. Ordinary hosted CI is cancelled under standing instruction; **draft until green hosted CI and clean external review**.

R130 reconciliation remains closed: required exits/fresh lexical state/independent reconstruction, address/arithmetic/bulk writing and all20matrix implemented; full lane-bank compaction partial/deferred. R131 error snapshots and R143 stores rejected. Further bounded-child trip proofs in dynamically funded phases, source pricing/constants/binding/arena/SIMD remain separate tasks; no rejected experiment enters this baseline.

Exact observations, failures, pins, guard excerpts and replay reports are in the [experiment index](../../benchmarks/mech-deflate/experiments.json), R145; large evidence stays external. Review follows the [current architecture](native-compiler-architecture.md). This finite increment is complete for review; performance parity remains a longer-term objective.
