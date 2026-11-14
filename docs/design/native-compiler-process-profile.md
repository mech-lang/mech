# Current compiler pipeline versus libdeflate and Roc

The latest matched level-12 Mech cohort uses the opt-in `compiler` build profile
and takes **17.420–17.638 seconds per complete
source-to-Object process**, after the [lexical index](native-lexical-index-compile-profile.md)
and [reusable visitation storage](native-thread-visits-compile-profile.md) changes,
followed by [selected liveness definitions](native-liveness-definitions-compile-profile.md),
[borrowed schema finalization](native-borrowed-schema-compile-profile.md),
[indexed liveness graphs](native-indexed-liveness-compile-profile.md),
[independent constant bounds](native-constant-bounds-compile-profile.md),
[prepared-owner reverse edges](native-visit-predecessors-compile-profile.md),
[retired SSA value compaction](native-compact-values-compile-profile.md),
the [optimized compiler build](native-compiler-lto-compile-profile.md),
[shared pending schemas](native-shared-schema-compile-profile.md),
[scope-sized validation](native-scope-graph-compile-profile.md),
[proof-local traversal](native-local-traversal-compile-profile.md),
[packed helper liveness](native-helper-liveness-compile-profile.md),
[index-range definition facts](native-index-facts-compile-profile.md),
[selected exit definitions](native-exit-definitions-compile-profile.md),
[nested parser predictions](native-nested-parser-compile-profile.md),
and [exit liveness at the requested start](native-point-liveness-compile-profile.md).
The measured peer builds take **0.41–0.48 seconds for libdeflate** and
**2.35–2.50 seconds for Roc** on the same Apple M3 Air. The fresh latest Mech
cohort uses 61 flags, including the independently qualified private overwrite-result
reuse option. Peer builds remain their earlier pinned observations; only Mech was
remeasured in this latest cohort. The prior 60-flag parser cohort stays in its own
report, and the original profile below retains its
52-flag pins and
**36.52–37.14-second** process observations. These configurations are not pooled.
Mech's largest problem
is the amount of semantic, analysis and temporary IR work before final machine
code emission. A faster parser is necessary, but cannot by itself reach the
sub-ten-second full-compilation goal.

The original 52-flag Mech observations below are both retained candidates from the
[qualified rootless-dimension cohort](native-rootless-dimension-compile-profile.md),
using compiler source `6d7390e59616e52c5d1dce30f7514cf89167731a` and binary SHA-256
`736b0fc0facdf7ed30d42ac501fe2fdee84a1d2fc9a511cd0676559b97184430`.
Peer builds, eight Mech stage CPU samples across three complete compiles, and a
separate complete main-entry IR dump are fresh observations on 2026-11-14.
After this packet was frozen, PR 973 advanced to `79e0649f0088cee334ed7d071f68ad085b112525`, adding an opt-in `MECH_CARRY_REDUCTION_BOUND` optimization. It is absent from this pinned 52-flag profile. The branch was updated before the next experiment.

Diagnostic compiles are excluded from the primary timing ranges. All Mech source,
bytecode, Object and complete metadata bytes match the qualified program; peer
instrumented outputs also match their clean outputs.

## Clean compilation and instrumented peer stages

| Implementation | Complete process observations | Output boundary |
| --- | ---: | --- |
| libdeflate | 0.484, 0.408 s | Encoder translation unit, Clang O2, arm64 Object |
| Roc | 2.500, 2.347 s | Runtime input, fixed level 12, uncached speed build, one job, arm64 static archive |
| Mech | 37.138, 36.520 s | Complete source through compact native Object, same 52 qualified flags |

These are individual serial observations, not population estimates. Source/tool
retrieval, compiler bootstrap and host linking are excluded. C and Roc support
other compression levels; this Mech program supports level 12 only. Implementation
sizes remain 5,485 C lines, 6,187 Roc lines and 1,930 Mech lines, including comments
and blanks and excluding wrappers. The [size comparison](native-compressor-size-compile-comparison.md)
explains source scope and pins.

Clang's separate trace reports **47 ms frontend** and **461 ms backend**, including
**210 ms optimization** and **250 ms code-generation passes**. Its frontend also
includes preprocessing and semantic analysis: there is no isolated C parser timer.
The instrumented complete process takes 0.574 seconds.

Roc's separate timing build reports **56 ms type checking**, including **3 ms
summed parsing**, **343 ms shared lowering/evaluation**, **1,260 ms specialization**,
including **1,188 ms LIR passes**, and **673 ms LLVM optimization/emission**. LLVM
IR generation takes 11 ms. The complete instrumented process takes 2.371 seconds.
A separate one-second CPU sample includes `range_prove` and `use_order`. Rounded
Roc module accounting and Mech's lossless document-tree stage have different
boundaries; they do not establish a precise parser speed ratio. Nested timings
must not be added twice.

## Mech's current time budget

| Stage | Clean observed seconds | Fresh CPU evidence |
| --- | ---: | --- |
| Parsing | 2.45–2.48 | Continuation dispatch; mutable journal pages; tree sink and copying |
| Frontend | 6.17–6.25 | Cloning dimension declarations; dimension canonicalization/normalization |
| Native canonical lowering | 3.76–3.81 | Lexical template tree traversal; operand schema lookup; temporary schema vectors |
| Native preparation | 8.51–8.59 | Numeric exit liveness queries; repeated lexical schema lookup and allocation |
| Numeric helpers: lowering, optimization and emission | 4.68–4.69 | Numeric traversal analysis; ordered sets; required-value liveness |
| Main source CFG/SSA construction | 2.45–2.55 | Nonlocal SSA variable lookup; definition maps; temporary block parameters |
| Main SSA finalization | 0.140–0.141 | Independently stamped interval; not separately sampled |
| Main CFG cleanup | 4.98–5.08 | Ordered block sets; phi removal; local phi simplification; threaded edges |
| Main machine-code generation | 1.98–2.00 | Register allocation, live ranges, copying and alias resolution |

The native rows are **inside** the 27.28–27.69-second native total. Additional
small intervals include flow proof, operation recipes, module/layout setup and
Object finalization. Compiler phase totals are **36.10–36.39 seconds**; complete
process time also includes startup and output work. Numeric-helper time already
includes those helpers' machine-code emission, so the final two-second main-entry
backend interval is not all machine-code work in the compiler.

Samples identify functions within finite stage windows; their self counts are
not full-stage percentages. The coordinator's `__ulock_wait` samples represent
waiting. The archive retains requested sample durations, observed stage triggers,
latest stage before each sample and complete raw/demangled stacks. Profiler command
completion includes report processing after the requested sampling window.

## The main entry constructs far more IR than survives

| Main-entry CLIF census | Before cleanup | After cleanup |
| --- | ---: | ---: |
| Blocks | 460,631 | 73,516 |
| Instructions | 1,263,768 | 141,307 |
| Block parameter declarations | 536,754 | 46,426 |
| Edge value arguments | 1,092,865 | 75,864 |
| Serialized SSA aliases | 19,546,177 | 0 |

Cleanup removes **84.0% of blocks**, **88.8% of instructions**, **91.4% of block
parameters** and **93.1% of edge arguments**. A second streaming census checks that
all **19,546,177 alias IDs are distinct**. Following the formatter's alias forest,
**17,041,123** reach block parameters, **2,466,524** reach integer zero,
**2,075** reach other integer constants and **36,455** reach other instruction
results. No alias roots are unclassified. This is more than repeated printing of
the same alias ID.

The pre-cleanup text is 584,030,150 bytes, largely alias records; the post-cleanup
text is 7,277,566 bytes. Text size is not an estimate of live compiler memory. The
source plan has 41,039 blocks and 22,460 values, so the expansion occurs during
native emission and SSA construction. The census covers the main entry, not all
numeric helper functions. It establishes substantial transient work; it does not
prove every eliminated block or phi can safely be omitted during construction.

## Optimization order

1. **Resolve immutable lexical sites once.**
   `native_collection_lexical_template` currently starts from the owner's root for
   each lookup. It scans operations and reconstructs operand-schema vectors even
   for preceding operations that are not the requested site. This is prominent in
   both canonical lowering and preparation. An artifact-owned descriptive index
   can remove repeated traversal while keeping factory selection, contracts and
   later current-visit proofs. Preserve demand/error order on malformed artifacts;
   eagerly rejecting a later invalid declaration would change earlier queries.

2. **Stop cloning and renormalizing semantic dimensions unnecessarily.**
   The frontend still spends most sampled work cloning declaration arrays and
   canonicalizing dimensions after the accepted rootless improvement. Reuse
   immutable canonical data at its owner or avoid materializing it when a result
   needs none. The prior combined scalar-constructor/helper extraction regressed
   and is [retained as rejected evidence](native-resolved-scalar-compile-trial.md);
   it does not justify another unmeasured general helper refactor.

3. **Construct less temporary SSA and control flow.**
   Find which lane definitions and unsealed joins produce the 19.5 million aliases.
   Reuse proven dominating immutable values, seal helper joins as soon as all
   predecessors are known, and avoid generating already-proven redundant branches.
   The emitter already seals auxiliary blocks after each source block; the work
   is to improve construction within operations and value transport, not simply
   add a missing end-of-function seal. Existing variable-definition maps already
   combine a dense prefix and sparse fallback. Replacing them wholesale misses
   the more fundamental overproduction. Keep checked errors, ownership, finite
   admission and efficient direct scalar emission intact.

4. **Share exact liveness/traversal facts within immutable plans.**
   Preparation samples still show `live_before` and `live_query`; numeric helper
   construction shows traversal analysis and ordered-set insertion. Reuse facts
   only with their actual plan/edge/query identity, with bounded fallback where
   needed. Never reuse a proof across a different graph, visit or admission.

5. **Reduce parser continuation/checkpoint work with the existing grammar.**
   Fresh samples still show base continuation dispatch and journal page mutation.
   The parser source tree is identical to the accepted prediction cohort, whose
   [prior counters](native-parser-prediction-profile.md) record 58.8 million
   base/operator frame pops, 22.7 million checkpoints and 18.2 million rewinds.
   These counters were not rerun for this profile. Candidate work includes
   extending existing grammar-derived prediction into nested calls and reducing
   journal copying on failed alternatives. Preserve trees, diagnostics, logical
   fuel/limits, resumability and Unicode behavior. No grammar or language change
   is needed to investigate those paths.

These are measured locations of work and concrete source-based hypotheses, not
promised savings. The next low-risk experiment is lexical-site lookup reuse;
the larger architectural opportunity is preventing temporary SSA/CFG expansion.
A complete parser rewrite or a backend replacement is not supported as the first
step by this profile. Sub-ten-second full compilation remains unachieved.

## Reproducible evidence

The [compact result](../../benchmarks/mech-deflate/native-profile/compiler-process-profile-apple-m3-20261114.json)
and [archive receipt](../../benchmarks/mech-deflate/native-profile/compiler-process-profile-apple-m3-20261114.archive.json)
pin the full raw CPU samples, traces, commands, flags, source inputs and pins,
clean outputs, full IR dumps and streaming census/replay logic. Download the
receipt's public archive, verify with the maintained `experiment.py verify`, then
run `python3 replay.py` in the extracted directory. Replay uses neither compilers
nor generated executables and requires none of the original scratch paths.

The included rootless-dimension receipt is its corrected v2. Its original RSS
narrative was wrong; both raw candidates have higher maximum RSS than both raw
controls, with nonoverlapping ranges. The original public archive remains available
for provenance. No memory saving or new compressor runtime-speed measurement is
claimed here. No source or runtime-code change is part of this profiling report.

The subsequent [lexical-site index qualification](native-lexical-index-compile-profile.md)
implements the first target on the updated PR 973 branch. It reduces complete
compiler time 12.6% to 31.49–31.53 seconds, preserving all emitted bytes between
matched subjects. The observations and CPU windows above retain their original
52-flag source/configuration pins.

The later [ownership-join SSA trial](native-ssa-forwarding-compile-trial.md)
reduces compile time but changes generated code and repeatedly slows some
compressor cases; it is reverted. The retained visitation change reduces cleanup
18.4% and compiler time 3.85% while preserving every emitted byte. Both newer
cohorts use the same complete 54-flag runtime configuration for their controls
and candidates. Their complete archives are [published in benchmarks](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md),
verified and replayed byte-identically offline.

The next liveness diagnostic observes 790 distinct exact query contexts, a
median 284 selected sites and 23,850 materialized definition facts per query.
Filtering unused definition facts retains original validation and generated
bytes while reducing preparation 11.39% and full compiler time another 2.52%.
Its complete-process observations are 29.61–30.03 seconds; these retain their
own 54-flag pins and are not pooled with the older cohorts above.

PR 973 subsequently adds an independently measured nested-reduction runtime
transformation. The [combined-head integration and memory trace](native-current-pipeline-memory-profile.md)
reproduces its pinned native Object exactly. Its continuous samples locate the
largest observed RSS in frontend semantic analysis at 6.913 GiB, close to the
host process peak. These two memory diagnostics are excluded from primary timing
and do not establish a new primary matched range.

Borrowing schema environments during finalization then reduces frontend time
24.33%, full compiler time 4.08% and peak RSS 7.62% in a new matched ABBA cohort.
Every emitted byte matches that integrated runtime baseline. Complete process
observations are 28.764–28.769 seconds versus 29.822–30.228 seconds for control;
this is the matched range before the later graph-indexing qualification. All
four observations, 357 passing tests, no-std check and 14 fixture comparisons
are in the fifth repository evidence packet. These fresh complete compiles use
no cached Object.

The next [liveness graph indexing qualification](native-indexed-liveness-compile-profile.md)
updates to terminal-append source `ada4fa0062984fa9687e179f72f6d6b3acbad58e`.
Both compilers use the same 55 flags and reproduce that runtime's qualified
Object. Query-local array indices preserve the exact graph, ordered validation
and fixed point while reducing preparation 41.49%, full compiler time 7.67%
and retired instructions 6.69%. Complete fresh processes take 26.248–26.254
seconds versus 27.724–28.978 seconds for control, with every generated byte
unchanged. This is the matched range before independent-constant-bound
canonicalization. The same packet retains both rejected backend-reuse prototypes
and all contrary observations. Frontend, helper generation and main cleanup remain substantial;
full fresh compilation still exceeds ten seconds.

The following [constant-bound canonicalization qualification](native-constant-bounds-compile-profile.md)
uses a fresh complete-compile CPU profile on the retained graph-indexing control.
It removes dependency/normalized-bound arrays only when every bound is already
a constant, keeping all original validation and canonical parameter order.
Frontend time falls 42.09%, compiler time 8.51%, instructions 10.21% and maximum
RSS 14.41%. Complete fresh processes take 23.508–23.517 seconds versus
25.195–26.034 seconds for control, with every emitted byte unchanged.
Both subjects retain the same 55 runtime flags;
no earlier configuration is pooled with them. Native analysis and code generation
now dominate the remaining time, and the full ten-second goal remains open.

The following [reverse-edge qualification](native-visit-predecessors-compile-profile.md)
replaces whole-program scans in numeric traversal with prepared-owner predecessor
lookups. Each scope and header is still checked fresh, with the same first
refusal. Compiler time falls 1.87%, numeric generation 11.39% and instructions
3.34% in the matched 55-flag cohort. Complete source-to-Object processes take
23.087–23.092 seconds, with all generated bytes unchanged. Four production-module
tests include 7,464,960 exhaustive reference comparisons; fourteen fixture pairs
also match. Its 360-file eighth packet retains both screens, all observations,
adverse phase differences, commands, outputs and pure offline replay.

The subsequent [SSA value compaction qualification](native-compact-values-compile-profile.md)
reduces the post-producer main value table from 20,787,570 IDs to 1,241,393 while
preserving every generated byte. The latest PR 973 short-reduction optimization
is integrated and enabled identically for both subjects. Eight current 60-flag
runs show 2.21% lower compiler time, 9.44% lower cleanup time including the added
compaction, and 1.95% fewer instructions. Complete processes take 22.672–22.879
seconds. Elapsed ranges overlap and peak RSS is 0.04% higher. The 907-file ninth
packet keeps a separate eight-run historical 55-flag cohort, all seven passing
tests on each base, both sets of fourteen fixture pairs, all screens and failed
prototypes, two CPU profiles, every adverse observation and pure offline replay.
The full ten-second target remains open.

The next [compiler-build qualification](native-compiler-lto-compile-profile.md)
adds an opt-in `compiler` Cargo profile, identically in the production workspace
and focused harness. It inherits release with Thin LTO and one codegen unit;
ordinary release/runtime profiles remain unchanged. Eight matched 60-flag fresh
compiles take 21.830–22.051 seconds, saving 3.99% compiler time, 4.11% whole-process
time and 4.33% instructions while preserving every generated byte. Frontend time
is an adverse 2.76% higher. The observed 608.6-second compiler bootstrap is a
separate cost. Its 514-file tenth packet contains both screens, all fourteen
fixture pairs, every primary observation, a fresh CPU profile and two discarded
SSA construction trials. The ten-second target remains open.

The latest [shared-schema qualification](native-shared-schema-compile-profile.md)
removes repeated pending-draft copying and pure canonicalization in the frontend.
Eight matched 60-flag compiles take 21.196–21.415 seconds: whole-process time falls
2.46%, frontend time 18.85%, instructions 3.27% and peak RSS 1.38%. Native time is
an adverse 0.46% higher. All generated bytes match, and the final source passes
66 unique frontend/codec tests. Its eleventh packet retains every observation,
both screens, fourteen fixture pairs, source patches and integration failures.
The ten-second target remains open.

The next [scope-topology qualification](native-scope-graph-compile-profile.md)
allocates current-visit validation rows and edges only for each reachable scope.
All checks and the conservative admission limit remain. Eight matched 60-flag
compiles take 20.368–20.777 seconds: process time falls 3.44%, preparation 15.83%
and instructions 2.37%. Peak RSS is an adverse 0.008% higher with overlapping
ranges. All generated bytes match, and 152 engine tests pass. The twelfth packet
retains five diagnostic control CPU windows, all measurements, fourteen fixture
pairs, screens, corrections and pure offline replay. The ten-second target remains open.

The latest [proof-local traversal qualification](native-local-traversal-compile-profile.md)
removes unrelated-loop enumeration and full-program omitted-predecessor scans
from numeric helper analysis. Eight matched 60-flag compiles take 19.741–20.163
seconds: process time falls 2.77%, helper generation 18.01% and instructions 4.88%.
Peak RSS is an adverse 0.028% higher; parser and frontend means also rise. All
output bytes match. Five topology tests include exhaustive query comparisons;
fourteen fixture pairs also match. The thirteenth packet keeps all observations
and a separate current helper CPU window. The ten-second target remains open.

The latest [packed-helper-liveness qualification](native-helper-liveness-compile-profile.md)
shares the existing bounded packed transfer kernel with exact helper effects.
Eight matched 60-flag compiles take 18.884–19.110 seconds: process time falls
3.74%, helper generation 24.15% and instructions 4.30%. Preparation, source CFG
and compaction means are adverse; memory ranges overlap. All generated bytes
match. Eighteen focused tests and fourteen fixture pairs pass. The fourteenth
packet also retains the same pre-existing bare runtime-only lint failure on both
parent and candidate, without waivers. The ten-second target remains open.

The latest [index-range facts qualification](native-index-facts-compile-profile.md)
keeps descriptive definitions for one exact analyzer lifetime, with lazy first
demand and unchanged range/guard calculations. Eight matched 60-flag compiles
take 18.691–18.903 seconds: process time falls 1.61%, helper generation 10.74%
and instructions 2.06%. Cleanup is an adverse 1.34% higher; memory is slightly
higher with overlapping ranges. All program and fixture output bytes match.
The fifteenth packet retains all observations and static source comparisons.
The ten-second target remains open.

The latest [exit-definition qualification](native-exit-definitions-compile-profile.md)
retains the original full-inventory validation demand and visits only values read
by each exact exit query. Sixteen matched 60-flag complete compiles take
18.690–19.107 seconds: process time changes
-0.27%, preparation -0.60% and
instructions -0.29%. Overall compiler time is effectively flat;
two balanced blocks get slower and two faster. All phases and adverse measurements appear
in the report. All generated bytes match; seventeen focused tests and fourteen
fixture pairs pass. The sixteenth complete packet includes every observation,
source patch, failed test setup and pure offline replay. The ten-second goal
remains open.

The latest [nested-parser qualification](native-nested-parser-compile-profile.md)
applies existing grammar-derived predictions to nested base calls, with exact
logical work and continuation-owned cache storage. Eight matched 60-flag complete
compiles take 18.467–18.483 seconds: parsing changes
-13.87%, process time -2.20% and
instructions -2.29%. Preparation and cleanup are adverse; all
phases and counters remain. All generated bytes match. There are 526 passing
document-module units, 1,667 identical full parse comparisons and fourteen fixture
pairs. Its seventeenth complete packet also includes seven separate CPU windows.
Those point to SSA variable lookup/storage and CFG cleanup as further targets.
The ten-second goal remains open.

The subsequent [local-span SSA storage trials](native-ssa-span-trials.md) reject
both layouts. In separate eight-observation cohorts, source CFG construction
increases 5.53% and 5.07%, compiler phases 0.61% and 0.94%, and instructions
0.55% and 0.48%. Both compiler-time blocks are slower for each variant. Tests,
complete IR comparisons and every program/fixture output byte match, but no
candidate change is retained. The eighteenth complete packet preserves both
trials. Shared word homes have common entry definitions; deferring unnecessary
zero seeds is a source-based next hypothesis with no saving credited.

The subsequent [word-home trials](native-word-home-compile-profile.md) do not
establish a useful saving: deferred zero seeds leave source CFG time effectively
unchanged, and an independent indexed-home repeat does not confirm its original
improvement. Across sixteen indexed-home observations, source CFG changes
+0.107% and instructions -0.031%; repeat blocks are mixed. Both candidates are
restored to the retained source. Every level-12 Object matches. The nineteenth
complete packet keeps all favorable/adverse observations, tests and fixture
outputs. Exact liveness-query graph construction and per-node read-set allocation
are the next preparation investigation; no saving is credited yet.

The latest [bounded exit-liveness qualification](native-point-liveness-compile-profile.md)
computes only the requested start result for broad graphs with sparse reads, with
a fixed search budget and original packed/sparse fallback. Exact graph and
validation construction stay unchanged. Eight fresh 61-flag full compiles take
17.420–17.638 seconds: compiler phases change -5.08%, preparation -32.52% and
instructions -4.97%. Both balanced blocks improve; all generated bytes match.
Compiler RSS is an adverse +1.93%. Sixteen focused tests, 790 independently
re-solved query graphs and fourteen fixture pairs pass. The twentieth complete
packet preserves all observations, kernel budgets, outputs and capture failures.
The ten-second goal remains open.
