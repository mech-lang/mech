# Native compression milestone

**R167 closes the direct-comparison experiment and identifies invariant nested phi state.**
No compiler change is retained. All20 exact streams and unchanged1.05 gates pass,
but the universal XOR-zero rewrite slows repeated32KB5.743583x to5.896412x
and periodic33KB6.529916x to6.695008x. Natural32KB improves4.585363x to4.514501x
(1.545377%), and natural16KB5.284359x to5.184130x (1.896709%); these gains remain
archived. Fourteen medians improve/six worsen; geometric elapsed improves only
0.095938%. Repeated/periodic instructions fall about0.9–1.0% while elapsed rises.
All20 heap metrics stay equal. Code grows51148B; entry stack shrinks144B.

Fresh current-binary profiling still finds guarded word reads, visit/balance work
and stack accesses hot. A byte-identical retained-binary diagnostic maps78 word-
loop header arguments and identifies74 invariant local phi chains anchored by
one dominating outside phi. The four changing fields remain separate. A minimal
JIT reproducer passes6 actual executions while retaining16 redundant child phi
fields through the existing backend. This is a concrete general compiler target;
no saved-copy count or speedup is yet claimed for phi simplification.

The three prototype unit tests,20 measured restored compression cases, separate
all20 instruction/heap cohorts and current profile are archived. Full GPU/scalar/
component/architecture qualification was not repeated for the rejected candidate.
Production and CI are restored byte-for-byte toad32f9a1ff; maintained versions
remain184/107/39/14 (candidate186 is experimental). See the
[R167 report](native-xor-comparison-experiment.md) for every timing, contrary
counter cohort, repaired harness error, exact pins and next acceptance witness.
#973 remains draft; ordinary CI is cancelled by instruction. Parity is not achieved.
Publication:29 public archives/58 server digests verified. Fresh retrieval verifies
1638 file occurrences/311 paths/13 historical versions and reproduces both reports
byte-for-byte. The initially omitted replay entrypoint/output hashes are corrected
in the index; replay succeeds in the required fresh directory. No compiler or
compression execution is involved in replay.

## Historical R166 rejected experiments

**R166 rejects the captured-window prototypes and preserves the reviewed compiler.**
The source and compiler checkpoints below remain retained. Fresh target screens
of three compiler variants all regress on repeated and periodic input. The last
screen is repeated5.737719x to18.922838x reference and periodic6.434306x to20.524145x;
these failed candidates are archived and removed from maintained code. Natural
text also regresses about2%; the contrary small-input observations remain.

Current binary profiles expose extensive native stack transport despite removal
of access checks. An explicit steady matching-word path has246 instructions,
including222 stack loads/stores around2 input loads. A scalar reference pattern
has10 instructions. These are static path counts, not retired counters. Dead-child
SSA resets leave the whole compressor Object identical and are also rejected.
The next diagnostic must identify which block parameters cause these transfers.

All1536 primary/1596 total screen calls have exact streams, independent inflate,
unchanged logical work, real buffers/allocator/timing and zero final managed
storage. Five development cases were measured per variant; a failed target
screen did not advance to full20/scalar/component qualification. Focused original/
decoded/current-limit/handoff/finite-error tests pass. Production and CI inventory
are restored byte-for-byte to49e280b056; retained IR/work/ABI/MBC184/107/39/14.
See the [R166 rejected-experiment report](native-captured-window-experiment.md)
for every timing, source/Object pins, failure dispositions and skipped coverage.
Parity remains unachieved. #973 stays draft; ordinary CI is cancelled by request.
Publication:20 archives/40 server digests verified. Fresh public retrieval verifies
1761 file occurrences/434 paths/25 historical versions and reconstructs both
R166 reports byte-for-byte, without compiling or compressing.

## R165 retained source experiment

**R165 completes the separately measured histogram-reuse source experiment.**
The compiler remains byte-identical to R164 and the pinned source stays the
default. In this fresh matched cohort, the explicit variant improves natural32KB
**4.577225× → 4.445344× Roc** (2.881238%), repeated32KB5.729042× →5.460320×
(4.690520%), periodic33KB6.451039× →6.147931× (4.698585%), and eight bytes
4.719510× →4.690838× (0.607531%). These are source gains, kept separate from the
compiler gains below. All20 gates and exact streams pass:17 faster/3 slower,
1.878198% geometric elapsed improvement; worst stored-only regression1.077931%.

Block planning reuses the all-literal histogram instead of counting the same
input bytes again. The original default-cost entry is unchanged. Search/pass
limits, ties, block decisions, tables, real scratch capacities, EOF rules,
allocator, input aliases, output reservation/clear/reuse and evaluate_into remain
fixed. Source/MBC have their own pins and per-case work values. Neither earlier
zero-trip nor independent-cost-table variants are combined.

The actual source variant retains four vector cost reductions. Fifteen instruction
medians fall/five rise, including repeated and periodic +about0.8% despite faster
elapsed; nineteen cycle medians fall/one rises. Requests fall10 or20 on optimal
paths, requested bytes fall6920–11608B, and incremental heap peaks fall16384–17832B.
Text grows37188B and entry stack shrinks320B. Fresh large-case samples remain
80–83% generated code. No instruction-partition, input-streaming or total-copy
claim is made. See the [R165 source report](../../benchmarks/mech-deflate/source-variants/histogram-reuse.md)
for all20 timings, contrary cohorts, exact source/compiler pins and commands.

Fresh R165 validation:7 focused tests (including shifted slices and original/
decoded failure/retry),236 unchanged components/8 composition/1 real-capacity
path, all20 actual restored linked cases, warnings denied and formatting.
R164's compiler/scalar/architecture receipts apply to the identical library and
are explicitly reused, not rerun. Source revisionc0ea51478d3dcfa66a361fe0c979695b04a1c3c3;
compiler9017ab40d19047f39bf1832c5c85d05f00accb5d; IR/work/ABI/MBC184/107/39/14.
One source optimization commit in #973; draft pending green CI and clean review.
Ordinary CI is cancelled by instruction. Parity is not achieved.

The brief's selected compiler experiment and one separate source experiment are
implemented and measured. Match-extension unroll/wider comparison remains an
unimplemented alternative and the next compiler candidate. Keep the pinned
source baseline for compiler attribution; do not fold this source win into it.

Publication:25 public archives/50 server SHA256 digests verified. Fresh downloads
verify1703 file occurrences/274 paths/10 preserved historical versions and
reproduce results.json and REPORT.md byte-for-byte. Replay round165 with the
maintained experiment.py entry and experiments.json index.

## R164 pinned-source compiler checkpoint

**R164 completes the selected compact scalar/SIMD cost-loop experiment.**
Natural32KB improves **5.242043× → 4.596870× Roc**
(22.839583 → 20.028563 ms, **12.307670%**); natural16KB improves
5.903462× → 5.250235× (**11.065162%**). Repeated32KB is
5.855326× → 5.740210× and periodic33KB is6.588496× → 6.479518×.
Eight bytes rises4.719225× → 4.731433× (+0.258703%). These are fresh
matched cohorts, not replacements for the historical observations below.

All20 exact-output and unchanged1.05 gates pass:17 faster/3 slower medians,
**1.725622% geometric elapsed improvement**. The worst adverse case is
eight-overlap (+1.157951%). All12 scalar controls pass without larger allowances.
Source/MBC/algorithm/tables/search limits/real scratch capacities/allocation
sites/allocator/input aliases/output clear-reuse and evaluate_into stay pinned.
No compressor source change is included.

The compiler uses the existing complete-child proof and current array windows
to recognize a generic ordered U32 minimum with an associated item. It emits
compact scalar costs for short intervals and remainders, with four-lane costs
for intervals of at least16 candidates. Incoming best and earliest improving
ties, U32 wrapping, exact item encoding, original terminal construction,
finite prices/error prefixes, aliases, publication and retry are preserved.
The actual complete-compressor Object contains four `umin.4s` bodies and real
vector loads/shuffles. The first vector-only screen was insufficient on natural
text and is preserved separately; its results are not pooled with this cohort.

Natural32 instructions fall274.605M → 260.132M (−5.270371%), with cycles
−11.395186%. Eleven case instruction medians fall/nine rise; fourteen cycle
medians fall/six rise. All20 heap requests/bytes/reallocations/incremental peaks
are exactly equal. Generated text grows9,248B and entry stack256B. Fresh
profiles sample82.48% generated execution on natural32,83.62% repeated and
80.89% periodic; these CPU-time contexts do not partition retired instructions.
The [full R164 report](native-ordered-reduction.md) retains all contrary results.

Measured compiler9017ab40d19047f39bf1832c5c85d05f00accb5d, parent
209193766599c1b125665b6063e9edf50d8a7997; IR/work/ABI/MBC184/107/39/14.
Final docs/evidence and cfg(test) module ordering are folded into the same
optimization commit in **#973 only**, with production file hashes unchanged.
Passing receipts:615 compatible GPU (same7 exclusions),27 focused,198 CI-selected
and executed,236 unchanged components/8 composition/1 real-capacity path,
132 workflow/15 architecture/runtime-only/format/warnings,12 scalar controls.
Ordinary hosted CI is cancelled by instruction; #973 remains draft pending
green CI and clean exact-head review. Parity is not achieved.

Brief steps1–4 and the selected path-cost experiment/qualification in steps5–6
are complete. The alternative match-extension unroll/wider comparison was not
implemented. Step7 remains: inventory previous source variants and select one
separately pinned, measured source experiment. Compiler and source gains must
remain separate.

Publication:41 public archives/82 server SHA256 digests verified. Fresh retrieval
validates5,785 file occurrences/1,026 paths/21 preserved historical versions and
reproduces results.json and REPORT.md byte-for-byte. Replay with
`python3 benchmarks/mech-deflate/experiment.py replay-round --index benchmarks/mech-deflate/experiments.json --round 164 --output /tmp/r164-replay`.

## Historical R163 checkpoint

**R163 settles exact visits once for proved complete native children.** All20
matched medians improve; geometric elapsed falls1.559354%, and the unchanged1.05
case and scalar gates pass. Natural32KB moves5.350369× to5.255555× reference
(23.274105→22.861667ms); natural16KB6.025953× to5.899914×. Repeated32KB is
5.955420× to5.876018×, periodic33KB6.612460× to6.514746×, and eight bytes
4.710214× to4.642000× (1.356542→1.336896ms). These are fresh matched cohorts.

The parent phase was already fully funded. The new source proof establishes a
complete, total two-tag unit-step child under current immutable windows and
charges its unchanged traversal prices and exact header visits once. Refusal
keeps the original child. Four static copies qualify; actual machine inspection
confirms removal of the repeated debit/counter update. Existing helper, read and
narrowing simplifications remain. Source/MBC, work prices, capacities, algorithm,
allocator, aliases and timing boundary stay pinned. Core/Engine/math are frozen.

Separate counters show16 instruction reductions and4 increases;11 cycle medians
rise. Natural32KB instructions fall1.634155%; eight-byte instructions fall0.027074%
while cycles rise2.418798%. All20 heap request/byte/reallocation/incremental peak
measurements remain equal. Text shrinks60,624B and entry stack grows16B. The
contrary cohorts are retained in the [full R163 report](native-child-visit-settlement.md).

Exact-PC profiles attribute at least13.284% of natural32KB time to path-length
selection and12.675% of repeated32KB to word extension. Native origins remain
unattributed for42.570% and54.883%, respectively. These CPU-time samples do not
partition instructions. Fresh reference disassembly shows an eleven-instruction
scalar cost loop; Mech still has duplicate control, masks and spills. Ordered
reduction/SIMD and one separately pinned source experiment remain unfinished.

Measured compiler19cebb4ca84a611c149ba9ac5d652156cf4042a5; consolidated parent
27081b4000d1f1764c14df32245bd3938984ed3f; retained R162 emitter
3ef813384ab5a1df7a90818ea89506c62ad03375. IR/work/ABI/MBC184/107/39/14.
Final formatting/tests/CI/docs/evidence are folded into the same optimization
commit on **#973 only**, with byte-identical diagnostic Object proof after
formatting. Passing receipts:609 GPU with the same7 exclusions;21 focused;
192 CI-selected and executed;236 components/8 composition/1 real-capacity path;
132 workflow;15 architecture; runtime-only, formatting and warnings denied;
12 scalar controls. Hosted ordinary CI is cancelled by request. #973 stays draft
pending green hosted CI and clean exact-head review. Parity is not achieved.

Publication:38 public archives and76 server SHA256 digests verified. Fresh
public retrieval verifies4548 file occurrences/1056 paths/15 preserved historical
versions and reproduces results.json and REPORT.md byte-for-byte. Replay with
`python3 benchmarks/mech-deflate/experiment.py replay-round --index benchmarks/mech-deflate/experiments.json --round 163 --output /tmp/r163-replay`.

## Historical R162 checkpoint

**R162 retains unsigned word proofs together with the guarded child windows.**
Against qualified R160, natural32KB improves6.223658x to5.341214x reference
(-14.178867%) and natural16KB6.964642x to6.004294x (-13.788912%). Repeated32KB
is6.037883x to5.895288x; periodic33KB6.792514x to6.636364x. Eight cases improve,
twelve worsen; geometric elapsed improves1.165633%, and every unchanged1.05
case/scalar gate passes. Eight bytes worsens4.613971x to4.689157x (+1.629548%);
the contrary observation remains. These fresh matched ratios do not replace
earlier cohorts or explain R161's failed biased128 result.

The existing early native branch cleanup now reuses unsigned SSA word bounds
to remove602 statically proved branches, including all three repeated masked
width checks in the guarded path loop. All20 retired-instruction counts fall;
natural32KB309.933474M to279.098834M (-9.948793%), with cycles-13.313405%.
All20 allocation requests, requested bytes, reallocations and incremental live
heap peaks stay equal. Text shrinks69,480B against R160; entry stack grows400B.
Actual current profiles place84.39% of natural32KB samples in generated code,
versus13.27% for eight bytes. Samples are not instruction partitions.

The [R162 report](native-word-range-cleanup.md) gives all20 timings, independent
counter/heap cohorts, exact source/bytecode pins and repaired build/check
failures. Compiler3ef813384ab5a1df7a90818ea89506c62ad03375; published base #988
d597f5468c83f6547013e0007e4ab3bd380969e0; IR184/work107/ABI39/MBC14. Passing
controls:602 compatible GPU tests (same7 exclusions),22 focused,185 scheduled
and executed,236 components/8 composition/1 real-capacity path,132 workflow,
15 architecture, runtime-only/format/warnings,12 scalar. Source, algorithms,
capacities, allocator and timing boundaries remain unchanged. Whole-body
totality, exact existing-loan settlement, ordered reduction/SIMD and the separate
source variant remain unfinished. Parity is not achieved. Ordinary hosted CI
is cancelled by request; the change remains draft pending green CI and review.

## Historical R160 and standalone R161 checkpoints

**R160 retains current immutable-array read facts in native numeric loops.** The matched comparison against reviewed #986 removes109 static duplicate read guards, including both path-loop loads in each clone. Natural32KB improves6.470792x to6.225117x reference time (-3.796679%); repeated6.422834x to6.109911x (-4.872034%); periodic7.131085x to6.836043x (-4.137400%). Fourteen of20 medians improve, six worsen (worst+0.949648%); geometric elapsed improves0.935680%, with all unchanged1.05 gates passing. Eight bytes is4.854203x to4.842748x (-0.235988% in this matched cohort). Every case's retired instructions fall, with natural32KB -4.199070%. Heap counts/bytes/reallocations/incremental live peaks stay equal. Generated text shrinks85,520 bytes and entry stack64 bytes.

The [current report](native-array-read-facts.md) contains exact compiler/source/bytecode pins, all20 times and independent counter cohorts,595 GPU/174 CI-selected tests, original/decoded/error/finite controls and corrected validation paths. The current compiler is4fe3888f4d98d1ea159cd3186a8bc0f3b81ef773, IR182/work107/ABI39/MBC14; base #986 is8ef1693a558b9a46544ce425ca2f222e10e3dddc. The [R159 report](native-u32-case-facts.md) remains historical evidence. Whole-loop entry windows, masked narrowing/totality, exact funding settlement, ordered reduction/SIMD and the separate source variant remain unfinished. Parity is not achieved; earlier published heads remain frozen.

R161's complete-child window prototype is **not retained standalone**: natural32KB improves6.218368x to5.975507x reference (-3.905553%), but128-biased regresses7.169355% and fails the unchanged1.05 gate. Five of20 medians improve,15 worsen, geometric elapsed+0.725321%. Eight bytes regresses2.186978%. The guarded loop emits correctly in all four path copies and natural32KB instructions fall2.711783%; all20 correctness checks and unchanged heap counts pass. See [prototype, failure and next proof target](native-loop-read-windows.md). Keep R160/#987 as the retained checkpoint while a combined candidate addresses remaining masked narrowing in the existing CFG cleanup. Source, algorithms, capacities and timing boundaries remain pinned.

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
