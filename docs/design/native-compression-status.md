# Native compression milestone

**R178 fixes exhausted payload transport in native reductions.** A valid source
transition reset the final minimum to zero; optimized execution bypassed that
update and returned 9. The maintained regression fails on the clean R177 parent.
The compiler now requires identity transport of every bypassed payload on every
exhausted path. Other cases use the existing checked native source loop. Original
and decoded artifacts now return the resident result, including changed/conditional
payloads, fresh invocations, finite failures, unchanged publication and retry.
Completion, totality, funding and ownership remain separate proofs.

The compressor's actual entry and entire text section are byte-identical to R177.
This is a correctness repair, with **no speedup claim**. All 20 unchanged 1.05
latency gates pass: nine medians faster, eleven slower, geometric change −0.105306%.
These matched ratios retain the adverse observations:

| Input | Before / Roc | After / Roc | Elapsed change |
|---|---:|---:|---:|
| Natural text, 32 KB | 4.174178× | 4.193258× | +0.457096% |
| Natural text, 16 KB | 4.941493× | 4.959524× | +0.364893% |
| Random, 32 KB | 5.370558× | 5.389583× | +0.354246% |
| Distribution change, 16 KB | 6.974300× | 7.009917× | +0.510686% |
| Repeated, 32 KB | 3.169128× | 3.199825× | +0.968629% |
| Periodic, 33 KB | 3.710310× | 3.712902× | +0.069851% |
| Eight literals | 4.557791× | 4.582620× | +0.544754% |

All 20 allocation and usable-heap medians are equal. Eight instruction medians
and eight cycle medians rise; eight-byte cycles rise 6.220448%. The initial
~27.39% eight-byte RSS increase is retained. Fresh four-session ABBA diagnostics
show +0.017729% RSS and −0.007032% physical footprint. No memory saving or cause
for the initial discrepancy is claimed. Code remains 8,955,224 B; stack 15,792 B.

Qualification: 17 focused / 635 GPU tests pass, with the same seven inherited
exclusions; all 218 selected CI tests execute. Fresh 236 components, eight
composition and one real-capacity path tests, 12 scalar controls, 15 architecture
checks, 132 workflow tests, runtime-only, formatting and warnings denied pass.
No allowance or expected value changed. The introduced test-syntax fault was
repaired and qualification rerun. Disk exhaustion interrupted the initial profile
export; successful timing/counter/scalar cohorts were preserved. Superseded build
outputs were removed, then only profiling was rerun. Five fresh profiles cover
16,705 complete calls with current byte-identical maps; unknown native attribution
remains 36.35% on natural32. Samples are not retired-instruction partitions.

Parent `369714775615e4adb85d67467b9e7409a69f92b4`; measured compiler
`09c57a46a33fd7d5420a59f60b71e3abf373a3b7`; corrected test source
`25a8fddfa354e2c36300476d9f36356ad219ebaa`. Production files are byte-identical
across the test-only correction. IR/work/ABI/MBC **198/107/39/14**, frozen R145
dependencies. Source/MBC/algorithm, real buffers, allocator and full API boundary
remain pinned. Largest input 33,027 B. See the [R178 report](native-exhausted-payload-transport.md)
for all 20 results, exact commands, negative-parent evidence, failures and skipped
coverage. The first public replay exposed a missing YAML source snapshot; the
packaging whitelist is corrected and an extra closure packet supplies it without
changing qualified sources or repeating timing. Review finding 4489282114 is addressed by this patch; resolution follows
publication of the fix and evidence. Ordinary hosted CI is cancelled by standing
instruction; #973 stays draft pending green CI and clean review. Parity remains
active. The mutable-parent fact project remains unfinished: next inspect current
hot parent/child guard and state paths before extending the effect analysis.

Publication verifies **40 public archives / 80 server digests**. Fresh public
retrieval verifies **6,067 file occurrences / 1,140 paths / 15 historical versions**
and reconstructs results.json and REPORT.md byte-for-byte, without compiling or
compressing. The initial missing-YAML failure remains in the supplemental packet.

## Historical R177 checkpoint

**R177 retains current affine read-address geometry.** All 20 complete-call
elapsed medians improve, with 1.255563% geometric improvement and unchanged
1.05 gates. Natural32KB improves **4.242162× → 4.177357× Roc** (1.527657%),
natural16KB **5.015910× → 4.955960×** (1.195194%), random32KB
**5.428153× → 5.375515×** (0.969728%), and distribution16KB
**7.034618× → 6.933858×** (1.432342%). Eight literals improve
4.630497× → 4.619792× (0.231182%). Periodic33KB still has four improving/four
adverse paired blocks. No contrary observation is discarded or gain compounded
with another cohort or the optional R172 source variant.

The emitter derives current forward/reverse bases from the existing admitted
immutable windows, then adds/subtracts the current scaled induction. The inspected
continuing scalar visit falls **19 → 16 instructions**, with its length stack
reload **one → zero**. Four setup instructions run once; at N=1 this address
segment is one instruction worse. The vector body falls 26 → 25 instructions,
while its length reload remains. Reference scalar counterpart: 11 instructions.
There is no dynamic activation census or whole-call saving inferred from samples.
Code shrinks 512 B to 8,955,224 B; stack stays 15,792 B. The source algorithm,
reads, threshold, ties, current proof, prices, fallbacks and allocation sites stay
unchanged. No runtime ABI, ownership or source change is introduced.

Natural32 instructions fall 1.289263%, natural16 1.235343%; 19 of 20 instruction
medians fall. Ten cycle medians rise, including random32 and repeated32. All 20
allocation/requested-byte/reallocation/usable-live-heap metrics stay equal. Initial
CPU/heap cohorts have higher absolute RSS. A separate fresh-session ABBA check
(40 full eight-byte calls, exact output/cleanup) does not reproduce the increase:
RSS −0.017717%, physical footprint −0.070080%. Initial readings and memory maps
remain available; their cause is not proved. No memory improvement is claimed.

Five fresh exact-worker profiles cover 16,803 complete calls. Independently
byte-identical compilation-only maps distinguish reduction phases. Natural32 has
83.23% inside the main generated entry, with 37.27% unknown native attribution;
eight bytes has 11.13% inside. Outside-entry includes other generated helpers and
validation. These are CPU-time samples, separate from retired instructions.

Corrected qualification passes 14 focused/632 GPU tests with the same seven
inherited exclusions; all 215 selected CI names execute. The independent oracle
now covers all U32/U64 forward/reverse width pairs, offsets 1/7, fresh current
lengths/values, thresholds/tails, original/decoded, direct/restored and aliases.
Fresh 236-component/eight-composition/one-real-capacity-path qualification,
12 scalar controls, all 15 architecture/catalog checks, 132 workflow tests,
runtime-only, formatting and warnings denied pass, with unchanged expectations
and allowances. Some scalar medians worsen, up to 0.646576%, within the existing
gate. The introduced test-module layout violation and provenance-attribute
assertion are repaired and retained with their original failed receipts.

**An inherited correctness finding remains open.** Review comment 4489282114
identifies an exhausted active-to-terminal payload update that the reduction
bypasses. The generic reproducer returns native (9,1,18) versus resident (0,1,18)
on both retained R176 and this candidate, for original and decoded artifacts.
All 20 pinned compression cases still match the reference. The finding blocks
readiness; the next task is its bounded proof/transport fix with this reproducer,
before another performance experiment. This address patch does not repair it.

Parent `0bca6a54cdc2d69df71ca2763a6f36993448fe02`; measured compiler
`379e7e140453265675df9230f4e3fedcc18290fa`; corrected test source
`5276932c5eb76dadf95e1735acfdbb1d90dd9ebf`. Production files are byte-identical
across the test-only layout correction. IR/work/ABI/MBC 197/107/39/14; frozen R145
dependencies, source/MBC/real buffers/allocator/full API boundary pinned. Largest
input 33,027 B. See the [complete R177 report](native-affine-read-addresses.md)
for all results, exact commands/pins, failures, profiles, memory confirmation and
skipped coverage. The broader mutable-parent fact project remains unfinished.
One optimization commit in existing draft #973; ordinary CI cancelled by standing
instruction and review preserved. Parity remains active.

Publication verifies 39 public archives / 78 server digests. Fresh retrieval verifies
5,124 file occurrences / 1,121 paths / 19 historical versions and reconstructs
results.json and REPORT.md byte-for-byte without compiling or compressing.

## Historical R176 checkpoint

**R176 retains simpler scalar reduction entry and exit control.** Natural32KB
improves **4.301808× → 4.220265× Roc** (1.895541%, all eight blocks improve),
natural16KB **5.105361× → 5.011239×** (1.843595%), and repeated32KB
**3.268479× → 3.199856×** (2.099534%). Across all 20 cases, 15 medians improve
and five worsen; geometric elapsed improves 0.588763%. Random32KB worsens
0.080022%; the three stored-only cases worsen 1.158304–1.277927%. All unchanged
1.05 elapsed gates pass. Every contrary pair remains in the full report.

The selected entry already proves a nonempty interval. Short intervals now enter
the scalar body directly, and each reached body tests its backedge after advancing.
An empty vector remainder goes directly to finish. The inspected continuing visit
stays 19 instructions (reference 11); one physical short kernel saves six
instructions per activation. No dynamic activation census or whole-call partition
is inferred from that local count. The vector threshold, arithmetic, strict ties,
source prices, checked fallback, current-window proof and allocation behavior
remain unchanged. No source or runtime ABI change is introduced.

Natural32 retired instructions fall 1.011277%, natural16 0.812969%; seven instruction
medians and ten cycle medians rise across the full matrix. All 20 allocation and
usable-live-heap metrics remain exactly equal. Eight literals still use 3,354
requests and 15,279,752 B incremental peak usable heap. Code grows 3,952 B to
8,955,736 B; entry stack stays 15,792 B. The optional R172 source variant and
rejected R175 permutation are excluded from this compiler-only comparison.

Five fresh exact-worker profiles cover 16,727 complete calls. A byte-identical
Object with compilation-only labels distinguishes reduction phases in four source
copies. Natural32 has 82.63% inside the generated entry, including 37.28% unknown
native attribution. Outside-entry samples include other generated helpers and
validation; they are not all removable runtime overhead. Sampling is independent
of the retired-instruction cohort.

All 14 focused and 632 GPU tests pass, with the same seven inherited exclusions;
all 215 selected CI names execute. Fresh qualification passes 236 components,
eight compositions and one real-capacity path, with unchanged values/allowances.
All 12 scalar controls pass existing gates, but the eight pure scalar medians are
0.44–1.10% slower. All 15 architecture/catalog checks, 132 workflow tests,
runtime-only, formatting and warnings denied pass. Largest input 33,027 B;
complete-call reference bytes, independent inflate, aliases, prefixes, real
capacities and final cleanup zero pass on every case.

Parent `1584097e757cecf1e3eaaff7e3eb18b9fbb8c31f`; measured compiler/test source
`0996d5ef649029e2be5de49f150619a222ef26a5`. IR/work/ABI/MBC 196/107/39/14,
frozen R145 dependencies. The final review commit adds docs/index with measured
source hashes unchanged. See the [R176 design and all results](native-scalar-reduction-backedge.md)
for exact commands, pins, profiles, tradeoffs and skipped coverage. Ordinary hosted
CI is cancelled by instruction; review stays enabled and #973 stays draft.

The requested mutable-parent fact project remains unfinished: this round uses
an already available local entry fact. The previous child-setup audit did not meet
the brief's research hurdle. A separate bounded address-geometry experiment is
proposed from current assembly; no future saving is claimed. Parity remains active.

Publication verifies 37 public archives / 74 server digests. Fresh retrieval verifies
4,161 file occurrences / 1,024 paths / eight historical versions and reconstructs
results.json and REPORT.md byte-for-byte without compiling or compressing.

## Historical R175 checkpoint

**R175 rejects a reverse-input permutation experiment.** Natural32KB improves
4.327658× →4.303257× Roc (0.563841%) and natural16KB5.135407× →5.099904×
(0.691339%), but all20 elapsed medians regress0.224967% geometrically:7 faster,
13 slower. Eight literals worsen0.896254%, repeated32KB0.505027%; all contrary
observations are retained. The production compiler remains exactly R174/IR194.
Four lane-specific high-half/tie regression patterns remain; the rejected patch
is exported for independent reuse rather than discarded.

The actual vector body stays26 instructions: mask-load/TBL becomes UZP1/REV64.
Four literal pools shrink code64B, stack unchanged;19 instruction medians rise,
14 cycle medians rise. All20 heap/requested-byte/reallocation metrics remain equal.
Five fresh exact-candidate profiles cover16587 complete calls without applying an
old source map to changed code. All20 exact streams/inflate/source work/aliases/
capacities/cleanup and existing1.05 gates pass.

An address audit of the retained R174 baseline puts8.864469% of natural samples
in one scalar condition/routing segment versus0.384615% in its vector body.
Fresh inspection of the pinned reference finds an11-instruction scalar loop,
versus19 on the selected Mech path. Next bounded target: consume the existing
nonempty-entry fact and test the compact scalar loop at its bottom. This is
proposed work, with no saving yet claimed.

Final restored production passes14 focused/632 GPU tests with the same7 inherited
exclusions; all215 selected CI names execute. All15 architecture/catalog checks,
132 workflow tests, runtime-only, formatting and warnings denied pass. Existing
236 components/8 composition/1 real-capacity path and12 scalar controls are explicitly
reused from the identical R174 production. Largest input33027B. See the
[R175 report](native-reverse-input-experiment.md) for exact pins, all20 results,
candidate-versus-final coverage and replay. One experiment commit in draft #973;
ordinary CI cancelled by instruction, review retained. Parity remains active.

Publication verifies26 archives/52 server digests. Fresh public retrieval verifies
2970 file occurrences/443 paths/11 historical versions and reconstructs results.json
and REPORT.md byte-for-byte without compiling or compressing.

**R174 retains one current funding gate for affine child loops.** Repeated32KB
improves **3.682374× →3.241896× Roc** (11.961799%); periodic33KB
**4.227835× →3.764477×** (10.959703%). All eight blocks improve in each case.
Across all20 cases,13 medians improve/7 worsen; geometric elapsed improves1.427851%.
Natural32KB regresses **4.247301× →4.305974×** (+1.381432%, all eight adverse).
Eight literals4.830847× →4.762135×, with3 improving/5 adverse blocks. All20
unchanged1.05 elapsed gates pass; contrary results remain retained.

The selected steady matching-word path falls25 →19 machine instructions
(reference10), with funding/visit machinery10 →4. Three stack reads remain.
Repeated instructions fall7.174753%, periodic6.726526%. All20 allocation/requested
byte/reallocation/usable-live-heap metrics remain equal. Initial adverse RSS readings
do not reproduce in a fresh ABBA diagnostic (RSS+0.035486%, physical+0.010519%);
both cohorts remain available. Generated text falls22532B and stack48B.

All632 GPU tests pass with the same7 inherited exclusions; all215 CI-selected
names execute. The unchanged236-component/8-composition/1-real-capacity-path
qualification,12 scalar controls,15 architecture/catalog checks,132 workflow tests,
runtime-only, formatting and warnings-as-errors pass. Five fresh current-executable
profiles cover16506 complete calls. Natural32 remains84.12% inside the native entry;
word extension is0.64%, with substantial unknown native attribution retained.

The gate proves current maximum compute/comparison/counter space after visit one
is already paid. Every actual visit still debits its original charge; fallible source
operations, checked fallback, current windows, ownership and failure semantics remain.
No source algorithm/allocation change, prepayment, deferred settlement or new ABI.
The separate R172 source variant is not combined. Latest brief is partly addressed:
early-exit funding is improved; mutation-aware parent facts remain unimplemented
because the measured child-setup opportunity did not meet its research hurdle.

Parent fb37ded7af3dd59bb6cd5f13f5044aa29e91833e; measured compiler/test source
65021c3593137219762477ccde0f2d3ad7993977. IR/work/ABI/MBC194/107/39/14;
frozen R145 dependencies, original source/MBC, capacities, aliases, allocator and
full evaluate_into boundary. Largest input33027B. See the
[R174 design and complete report](native-child-funding.md). Next bounded inquiry is
the hot DP initializer/reduction continuation and its state/control transport.
One optimization commit in draft #973; ordinary hosted CI cancelled by instruction.
Compiler parity remains active.

Publication verifies38 archives/76 server digests. Fresh public retrieval verifies
5025 file occurrences/1147 paths/17 historical versions and reconstructs results.json
and REPORT.md byte-for-byte without compilation or compression.

**R173 rejects an empty-block compiler experiment.** It removes112720 empty
intermediate blocks, but generated code grows47336B and every retired-instruction
median increases. Twenty-case elapsed regresses0.943248% geometrically:5 faster,
15 slower. Natural32KB4.260443× →4.308305× Roc; eight bytes4.649535× →4.692437×.
All heap metrics remain equal. The production compiler is restored byte-for-byte
to the previous GPU source; the rejected patch and complete evidence are retained.
The separate R172 source variant below is unchanged. See the
[R173 report](native-empty-block-experiment.md) for all20 results, five current
profiles,631 candidate tests, exact pins, replay and skipped coverage. The next
bounded compiler target is funding inside already admitted pure early-exit loops.

**R172 retains a separate convergence-frequency source variant.** On unchanged
R170 compiler libraries, eight literals improve **4.668385× → 4.279426× Roc**
(8.331763%), distribution16KB **7.147809× → 6.830304×** (4.441991%), and
random32KB6.102742× →5.972756× (2.129949%). All20 unchanged1.05 elapsed gates
pass:19 faster/1 slower;5.297718% geometric improvement. Natural32KB rises0.319922%.
This is optional source work, separate from the pinned source and compiler-only
R171 results below. The gains are not combined or compounded.

The reference diagnostic ledger finds56 dynamic passes across20 blocks, zero
unchanged-model improvements,18 equal-frequency convergence passes across17 inputs,
eight static replays and zero saved replays. The variant keeps every current path
and tally, reusing prior deterministic codes/price only on exact frequency equality.
The original combined tally remains for other callers. Code sizes, search/EOF/tie
rules, real buffers, current ownership, input aliases, allocator and complete
API timing stay fixed apart from the explicit source-work removal.

Eight-byte instructions fall10.775886%, allocations3354 →3109. Distribution
instructions fall4.671398%, allocations5679 →5086. Requests fall on all17 compression
cases and stay equal on stored-only cases; reallocations stay equal. Peak live heap
rises117168–122880B on all20 cases, up to3.598825% on stored-only input. This memory
cost is retained. Generated text rises34800B; entry stack falls160B. Five fresh
exact-binary CPU profiles cover16792 complete calls with no inherited source map.

The proposed mutation-aware compiler project has **not** been implemented. Current
R170 child-setup samples are0% random/distribution,1.034671% repeated and0.237140%
natural; unknown native origins remain explicit. The requested approximately10%
research hurdle is not established. Scope size, nested-child edges and terminal
IntegrityFailure also obstruct these parents; supporting writes alone is insufficient.
The [full source report](../../benchmarks/mech-deflate/source-variants/frequency-reuse.md)
contains actual disassembly, the source ledger, all20 timings/counters, exact pins,
remaining compiler work and every repaired setup fault.

Qualification:3 new original/decoded tests, including rejection at half the observed work,
unchanged publication/alias contents, immediate cleanup and successful retry;
236 unchanged components/8 composition/1 real-capacity path rerun;15 architecture/
catalog checks and132 workflow tests pass. Existing R170/R171 GPU/scalar/runtime
qualification is explicitly reused with the same7 inherited exclusions. Source
regeneration is byte-identical to the measured source. No expected value or
existing allowance changes; no unresolved introduced functional failure remains.

Base2364305438e87c0a3e19fb5964af6d98401ea288; measured source generator6b622d5700;
compiler before/after3a3ae5dca4a (R170), IR/work/ABI/MBC191/107/39/14. Frozen R145
Core/Engine/math/dependencies. Current pinned-source compiler remains R171/IR192.
One optimization commit in #973, draft kept; ordinary hosted CI cancelled by
instruction. Largest timed input33027B. Compiler parity is not achieved.

Publication:29 archives/58 server digests verified. Fresh public retrieval verifies
2916 file occurrences/593 paths/10 historical versions and reproduces
results.json and REPORT.md byte-for-byte without compilation or compression.

## Pinned-source R171 checkpoint

**R171 retains generic vector clamped-subtraction maps.** Random32KB improves
**6.103196× → 5.411240× Roc** (11.337597%); repeated32KB
**3.899690× → 3.678737×** (5.665892%); periodic33KB
**4.418974× → 4.203054×** (4.886192%). Natural32KB improves
4.336329× → 4.271206× (1.501793%); eight literals4.696510× →4.657510×.
All20 unchanged1.05 gates pass:15 faster/5 slower medians,1.774875% geometric
improvement. Natural16KB rises0.278163% and eight-literals-prefix2.092883%;
all contrary observations remain. The matched comparison is retained R170.

An exact proof over the already admitted total scalar collection selects a64-byte
vector prefix and the existing checked scalar tail. Actual ARM64 helpers use four
UQSUB.8H operations per32 U16 entries, as does the inspected reference tree slide.
Diagnostic calls verify the full262144-entry tree in four large cases; small cases
without those helpers have no loop-specific saving claim. Main entry machine bytes
are identical to R170. Generated accounted text grows232B; entry stack stays15840B.

Nineteen instruction medians fall; empty rises14 instructions. Three adverse cycle
medians remain. All20 allocation-request/byte/reallocation/incremental heap-peak
metrics are exactly equal. Five exact-binary profiles execute16277 complete calls;
unknown native attribution and generated helpers outside entry remain explicit.
A separate60-call diagnostic counts current runtime callbacks:4303 random32KB,
3248 repeated32KB,9741 natural32KB,8074 distribution16KB,4466 eight literals.
These are frequencies, not a partition of execution time or hardware instructions.

Qualification:3 focused width/tail/current-capture/finite-failure tests;629 GPU
passes with the same7 inherited exclusions; all212 CI-selected names execute and
pass;236 unchanged components/8 composition/1 real-capacity path;12 original/
decoded scalar controls;132 workflow tests;15 architecture/catalog checks,
runtime-only, formatting and warnings denied. Streams, independent inflate,
source work, input aliases, capacities and managed cleanup zero hold in all cases.
Largest input33027B. No unresolved introduced functional failure remains.

Parent9035d3ab3c34e802a652f2154c8d19ba0fae5597; measured compiler
9a0fb83a20ae9ce12f4251d209c03b754dbc82da; comparison3a3ae5dca4a (R170).
IR/work/ABI/MBC192/107/39/14. Frozen dependencies, source/MBC, allocator,
algorithm/tables/search/EOF/real scratch capacities and complete evaluate_into
stay pinned; the separate R165 source experiment is excluded. See the
[full report](native-vector-clamps-experiment.md) for all20 results, exact pins,
commands, adverse cohorts, repaired setup faults and skipped coverage.

The new brief's next step is a bounded investigation of lost facts through mutable
parent loops and redundant current child admission. Establish actual dynamic
weight before extending the existing analysis. Model-stability/frequency/replay
counts are a separate source investigation. Minimum-reduction tuning is deferred.
One optimization commit in #973; the PR remains draft. Ordinary hosted CI is
cancelled by standing instruction. Parity is not achieved.

Publication:39 public archives/78 server digests verified. Fresh retrieval verifies
4875 file occurrences/1132 paths/26 historical versions and reconstructs
results.json and REPORT.md byte-for-byte without compilation or compression.

## Historical R170 checkpoint

**R170 retains corrected vector minimum selection.** Natural32KB improves
**4.430760× → 4.347484× Roc** (1.879497%), and natural16KB
**5.165119× → 5.088877×** (1.476092%). All20 unchanged1.05 gates pass:
11 faster/9 slower medians,0.226285% geometric improvement. Repeated32KB rises
0.477787%, periodic33KB0.172251%, and eight literals0.264176%. These costs remain.

The first vector version is rejected (+0.400464% geometric elapsed). Actual
machine code revealed TBL with literal masks for a same-input rotation. A minimal
independent Object proves equivalent concatenated indices select one EXT instead;
the corrected full compressor confirms the EXT sequence. Incoming-best and global
earliest-index ties, the scalar remainder, threshold, prices and admission stay
on the existing proof/path. This is compiler lowering, with the pinned source and
separate histogram source experiment excluded from the comparison.

Hardware work remains a tradeoff:11 instruction medians rise, including about0.8%
on natural32KB despite its elapsed win. All20 heap request/byte/realloc/incremental
peak metrics remain equal. Corrected text grows53,088B and entry stack112B;
correcting the first masks removes864B of that version's frame growth. The
[full report](native-vector-minimum-experiment.md) retains both complete cohorts,
all adverse counters, six exact-binary profiles across the two sources, C/NEON
and Cranelift instruction witnesses and replayable observations.

Qualification:626 GPU tests with the same7 inherited exclusions; all209 selected
CI names actually pass;3 focused ordered-reduction tests;236 unchanged components/
8 composition/1 real-capacity path;12 original/decoded scalar controls;132 workflow
tests;15 architecture/catalog checks, runtime-only, formatting and warnings denied.
Fixture/dependency setup faults and replay assumptions are corrected and recorded.
No unresolved introduced functional failure remains. Every compression case has
exact reference bytes, independent inflate, aliases/capacities/work preserved and
zero final managed bytes/nodes. Largest input33027B.

Parent86dc8106cfa571b56c5a0ae0485cef1f68c49ea9; corrected measured source
3a3ae5dca4aa4eda77b928a9073110949295770d; comparison fdb1c5c4e7 (retained R169).
IR/work/ABI/MBC191/107/39/14. Source/MBC/allocator/algorithm/real buffers and
complete evaluate_into remain fixed; frozen R145 dependencies avoid a math rebuild.
#973 stays draft, ordinary hosted CI cancelled by instruction. Parity is not achieved.

Publication:58 archives/116 server digests verified in evidence release5. Fresh
retrieval verifies5132 file occurrences/1201 paths/20 historical versions and
reconstructs both variant results and REPORT.md byte-for-byte without compiling
or compressing. The archive scan additionally checks58 manifest files, accounting
for5190 total archived files.

## Historical R169 checkpoint

**R169 retains current affine-window admission with general native loop-entry normalization.**
Repeated32KB improves **5.415459× → 3.845247× Roc** (28.995002%);
periodic33KB **6.123566× → 4.424907×** (27.739703%). Natural32KB rises
4.371874× → 4.449700× (+1.780137%); eight literals4.727993× → 4.768340×
(+0.853361%). The confirmation has6 faster/14 slower medians and2.690497%
geometric improvement; all20 unchanged1.05 gates pass.

The initial full20 cohort failed the32B high-entropy gate (+5.679977%). A single
predeclared full20 confirmation uses identical binaries/counts/gates and passes.
Both cohorts remain separate; timing variation has no established cause. The
window-only prototype is rejected (+69%/+66% repeated/periodic). Merging distinct
activation entries once before the loop makes its current arguments invariant
within that activation, enabling the existing SSA cleanup.32 preheaders permit
16,298 redundant phi removals; the identified matching path falls70 to25 machine
instructions and19 to3 stack accesses, with the same2 input loads. These are
static counts, not hardware instructions retired by a complete compression.

Separate retired-instruction medians fall35.923226% repeated and34.472107%
periodic;15 other instruction medians rise. All20 allocation-request/byte/realloc/
incremental live-heap metrics are equal. Text shrinks5480B and entry stack64B.
Six fresh current-Object profiles execute73,481 complete calls. Word extension
still consumes21.445390% of repeated running-timer samples; other native scopes
dominate natural text. Samples are not an exact host/native instruction split.

Validation:626 compatible GPU tests with the same7 inherited exclusions; all209
selected CI names actually pass;6 focused phi and5 current captured-window tests;
236 unchanged components/8 composition/1 real-capacity path;12 original/decoded
scalar controls with unchanged gates;132 workflow tests;15 architecture/catalog
checks, runtime-only, formatting and warnings denied. All timings preserve exact
reference bytes, independent inflate, aliases, capacities, logical work and zero
final managed bytes/nodes. Largest input33027B. No full multi-megabyte claim.

Parent4ab701de9b4ca994a1ae151259d5a697132f1210; measured effective compiler
fdb1c5c4e738ad3bb69233ef92448efcf075aa81; comparison is retained R168, not the
failed window-only variant. IR/work/ABI/MBC189/107/39/14. Frozen R145 dependencies,
source/MBC/allocator/algorithm/tables/real buffers and evaluate_into stay pinned.
The separate R165 histogram source gain is excluded. The [full report](native-affine-loop-windows.md)
records20 results, all cohorts, errors, exact pins and skipped coverage. #973 stays
draft; ordinary hosted CI is cancelled by instruction. Parity is not achieved.

The selected path-cost milestone and separate source experiment are complete.
Word-loop funding/paid-flag work and bounded wider comparison remain candidates;
neither is an established new saving. Finish publication before another compiler
implementation.

Publication:67 archives/134 server digests verified in evidence release5. Fresh
retrieval verifies5603 file occurrences/1255 paths/22 historical versions and
reconstructs results.json and REPORT.md byte-for-byte, without native execution.

## Historical R168 checkpoint

**R168 retains dominance-scoped invariant phi cleanup in native loops.**
All20 final-source matched cases and unchanged1.05 gates pass:19 faster/1 slower,
**2.639531% geometric elapsed improvement**. Natural32KB improves
**4.574977× →4.352844× Roc** (4.855392%); repeated32KB5.748624× →5.416119×
(5.784078%); periodic33KB6.443533× →6.105817× (5.241168%); natural16KB
5.254551× →5.031433×; eight bytes4.721208× →4.650969× (1.487746%).
Random64 is the contrary elapsed case, +1.341991%, preserved within the existing
5% gate. These are fresh matched cohorts, not replacements for historical results.

The pass treats changing outer SSA values as fixed current atoms during a child
activation. It aliases phi-only local chains to one dominating outside value,
removing10,180 redundant parameters across the actual compressor. The identified
word-loop header falls78 to5 parameters before/after backend optimization;73 is
an actual formal-parameter reduction, not a measured memory-copy count. Multiple
seeds, true changes, effects, type mismatches and non-dominating entries retain
the maintained path. No source operation, charge, CFG edge or error is removed.

All20 retired-instruction medians fall. Repeated instructions fall141.125M to
129.786M (8.034527%); periodic147.350M to135.869M (7.791342%); natural32KB
259.835M to247.776M (4.640900%). Seven separate cycle medians rise; contrary
cohorts remain. All20 allocation counts/bytes/reallocations/incremental heap peaks
are equal. Text grows22,564B; entry stack shrinks688B. One outside-timer image
restoration is36.839s before/37.549s after, with no startup speed claim.

The final matrix has3968 primary/4048 total complete calls and exact streams,
independent inflate, aliases, prefixes, capacities and cleanup zero. Initial and
pre-layout cohorts remain separate, including a contrary eight-byte observation.
Required Rust module relocation leaves the entire compressor Object identical
but changes linked host text, so the authoritative20-case cohort was rerun on the
final binary. Five current exact-Object profiles and18 hot-PC disassembly windows
are retained. CPU-time samples do not partition retired instructions.

Validation:4 focused JIT tests;619 compatible GPU tests with the same7 inherited
exclusions; all202 CI-selected names actually passed in that suite;236 unchanged
components/8 composition/1 real-capacity path;12 balanced original/decoded scalar
controls under unchanged latency/allocation gates;132 workflow tests;15 architecture/
catalog checks, runtime-only, formatting and warnings denied. Module path/layout
and exact test-only unsafe registration failures are repaired without weakening
checks. No unresolved introduced functional failure remains.

Optimization parent4967229cd93b8acd7bd6f26fffa5d79f10e576d2; effective measured
compilerf1c4b67e774f55255e2c72ea4422e27d36b6be3c; baseline compiler9017ab40d1
(published8364514bd5). IR/work/ABI/MBC187/107/39/14. Frozen R145 Core/Engine/math/
dependencies, pinned source/MBC/allocator/algorithm/real buffers and evaluate_into
remain unchanged. The separate histogram source variant is excluded. The
[full report](native-local-phi-cleanup.md) records exact pins,20 results, failures,
skipped coverage and numerical replay. #973 stays draft; ordinary hosted CI is
cancelled by standing instruction. Parity is not achieved.

The brief's selected path-cost proof/scalar/funding/four-lane experiment and one
separate source experiment are complete. Match-extension bounded unroll/wider
comparison remains open. R168 removes demonstrated invariant state transport
before another bounded experiment; no further implementation starts this round.

Publication:61 public archives/122 server digests verified. Fresh retrieval
verifies5840 file occurrences/1276 paths/22 historical versions and reconstructs
results.json and REPORT.md byte-for-byte. Replay uses archived receipts only.
Invariant IR is published once; transparent compaction preserves measured payload
hashes and every superseded unpublished archive manifest/receipt, saving1.016GB.

## Historical R167 checkpoint

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
