# Scoped numeric plan facts

Fresh level-12 source-to-Object compilation takes **40.234–40.253 seconds**,
compared with **42.004–42.463 seconds** for its matched control on the Apple M3
Air. Mean complete compiler time falls **4.71%**, native time **5.07%** and
retired instructions **4.32%**. Source change
`3225c7101b7e2142aad8b8f9e3f38ddf4143445b` shares descriptive plan inventories
across numeric analysis while preserving every emitted program byte.

Atom resolution, branch inspection, recurrence bounds and captured-coordinate
analysis previously constructed fresh `NativeProgramFacts` repeatedly for the
same immutable plan. Their first definition/identity lookup rebuilt and later
dropped full inventories. Numeric helpers now borrow the existing facts through
their derivation scope. Preparation reuses its current facts; compact phase
discovery and capture analysis retain facts through their local iteration.

These are descriptive definition and identity inventories. Predicate assumptions,
traversal results, loop contexts, recursion limits and execution permissions keep
their original computation. Facts remain tied to an immutable plan borrow;
nothing is serialized, stored globally or retained across plan rewrites. The
runtime's finite prices, current-view gates and eligibility remain unchanged.

| Balanced run order | Parse | Frontend | Native | Preparation within native | Numeric generation within native | Compiler total | Whole process |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Control 1 | 2.460 s | 9.107 s | 30.814 s | 11.236 s | 5.358 s | 42.463 s | 43.634 s |
| Candidate 1 | 2.486 s | 8.202 s | 29.479 s | 10.623 s | 4.683 s | 40.253 s | 41.333 s |
| Candidate 2 | 2.461 s | 8.304 s | 29.389 s | 10.636 s | 4.673 s | 40.234 s | 40.311 s |
| Control 2 | 2.447 s | 8.279 s | 31.195 s | 11.329 s | 5.354 s | 42.004 s | 42.203 s |

Preparation falls **5.79%** and numeric generation **12.66%**. These intervals
use independently stamped stage messages and are included in native time.
All four processes use the same 52 flags, source and toolchain. No builds,
tests, fixture comparisons or CPU sampling overlap primary timing. Candidate
bootstrap takes 79.29 seconds, separately excluded. Both initial launches retain
about one second of extra process startup outside compiler phase timers.

Both pairs improve, but four observations do not establish a statistical
population. The first control's longer 9.107-second frontend interval remains
in the cohort; this change does not establish a frontend improvement or explain
all of the complete-compiler difference. Candidate maximum RSS has a higher
mean: **7,596,392,448–7,597,654,016 bytes**, versus
**7,395,672,064–7,600,275,456 bytes** in control. The ranges overlap and the cause
is not isolated. No whole-compiler memory saving is claimed.

Source, canonical bytecode, native Object and complete metadata match
byte-for-byte in all four observations. The Object retains SHA-256
`4ddc16b053bda6256656e0d5cbf95ba60cab31b0911b9aadf526b66e1f3cab67`;
metadata retains
`0813625d0178e329bfd780c8a0d5f714bbd15a23727d22505172d1136bd0b44d`.
Runtime implementation and generated instructions stay unchanged. No new
compression-runtime timing campaign is claimed for this compiler analysis change.

All **61 focused tests across five suites** pass. Six complete additional
programs also retain byte-identical source, bytecode, Object and metadata:
nested numeric phases, enum-state children, private fixed nested buffers,
changing captures, array windows and multiple children. These are compilation
comparisons, without new execution claims. The initial fixture launcher treated
the enum-state helper fragment as a complete program; both compilers rejected
it identically. The corrected source follows the maintained nested-phase test's
assembly. All initial failures and corrected outputs remain archived.

A separate five-second frontend CPU window points to dimension-context cloning,
remapping and canonicalization during expression lowering. Its elapsed is
diagnostic only. Parsing
plus frontend work still exceeds ten seconds, independently of native work.

The [compact result](../../benchmarks/mech-deflate/native-profile/numeric-facts-compile-apple-m3-20261113.json)
and [receipt](../../benchmarks/mech-deflate/native-profile/numeric-facts-compile-apple-m3-20261113.archive.json)
pin the [public raw evidence](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/level12-numeric-facts-compile-apple-m3-20261113.tar.gz):
213 files, 6,160,965 compressed bytes, SHA-256
`76edf5d192db092d8a3c6b9a796da0dfa1363b5d90dfe37d07ea2ed52169a4f6`.
Independent public retrieval verifies the complete manifest and reproduces the
compact result byte-for-byte with `python3 replay.py`. Compiler executables
stay local; source, Object and metadata evidence remain retrievable.

The [preceding SSA storage change](native-ssa-sparse-compile-profile.md) reduced
source CFG construction. The subsequent
[inline dimension-remapping change](native-frontend-affine-compile-profile.md)
reduces full compilation to about 39–40 seconds; the sub-10-second goal remains open.
