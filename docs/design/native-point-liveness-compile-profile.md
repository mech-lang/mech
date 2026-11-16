# Exit liveness at the requested start

Complete fresh level-12 source-to-Object compilation takes **17.420–17.638 seconds**
on the Apple M3 Air, compared with **18.265–18.677 seconds** for the matched
control. Compiler phases change **-5.08%**, complete process **-4.84%**,
preparation **-32.52%** and retired instructions **-4.97%**. Both balanced blocks
improve. Every source, MBC, Object and complete metadata byte matches. Compiler
maximum RSS is an adverse **+1.93%**. The sub-ten-second goal remains open.

The existing solver computes liveness at every node even though these callers
return only the set at one start node. Broad graphs whose reads are sparse now
try a bounded reverse search for each read value, stopping that value's search
as soon as it reaches the requested start. A local definition stops an incoming
successor fact; a read at the defining block itself remains live before that
definition. Reverse edges use flat offset/parent arrays, and generation stamps
avoid clearing a visited array for each value. Source value IDs never size an
allocation.

Eligibility requires at least 1,024 nodes and no more retained read occurrences
than nodes. The search counts node and edge inspections and permits at most
64 times the node count, capped at 16,777,216 steps. Exhaustion retains the
existing packed/sparse solver. An unusual eligible query can pay this bounded
probe cost before falling back; the selected 299 benchmark queries all finish
within budget. No source-specific IDs or program patterns select the path.

The complete exact graph, source reads, predicate pruning and definition
validation still run first in the original order. The change is confined to the
pure liveness result calculation. Whole-inventory definition validation, root
stops, current predicate context and the original first refusal remain. There is
no cross-visit proof cache. Grammar, language, scalar emission, checked errors,
shape/factory/visit/ownership checks, finite admission and accounting stay on their
existing paths.

The source is `ab4e6d8bc6f0128b1899c292838c36470295424a`, measured against
`9cb89a2a8e2d3c2947b688e869bdbbc6686b93f6`. The Git commit patch is byte-identical
to the tested and measured patch.

## Current configuration and measurement

These are fresh measurements of PR 973 after private overwrite-result reuse,
with **61 identical runtime flags**, including
`MECH_COMPACT_REUSE_OVERWRITTEN_RESULT=1`. They are separate from historical
60-flag cohorts. The source and canonical bytecode remain fixed; the updated
control has its own pinned Object and metadata.

Eight serial observations run ABBA then BAAB, four control and four candidate.
Each starts a fresh process and compiles complete source through parsing,
frontend, canonical artifact, bytecode, Object and metadata. Both use the locked
optimized `compiler` profile (Thin LTO, one codegen unit). Builds, tests, graph
capture, kernel experiments, independent oracle work, hashing, copying and
archive work do not overlap primary compiles. Screens and the two earlier
unpaired current-baseline observations remain separately in the packet.

The table contains arithmetic means. Positive changes mean more time/work/memory.
Phases and nested intervals overlap and must not be added. Whole-process time
includes the profiling driver's polling tail; compiler phases are also reported.
Eight observations are not a population estimate. All adverse observations remain.

| Measurement | Control mean | Candidate mean | Change |
| --- | ---: | ---: | ---: |
| Complete process | 18.4755 s | 17.5813 s | -4.8395% |
| Compiler phases | 18.3328 s | 17.4018 s | -5.0784% |
| Parsing | 2.0700 s | 2.0839 s | +0.6729% |
| Catalog | 0.0364 s | 0.0377 s | +3.7128% |
| Frontend | 2.5326 s | 2.5502 s | +0.6950% |
| Artifact | 0.0259 s | 0.0264 s | +2.0024% |
| Bytecode encoding | 0.0093 s | 0.0093 s | +0.3350% |
| Native | 13.6579 s | 12.6935 s | -7.0614% |
| Canonical lowering | 0.0996 s | 0.0986 s | -0.9915% |
| Preparation | 2.9865 s | 2.0152 s | -32.5221% |
| Numeric helper generation | 2.1306 s | 2.1278 s | -0.1292% |
| Source CFG construction | 2.3437 s | 2.3577 s | +0.5985% |
| Cleanup including compaction | 3.3860 s | 3.3863 s | +0.0080% |
| CFG cleanup | 3.2192 s | 3.2167 s | -0.0794% |
| Value compaction | 0.1668 s | 0.1696 s | +1.6962% |
| Retired instructions | 271.2014 G | 257.7202 G | -4.9709% |
| Maximum RSS | 5.9109 GB | 6.0251 GB | +1.9328% |
| Peak footprint | 5.8804 GB | 5.9949 GB | +1.9460% |

The balanced-block compiler mean changes are **-4.68%** and **-5.47%**.
Every candidate compiler, whole-process, preparation and instruction observation
is below every matched control observation. Maximum RSS increases from 5.9109 GB
to 6.0251 GB, about 114 MB; footprint increases 1.95%. Its cause is not isolated.
Parsing, catalog, frontend, artifact, encoding, source CFG construction and
compaction have adverse means. These unrelated-stage changes are not attributed
to the new solver. Cleanup including compaction is effectively flat (+0.008%).

Both subjects retain SSA value compaction and its cost. Every run retains the
same 1,242,559 values from 20,798,230 pre-compaction IDs. No SSA construction or
emission ordering changes. The earlier SSA-forwarding shortcut remains rejected.

## Where preparation spends time

Two separate diagnostic compiles instrument the original solver. Both reproduce
every output byte and have identical query/graph/result cardinalities. All 790
query events occur within preparation. The lower-overhead summary mode records:

| Recorded work | Amount |
| --- | ---: |
| Exact queries | 790 |
| Visited nodes across queries | 11,983,473 |
| Edges across queries | 12,615,807 |
| Retained read occurrences after wanted filtering | 2,656,350 |
| Definition occurrences | 174,560 |
| Graph/read construction | 0.5179 s |
| Definition selection | 0.0394 s |
| Query-local reindexing | 0.0127 s |
| Solving | 1.1955 s |
| Destruction of query temporaries | 0.2409 s |

The 299 filtered queries account for 11,755,849 node visits and 1.0427 seconds of
solving. The other 491 queries account for 227,624 visits and 0.1528 seconds of
solving. The histogram of retained reads per node is preserved; it is not a count
of all allocations before filtering.

Detailed mode adds per-node action-read and edge/condition clocks. Its graph/read
interval rises to 1.1006 seconds versus 0.5179 seconds in summary mode, showing
the clock overhead. Detailed intervals do not qualify a performance improvement.
Core clocks exclude post-core statistics/log formatting and report temporary
destruction separately. Other preparation work remains outside these intervals.

## Exact graph experiments and correctness

The original production solver captures all **790** validated query inputs and
expected result sets, totaling **176,143,814 bytes**. Each contains successor
lists, ordered reads, ordered definitions, the start ordinal and expected live
values. Capture preserves the original summary cardinalities and every program
output byte. A separate Python full-bitset worklist independently re-solves all
790 graphs and reproduces every expected value, without using the bounded
per-value search.

Kernel experiments stream those exact graphs. Result checks, JSON loading and
warmups stay outside the per-call timers. Orders alternate per graph; each mode
has two observations per query. All 12,640 timed returned results match their
expected sets. The original and all trial budgets remain in raw observations.

| Trial | Original, mean pass | Trial plus fallback, mean pass | Successful point searches per pass |
| --- | ---: | ---: | ---: |
| All graphs, 1 × nodes budget | 1.1297 s | 0.4140 s | 285/790 |
| All graphs, 4 × nodes budget | 1.1297 s | 0.3622 s | 293/790 |
| All graphs, 16 × nodes budget | 1.1297 s | 0.3043 s | 488/790 |
| Sparse-read eligibility, 16 × nodes budget | 1.0991 s | 0.2874 s | 298/299 attempted |
| Sparse-read eligibility, 64 × nodes budget | 1.0991 s | 0.2194 s | 299/299 attempted |
| Sparse-read eligibility, 256 × nodes budget | 1.0991 s | 0.2207 s | 299/299 attempted |

The 64-times-node allowance finishes all 299 selected queries. A 256-times-node
allowance performs the same search work and adds no benefit. Eligibility scans
are outside these kernel timers; the complete compiler cohort includes them.
Kernel improvements are not substituted for full source-to-Object timings.

All **16 maintained focused tests pass** with warnings denied. The new controls
compare 4,096 varied graphs at 15,616 start points and five work budgets against
an independent set-based fixed point. They exercise cycles, duplicate edges,
unreachable regions, sparse/high value IDs, reads at their own definitions,
branch kills, a 100,000-node cycle and explicit budget exhaustion with the existing
fallback. The final production search body matches the measured prototype after
whitespace normalization.

Fourteen successful original/candidate fixture pairs preserve all four output
files, including numeric phases, nested/changing captures, array windows and
Bool/U64/I64/F64 controls. All full benchmark screens and primary observations
preserve source, MBC, Object and complete metadata. Generated program instructions
and metadata are identical. No new compressor-runtime latency campaign is claimed
and no scalar machinery is retired.

## Build and capture records

Observed warm-cache compiler-executable builds take **274.646 seconds** for the
control and **281.129 seconds** for the candidate. The candidate rebuild overlaps
the separate 19.231-second offline oracle check; these are not isolated matched
bootstrap measurements and are excluded from Mech compilation. The diagnostic
build takes 276.349 seconds. An attempt to shorten only the capture link by
disabling LTO fails after 110.258 seconds: Apple's LLVM 15 linker cannot read the
Rust toolchain's LLVM 22 bitcode. The normal Thin LTO capture link succeeds in
165.827 seconds with cached dependencies. All commands, logs and patches remain.
There is no cold-bootstrap comparison.

## Remaining work and replay

Full compilation remains above ten seconds. Graph/read construction and temporary
allocation/destruction remain repeated preparation work. Source CFG construction
(2.36 seconds), cleanup including compaction (3.39 seconds), frontend (2.55 seconds)
and parsing (2.08 seconds) remain substantial. These are nested measurements,
not an additive estimate. Future changes must retain exact current validation
and generated-program speed; no saving is credited for those next targets.

The [compact result](../../benchmarks/mech-deflate/native-profile/point-liveness-compile-apple-m3-20261114.json),
[receipt](../../benchmarks/mech-deflate/native-profile/point-liveness-compile-apple-m3-20261114.archive.json)
and [packet index](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
pin every raw observation, graph, source snapshot, measured patch, command,
correction, test log, output, executable hash and license. Executables are excluded.
The repository archive copy is verified and freshly extracted before publication.
Pure offline `python3 replay.py` reconstructs the numerical result and source
patches, checks stored outputs and tests, and independently re-solves all graph
inputs. It needs no compiler, benchmark execution, original scratch paths or
network; it does not rerun the timing measurements.
