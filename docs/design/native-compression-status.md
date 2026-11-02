# Native compression: current status

Raw observations, diagnostic drivers, full censuses and rejected experiments stay
outside source. The maintained harness and [compact evidence index](../../benchmarks/mech-deflate/experiments.json)
retain the reproduction path. [PR #973](https://github.com/mech-lang/mech/pull/973)
stays draft pending green CI and clean review. Hosted CI is not awaited under
instruction; no green hosted-CI claim is made.

## R116: fund native scalar aggregate words through the ordinary path

Base **`d5e88ff2e66714ad4ee1d1f200518958401cea9a`**; retained compiler/runtime
**`509c0d8e05a234020372b43832d7785f255c1b59`**; focused regression
`b91ab7e6a3`. Independently derived private scalar aggregate actions already
emit checked native words. They now share the existing ordinary scalar funding
path, without a new owner frame, argument bank or cache. Owned-map selection,
lifecycle boundaries, lexical failures and the shared aggregate emitter remain
unchanged. Image loading rederives the same groups after validating the scalar
word layout. IR145 rejects IR143 and the held IR144; work policy99, private
ABI38 and bytecode14 remain.

| Matched elapsed cohort | Baseline / reference | Candidate / reference | Change |
|---|---:|---:|---:|
|8B, four prepared pairs|7.979811×|7.874930×|−1.314335%|
|1KB, alternating blocks|113.499239×|104.174555×|−8.215636%|

8B takes 2.322125→2.2916045 ms against a fresh 0.291 ms reference. All four
prepared pairs improve; 61/64 descriptive blocks improve and three worsen.
The cohort retains 3,968 primary / 3,984 total observations. The separate 1KB
cohort takes 34.163271→31.356541 ms against a fresh 0.301 ms reference, with
128 observations per version; all eight blocks improve. Each row has its own
common reference. Historical ratios and independent cohorts are not pooled or
compounded. The target remains 1×; no larger-input or full-scale speed is claimed.

| Separate hardware cohort | Baseline | Candidate |
|---|---:|---:|
|8B retired instructions|26,773,029|26,168,226|
|8B cycles|7,394,977.5|7,211,731.5|
|1KB retired instructions|552,038,310|494,396,024|
|1KB cycles|109,262,186.5|100,770,407.5|

At 1KB, instructions fall **10.441718%** and cycles **7.771928%**, independently
of the primary elapsed cohort. Hardware has 32 observations per subject; heap
has eight Mech observations after controls are excluded. Instrumented elapsed
is not a primary timing result. The fresh 1KB reference uses 1,690,857 instructions
and 974,536.5 cycles. Samples are CPU-time weights, not function instruction counts.

Per-call heap demand is exactly unchanged: 8B 4,257 requests / 95,876,945 requested
bytes / 17,782,424B incremental live peak; 1KB 47,796 requests / 7,523,247,338 requested
bytes / 17,773,224B peak. Reallocations 68, included in requests. Requested capacity
is neither initialized bytes copied nor live memory. Between-call allocator live
bytes rise **1,224B** at both sizes in the whole prepared harness; that metric
includes current values and is not an exclusive prepared-metadata census. This
small increase remains explicit rather than claiming a universal heap reduction.

Generated native text shrinks **20,436B**, 14,604,696→14,584,260B; machine entry
stack shrinks **768B**, 135,728→134,960B. Whole linked instruction text falls
21,844B to 63,660,796B; executable bytes fall 15,744 to 172,981,960. These are
actual binary/metadata measurements, not a complete host-stack census. Fresh
bounded ARM64 inspection covers helper, entry and exit functions. The pinned
reference is inspected again: its complete 195,456B text includes platform and
allocator, while the Mech generated-text row excludes its host runtime. Opaque
reference symbols are not assigned source names.

### Executed mechanism

| Registered ABI callbacks per complete call | 8B before → after | 1KB before → after |
|---|---:|---:|
|Ordinary admission|4,890 →3,823|201,648 →100,264|
|Helpers, including fused callbacks|4,911 →4,901|96,655 →96,648|
|Total|9,801 →8,724|298,303 →196,912|
|Numeric entries/exits|938 →933|31,916 →31,912|

Two diagnostic executions agree at each size and compiler. The observer counts
the registered admission callback and helper separately; a fused helper counts
once. Internal admission routines are not falsely counted as another ABI call.
These are callback counts, not payload copying or retired instructions. Array
alias exports remain 981 at 8B and 39,527 at 1KB. Ordinary scalar groups rise
1,372→1,373, with 8,642→8,939 covered static sites and 35 groups containing native
aggregate actions. Emitted numeric pieces decrease 484→479. This removes repeated
funding work instead of adding the extra owner-entry boundaries seen in R115.

The exact blocking source remains `#OptimalPathMatches :Try(m,k,best,item)` at
pinned source lines 700–714, including four-word tuple and enum packing. All
algorithm code, tables, search limits, near-EOF bounds, real capacities, source
allocation sites, allocator and evaluate_into timing boundaries are unchanged.
Mech performs the complete compression. Exact raw bytes, independent inflation,
input aliases, output capacity 13/1029, clear/rebind outside the timer and final
managed bytes/nodes 0 pass at both sizes.

### Correctness and validation

The existing scalar-funding regression is extended with a private tuple and
changing true/false arguments, retaining its original source variant and expected
behavior. It fails on the clean parent at missing aggregate group coverage.
Original/decoded and independently restored artifacts then pass native interior
arithmetic origins, one-short funding fallback, assertion-before-overflow order,
unchanged prior publication, live aliases and success→failure→retry. Test inventory
counts and allowances do not increase. No R115 numeric-aggregate SSA selection is
enabled. Successful numeric exit clears both the scalar marker and its shared
readiness lane so an unfunded ordinary suffix retains its original admissions.

Passing selections: runtime 356, work/memory 33, guarded native 65, image 1;
unchanged components 236 / connected 8 / path 1 with actual listed and executed counts;
linked 16+12+12; scalar 12 and frame 8. Scalar's maximum elapsed ratio is 1.005790
under the unchanged 1.05 gate; pure scalar requests/bytes are equal and loop
allocation slope is zero. Frame requests/bytes/work do not increase. Workflow 132,
architecture/type/memory 15, runtime-only cfg and full formatting pass, with
warnings-as-errors. No golden or allowance changes. No fresh Core/no_std,
all-features or minimal dependency graph qualification is claimed; the existing
expensive knucleotide-body exclusion remains explicit. Hosted CI is not awaited
and the PR stays draft pending green CI and clean review.

Logical resource work is not hardware instructions. Existing policy99 prices
the changed funding/partition geometry: 42,410,605,541→42,390,445,317 at 8B,
1,970,398,096,074→1,970,438,688,821 at 1KB. Both actual work values are pinned
separately in every timing and hardware cohort. No source operation is removed
to lower that counter. Fresh serial profiles cover 695 complete calls per 8B
subject and 130 per 1KB subject. Remaining sampling cost is concentrated in numeric
entry/exit, budget checks and cleanup; callback sample shares are not a causal
instruction ledger. The next measurement should split those remaining entry
checks before another implementation change.

No newly introduced compiler/runtime or diagnostic failure remains in executed
scope. The baseline negative regression is intentional. Obsolete failed-test
executables from R115 are deleted after their source/commands/receipts were
archived, freeing 337,841,784B; active baseline and candidate binaries remain pinned.

## R115: scalar aggregate parent pieces held

Base `e7644df6bb40969bfbe3b7e38856071fc4ef2090`; initial candidate
`0bb239f3849df2bea16dc4929581d1ba83f61ca8`, corrected compiler
**`2f0cc8b85e7dbcef53abfe16d76742872a7ceca2`**, test-only correction
`c1b0265887`. The candidate uses the existing independently derived scalar
aggregate word layout and shared checked SSA emitter inside acyclic parent
pieces. IR144 rejects old IR143 identities; work policy 99, ABI38 and bytecode14
remain. A fresh complete-compressor Object is emitted, not reused.

| Cohort | Baseline / reference | Candidate / reference | Elapsed change |
|---|---:|---:|---:|
|8B, four prepared pairs|8.380415×|8.363944×|−0.196543%|
|1KB, original|112.018587×|113.075641×|+0.943642%|
|1KB, independent confirmation|113.620849×|114.912444×|+1.136759%|

The 8B cohort retains 3,968 primary / 3,984 total observations; two of four
pairs improve, with 47 improving and 17 adverse descriptive blocks. Each 1KB
cohort has 128 observations per version; neither improves any of its eight
blocks. The independent confirmation was declared after the contrary hardware
cohort, before its own observations. All original observations remain. Each row
uses its own common reference; historical multiples are not pooled or compounded.

Actual 8B elapsed is 2.321375→2.3168125 ms, reference 0.277 ms. The 1KB times
are 34.2776875→34.601146 ms, reference 0.306 ms; confirmation
34.1998755→34.5886455 ms, reference 0.301 ms. No full-scale run is made.

The separate 1KB hardware cohort is favorable: instructions
552,040,295.5→545,942,708.5 (**−1.104555%**), cycles
109,309,426→108,070,921.5 (**−1.133026%**), 32 observations per subject.
These counts remain explicit contrary evidence to both paired elapsed cohorts.
Their instrumented elapsed is not a primary speed claim. The fresh reference
uses 1,690,807 instructions / 968,725 cycles in that cohort.

Heap demand is exactly unchanged: 47,796 requests, 7,523,247,338 requested bytes,
68 reallocations and 17,773,224B incremental live peak, eight measured calls per
Mech subject. The separate 1KB reference has 84 requests / 9,012,988 requested
bytes / 9,257,328B incremental peak. Requested capacity is neither initialized
bytes copied nor live memory. No new 8B hardware/heap cohort is claimed.

Generated native text grows **87,040B**, 14,604,696→14,691,736B; native entry
stack grows **688B**, 135,728→136,416B. Whole linked text grows 86,868B to
63,769,508B; the executable grows 98,064B. Fresh bounded ARM64 disassembly
covers actual helper/entry/exit functions. The pinned reference is inspected
again: its complete text is 195,456B, including platform and allocator; Mech's
generated-text row excludes its host runtime. Static sizes are not executed
instruction counts, and opaque reference symbols are not assigned source names.

### What the census changed

| Exact counts per 1KB computation | Baseline | Candidate |
|---|---:|---:|
|Native numeric entries/exits|31,916|34,988|
|Runtime helper callbacks, excluding ordinary admissions|96,655|102,796|
|Exported array aliases|39,527|39,527|
|Exported managed frame fields|52,432|52,438|

Two repeated diagnostic executions agree at each size and compiler revision.
The extra 3,072 entries are dominated by two six-site pieces reached 1,024 and
2,048 times, each with two aggregate actions, no numeric views, no managed
sources and no frames. The selection formula counts source sites plus actions,
although aggregate actions already emit native word instructions without an
ordinary helper. The candidate adds helper boundaries without reducing array
alias exports. These are measured mechanisms, not a causal attribution of every
elapsed or cycle difference.

The real source witness is `#OptimalPathMatches :Try(m,k,best,item)` at pinned
source lines 700–714, including four-word tuple and enum packing; one original
frontier reaches that pack 2,048 times at 1KB. The next bounded target is to fund
already-native aggregate words through the existing cheap scalar-admission
path and base selection on actual callback work. No additional ownership cache
is proposed. The positive instruction result is preserved for that refinement.

### Correctness, failures and disposition

The new minimal regression fails on the retained parent because aggregate
actions are absent from its native piece. The first implementation then exposes
an introduced cursor failure: piece 25 executes 91→7, but an unchanged readiness
word incorrectly skips ordinary finish 7 and attempts 91→92. Its exit now clears
the readiness lane together with `scalar_path`; the test requires that exact
partial ordinary-group suffix. The first error-class expectation incorrectly
looked for `Bounds`; it now requires the maintained `IndexOutOfRange` and adds
a simultaneous failed-assertion / zero-divisor control. Earlier lexical load
failure wins. Both failed receipts remain.

Corrected candidate execution passes runtime 357, work/memory 33, guarded native
inventory/execution 66 and image 1, including explicit IR143 rejection. The new
original/decoded and independently restored controls check current Boolean
words across recursive visits, success→assertion/arithmetic/bounds failure→retry,
unselected child calls, live aliases, unchanged prior publication and cleanup.
The complete linked compressor matches exact raw bytes and independent inflation
at 8B and1KB; capacity 13/1029, clear/rebind outside evaluation and final managed
bytes/nodes 0 remain. Source, algorithm, tables, limits, real buffers, source
allocation sites, allocator and timing boundaries are unchanged.

Logical resource prices are not hardware instructions. The changed piece
partition charges 42,410,605,541→42,381,503,754 at 8B and
1,970,398,096,074→1,970,444,323,983 at 1KB under the existing policy 99 formula.
The restored default retains the baseline charges. No component golden or
allowance is changed. Candidate workflow 132, architecture/type/memory 15,
runtime-only cfg and formatting pass. Fresh component 236 / connected 8 / path 1,
linked 40 mini-program, scalar 12 / frame 8 performance, Core/no_std/all-features
and minimal-graph qualifications are skipped for this held trial; no full
retained-candidate or hosted-CI qualification is claimed. Fresh serial profiles
cover 130 complete 1KB calls per subject, outside primary timing.

Other preserved diagnostic failures: a reader requested sites beyond a 94-block
fixture; its already-created descriptor was reread at valid sites without new
execution. An initial hardware driver could not inspect host activity; it failed
before any observation and succeeds with authorized access. A stale initial
diagnostic-only scope sentence is corrected by an explicit addendum; actual
primary specs, artifact identities, version fields and numerical cohorts were
correct. No hardware resource denial or unresolved retained compiler failure
remains in executed scope.

**Held, not default.** Experiment branch
`codex/scalar-aggregate-pieces-r115-experiment` preserves checkpoint
`7ddc7acd1b8cf812f38bf9da682e69d7f5f206f9`. Restoration
**`94c7447f7fb5c30073340388b8cf865a0d1a888d`** returns all nine compiler,
runtime, test and CI files exactly to base R114. Default versions remain
**143/99/38/14**; its prior qualifications remain applicable. This update adds
evidence and status relative to that base.

All 15 R115 archives (19,477,535 compressed bytes), plus sidecars, have verified
server digests. Fresh independent retrieval verifies 1,087 file occurrences /
324 unique paths, including all eight paths with multiple preserved versions,
and reproduces all three indexed reports byte-for-byte. Native binaries remain
local. The maintained `experiment.py replay-round --round 115` entry verifies
and reconstructs these numbers without compiling or compressing.

## R114: inline private dispatch at the callback boundary

Base `a105e9a76978ff7bbb4a8dd56968a3c7b8e457b8`; retained runtime change
**`f327ef9489aaa0997f4d8627256f1a14eb70b0c4`**. The complete production change is
one `#[inline(always)]` attribute on `Invocation::helper`. The source program,
algorithm, tables, search limits, real buffer capacities, allocation sites,
allocator, logical prices, work and evaluate_into boundaries remain pinned.
Versions remain IR/work/private ABI/bytecode **143/99/38/14**.

| Cohort | Same-round baseline / reference | Candidate / reference | Elapsed change |
|---|---:|---:|---:|
|8B original|8.003090×|7.966452×|−0.457797%|
|8B independent confirmation|8.104747×|8.047038×|−0.712040%|
|1KB original|111.327477×|110.976443×|−0.315317%|
|1KB independent confirmation|112.166735×|111.884938×|−0.251231%|

The two 8B cohorts each retain 3,968 primary/3,984 total observations across four
fresh prepared pairs. Original pairs improve 3/4, confirmation 4/4; descriptive
blocks improve 41/64 and 51/64. The separate 1KB cohorts each retain 128 observations
per version in alternating 16-call blocks; 7/8 and 6/8 blocks improve. Every adverse
observation remains. Each row uses its own common fresh reference; these are not
pooled or compounded gains. The target remains 1× and larger-input throughput is
still unestablished. No full-scale timing is run.

8B elapsed 2.320896→2.310271 ms (reference 0.290 ms), confirmation
2.3260625→2.3095 ms (reference 0.287 ms). The 1KB times are 34.066208→33.9587915 ms
(reference 0.306 ms), confirmation 34.0986875→34.013021 ms (reference 0.304 ms).
The small effect motivated the separately declared confirmation before retention.

| Separate counter cohort | Baseline | Candidate | Change |
|---|---:|---:|---:|
|8B retired instructions|26,860,744|26,774,972|−0.319321%|
|8B cycles|7,333,426|7,353,423.5|+0.272690%|
|1KB retired instructions|552,670,712|551,997,052.5|−0.121892%|
|1KB cycles|109,466,354.5|110,060,389.5|+0.542664%|

**Both cycle cohorts worsen.** Preserve this contrary result alongside the four
matched elapsed improvements. Counter, heap, sample and primary timing cohorts
are distinct; instrumented elapsed is not a speed claim. Hardware has 32
observations per subject; heap has 32 reference / eight Mech observations after
controls are excluded. No per-function instruction ledger is inferred from samples.

Heap demand is exactly unchanged: 8B 4,257 requests/95,876,945 requested bytes/
17,782,424B incremental live peak; 1KB 47,796 requests/7,523,247,338 requested bytes/
17,773,224B incremental peak. Reallocations 68, included in requests. Requested
capacity is not copying volume or live memory. No new cache or retained data bank
is introduced; this is not a measured whole-prepared-memory reduction. Retention
is a small speed/instruction win with a 4 KiB code-size cost, not a memory win.

### Mechanism and code footprint

The native error type and `Result<Option<u64>, Error>` each occupy 80B;
`Option<u64>` occupies 16B. Actual baseline ARM64 supplies a stack return place to
the separate generic dispatcher. Inlining removes that internal call from the
hot callback while retaining the original body and check order. This does not
prove 80B copied on each successful return. The callback frame remains 320B;
its symbol span grows 884→3,044B and the standalone 19,276B dispatcher symbol
is absent. Two additional internal String/map fallback callers also exist; the
change is not based on a claim that there is only one production caller.

Whole linked instruction text grows **4,096B**, 63,678,544→63,682,640B. Total
executable bytes fall 172,998,888→172,997,704; other sections differ. Report both
rather than claiming a code-memory reduction. Native program text remains
14,604,696B and entry stack 135,728B. No fresh complete host-stack census is claimed.

The exact retained R108 Object/descriptor/metadata are reused, after verifying
that the only GPU source delta is this runtime attribute. Code generation,
protocol, types, image semantics and all dependencies remain unchanged. Object
producer source is `705c5acdc4fc92be5beeacf28d02721ea4e39ddf`; runtime source is
`f327ef9489aaa0997f4d8627256f1a14eb70b0c4`. All three pins and actual native text
are byte-identical. No fresh full-compressor Object emission or matched compiler-
time improvement is claimed. The actual GPU-only release build takes 23.231s;
all project/external dependencies remain frozen from R105.

### Qualification and remaining cost

Passing current selections: runtime 356, work/memory 33, guarded native CI 65,
image 1, unchanged components 236 / connected 8 / path 1 with listed inventories,
linked 16+12+12, scalar 12 and frame 8. Scalar's maximum ratio is 1.015528 under the
unchanged 1.05 gate; pure scalar requests/bytes are equal and loop allocation
slope is zero. Frame requests/bytes/work do not increase. Workflow 132,
architecture/type/memory 15, runtime-only cfg and full formatting pass. No golden
or allowance changes. Original/decoded/restored and linked controls retain current
arguments, live aliases, checked failures, unchanged publication, cleanup and retry.
Exact raw bytes and independent inflation pass at both sizes; output capacity 13/
1029 with clear/rebind outside the timer and final managed bytes/nodes 0 remain.
The existing expensive knucleotide-body exclusion stays explicit. No fresh Core,
no_std, all-features or minimal dependency graph qualification is claimed.

Fresh serial profiles cover 695 complete calls per 8B subject and 130 per 1KB subject.
CPU weights are not retired instructions; ancestry categories overlap. The next
bounded diagnostic is an actual census of which managed operations end native
pieces and force array publication, to select a larger boundary reduction.

The 16-case linked run passed before its receipt hit a protected Git-metadata write.
The exact archive/index entry was verified and its pending commit completed with
authorized access; tests were not rerun. Two numerical-reader path assumptions
were corrected without new measurements: the section receipt is at
`binary-audit/sections.json`, and an already-symbolized raw sample may have no
separate resolved file. Original sealed packets and failures remain, with a new
corrected-reader packet. No retained compiler/runtime failure remains in executed
scope. The PR stays draft pending green CI and clean review; hosted CI is not awaited.

All 24 R114 archives (12,027,300 compressed bytes), plus their sidecars, have
verified server digests in the continuation evidence release. Independent fresh
retrieval verifies 1,747 file occurrences / 491 unique paths and reproduces all
four indexed numerical reports byte-for-byte. No native binaries are published.

## R112–R113: adjacent numeric admission trials held

R112 source `c85dfe8bcea3bc39a626a5287f2fa1ee5a22ad5c` combines source admission
and numeric entry through a private ABI39 tag. R113
`62b901e9b0cb81b2c84359bc41a48a50f56ecfa5` reuses the already computed scalar
fallback decision. Both preserve source work and exact output, and both are held.

| Trial | 8B baseline → candidate / reference | Elapsed change | 1KB baseline → candidate / reference | Elapsed change |
|---|---:|---:|---:|---:|
|R112|8.111014× → 8.115967×|+0.061063%|112.203467× → 116.062363×|+3.439195%|
|R113|8.038864× → 8.128734×|+1.117953%|106.951137× → 107.672749×|+0.674712%|

Each 8B trial retains 3,968 primary/3,984 total observations; each 1KB trial retains
128 observations/version. R113 has no improving 8B pair or 1KB block. Different
reference times explain different baseline multiples across rows; baseline 1KB
elapsed is 34.109854 ms and 34.063937 ms respectively. Do not treat those historical
multiples as a compiler improvement.

R112's separate 1KB instructions increase 552,670,237→553,848,641 (+0.213220%);
cycles rise 109,696,621.5→109,868,435 (+0.156626%). This small counter-cycle delta
contrasts with the primary 3.44% elapsed regression and remains explicit. Heap
requests 47,796, requested bytes 7,523,247,338 and live peak 17,773,224B are unchanged.
R113 has no new hardware/heap cohort. Both have fresh serial 1KB CPU profiles and
actual bounded ARM64 disassembly; samples are not instruction attribution.

Actual program text falls 16,300B to 14,588,396, but native entry stack grows 272B
to 136,000. The callback frame grows 320→400B in R112 and 464B in R113. R112 adds
an outlined 184B scalar-decision lookup; R113 removes the redundant lookup but
still regresses. These are verified code differences, not a complete causal
explanation of elapsed changes. R112/R113 Objects are byte-identical to each other,
with changed ABI identity versus R108. Old ABI rejection remains strict.

Preserved diagnostic failures: the first R112 unit driver started before its
library manifest existed, before Rust compilation; after the dependency completed,
the driver succeeds. Its one-short-fuel observer originally injected only through
the old callback; shared injection at both real admission forms fixes the observer
in `f410781775` without relaxing assertions. Nine focused cases pass. R113's full
runtime run passes 354 cases and fails two stale ABI38 assertions against ABI39.
Both pass on retained ABI38; test-only update `d4a7f89b9b` makes both pass in focused
reruns. No new all-356-pass cohort is claimed for the held ABI39 candidate.

Restoration **`951eb640a6`** returns all 12 changed GPU files exactly to the retained
ABI38 implementation. Source commits, tests and raw adverse results remain on
`codex/numeric-admission-r112-experiment` and
`codex/numeric-admission-r113-experiment` and in the published history. Full
component/linked/scalar/frame qualification is not claimed for either held trial.
These findings do not reopen the previously completed assertion, fixture or CI
work. R114 retains ABI38 and only the separate runtime inline attribute.

R112 publishes 11 archives (10,356,141 compressed bytes), R113 10 (10,279,867),
plus sidecars, with verified server digests. Fresh independent retrieval verifies
705 / 643 file occurrences and reproduces both reports for each trial exactly.
The maintained `experiment.py replay-round` entry performs verification and
numerical replay without compiling or compressing. R112–R114 together retain
45 immutable archives and eight byte-identical derived reports.

## R109–R111: frame geometry trials held; default remains R108

The speed target remains 1×. The earlier R106 memory-saving ownership change
remains held rather than becoming the default. Its 1KB gain and smaller-input
regressions are preserved below; cumulative requested capacity is not live memory.

Three subsequent GPU-runtime trials reuse immutable frame-field geometry during
numeric handoffs. Every comparison uses the retained R108 executable, source
revision `705c5acdc4fc92be5beeacf28d02721ea4e39ddf`, with a fresh reference for
that comparison. The candidate source revisions are:

- R109 `0cbf5b9f6b5870c40aac5a75bc7c89bb23c9b32a`: lazy layout reuse plus field writers.
- R110 `7a47b86b5e05e696beac21b7fe673c8b171c4fd2`: explicit lazy conditionals remove an outlined Option helper.
- R111 `727a955105fe43ee99d120560157bbdf985dacd2`: resolve the immutable layout before the field loop and borrow field descriptions.

| Trial | 8B baseline → candidate / reference | Elapsed change | 1KB baseline → candidate / reference | Elapsed change |
|---|---:|---:|---:|---:|
|R109|7.973493× → 8.037212×|+0.799135%|110.613871× → 111.660288×|+0.946009%|
|R110|7.944563× → 7.969462×|+0.313409%|112.791323× → 113.392659×|+0.533140%|
|R111|7.948558× → 7.967464×|+0.237852%|112.394942× → 112.059211×|−0.298707%|

Each 8B trial retains 3,968 primary/3,984 total observations over four prepared
pairs. The separate 1KB cohorts retain 128 observations/version in alternating
16-call blocks. R111 improves two of four 8B pairs and seven of eight 1KB blocks;
its positive result is retained alongside the adverse one. These trials do not
establish a consistent speed improvement across both measured sizes. Neither
historical reference ratios nor results from independent cohorts are combined.

Address-bounded inspection of the actual ARM64 executables finds a new per-field
outlined Option helper in R109 and 32-byte field-description transport at the
new writers. R110 removes the Option helper. R111 borrows the field description
and removes that aggregate copy at the scalar writer. These are verified binary
changes, not a causal attribution of the elapsed differences. No instruction,
heap, prepared-memory or fresh CPU-sample cohort is claimed for these held trials.
The earlier weighted layout-lookup prediction remains explicitly a prediction;
it is not an executed-call census for the final eager-lookup variant.

All trial Objects/descriptors/metadata remain byte-identical to R108. Generated
compressor text remains 14,604,696B and machine entry stack 135,728B; this does not
claim unchanged host helper stack. Versions 143/99/38/14, source/bytecode pins,
algorithm, real capacities, allocation sites, allocator, source work and timing
boundaries remain unchanged. Both input sizes preserve exact raw bytes,
independent inflation, aliases, output capacity13/1029 with clear/rebind outside
the timer, and final managed bytes/nodes0. No larger-input timing is run.

The new R111 focused regression passes for original/decoded artifacts, covering
invalid Bool bits, wrong lane/schema, unchanged state/accounting and old aliases,
error precedence with absent backing, and recovery. Its initial expectation
incorrectly assumed every fixture field contained 7; the existing helper uses
7 plus the leaf ordinal. Test-only fix `11b61ba7f7` captures each alias's actual
pre-update draft. The failed receipt is retained. The existing collection-lifetime
regression also passes. Full runtime/component/linked/scalar/frame qualification
was not rerun for these unretained trials; no inherited allowance or golden changed.

Candidate source/tests/CI inventory are saved on local branch
`codex/frame-geometry-r109-r111-experiment` at `af69cc894c`. Restoration commit
**`8bd35198ca69e0dac5c78e967c758f3397c7e3b7`** returns all compiler, runtime, test
and CI files exactly to published R108 head
`3b310eb80f84ab16757923f270931fd53cf1477b`. The default diff since that head
contains only evidence/index/status updates. Source commits remain in history,
and diagnostic patches/receipts preserve the experiments for later comparison.
No new optimization is retained from R109–R111; R108's validation scope below still
applies to the unchanged implementation.

All31 archives and31 sidecars (29,015,328 archive bytes, no native binaries) are
published with verified server digests. Fresh maintained retrieval reproduces all
six timing/validation reports byte-identically from raw observations without
compiling or compressing. It verifies the original and corrected fixture receipts
as well as every earlier packet version. The restored default passes132 workflow
checks,15 architecture/type/memory checks and full formatting. Exact commands,
replay counts, omissions and report hashes are in the compact index. Hosted CI
is not awaited; the PR remains draft pending green CI and clean review.

## R108: reuse current owner references within one numeric entry

Base `ac80ae5e7766c0aef16978f5ba39b90aafd198c2`; compiler/runtime change
**`705c5acdc4fc92be5beeacf28d02721ea4e39ddf`**. Only GPU runtime code changes;
all project and external dependencies stay frozen from R105. GPU release build
22.597s; Object emission59.816s. These are descriptive setup durations, not matched
compiler-time improvement claims. The R106 ownership candidate remains separate.

A numeric entry first resolves and validates every current source. It now keeps
eight borrowed owner references for the rest of that same entry. Higher source
indices keep the existing lookup path; this is no program-size/eligibility limit.
There are no owner clones, heap allocations, persistent caches or new permissions.
Current schema, footprint, typed-view, account, funding, failure and publication
checks retain their order. Argument-word writes touch the separate admitted bank;
shared owner borrows cannot mutate owner tables, and no bank reference escapes.
No source, algorithm, table, capacity, allocator, ABI or pricing change occurs.

External diagnostics find all938/31,916 entries still selected on8B/1KB. Source
lookups fall5,540→2,877 and211,691→112,793 respectively:2,663/98,898 avoided.
Only38 repeats remain in each witness, from the ordinary larger-index fallback.
Both diagnostic repetitions agree. Logical work stays42,410,605,541 /1,970,398,096,074;
diagnostic timings and allocations are excluded from performance claims.

| Input | Fresh baseline / reference | R108 / reference | Elapsed change |
|---|---:|---:|---:|
|8B|8.098347×|7.993464×|−1.295113%|
|1KB|113.933211×|111.848379×|−1.829872%|

8B elapsed2.3485205→2.3181045ms, common fresh reference0.290ms;3,968 primary /
3,984 total observations, all four pairs improve,58 descriptive blocks improve
and6 worsen. The separate1KB cohort is34.8635625→34.225604ms, reference0.306ms,
128 observations/version; all eight alternating16-call blocks improve. All adverse
observations remain. Each row uses its own common reference; historical ratios
are not combined. The1KB input is the retained deterministic xorshift prefix;
all capacities, exact raw output, independent inflate, input aliases, clear/rebind
outside evaluate_into and final managed bytes/nodes0 pass. No full-scale run.

| Separate counter cohort | Baseline | R108 | Change |
|---|---:|---:|---:|
|8B instructions|27,013,922.5|26,861,966.5|−0.562510%|
|8B cycles|7,417,961|7,328,817.5|−1.201725%|
|1KB instructions|558,180,681|552,668,745.5|−0.987482%|
|1KB cycles|111,651,054.5|109,037,226|−2.341069%|

Heap demand is exactly unchanged:8B4,257 requests/95,876,945 requested bytes/
17,782,424B incremental peak; 1KB47,796 requests/7,523,247,338 requested bytes/
17,773,224B incremental peak. Reallocations68 in both cases, included in requests.
These are separate hardware/heap cohorts, not primary timings or copying totals.

Object/descriptor/metadata stay byte-identical. Generated text14,604,696B and
native entry stack135,728B remain; whole linked text shrinks128B. Actual bounded
ARM64 helper inspection finds a480→560B host frame (**+80B**) and4,060→3,868B
symbol span. Eight borrowed references are local temporary storage, not a heap
or prepared-owner bank. No fresh full prepared-heap census is claimed.
Versions143/99/38/14 and every component golden/allowance remain unchanged.

Passing selections: runtime356; work/memory33; guarded native CI65; image1;
unchanged components236/connected8/path1; linked16+12+12; scalar12 (maximum
1.003199 under unchanged1.05 gate, pure counts/bytes equal, loop allocation slope0);
frame8 (no request/byte/work increase); workflow132; architecture/type/memory15;
runtime-only feature cfg and full formatting. Original/decoded/restored images,
fresh recursive/current arguments, live aliases, finite funding, rejection,
unchanged prior publication and retry are covered by these maintained controls.
No fresh Core/no_std/all-features/minimal dependency graph is claimed; the existing
expensive knucleotide-body exclusion remains. Hosted CI is not awaited.

Fresh serial profiles cover695 calls/subject at8B and130 at1KB. Current sample
weights1,269/1,319 are not retired instructions. At1KB, generated entry has54/1,319
exclusive weights; other callbacks1,096, tail cleanup109 and private-frame49.
Callbacks include actual algorithm work. Numeric entry, visit/dispatch, debit/TLS
and exit remain prominent. Next count repeated frame geometry/entry-exit work;
do not infer function instruction totals by multiplying sample shares.

One diagnostic disassembly reader incorrectly assumed every byte before the next
symbol decoded as an instruction. Eight padding/non-mnemonic bytes violate that
precondition. The repaired reader bounds addresses and reports that gap; original
failure is retained. No compiler/runtime correctness failure remains in executed
scope. The source-borrow change is retained; PR stays draft for CI/review.

Sixteen diagnostic archives and sidecars (11,383,144 archive bytes, no native
binaries) have verified server digests. A fresh independent maintained replay
verifies1,272 file occurrences/473 paths and five preserved multi-version paths;
all four indexed reports reproduce byte-identically without compilation or
compression. Exact commands, exclusions, raw pairs and report hashes are indexed.

## R107: input scaling and CPU/memory profile; default compiler unchanged

The default implementation stays R105 at `8f6e6a8de9e4928a88f1341dd0538504bd2336e6`.
No new compiler/runtime code is added here. The ownership candidate
`4834caa483d1f79437dace189accdba82eaf9770` is saved separately on
`codex/branch-ownership-r106-experiment` at local checkpoint `1693fe593f`.
It remains held, with no universal speed claim. Its existing implementation,
controls, raw adverse observations and1KB gain are preserved.

### Matched uninstrumented timing of the held candidate

| Input | R105 / reference time | Candidate / reference time | Candidate time change |
|---|---:|---:|---:|
|8B|8.027041×|8.081192×|+0.674603%|
|64B|25.204773×|25.595225×|+1.549122%|
|256B|55.363354×|56.026910×|+1.198547%|
|1,024B|115.013911×|113.918447×|−0.952462%|

The8B primary comparison has3,968 observations and no improving pair. Larger
small cases are a separate cohort:128 observations/version/size, alternating
16-call blocks with the peer blocked on stdin. All eight descriptive blocks agree
with each size's direction; they are not independent experiment replications.
The64/256/1,024B inputs are deterministic prefix-consistent xorshift bytes. Their
exact reference raw bytes and independent inflate pass. Inputs differ from the
8B alphabetic case; the table does not isolate size from content across that row.
Original level12 computation, source/bytecode, search limits, real scratch sizes
and pinned allocator remain. Output capacity is the original bound `n+5` for these
small inputs, cleared/rebound outside evaluate_into; aliases and final managed
bytes/nodes0 pass. No full-scale/long benchmark is run.

### Fresh1KB counters, independent of timing and CPU sampling

| Metric | Reference | R105 | Held candidate |
|---|---:|---:|---:|
|Retired instructions|1,690,807|558,388,496|559,500,546|
|Cycles|973,713.5|112,154,903|111,583,370|
|Allocation requests, including reallocations|84|47,796|43,100|
|Cumulative requested bytes|9,012,988|7,523,247,338|20,234,842|
|Incremental peak live usable heap|9,257,328B|17,773,224B|15,173,352B|

Hardware has32 observations/subject; heap has32 reference and8 Mech observations/
subject after controls are excluded. Heap-instrumented instructions and elapsed
are excluded from hardware and primary timing claims. Counter read overhead is
retained. Candidate instructions rise0.199154% while cycles fall0.509593%.
Cumulative reservation requests are not bytes copied or resident memory. Despite
7.50GB fewer requested bytes, actual incremental heap peak falls only2.60MB and
whole-subject physical footprint stays roughly229MB. Reference footprint is about
14MB. This includes setup/image/runtime ownership, not merely the1KB input;
no exact whole-footprint attribution is established.

### CPU evidence and next bounded target

Fresh serial two-second samples contain1,329 Mech and1,517 reference exclusive
weights. Only65/1,329 Mech weights are generated-entry body;1,112 are other host
callbacks,101 tail cleanup and39 private-frame callbacks. Categories are disjoint;
callbacks include real algorithm work and are not all removable overhead.
Reference's leading opaque generated function has1,362/1,517 exclusive weights;
no source name is inferred from its hash. Sampling weights are not instructions.

The leading Mech leaves are numeric-segment entry85, ordinary visit82, compute
debit70, helper dispatch68, thread-local lookup64, numeric-segment exit53 and tail
cleanup40. Next, count actual repeated source previews/validation/footprint work
within each numeric entry, then test entry-local reuse before adding persistent
caches or changing admission. This targets work now demonstrated at the larger
small input. Shared debit inlining, buffer ownership, shape forwarding and prior
frame changes are already investigated; this report does not re-propose them.

An environmental removal around midnight deleted the pinned source/bytecode and
98 otherwise-clean tracked files. Missing tracked files were restored from exact
HEAD; source assembled byte-identically and bytecode restored from an existing
pinned snapshot. Both longstanding hashes match, with a persistent backup outside
temporary storage. A current-producer regeneration differed and was rejected
before execution. The cause of the removal is not established. Initial reference
platform-path and missing-input diagnostic failures remain in the packet; corrected
builds, counters and profiles pass. There is no compiler semantic change or fresh
compiler/CI qualification claim in this round.

Three diagnostic archives/sidecars (647,188 archive bytes, no native binaries)
are published with verified server digests. Fresh independent downloads verify
276 file occurrences/239 paths; only packet manifests have multiple versions.
All four timing/counter/sample reports replay byte-identically without a compiler
or compressor run. The compact index retains exact download URLs and hashes.

## R105: preserve live immutable shapes when finalizing private appends

Published parent `accd7f478628a073e3f4e0007a830c1f95ccb82a`; production fix
`341bf02918f4573fb518e12c5435ddd93efe0598`, measured compiler
`ec55e3f856306b07d34050b8b95ac0289a28fb5f`. Test-only canonical fixture repair
`05d3a9ce5`; separate guarded CI inventory `ec55e3f85` requires exactly four
private-append tests.

Review identified an introduced R103 correctness bug: a replacement shares its
immutable shape with a live old alias, but private append finalization tried to
mutate that shared owner. Two maintained Core tests reproduce the parent panic
`private shape`, including when the append branch is unselected. Finalization now
copies current parameters into its already admitted independent shape scratch,
performs the original checked length update, and installs that owner consistently.
No new allocation site, budget allowance or publication identity is introduced.

Fresh complete eight-byte elapsed **8.125431×→8.124210× Roc's time**:
2.356375→2.356021ms, common fresh reference0.290ms, −0.015023%. Only one of four
pairs improves; 28 descriptive blocks improve and36 worsen. All3,968 primary /
3,984 total observations remain. **Retain as a correctness repair, without an
elapsed-win claim.**

| Metric | Fresh retained R104 | R105 |
|---|---:|---:|
| Allocation requests |4,257|4,257|
| Reallocations, included above |68|68|
| Requested bytes |95,876,945|95,876,945|
| Incremental live heap peak |17,782,424B|unchanged|
| Instructions, separate cohort |27,013,011|27,014,335.5 (+0.004903%)|
| Cycles, same hardware cohort |7,388,324.5|7,411,965.5 (+0.319978%)|
| Whole linked text |63,678,544B|63,678,672B (+128B)|

The fresh complete census reconciles4,257 requests/2,683 stack groups/1,118 PCs
with the separate heap counters. Object, descriptor and metadata remain
byte-identical; generated text14,604,696B and entry stack135,728B stay fixed.
Logical work42,410,605,541 and versions143/99/38/14 remain. Algorithm, source and
bytecode hashes, allocation sites, real tables/capacities, pinned allocator, input
aliases and evaluate_into timer are unchanged. Exact raw bytes, independent
inflate, output capacity13 clear/rebind and final managed bytes/nodes0 pass;
physical output backing still changes.

Actual Core unit root:52 tests pass, including all four append cases, all unsigned
widths, selected/unselected append, live aliases, abort cleanup and a preparation
from an older value with a different current length. Current Core no_std check
passes locked/offline with warnings-as-errors. Twelve Core-dependent project
crates rebuild coherently under the exact retained selected-operation features;
external dependencies remain frozen. Full current formatting also passes.

Existing checks pass unchanged: runtime356, work/memory33, native CI65, image1;
components236, connected8, path1; linked16+12+12; scalar12 with maximum1.009568
under unchanged1.05 gate, pure-scalar counts/bytes equal and zero loop allocation
slope; frame8 with no request/byte/work increase; workflow132 and architecture /
type/memory15. The expensive knucleotide-body exclusion remains. No fresh full
all-features/minimal dependency graph, full no_std test suite or larger-input run
is claimed. Hosted CI is not awaited; the PR remains draft.

Failure disposition: the introduced parent panic is repaired. One added control
initially provided an unused third schema parameter, which canonicalization
prunes; its initial fixture error remains, and the corrected legal two-parameter
case passes. A proposed native source witness hit an existing exact match-schema
restriction before execution and was removed, without broadening source semantics;
it is not claimed as a runtime reproducer. A separate external diagnostic initially
omitted an existing fixture path; an unchanged-fixture symlink repairs that copy.
One successful linked test's archive write could not update Git metadata under
restricted access; its exact receipt was committed through authorized access.
No timing was rerun or altered for bookkeeping. No unresolved correctness failure
remains in this repair's executed scope.

Fresh isolated profiles retain695 complete calls per subject and1,265/1,308 sample
weights, excluded from primary timing. Current frame-action ancestry128/1,308,
tail cleanup95/1,308 and registration admission/removal23/1,308 are descriptive
samples with overlapping ancestry groups, not retired-instruction partitions.

The earlier registration question is now measured separately: each of two complete
eight-byte diagnostic visits has336 changed replacements,252 may-consume visits,
166 actual consumes, and all166 have an actually exclusive detached registration
in the same account/arena with the old root dead.24 nonreplacement consumes remain
separate. This establishes a166-candidate envelope, not an implemented allocation
saving. Diagnostic timings and allocations are excluded from performance claims.

Publication verifies17 archives and17 sidecars in the existing continuation
release, without native binaries. Fully fresh retrieval verifies1,372 file
occurrences /602 paths and reproduces all four indexed reports byte-identically.

The next main compiler target follows the branch-ownership brief: refine the
old-value query using one exact current Boolean, discard it at invalidating
boundaries, and retain Core's actual alias/account/arena/ownership checks. All four
current scanner append witnesses independently reproduce the impossible false-arm
read. The prepared-schema148-request change is already complete in R104; borrowed
descriptor validation and initial-turn plan overrides remain separate follow-ups.

## R104: reuse prepared schema headers; allocation improvement only

Published parent `a88acd5d87650e12cca6d28015334fa027989594`; production/compiler
`636da5e074add0632f7154f14d8d39848721cd16`. The main implementation is
`22554ca76fdbc9841245e28f696223550b50648c`; its explicit key-type repair is the
measured compiler above. Separate CI inventory: `f51c45f8febc5d4f9855ac970931a2bc3292ab77`.

Binding borrows existing prepared input/output keys and checks the retained
schema entry and template. Uncached live footprint calculation borrows the exact
retained encoded length/key independently of a DATA transaction; current values,
retained arenas, shapes, payload chunks and their original charges remain checked.
Unsigned-buffer no-ops use the existing prepared output key. No new cache, stored
prepared fields, permission, ABI or pricing change is introduced.

Fresh matched complete eight-byte elapsed **8.139130×→8.151457× Roc's time**:
2.3522085→2.355771ms, common fresh reference 0.289ms, **+0.151453%**. Two of four
pairs improve and two worsen; 29 descriptive blocks improve and 35 worsen.
Geometric block ratio 0.997238553 also remains recorded. All 3,968 primary/3,984
total observations are retained and committed. **No elapsed win is established.**

| Metric | Fresh retained R103 | R104 |
|---|---:|---:|
| Allocation requests |4,405|4,257 (−148; −3.359818%)|
| Reallocations, included in requests |162|68 (−94)|
| Requested bytes |95,881,838|95,876,945 (−4,893B)|
| Incremental live heap peak |17,782,424|unchanged|
| Instructions, separate cohort |27,046,906|27,012,942 (−33,964; −0.125574%)|
| Cycles, same hardware cohort |7,434,770.5|7,474,271.5 (+0.531301%)|
| Whole linked text |63,680,080B|63,678,544B (−1,536B)|

Retain as an immutable-metadata/construction improvement. The fresh complete
allocation censuses reconcile exactly with the separate heap counters. Binding
loses 66 requests/2,263B/42 reallocations; footprint loses the same; buffer no-ops
lose 16 requests/367B/10 reallocations. All three targeted encoding families become
zero, and the whole size/kind histogram difference is exactly those families.
Reallocations are already counted in requests. No other size/count bucket changes.

Object, descriptor and metadata are byte-identical; generated text14,604,696B and
entry stack135,728B stay fixed. Payload copying branches are unchanged; this is no
claim of measured payload-copy savings. No stored prepared fields/cache are added;
a separate prepared-heap census is not claimed. Work42,410,605,541 and versions
143/99/38/14 remain. Source/bytecode, level12 algorithm, real tables/capacities,
allocator, input aliases and evaluate_into boundary stay pinned. Exact raw bytes,
independent inflate, output capacity13 clear/rebind and final managed bytes/nodes0
pass. Public output physical backing still changes.

Passing current checks: 20 distinct actual Engine unit tests (two new; header,
current-payload, wrong-contract/arena, equal foreign IDs, overflow and exact finite
denial/spent-prefix controls); runtime356, work/memory33, native CI65, image1;
unchanged components236, connected8, path1; linked16+12+12; scalar12 and frame8;
workflow132 and architecture/type/memory15. The existing scalar gate1.05 is
unchanged; maximum ratio1.025436251. Pure-scalar requests/bytes remain equal and
loop allocation slope remains zero. Frame request/byte/work counts do not increase.
The two new tests have a separately committed CI step requiring exactly two
discovered tests. Local execution uses the retained richer selected-operation
profile; a fresh minimal graph for that CI profile is not claimed.

Five Engine-dependent crates rebuild coherently with the retained optimized
warnings-as-errors flags; Core, Syntax, Bytecode, math and external dependencies
remain frozen. Existing expensive knucleotide-body runtime exclusion remains;
no fresh all-features/no_std graph or larger-input benchmark is claimed. Hosted CI
is not awaited under instruction, and the PR remains draft pending green CI and
clean review.

Failures and diagnostic corrections are retained. The first two builds reject a
missing Option<SchemaKey> closure annotation (introduced, repaired in636da5e074).
Both corrected actual builds pass. A diagnostic disassembler initially counted
following symbols; original excerpts remain, and corrected static counts are
bounded by each symbol's address range. No performance claim uses those initial
counts. Static module matches may include cold/drop helpers; the census supplies
the executed encoding-family proof. No correctness failure remains.

The initial evidence upload hit GitHub's 1,000-asset release cap (HTTP422), an
environmental publication failure. The [continuation evidence release](https://github.com/mech-lang/mech/releases/tag/evidence-native-compression-experiments-2)
retains this round's complete packets. Partial original uploads and original
sidecars remain. Payload archive hashes are unchanged; canonical sidecars and the
round index point to the continuation. The maintained replay harness accepts an
optional per-round release tag while preserving historical default downloads.

The initial fresh replay verified all15 packets and reproduced three reports,
but the single entrypoint omitted the profile report generator. The original
failure and matching reports remain. The entrypoint now invokes its sibling
profile reader; a clean empty-output control passes. A supplemental immutable
packet retains that repair. Fully fresh retrieval verifies all16 packets,
1,402 file occurrences, 663 paths and ten multi-version paths; all four numerical
reports reproduce byte-identically. The prior R103 default-release path also
retrieves16 packets and reproduces its original four reports, and all132 workflow
tests pass again after the per-round release change. No compiler build or
performance observation was changed for these evidence-only repairs.

Fresh isolated profiles retain695 complete calls per subject, excluded from
primary timing. Their1175/1292 sample weights are not retired instructions.
Current registration admission/removal accounts for29/1292 weights, frame-copy
ancestry24/1292 and tail cleanup74/1292; ancestry groups overlap. These are no
removable-instruction estimates. Next, classify actual registration exclusivity
through consuming updates before attempting physical registration transfer;
166 remains a candidate envelope, with node release/reacquisition and fresh data
identities preserved. Shared immutable descriptor contracts follow that bounded
ownership check.

## R103: forward immutable shape owners; allocation improvement only

Production `a43888583dc37806a1c9e39b4b98d6dece3dac52`, measured compiler
`6b064cdeb678f2e7c89e5c2a39143e5ac2f36e79`, published parent
`9bc0fae10a806eeb5e0a8f980f3ce6d6df11ad72`. The Core unsigned reconstruction
constructor retains the existing immutable shape owner without retaining source
payload, root, registration or mutation permission. Both aliased and consuming
updates use ordinary numeric validation and create fresh frozen data identities.
Original conservative admission, checks, work and transfer order remain.

Fresh matched eight-byte elapsed **8.205418×→8.233392× Roc's time**:
2.3467495→2.354750ms, reference 0.286ms, **+0.340918%**. One of four pairs
improves; 23 descriptive blocks improve and 41 worsen. All 3,968 primary/3,984
total observations remain. **No elapsed improvement is established.** Historical
reference ratios and separate counter cohorts must not be combined into a gain.

| Metric | Fresh retained R102 | R103 |
|---|---:|---:|
| Allocation requests |5,077|4,405 (−672; −13.236163%)|
| Requested bytes |95,895,278|95,881,838 (−13,440B)|
| Incremental live heap peak |17,782,744|17,782,424 (−320B)|
| Reallocations |162|unchanged|
| Instructions, separate cohort |27,004,221|27,045,073 (+40,852; +0.151280%)|
| Cycles, same hardware cohort |7,868,378.5|7,401,114 (−5.938511%)|
| Whole linked text |63,675,184B|63,680,080B (+4,896B)|

Retain as an allocation/representation improvement with adverse elapsed and
instructions explicit. Fresh complete traces reconcile all 5,077/4,405 events
with bytes and reallocations. The whole size/kind histogram changes only by
336 eight-byte allocations and 336 32-byte shape owners; both disappear from
unsigned replacement ancestry. Compression payload copy branches stay unchanged;
13,440 requested bytes are metadata, not saved payload copying. Source implies
2,688 parameter bytes no longer cloned; no separate memcpy-volume cohort is claimed.

The complete Object, descriptor and metadata remain byte-identical, generated
text 14,604,696B and entry stack 135,728B unchanged. No prepared fields/cache are
added; a separate exact prepared-heap census is not claimed. Logical work
42,410,605,541 and IR/work/ABI/bytecode 143/99/38/14 stay fixed. Source, algorithm,
tables, capacities, allocator, aliases, cap13 output clear/rebind and evaluate_into
boundary stay pinned. Exact bytes, independent inflate and final managed
bytes/nodes zero pass. Public output backing still changes.

Current actual Core selections pass 48 tests: numeric8 (three new), unsigned
ownership14, string ownership7, identity3, tokens5 and admission11. Fresh locked,
offline no_std Core check passes with warnings-as-errors. Twelve dependent crates
rebuild coherently using unchanged selected features and frozen external dependencies.
Runtime356, work/memory33, native CI65, image1, unchanged components236/connected8/
path1, linked40, scalar12 (unchanged1.05 gate, max1.024748; allocation equality/
zero loop slope), frame8, workflow132, architecture15 and runtime-only GPU cfg pass.
Fresh separate profiles cover695 complete calls per subject.

One introduced test-only audit failure is repaired in `750adeea5`: raw-pointer
casts are replaced by Arc identity comparison. Clean published parent audit
passes; corrected actual Core48 and all15 architecture checks pass. Runtime
production files and measured binaries are unchanged, so measurements are retained.
Original failures/report precondition, repair, parent proof and reruns stay archived.
CI inventory5→8 is a separate commit, matching the three new tests; the old5 guard
is an explicit negative control. No allowlist, warning or allowance is weakened.

The existing giant knucleotide-body exclusion remains. No fresh all-features
dependency graph, full no_std suite, larger-input benchmark or green hosted CI is
claimed. Next: prepared schema-header reuse (148-request candidate, conditional
4,257 remaining); then classify registration reuse eligibility. Immutable contract
sharing/private-owner continuity and the independent frame/zero-argument/source
instruction track remain later work.

## Previous retained checkpoint: R102 — inline paired budget debit; retain measured gain

Compiler **`fa1ffe75e78c7ace538d7d14aa5f3ebc74783166`**, previous published head
`c489e5fc30a4e9cea549cc1de66e2c94ab2f9eb9`, stack base
`eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`. One `#[inline(always)]` keeps the
shared paired-debit checks in their hot callers. No checks, fee, limit, debit
order, worker atomicity or benchmark source change. R101's ordinary `#[inline]`
was ineffective, measured adverse, archived, independently replayed and reverted.

Fresh matched complete eight-byte compression **8.477859×→8.156141× Roc's time**:
2.4331455→2.3408125ms against a0.287ms reference, **3.794800% less elapsed time**.
All four matched pairs and64 descriptive blocks improve. All3968 primary/3984
total observations remain; blocks are not independent experiments. Historical
ratios with different reference clocks are not gain estimates.

| Metric | Fresh retained R100 | R102 |
|---|---:|---:|
| Instructions, separate cohort |27,930,744.5|26,988,708 (−942,036.5; −3.372758%)|
| Cycles, same hardware cohort |7,594,187|7,435,497 (−2.089625%)|
| Requests / reallocations |5,077 /162|unchanged|
| Requested bytes / incremental live peak |95,895,278 /17,782,744|unchanged|
| Generated text / entry stack |14,604,696B /135,728B|byte-identical /unchanged|
| Whole linked text, including runtime |63,674,544B|63,675,184B (+640B)|

Actual host disassembly removes the separate520-byte helper. The library caller
512→736B and observer428→628B now contain the checks directly. The complete
Object, descriptor and metadata stay byte-identical. This is a controlled
R100-to-R102 result; it does not assign all historical R99 instruction growth or
all Result transport to outlining. Fresh isolated profiles cover695 calls per
subject; samples remain separate from retired instructions.

Passes: actual Engine budget108, runtime356, work/memory33, guarded native CI65,
image1, unchanged components236/connected8/path1, linked40, scalar12 (same1.05
gate; max1.016923, pure-scalar allocation equality, loop slope0), frame8,
workflow132, architecture15 and current runtime-only GPU cfg. Five affected
crates rebuild coherently, Core/math frozen, warnings-as-errors unchanged. A
Git metadata permission denial occurred only when recording sixteen already
passing linked cases; the same sealed packet was committed with authorized
access. No program rerun, receipt rewrite or source/allowance change was needed.

Logical work42,410,605,541 and IR/work/ABI/bytecode143/99/38/14 are unchanged.
Algorithm, tables, buffers, allocator, input aliases, cap13 output clear/rebind
and evaluate_into timing stay pinned. Exact bytes, independent inflate and
final zero managed bytes/nodes pass. The existing expensive knucleotide-body
exclusion remains; no full math/all-features/minimal graph, larger-input timing
or green hosted-CI claim. Public output backing still changes.

The new complete allocation trace independently reconciles5077 requests,
95,895,278 bytes and162 reallocations. It finds336 eight-byte replacement
requests (immediate callee deduplicated; exact ancestry/return offsets retained)
and336 32-byte final shape owners. The672-request/13,440-byte shape-forwarding
candidate is confirmed as a current target, not an achieved saving. Next: shape
owner forwarding, then prepared schema headers. The independent instruction
track remains prepared frame transfers, known-zero arguments and entry-local
checked-source reuse.

## Previous retained checkpoint: R100 — move ordinary binding metadata; no elapsed win established

Compiler **`bebe5a245d312796c630de76fc558fbcd2c2f2ff`**, published parent
`c84e307aa64e136bd0d3f6d2fc2bbf869b4e3b25`. The no-witness binding branch now moves
the original BoundCall into its sole kernel consumer. Certified scalar kernels
still retain independent executor/seal metadata; all builder invalidation,
accounting, current-value validation and admission order stay unchanged.

Fresh matched complete eight-byte compression is **8.268488×→8.281428× Roc's
time**, 2.422667→2.4264585ms against a 0.293ms reference: **+0.156501%** pooled
median elapsed. Two pairs improve and two worsen; 25 descriptive blocks improve
and 39 worsen. All observations remain: 3,968 primary and 3,984 total. No elapsed
improvement is established. R99's historical 8.363107× used a different reference
clock and must not be subtracted from this ratio to claim a gain.

| Metric | Fresh R99 baseline | R100 |
|---|---:|---:|
| Instructions, separate cohort |27,950,216.5|27,933,245 (−16,971.5; −0.060720%)|
| Cycles, same hardware cohort |7,697,293.5|7,622,510.5|
| Allocation requests |5,149|5,077 (−72; −1.398330%)|
| Requested bytes |95,900,682|95,895,278 (−5,404B)|
| Incremental peak live bytes |17,782,744|unchanged|
| Reallocations |162|unchanged|
| Object/image/metadata |R99 artifacts|all byte-identical|
| Generated text / entry stack |14,604,696B / 135,728B|unchanged|

Retain this as a small allocation/instruction cleanup, with no elapsed-win or
peak-live-heap claim. Price formulas, logical work 42,410,605,541 and versions
IR/work/private ABI/MBC143/99/38/14 stay unchanged. Source, tables, search limits,
real scratch capacities, allocator, output capacity13 clear/rebind and evaluate_into
timing remain pinned. Mech computes the exact raw bytes and independent inflate
passes; input aliases and final zero managed bytes/nodes remain verified.

Passing current selections: nine actual Engine semantic-seal unit tests, runtime356,
work/memory33, guarded native CI65, image1, unchanged components236/connected8/path1,
linked40, scalar12, frame8, workflow132 and architecture15. Scalar gate remains1.05;
the isolated rerun's maximum ratio is1.010357, pure scalar requests/bytes stay
equal, and loop allocation slope remains zero. The initial scalar batch also
passed but is excluded because a descriptor read overlapped its start. All its
observations are preserved. An auxiliary Engine test driver initially used
metadata-only extern paths and an incompatible filename flag; switching to the
same frozen rlibs fixes the driver without compiler or allowance changes.

Five Engine-dependent crates rebuild coherently with the retained optimized
operation profile and warnings-as-errors; Core/math dependencies remain frozen.
The additional actual Engine unit root uses opt-level1 and the same selected
features. No new Core, all-features, full-math or larger-input qualification is
claimed. The existing expensive knucleotide-body runtime exclusion remains.
Fresh serial before/after profiles cover695 complete calls per subject.

### New instruction finding: runtime budget helper outlining

A current calibrated R97/R99 audit finds exactly9,801 callbacks in both versions.
R99 adds only eight ownership-candidate checks (980→988) across200 matrix updates;
57 buffer visits and260 buffer checks stay equal. Broad additional retirement
scanning is not established as the instruction regression's cause. Four fresh
hardware pairs reproduce about2.05% more instructions in R99, with lower cycles.
Instrumented windows remain estimates, separate from uninstrumented counters.

The budget source file is byte-identical across R97 and R99 (Git blob
`bbcc2b4c97ac2a61ebe6b0a912349c8a85ef33cb`). Actual disassembly finds a732-byte
library charge routine with its checks inline in R97. R99 instead has a512-byte
caller plus a separate520-byte charge_control_account_at_limits routine, with
intermediate Result transport. R100 still has this split. This establishes a
binary/code-generation difference; its exact contribution to the regression
remains unproved. The next bounded experiment explicitly requests inlining of
that helper and measures it separately, retaining every finite check and error
order. Prepared flat frame transfers and per-entry source reuse follow.

Seventeen immutable evidence packets verify1,249 file occurrences across538
paths and29 preserved multi-version paths. Fresh retrieval reproduces all four
reports byte-identically. The first retrieval exposed a missing linked metadata
input; a new replay-input packet fixes packaging without changing compiler source
or measured data. Commands, all raw cohorts, profiles, assembly excerpts and
diagnostic readers remain archived. Native binaries remain local. Local
validation is complete; hosted CI is cancelled by instruction and the PR stays
draft pending green CI and clean review.

## Previous retained checkpoint: R99

### Retire finished ordinary collection sources

Complete eight-byte level-12 compression improves from **8.470875× to 8.363107×
Roc's time**, **1.272220% less elapsed time** against the fresh matched baseline.
All four pairs improve; 61 descriptive blocks improve and three worsen. All
adverse observations remain. There are 3,968 primary and 3,984 total observations.
Blocks within a pair are descriptive, not independent experiments. The reference
is 0.289ms; Mech is 2.448083→2.416938ms. The target is 1×. Larger-input throughput
remains unestablished. R97's historical 8.378794× used a different reference clock;
it is not the baseline of this comparison.

| Metric | Fresh retained R97 baseline | R99 |
|---|---:|---:|
| Instructions, separate 32-call cohort |27,389,660.5|27,947,877 (+2.038056%)|
| Cycles, same hardware cohort |7,732,271.5|7,622,493 (-1.419745%)|
| Allocation requests including reallocations |5,153|5,149|
| Requested allocator bytes |97,997,834|95,900,682 (−2,097,152B)|
| Incremental peak live allocator bytes |17,782,744|unchanged|
| Reallocations |162|unchanged|
| Generated compressor text |14,604,696B|byte-identical|
| Machine entry stack |135,728B|unchanged|

The independent reference hardware cohort retires 1,364,372 instructions and
921,002 cycles. Reference allocation measurements retain 95 requests,
9,021,923 requested bytes and 9,257,328 incremental peak live bytes. These are
observed implementation results, not semantic minima. Hardware, heap, samples
and primary elapsed are separate cohorts. Requested capacity is not initialized
copy volume or live memory. Peak live heap does not improve. Hardware-cohort RSS
is 319,963,136→319,700,992B and footprint is 228,968,128→228,919,040B; these include
prepared code, libraries and harness state, rather than just compressor buffers.

## What changed

The Engine's old-value graph query previously treated CollectionSource as opaque,
so an ordinary completed comprehension could keep its input alive indefinitely.
The query now places ordinary materialized source reads at initialization,
header, body, latch, exit and collection finish. Finish matters: shape-preserving
finalization reads the arena source after the generator releases its source pin.
The same graph traversal preserves backedges, fresh definitions and read/write
order. Unsupported consumers remain conservative. Private-frame eligibility and
Core's current same-account/exclusive ownership check remain in place.

The real tree trace finds aliases 882 and 908 before each update. Previously 882
remained after retirement and forced a copy. After the fix neither remains; all
four 262,144-entry U16 updates reuse their 524,288-byte backing. Four complete
calls verify all 16 updates. This establishes exactly **2,097,152 initialized
bytes no longer copied per complete call**. The trace's arena scan is diagnostic
only; no production owner scan, source-field specialization or algorithm change
is introduced. The selected site's static inventory grows 45→47, adding 882 and
1857. Later readers and externally retained aliases still prevent unsafe reuse.

An independent Mach-O section audit finds the entire generated text byte-identical:
SHA-256 `65099c20101e43aaa1e355c0758a40316fc7ed6aa3ad358b1e0b5c8ba07020d4`.
Object/image identity and ownership metadata change. IR143 reconstructs the
new proof independently on image restoration; rejected R98's IR142 is not reused.
IR/work/private ABI/MBC are **143/99/38/14**, scalar ABI1. Strict prior-image
rejection remains. Price formulas are unchanged; actual logical work falls
42,449,879,781→42,410,605,541 as already-accounted copying/ownership paths change.
Logical work is not a hardware instruction count.

## Instruction tradeoff and fresh profiles

Whole-call instructions increase by 558,216.5 despite lower elapsed time and
cycles. This adverse result remains explicit; R99 is retained for its elapsed
and physical-copy reduction, without an instruction-reduction claim.

Separate nonnested windows around the four tree helpers observe raw instruction
totals 473,722→87,110.5 and cycle totals 209,935→32,007.5 per full call, over 32 calls
per subject. Empty read-pair calibration averages 5,006 instructions; raw window
totals include counter overhead and are not subtracted from the uninstrumented
whole-call cohort. No complete per-function instruction ledger is established.
The whole-call increase remains unattributed outside that bounded diagnostic.

Fresh serial profiles cover 695 complete calls per subject and 1,302/1,293
exclusive sample weights. Copy/move ancestry falls 41/1,302→15/1,293 weights;
frame-action ancestry is 131/1,302→113/1,293. These are overlapping ancestry unions
of CPU samples, not retired instructions or exact timer fractions. Runtime
helper, admission, native-segment and cleanup costs remain prominent.

## Revisions and qualification

Published parent: `64705927265fa331735ef39cdb3185471015054e`.
Retained comparison compiler: `20f002f4ff5ff2186888b7efc8517261756b425e`.
Regression/CI inventory: `bdf3df43a2c0d5dc9483ae3ade9793f85de2a0f7`.
Compiler fix: **`5c219ed220879b05ee147bdf5d3f85680311eae2`**.
Test-only final-root publication correction: `348101721a2870ab2f503508491e7d3395c94b32`.
Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.

The new regression fails against the retained Engine at missing source eligibility,
then passes after the fix. Original/decoded artifacts and original/restored images
cover selected/unselected mapping, current arguments, a later published reader,
external input aliases, unused failing results, assertion/index error order,
unchanged prior publication and retained accounting through failure and retry.
Final managed bytes/nodes remain zero.

Passing current selections: runtime356, work/memory33, native CI65, image1,
unchanged components236, connected8, path1, linked16+12+12, scalar12, frame8,
workflow132 and architecture/type/memory15. CI's guarded inventory is updated
64→65 and actually lists and executes all65. Scalar allowance stays 1.05; the
maximum ratio is 1.018146. Pure scalar requests/bytes stay equal and loop allocation
slope remains zero. Frame request/byte/work counts do not increase. Component
expected values and allowances are unchanged.

Engine, Stdlib, Compute, Runtime and GPU are coherently rebuilt with the same
selected operations, optimized flags and warnings-as-errors. Core, Syntax,
Bytecode, math libraries and external dependencies stay frozen. No fresh Core,
full-math, minimal-dependency or all-features graph qualification is claimed.
The existing expensive knucleotide-body runtime exclusion remains explicit.
Formatting and architecture/catalog/type/memory checks pass.

Preserved preparation failures: an initial diagnostic reporting prefix of16 was
insufficient for45 static candidates and was replaced by a complete inventory;
this was no production resource denial. An initial narrow-arithmetic witness was
outside the retained lowerable subset and was corrected to the compressor's wide
arithmetic/cast pattern. The new test's initial publication expectation was
corrected to the language's final-root rule. No existing component golden changes.
No current correctness/test failure remains; the instruction increase remains
an adverse performance result.

## Fidelity, evidence and next bounded work

Source SHA-256:
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
Bytecode SHA-256:
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
Level12 retains the 262144-entry U16 tree, 1500513-word U32 cache, 305257-entry U64
path capacity, depth300, nice258 and original near-EOF rules. Search and skip stay
separate. Algorithm, tables, source allocation sites, allocator and evaluate_into
timing stay pinned. Exact raw output is `[75,76,74,78,73,77,75,207,0,0]`, checked by
independent inflate. Input aliases and output capacity13 clear/rebind remain.
Physical public-output backing still changes; physical output reuse is not proved.

Every measured cohort is sealed and committed. Fourteen uploaded evidence
archives verify 1,032 file occurrences, 463 paths and two preserved multi-version
paths. Fresh independent retrieval reproduces all four numerical reports
byte-identically without compilation or compression. The initial automatic upload
rejection was resolved by explicit authorization of evidence uploads. Native
Objects and executables stay local; descriptor/excerpt/hash retrieval does not
establish native-Object retrieval. No larger-input run is performed.

Next, explain the remaining instruction cost using exact callback/ownership
visits, remove the no-witness BoundCall clone, then stage immutable constructor
binding/planning and prepare flat frame-transfer geometry. Current funding,
values, error order and publication remain dynamic. The rejected R98 direct child
seed experiment stays reverted; its 112 executed handoffs were slower and are
not proposed again as unexplored work.
