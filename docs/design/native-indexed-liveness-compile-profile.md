# Use array indices in liveness graphs

Query-local node indices reduce native preparation **41.49%**, full compiler
time **7.67%** and retired instructions **6.69%** on the level-12 compressor.
Complete fresh source-to-Object processes take **26.248–26.254 seconds**, versus
**27.724–28.978 seconds** for control. Every source, bytecode, Object and complete
metadata byte matches the current qualified runtime baseline.

Source `619386076692d819778fdc5c53579b3b7604b062` changes three preparation/test
files on base `ada4fa0062984fa9687e179f72f6d6b3acbad58e`. Before this change,
each cold-continuation query allocates source-site trees for edges, reads and
definitions, then searches sorted source IDs again while constructing solver
predecessors. The new representation assigns a local array index during the
same DFS traversal, keeps edges and reads in parallel rows, and materializes
ordered definition vectors for selected rows. Dense and sparse solving use
those indices directly.

Every original action/terminator read, predicate fact, root stop and ordered
definition lookup remains, including lookups for unread values. Repeated
records retain set semantics. The transfer still preserves local reads before
killing successor facts. Traversal/validation order remains unchanged; solver
row order may follow DFS, and the same fixed point escapes as a sorted set.
No graph, predicate context, ownership decision, semantic proof or liveness
answer is cached. Grammar, language, finite admission, current-visit ownership
checks, direct scalar emission and backend optimization level are unchanged.

## Matched compilation on the updated PR

PR 973 advanced with the independently qualified
[terminal-append runtime change](native-terminal-byte-appends.md). Both frozen
compilers are rebuilt on that source and use the same **55 runtime flags**,
including `MECH_TERMINAL_BYTE_APPENDS=1`. The clean control reproduces its
published Object
`38b194ec2c931fbd5d584ace06bdaba1efb59381e64eef1d662a5f2f12bd96b4`.
The candidate reproduces that Object and complete metadata exactly.

Four serial ABBA observations use identical host-statistics access. Builds,
tests, separate screens, fixtures, profiles and archive work are excluded;
none overlap the primary cohort. Each observation starts a fresh compiler
process and compiles the complete source. No cached Object or decoded-only
substitute is used.

| Measurement | Control | Candidate | Mean reduction |
| --- | ---: | ---: | ---: |
| Compiler source through Object | 27.598, 28.891 s | 26.022, 26.134 s | 7.67% |
| Complete process | 27.724, 28.978 s | 26.254, 26.248 s | 7.41% |
| Native preparation | 6.203, 6.626 s | 3.730, 3.775 s | 41.49% |
| Retired instructions | 432.739, 432.941 billion | 404.363, 403.416 billion | 6.69% |
| Maximum RSS | 6.520, 6.515 GiB | 6.506, 6.521 GiB | 0.07% |

Both candidate compiler/process observations are below both controls. Peak RSS
is essentially flat, with overlapping ranges; no memory saving is claimed.
Peak footprint is slightly higher. Parse, frontend, helper-generation, source-CFG
and cleanup elapsed observations are slower on average despite unchanged work
in those stages. All contrary counters remain in the compact result without
causal attribution to this representation change. Four compiles are descriptive
observations, not a population estimate. The sub-ten-second goal remains open.

The initial 54-flag screen takes 26.430 seconds and passes 14 fixture pairs
before the PR update. Those observations are separate diagnostics. They are
not pooled with the current 55-flag qualification. Cargo test builds the
compiler binary too; subsequent 0.1-second Cargo build checks are cached compiler
build checks, not cold compiler-bootstrap measurements. Complete build/test logs
and durations remain separate from program compilation timings.

## Qualification

All **13 solver tests** pass on both bases. New differential coverage compares
dense and sparse solving against the original reference on 128 graphs, four
node orderings and 17 starts per graph. It includes cycles, duplicate edges,
definition duplicates, sparse labels, word boundaries, maximum u32 value IDs
and forced dense-to-sparse fallback. Existing reference, projection,
read-before-definition and propagation-bound checks remain.

All four outputs also match between subjects for six complete array/enum
fixtures and eight checked Bool/U64/I64/F64 scalar programs on both bases.
The assembled state fixture retains its earlier assembly provenance. The
current screen and all four primary observations preserve Object and complete
metadata bytes. No new runtime-latency campaign is claimed: this compiler
optimization emits the same program for the current runtime.

## Rejected backend work stays in the evidence

Two earlier prototypes try to reuse full, equal Cranelift stencils while
applying each function's current parameters and relocations. The hash-indexed
screen finds 46 reuses among 344 phase bodies. Retaining only the preceding body
finds 123 reuses and passes seven focused tests, including relocation retargeting,
exact-input matching, malformed IR, bounded retention and diagnostic output.
Both preserve all four complete program outputs.

The adjacent prototype's matched 54-flag cohort saves only **1.80%** of numeric
generation. Compiler time is **0.62% worse** and retired instructions are
essentially flat (0.041% lower). Whole-process time falls 1.43%, but the first
control has a much larger interval outside the compiler timer. These mixed
observations do not establish a useful overall compiler saving. Both prototypes
are rejected and restored before the retained graph change; no backend cache
remains in production.

The packet retains their full patches, source snapshots, compiler pins, all
observations and passing tests. Four transient build/test corrections remain;
one test-oracle correction initializes the intended target before simulating a
full retention budget. A failed cohort launcher points at an older directory
and stops on the existing-output assertion before launching a compiler. Its
log and script remain, and prior evidence is unchanged. None of these corrections
is treated as a timing observation or converted into a passing test.

## Evidence

The [compact result](../../benchmarks/mech-deflate/native-profile/indexed-liveness-compile-apple-m3-20261114.json),
[archive receipt](../../benchmarks/mech-deflate/native-profile/indexed-liveness-compile-apple-m3-20261114.archive.json)
and [complete 771-file packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/mech-indexed-liveness-20261114.tar.gz)
are published in benchmarks. The repository copy verifies every manifest/file
hash and Git blob identity and reconstructs the compact result byte-identically
using pure offline Python. The [evidence index](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
provides extraction and replay commands, including all five earlier packets.
