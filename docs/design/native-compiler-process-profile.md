# Current compiler pipeline versus libdeflate and Roc

The current level-12 Mech program takes **36.52–37.14 seconds per complete
source-to-Object process**, compared with **0.41–0.48 seconds for libdeflate** and
**2.35–2.50 seconds for Roc** on the same Apple M3 Air. Mech's largest problem
is the amount of semantic, analysis and temporary IR work before final machine
code emission. A faster parser is necessary, but cannot by itself reach the
sub-ten-second full-compilation goal.

The Mech primary observations are both retained candidates from the
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
