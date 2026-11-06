# Native compression milestone

**R157 retains shared current-traversal projection facts for numeric lowering.** The direct 20-case comparison against reviewed R154 passes unchanged gates: natural32KB improves7.719026x to6.733391x reference time; the all-case geometric mean improves1.789001%. Allocation counts and incremental live heap remain equal. Periodic/repeated instructions increase1.91%/1.99%, an unresolved performance tradeoff. See the [current report](native-numeric-traversal-facts.md) for exact pins, contrary observations,589 GPU/168 CI tests and remaining loop work. Parity is not achieved; earlier published heads remain frozen.

The R154 checkpoint below remains as historical context for the reviewed base.

| Checkpoint | Revision |
|---|---|
| Published base | #983 at `5b77cb413648ac5fb245d64c62eaf7cb489cbf03` |
| Before production compiler | `2d27e4cc44261a02c6858a232960d8999ac712eb` |
| Actual retained before executable emitter | `2b6953e9a8532bae417588d607a553f9c8d86a6d` |
| Current production compiler | `58690b58dfc4ccdf44b074c15d3c140a28603866` |
| Final focused tests | `9f750d5ed41553840884b24f96a588a48a217153` |
| IR / work / ABI / bytecode | **178 / 107 / 39 / 14**; old IR177 rejected |

The selected operation is the already-native word-read comparison in match extension. Fresh exact-PC diagnosis on the retained R153 binary found the same hot windows for periodic and repeated inputs. The source already checked `available >= width`, executed checked `available - width`, and selected the `at <= available - width` arm. Native packed-read emission then recomputed the same window condition. This patch reuses the freshly executed source fact and removes that duplicate emitter check. Original source arithmetic, comparisons, errors, visits and funding remain; the current row/column-vector test and complete scalar fallback remain. No compressor source changes or new host-to-native coverage are included.

The private emission fact requires a positive first offset, the exact same current array view, one fresh checked subtraction of the exact pack end, and a fresh unsigned comparison whose true arm alone reaches the pack. The whole slice has one predecessor at each step, stays in the same selected iteration, and crosses no array/frame/helper, source loop, funding or cold-exit boundary. Joins, different arrays, short windows, wrapping subtraction and false arms decline. This is generic source/CFG recognition, without function names, benchmark names or site IDs. Independent image reconstruction derives it again; no saved permission is serialized.

The actual ARM64 Objects and both linked executable windows agree. Following the retained subtraction error identifies source origin **2823**, followed by pack **2829**. The source window is `0x6a9f4..0x6aa88` before and `0x67e5c..0x67ed4` after. Its successful vector path has **34→27 static instructions**, removing duplicate enough/fits booleans and their combination. The source arithmetic error, source selection, two geometry comparisons and word load survive. The earlier selection note uses pack1488 as another source example, rather than the exact hot copy. **45 static guarded pack emissions in four existing regions are not dynamic visit counts.** Whole-call counters below are measured independently.

The final matched 20-case cohort has **3,968 primary / 4,048 total complete calls**, exact reference bytes, independent inflate, input aliases, prefixes, capacities and zero final managed storage. Elapsed improves **0.442337% geometrically**: **13 faster / 7 slower** medians, worst **+0.879027%**, all unchanged **1.05** gates pass. Source plans, complete numeric recipes, source prices and logical work are **exactly equal across all 20 cases**. The separate development screen retains its eight-byte +0.771339% observation; it is not pooled with the final matrix.

| Input | Current ms | Mech / Roc before→after | Change |
|---|---:|---:|---:|
|eight-literals|1.343917|4.618822×→4.634197×|+0.332858%|
|16384-distribution-change|6.210417|7.827752×→7.811845×|-0.203212%|
|33027-periodic-window|8.754792|7.134098×→7.015057×|-1.668613%|
|random-32769|6.744604|6.448414×→6.435691×|-0.197301%|
|repeated-32769|8.325416|6.357938×→6.226938×|-2.060411%|
|reymont-prefix-32769|33.594437|7.719281×→7.708682×|-0.137298%|
|reymont-prefix-16384|18.292958|8.539848×→8.538137×|-0.020042%|
|random-1024|1.245229|4.107684×→4.123275×|+0.379558%|
|repeated-1024|1.517312|4.673782×→4.625951×|-1.023385%|
|reymont-prefix-1024|1.909958|5.782739×→5.805343×|+0.390902%|
|320-overlap|1.323917|4.495629×→4.480261×|-0.341862%|
|random-256|1.303313|4.432223×→4.403084×|-0.657424%|
|128-biased-dynamic|1.137125|4.010161×→3.989914×|-0.504902%|
|random-64|1.246833|4.458993×→4.390257×|-1.541512%|
|32-high-entropy-stored|1.225104|4.308698×→4.298611×|-0.234125%|
|eight-overlap|1.304166|4.458835×→4.481670×|+0.512130%|
|eight-literals-prefix|1.370041|4.955791×→4.773664×|-3.675038%|
|seven-stored-prefix|0.120875|clock-limited reference|+0.659125%|
|one-stored|0.120437|clock-limited reference|+0.382150%|
|empty|0.117230|clock-limited reference|+0.879027%|

Hardware counters and heap probes are separate complete-call cohorts for all 20 cases. Periodic instructions **167,090,877.5→152,459,187 (−8.756726%)** and cycles **28,549,642→28,105,331 (−1.556275%)** fall. Repeated instructions **160,750,824→146,208,298 (−9.046626%)** and cycles **27,107,437→26,756,702 (−1.293870%)** fall. Eight-byte instructions fall 0.028534% while its primary elapsed median rises 0.332858%. The much smaller timing gains are retained alongside the instruction reductions; static instructions, visits, samples and retired instructions remain separate quantities.

All 20 cases have identical allocation requests, requested bytes, reallocations, relocation-copy upper bounds and usable-heap measurements on both sides. Periodic remains **3,679 requests / 15,798,298 requested B / 15,314,928 incremental peak usable B**. Eight-byte remains **3,354 requests / 17,643,969 requested B / 15,279,752 incremental peak usable B**. The separate OS cohort observes 524,288 B less lifetime physical footprint and roughly 770–803 KB less resident storage; these are not claimed as copied-byte or live-heap reductions. The heap-instrumented cycle medians for periodic and repeated rise, while the separate uninstrumented CPU-counter cycles fall; no mixing of these cohorts is used to establish elapsed gains.

| Fresh current CPU-time samples | Generated code | Host callbacks/cleanup | Outside |
|---|---:|---:|---:|
|16384-distribution-change|58.05%|39.51%|2.43%|
|reymont-prefix-16384|83.70%|14.98%|1.33%|
|repeated-32769|85.29%|12.57%|2.14%|
|33027-periodic-window|81.15%|16.26%|2.59%|

Samples describe time contexts, not source operation coverage or retired-instruction shares. The earlier exact-PC diagnosis uses R153 Object `4f56f4bb…`, whereas these after profiles use current Object `2cfd693c…`. The `pc-analysis` data archive was anchored after the compiler snapshot advanced; its binary maps and records still explicitly identify the before binary. No new coverage or causal host-share improvement is claimed from this instruction-only patch.

The code tradeoff is **9,118,504→9,045,788 B generated text (−72,716)** and **15,872→15,984 B entry stack (+112)**. Cold restoration, outside `evaluate_into`, is **37.388→37.528 s** in the development screen and **37.379→37.402 s** in the final cohort. These are two descriptive setup pairs, not a repeated matched startup experiment or a whole-command speedup. Fresh Object generation took 130.585 s; the earlier R153 build took 143.475 s in a different build cohort, so that difference is not attributed as a compile-time gain.

Qualification: **585 GPU pass / same 7 exclusions**, six focused tests, **160 CI tests listed/equality-guarded/executed**, 132 workflow checks, 15 architecture checks, runtime-only and formatting pass. Unchanged **236 component / 8 composition / 1 real-capacity path** controls pass. **12 scalar** controls pass the fixed 1.05 gate, with all 12 adverse medians (maximum 1.008775 after/before), no allocation increase and no 0→64 iteration allocation slope. **8 frame** controls pass with no request/byte/work increase. Nightly 2026-03-03/aarch64, warnings denied, library opt3 and unit/component roots opt1; Core/Engine/math/dependencies frozen and rehashed. CI inventory 155→160 is separately committed.

The new source controls cover widths2/4/8, both literal orders, row/column/rectangular geometry, short and near-EOF inputs, empty dimensions, zero iterations at U64MAX, repeated current visits, original/decoded and direct/restored execution. Failure controls compare ordinary and native origins/errors for retained source underflow, tail overflow and an arithmetic failure after a successful packed prefix; prior published output, aliases, cleanup and retry are preserved. Six finite resource dimensions use the existing boundary helper; no claim is made that all six zero limits deny this zero-comparison witness. Selection mutations refuse missing members, both cold endpoints and five funding frontiers. The clean parent passes source semantics but fails the new native coverage assertion (0 versus1), as expected.

No unresolved introduced functional failure remains. The same seven GPU exclusions, full dependency/all-features rebuild, multi-MB timing and separate ten-workload suite remain skipped. Ordinary hosted CI is cancelled under standing instruction; **draft until green hosted CI and clean external review**. R153 was externally reviewed at its frozen head with no findings before this work.

The next brief starts with a focused current-binary cost check for `OptimalPathLengths` versus match extension. Select one materially hot bounded loop with a plausible route to roughly a 10% important-large-case improvement; then separately measure loop-wide window/arithmetic simplification, a total-body funding envelope, and any unroll/SIMD experiment. Preserve current array versions, original finite-error prefixes and incremental fallback. Source variants stay separate. This next semantic change begins only after this patch is reviewed.

R152 remains rejected/reverted after its eight-byte +5.821563% failure of the unchanged gate; IR176 stays reserved. R150 remains rejected for byte-identical generated code. Earlier retained/rejected results and immutable contrary observations remain in the [experiment index](../../benchmarks/mech-deflate/experiments.json). Raw evidence is external; this status file is the current bounded report.

Publication: **49 public archives / 98 server digests verified**. A fresh public download verifies **4,664 file occurrences / 1,033 paths**, retains **19 paths with historical versions**, and reproduces `results.json` and `REPORT.md` byte-for-byte. Replay with `python3 benchmarks/mech-deflate/experiment.py replay-round --index benchmarks/mech-deflate/experiments.json --round 154 --output /tmp/r154-replay`; this replays numerical receipts without compiling or compressing.

R155 completes the new brief's focused selection check: the current path-length loop has **23.347639%** of natural32KB source-attributed CPU weights, versus about3% on periodic/repeated inputs. The existing packed-loop edge-fact engine and shared aggregate guard policy have not reached its numeric projection path. Its funding is already prepaid, though visit/debit/state bookkeeping remains. No new compiler change or primary timing result is claimed; see the [bounded-loop study](native-loop-cost-study.md) for exact current addresses, attribution gaps and the next acceptance witness.
