# Bounded old-value reachability

The level-12 compressor now compiles from fresh source to native Object in
**47.079–47.382 seconds**, compared with **50.140–50.634 seconds** for its matched
control on the Apple M3 Air. Mean compiler time falls **6.26%** and retired
instructions fall **6.30%**. Preparation falls from **14.033–14.192 seconds to
11.309–11.348 seconds**, a **19.73% reduction**. Source change
`ee9dc32f50c2000295e2d95f5d1cffdd175c791a` preserves the source language,
ownership decisions, runtime implementation and every emitted program byte.

The ownership query formerly walked forward through the control graph for each
value/frontier pair. Proving that an old value has no later reader often scanned
the rest of the program. The new query reuses read/definition inventories and
generation-tagged traversal scratch within one immutable plan borrow. After
128 visited blocks, an unfinished query can instead derive a reverse liveness
bitset: start at that value's readers and stop at fresh definitions. Subsequent
queries reuse that value's bitset.

Action reads still precede writes; a write kills the old value before its
terminator. The consuming frontier's own terminator, backedges and ordinary
collection boundaries retain their existing treatment. Unsupported ownership
roles remain conservative. Reverse indexing first requires a valid whole graph;
an unrelated invalid edge must not introduce an error into an early successful
forward query. Failed indexing resumes the saved ordered traversal. A 32 MiB
index cap also retains forward traversal, without restricting program size or
changing eligibility. Nothing is serialized or reused across plan rewrites.

| Balanced run order | Parse | Frontend | Native | Preparation within native | Compiler total | Whole process |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Control 1 | 2.430 s | 8.778 s | 39.351 s | 14.192 s | 50.634 s | 50.826 s |
| Candidate 1 | 2.496 s | 8.438 s | 36.366 s | 11.309 s | 47.382 s | 48.484 s |
| Candidate 2 | 2.448 s | 8.215 s | 36.338 s | 11.348 s | 47.079 s | 47.247 s |
| Control 2 | 2.457 s | 8.434 s | 39.165 s | 14.033 s | 50.140 s | 50.374 s |

Both pairs improve. These four serial observations use the same 52 flags,
source and toolchain, with no overlapping builds, tests, counters or sampling.
Bootstrap builds take about four minutes each and are excluded. Preparation
uses independently stamped stage-message boundaries; it is included in native
time. Unrelated frontend variation remains in the complete totals. The first
candidate launch has about 0.99 seconds of extra process startup outside the
compiler phase timers. Four observations do not establish a statistical
population.

Compiler maximum RSS is **7,600,701,440–7,765,082,112 bytes** for the candidate,
versus **7,380,140,032–7,762,968,576 bytes** for control. The candidate mean is
higher and the ranges overlap. No whole-compiler memory improvement is claimed.

All source, revision-14 canonical bytecode, native Objects and complete metadata
files are byte-identical across both the final cohort and the preceding pilot.
The 1,237,696-byte Object retains SHA-256
`4ddc16b053bda6256656e0d5cbf95ba60cab31b0911b9aadf526b66e1f3cab67`;
the complete metadata retains
`0813625d0178e329bfd780c8a0d5f714bbd15a23727d22505172d1136bd0b44d`.
Generated instructions and runtime source are unchanged. This is compiler
qualification, without a new compression-runtime timing campaign.

All **61 tests across five focused suites** pass, including eight old-value
query tests. They compare against the original ordered traversal over every
two-block graph/read/write combination, several forward budgets, loops,
malformed edges, exhausted cache budgets, suspended worklists and epoch wrap.
A separate diagnostic compares **14,314 queries on the compressor's actual
41,039-block plan**: every result agrees. It checks every non-scalar action-read
value at its action frontier, a superset rather than an exact census of
preparation callers.

In that relaxed-atomic diagnostic, original forward visits are **315,143,609**;
the candidate performs **908,638 forward plus 4,517,622 reverse visits**, a
**98.28% reduction in visited blocks**. This is a work count, not an elapsed
percentage. Its 6,501 reverse indexes occupy 33,389,136 bytes within the
33,554,432-byte cap. Diagnostic elapsed is unqualified.

The packet retains an earlier adaptive attempt that waited for cumulative
forward work equal to a whole graph before indexing. Its two candidate compiles
take 49.486–49.767 seconds; preparation improves only 2.97%. It is superseded,
with all four observations preserved. Two corrected test-driver compilation
failures also remain in the evidence; neither enters a measured cohort.

The [compact result](../../benchmarks/mech-deflate/native-profile/owner-read-compile-apple-m3-20261113.json)
and [receipt](../../benchmarks/mech-deflate/native-profile/owner-read-compile-apple-m3-20261113.archive.json)
pin the [public raw evidence](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/level12-owner-read-compile-apple-m3-20261113.tar.gz):
161 files, 10,154,660 compressed bytes, SHA-256
`786cfaf16a2edc2cb70405de2450d6357ae9b313a03e070d766e211f690744eb`.
Independent public retrieval verifies every manifest entry and reproduces the
compact result byte-for-byte with `python3 replay.py`. Compiler and test
executables stay local; source, Object and metadata evidence are retrievable.

Parsing remains about 2.5 seconds after the
[predictive parser change](native-parser-prediction-profile.md). Frontend work
takes about 8.3 seconds and native compilation about 36.4 seconds. Full
compilation remains about 47 seconds; the sub-10-second goal remains open.

The later [sparse SSA lookup change](native-ssa-sparse-compile-profile.md)
reduces full compilation to about 42 seconds with the same generated program.
