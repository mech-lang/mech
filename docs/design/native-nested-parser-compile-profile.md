# Nested canonical parser predictions

Complete fresh level-12 source-to-Object compilation takes
**18.467–18.483 seconds** on the Apple M3 Air,
compared with **18.887–18.903 seconds** for the matched
control. Parsing changes **-13.87%**, compiler phases
**-1.99%**, whole-process time **-2.20%**
and compiler instructions **-2.29%**. Both balanced blocks
improve. The sub-ten-second compilation goal remains open.

The parser previously applied its existing grammar-derived prediction only
when entering a base continuation. Nested calls still built checkpoints, rule
frames and failed alternatives. The same bounded prediction now runs before
charging any eligible pending base call, including nested calls. Its logical
work already includes that call frame, so no extra charge is added. The active
continuation borrows its own existing cache and returns it to the same parser
with its storage after completion.

This removes unnecessary execution within the current parser architecture.
Grammar productions, shared base/operator specifications, Unicode handling and
language stay unchanged. Source comparisons confirm that all prediction admission
and summary calculations are byte-identical except for borrowing the cache
explicitly. No semantic-result cache or cross-visit proof is added.

The source is `ee77e992e12f1880901c2e6b0745e23fcf7f1fe6`, measured against control
`414693ce793b1ea1e99a6dcac34cb2533d9c856c` with build base `0876d380c0cffcc0501bede2cc833553a48c4e8c`.
The exact source patch, dependency pins and executable hashes are retained.

## Matched full compilation

Eight serial observations run ABBA then BAAB with identical 60 runtime flags,
toolchain and opt-in `compiler` build profile. Each starts a fresh process and
compiles the full source through parsing, frontend, canonical artifact, bytecode,
native Object and complete metadata. SSA value compaction and its cost remain
in both subjects. Builds, tests, standalone probes, fixtures, CPU sampling,
hashing, copying and archive work do not overlap the primary observations.

The table contains arithmetic means of all aggregate comparison fields. Phase
and nested-stage intervals overlap and are not additive. Adverse values and all
individual observations remain in the report/result; unrelated-phase changes
are not attributed to the parser. Eight observations are not a population estimate.

| Measurement | Control mean | Candidate mean | Change |
| --- | ---: | ---: | ---: |
| Complete process | 18.8914 s | 18.4764 s | -2.20% |
| Compiler phases | 18.7092 s | 18.3365 s | -1.99% |
| Parsing | 2.4245 s | 2.0882 s | -13.87% |
| Catalog | 0.0376 s | 0.0370 s | -1.72% |
| Frontend | 2.5428 s | 2.5413 s | -0.06% |
| Artifact | 0.0263 s | 0.0258 s | -1.89% |
| Bytecode encoding | 0.0092 s | 0.0092 s | +0.64% |
| Native | 13.6681 s | 13.6344 s | -0.25% |
| Canonical lowering | 0.0986 s | 0.0983 s | -0.27% |
| Preparation | 2.9965 s | 3.0088 s | +0.41% |
| Numeric helper generation | 2.1282 s | 2.1216 s | -0.31% |
| Source CFG construction | 2.3601 s | 2.3382 s | -0.93% |
| Cleanup including compaction | 3.3877 s | 3.4107 s | +0.68% |
| CFG cleanup | 3.2187 s | 3.2413 s | +0.70% |
| Value compaction | 0.1690 s | 0.1694 s | +0.25% |
| Retired instructions | 277.3476 G | 270.9905 G | -2.29% |
| Maximum RSS | 5.9117 GB | 5.9119 GB | +0.0031% |
| Peak footprint | 5.8813 GB | 5.8815 GB | +0.0033% |

Preparation is an adverse +0.41%, cleanup including
compaction +0.68%, bytecode encoding
+0.64% and memory slightly higher with overlapping ranges.
The two balanced-block compiler mean changes are
-2.37% and
-1.61%.
Every candidate whole-process, compiler, parse and instruction observation is
below every matched control observation.

The separate observed compiler-executable rebuild takes
**403.580 seconds**, using existing Cargo caches.
It is excluded from Mech compilation and is not a matched cold-bootstrap
comparison. Both screen runs remain in the packet; the first candidate launch's
startup overhead is excluded from the primary cohort.

## Parser equivalence and program speed

All **526 production document-module unit tests pass**. Existing ASCII-head,
Unicode, fuel/event/nesting, recovery, incremental and stream controls remain.
New comparisons exercise work slices 1, 4095, 4096, 4097, 8192 and unbounded,
resource limits, input sealing, suspended state and return of the nested cache
to the same parser.

A separate **1,667-case** original/candidate comparison preserves complete trees,
node/token identities, diagnostics, public statistics and restart records
byte-for-byte. Both 4,684,363-byte result streams have SHA-256
`d5ad4ed9d87c64d3295f31eed97d167f6ba5b41ac1a3852b3e00ff1af8e8b2e7`.
The full compressor retains 1,967,640 parser steps, 434,637 emitted events,
92,316 final nodes and zero recovery bytes.

Standalone unit/differential probes use the same coherent, hash-pinned release
dependency set for both subjects; their document module imports mech-core only
for the SourceLocation record. They compile the actual current document-module
source. The full production compiler independently builds with its current
locked compiler-profile dependencies. Standalone timing screens (about
2.44–2.48 seconds versus 2.06–2.10 seconds) are kept separately from the
production compilation cohort above.

Every source, MBC, Object and complete metadata byte matches the qualified
program in both screens and all primary observations. Fourteen successful
fixture pairs also match all four files, including numeric phases, nested and
changing captures, array windows and Bool/U64/I64/F64 scalar controls. Generated
instructions and metadata are identical. There is no new compressor-runtime
latency campaign and no scalar execution machinery is retired. Fresh shape,
factory, visit and ownership checks, checked errors, finite admission, runtime
accounting and backend settings remain on their existing execution paths.

Probe setup corrections remain visible: duplicate Cargo artifacts first stopped
dependency selection; mixed Thin-LTO/non-LTO dependencies caused two linking
failures before using the coherent release set. A new test's mutable reference
to a constant was replaced with a local allowance variable. No warning allowance
or production policy change was added. Final warnings-as-errors probe builds,
all 526 tests, production build and formatting checks pass.

## Current CPU work and next target

A separate fresh full compile captures seven requested one-second CPU windows
at five-millisecond intervals, triggered by stage markers. These diagnostics are
excluded from timing qualification and also reproduce every output byte. The
following are built-in top-of-stack self counts at least five. Waiting states
are retained in raw samples and separated from active symbols. Windows can
cross stage boundaries; these counts are not exclusive elapsed-time shares.

| Window | Selected active self counts |
| --- | --- |
| Parse | Journal chunk mutation 24; base dispatch 20; memory move 13; tree sink 9 |
| Frontend | Dimension canonicalization 35; declaration-vector cloning 32; schema embedding 9 |
| Preparation | Exact exit liveness query 79; memory move 9; ordered-map insertion 7 |
| Helpers | Freeing 17; definition inventory initialization 9; CFG recomputation 8 |
| Source CFG | SSA nonlocal lookup 64; variable-definition insertion 24; sparse-map growth 15 |
| Cleanup | Ordered pipeline 20; constant-phi removal 16; parameter removal 13; dominance 9; verifier 8 |
| Backend | Register-bundle allocation 21; allocator initialization 17; memory move 16 |

The next hypothesis is improving **SSA variable-definition storage**, preserving
every existing lookup/insert result, pending-None state, traversal, alias creation
and ordered enumeration. Its current dense region begins at block zero, so late
locally clustered IDs remain hashed unless the whole prefix is dense. A bounded
local span may avoid hash growth without changing SSA construction. Sparse IDs,
allocation bounds and ordered enumeration both below/above a span need explicit
controls. This is unimplemented and no saving is credited here. The previously
rejected SSA-forwarding shortcut remains rejected.

Batch removal of already-selected block parameters is a secondary candidate.
Existing callers already delete rightmost first; batching must preserve raw
Value IDs, retained parameter order, aliases and branch arguments while improving
on that baseline. Neither current validation nor execution checks are removed.

## Reproduce the results

The [compact result](../../benchmarks/mech-deflate/native-profile/nested-parser-compile-apple-m3-20261114.json),
[receipt](../../benchmarks/mech-deflate/native-profile/nested-parser-compile-apple-m3-20261114.archive.json)
and [complete packet index](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
pin every observation, source and patch, input, command, output, correction,
CPU window and license. Compiler/probe executables are excluded and hash-pinned.
The maintained verifier checks the published repository copy, manifest, safe
paths and every file. Pure offline `python3 replay.py` reconstructs the result
and checks stored outputs/samples without a compiler, benchmark execution,
original scratch paths or network.

The subsequent [local-span storage trials](native-ssa-span-trials.md) reject both
variants: restoring the original record size still slows source CFG construction
and compiler work. Their full measurements remain published. Source inspection
finds shared word homes initialized together in entry; investigating unnecessary
zero seeds is the next unimplemented hypothesis. This does not alter the retained
parser results above.
