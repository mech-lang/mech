# Rejected word-home compiler experiments

Neither experiment establishes a useful compilation saving, and neither is
retained. Both candidates were reverted to the measured base
`51b18380c07cbbdd4e0c0ffd472c3820289dbc0d`, the retained nested-parser compiler
plus benchmark publication commits. PR 973 advanced during publication to
`c1280f3ecb` (private overwrite-result reuse). These observations predate that
update and use the original 60 flags; they do not qualify its new opt-in runtime
flag. The sealed packet preserves the report as it stood before that update.
Fresh compiler measurements are needed for the combined current branch. The
sub-ten-second goal remains open.

Deferring entry zero seeds leaves source CFG construction effectively unchanged
(**+0.015%**) in eight observations. Compiler phases change **-0.465%**, but
instructions only **-0.078%**, with substantial timing movement in other stages.
A direct-indexed word-home table initially appears faster, but a separate repeat
does not confirm that result. Across all sixteen indexed-home observations,
source CFG construction changes **+0.107%**, compiler phases **-0.377%**, complete
process **-0.185%** and instructions **-0.031%**. The repeat has mixed block
directions. Favorable original results and adverse repeat results both remain.

The prior retained compiler qualification remains
[18.467–18.483 seconds](native-nested-parser-compile-profile.md). The controls
below are fresh observations of that same production compiler, kept with their
own matched candidates. They are not pooled with earlier experiments.

## Changes tested

The first candidate introduces an explicit zero-default constructor for shared
word homes. Inputs and actual constants remain defined in entry; other homes are
declared without an eager zero definition, relying on the existing SSA builder's
lazy-zero behavior when a read has no definition. The complete constant-domain
validation loop remains explicitly in the compact emitter. Existing local-memory
constructors retain their fallback behavior. A missing initializer in those old
constructors means constant/local load, so simply returning `None` there would
have been incorrect. This new initialization policy is rejected.

The second candidate restores the original initialization and changes only the
private word-home lookup representation. It first builds the original ordered
map, preserving every declaration and definition in order, then converts a
sufficiently dense span into indexed optional cells. Dense capacity is bounded
at twice the number of homes; sparse and overflow spans retain the map. Reads,
writes, spills, reloads, extent tracking and constructors are byte-identical
after normalizing the private storage wrapper and equivalent ordered slot
inventory. This representation change is also rejected after measurement.

## Matched full compilation

Both trials use identical 60 runtime flags, locked dependencies, toolchain and
opt-in `compiler` build profile (Thin LTO, one codegen unit). Each observation
starts a fresh process and compiles complete source through Object and metadata.
The first trial has one serial ABBA/BAAB cohort of eight observations. The second
has an original eight-observation cohort and one disclosed independent balanced
repeat with unchanged source/binaries. All sixteen observations and four block
directions enter its aggregate; original/repeat summaries also remain separate.
Screens are excluded. Builds, tests, corpus preparation, copying, hashing and
archive work do not overlap primary timing.

Arithmetic means follow. Positive changes mean more time/work/memory. Phases and
nested stages overlap and must not be added. Small cohorts and tiny counter
changes do not establish population estimates or isolate individual causes.
Whole-process measurement also includes the profiling driver's polling tail;
compiler-phase time is reported separately.

### Deferred zero seeds: eight observations

| Measurement | Control mean | Candidate mean | Change |
| --- | ---: | ---: | ---: |
| Complete process | 18.5302 s | 18.3180 s | -1.1452% |
| Compiler phases | 18.3067 s | 18.2215 s | -0.4653% |
| Parsing | 2.0919 s | 2.0814 s | -0.5003% |
| Catalog | 0.0367 s | 0.0369 s | +0.5236% |
| Frontend | 2.4483 s | 2.4746 s | +1.0748% |
| Artifact | 0.0259 s | 0.0262 s | +1.1337% |
| Bytecode encoding | 0.0093 s | 0.0093 s | +0.3172% |
| Native | 13.6940 s | 13.5925 s | -0.7413% |
| Canonical lowering | 0.0984 s | 0.0991 s | +0.7481% |
| Preparation | 3.0249 s | 2.9949 s | -0.9916% |
| Numeric helper generation | 2.1364 s | 2.1214 s | -0.7016% |
| Source CFG construction | 2.3679 s | 2.3683 s | +0.0150% |
| Cleanup including compaction | 3.4016 s | 3.3620 s | -1.1636% |
| CFG cleanup | 3.2364 s | 3.1974 s | -1.2044% |
| Value compaction | 0.1652 s | 0.1646 s | -0.3659% |
| Retired instructions | 269.8425 G | 269.6325 G | -0.0778% |
| Maximum RSS | 5.9119 GB | 5.9117 GB | -0.0039% |
| Peak footprint | 5.8814 GB | 5.8813 GB | -0.0026% |

Compiler-time block changes: -0.5723%, -0.3581%.

### Indexed word homes: all sixteen observations

| Measurement | Control mean | Candidate mean | Change |
| --- | ---: | ---: | ---: |
| Complete process | 18.6098 s | 18.5754 s | -0.1848% |
| Compiler phases | 18.4343 s | 18.3648 s | -0.3768% |
| Parsing | 2.0803 s | 2.0819 s | +0.0769% |
| Catalog | 0.0369 s | 0.0377 s | +2.0320% |
| Frontend | 2.5439 s | 2.5520 s | +0.3183% |
| Artifact | 0.0257 s | 0.0258 s | +0.2608% |
| Bytecode encoding | 0.0092 s | 0.0092 s | +0.2354% |
| Native | 13.7377 s | 13.6576 s | -0.5831% |
| Canonical lowering | 0.0987 s | 0.0977 s | -1.0120% |
| Preparation | 3.0224 s | 2.9862 s | -1.1993% |
| Numeric helper generation | 2.1378 s | 2.1304 s | -0.3470% |
| Source CFG construction | 2.3511 s | 2.3536 s | +0.1072% |
| Cleanup including compaction | 3.4530 s | 3.4178 s | -1.0172% |
| CFG cleanup | 3.2828 s | 3.2469 s | -1.0934% |
| Value compaction | 0.1702 s | 0.1710 s | +0.4528% |
| Retired instructions | 271.0106 G | 270.9268 G | -0.0309% |
| Maximum RSS | 5.9122 GB | 5.9119 GB | -0.0056% |
| Peak footprint | 5.8817 GB | 5.8815 GB | -0.0033% |

Compiler-time block changes: -0.8414%, -1.3780%, +1.2175%, -0.4839%.

### Original indexed-home result versus repeat

| Measurement | Original eight: change | Repeat eight: change | All sixteen: change |
| --- | ---: | ---: | ---: |
| Compiler phases | -1.1103% | +0.3651% | -0.3768% |
| Complete process | -1.1988% | +0.8438% | -0.1848% |
| Source CFG construction | -1.0607% | +1.2931% | +0.1072% |
| Retired instructions | -0.0503% | -0.0115% | -0.0309% |

The original indexed-home cohort's 1.11% compiler improvement is not presented
as the final result. The aggregate retains adverse parsing, catalog, frontend,
artifact, encoding, source-CFG and compaction movement. Preparation and cleanup
improve in the aggregate, but no isolated causal attribution to home lookup is
established. A tiny instruction reduction and mixed repeat do not justify this
additional representation under the current goal.

Separate observed compiler-executable rebuilds take **275.515 seconds** and
**271.459 seconds** using existing Cargo caches. They are excluded from Mech
compilation and are not a matched cold-bootstrap comparison. Both first-launch
screens, including their startup overhead, remain in the packet.

## Correctness, machine code and tests

The deferred-zero candidate passes three focused Words tests. Four eager versus
deferred comparisons cover branches, loops, unwritten padding lanes and
spill/reload boundaries; both functions verify and their machine code is
identical in each case. A 4,096-home control checks that unused defaults emit
neither entry loads nor initializers, and that an actual undefined read resolves
to zero. The existing boundary alias-region check passes.

The indexed-home candidate passes two focused tests, including 90,000 randomized
lookup probes plus full populated/neighbor queries. They cover empty, singleton,
dense, holey, sparse-outlier, high-base and near-maximum slot sets, ordered
inventories, below-base rejection, overflow fallback, the capacity bound and
boundary memory flags. Every constructor and method body is statically compared
with the original after normalizing only the storage wrapping/enumeration.

The focused harness compiles the exact production Words module and its own tests
against coherent hash-pinned compiler-profile Cranelift libraries. It discloses
minimal unused CodePlan/error/offset interface glue. The separate full locked
compiler builds use all actual production interfaces. No production warning or
validation waiver is added.

Every source, MBC and Object byte matches the retained level-12 program in both
screens and all 24 timed observations. Indexed homes also preserve every complete
metadata byte. Deferred zero changes only two diagnostic metadata fields:
`cleanup.before_instructions` 1,263,768 to 1,263,770 and
`cleanup.after_instructions` 141,307 to 141,309. Every other metadata value is
identical. Its pre-compaction SSA values increase from 20,787,570 to 20,787,574,
and retained values from 1,241,393 to 1,241,395. Indexed homes preserve both counts.

Fourteen successful fixture pairs per trial preserve every source, MBC, Object
and complete metadata byte, including numeric phases, nested/changing captures,
array windows and Bool/U64/I64/F64 controls. No new compressor-runtime latency
campaign is claimed: the level-12 Objects are identical. An 81-case runtime
corpus was prepared and independently checked for a possible Object change, but
no runtime worker was built or executed. Its preparation, inputs and pins remain
in the raw packet. No scalar machinery is retired. Grammar, language, fresh
shape/factory/visit/ownership checks, constant validation, checked errors,
resource admission, accounting and backend settings remain unchanged.

## Next investigation

These trials do not materially reduce the current compilation work. Preparation
still costs about three seconds. The existing separate CPU windows identify
exact liveness queries as substantial work. Source inspection shows that each
query walks its selected graph and allocates a `BTreeSet` of reads for each
visited node before selecting definitions and invoking the solver.

The next step is to measure graph construction/read collection, definition
selection and solving separately, including query/node/read counts. Any change
must preserve the exact current graph, root stops, predicate context, first
validation failure and definitions. Reusing a source-level conclusion across
unrelated visits is not authorized. No saving is credited for this unimplemented
investigation. The previously rejected SSA-forwarding shortcut remains rejected.

## Inspect and replay

The [compact result](../../benchmarks/mech-deflate/native-profile/word-home-trials-apple-m3-20261114.json),
[receipt](../../benchmarks/mech-deflate/native-profile/word-home-trials-apple-m3-20261114.archive.json)
and [packet index](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
pin raw observations, original/repeat summaries, measured patches, full candidate
sources, commands, tests, outputs, preparation records, executable hashes and
licenses. Candidates are the base `51b18380c07cbbdd4e0c0ffd472c3820289dbc0d`
plus their separate measured patches; neither candidate is retained.

The maintained verifier checks the repository archive copy, manifest, safe paths
and every file. Pure offline `python3 replay.py` reconstructs candidate sources
from the base and patches, checks stored tests, SSA counts, allowed diagnostic
metadata changes, output bytes, fixture outputs and every numerical summary.
It needs no compiler, benchmark execution, original scratch paths or network.
Replay checks stored measurements; it does not rerun them.

The subsequent [61-flag preparation profile and solver change](native-point-liveness-compile-profile.md)
measures the repeated exact-query work and retains bounded liveness at the
requested start. It preserves graph/validation construction and every output
byte, reduces matched compiler time 5.08%, and records compiler RSS +1.93%.
The earlier word-home trials above remain rejected and are not pooled with it.
