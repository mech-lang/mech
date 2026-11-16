# Numeric forest reachability

Fresh level-12 source-to-Object compilation takes **37.113–38.223 seconds**,
compared with **39.236–39.710 seconds** for its matched control on the Apple M3
Air. Mean native preparation falls **17.09%**, native compilation **5.87%**,
complete compiler time **4.57%** and retired instructions **5.07%**. Source
change `cb468a58f051154575b594c6dc6e10ca2517e08d` changes temporary reachability
storage and preserves every emitted program byte.

Numeric-exit analysis repeatedly walks the same immutable forest from its root,
stopping at the root latch, while excluding one guard edge. Previously every
continuation target repeated that walk, rebuilding ordered visited sets and
successor lists. One analysis owner now builds the filtered adjacency once and
retains reachability rows keyed by the excluded edge. The graph, root and latch
are fixed for that owner. Predicate truth, consistency checks and execution
permissions remain freshly derived in their original order.

Rows have a **32 MiB / 1,024-row limit**. Exhausting either limit uses an exact
walk with dense visited storage, retaining the original early-target and latch
behavior. These are temporary local data, without a global or serialized cache.
Grammar, language semantics, validation and generated runtime code do not change.

| Balanced run order | Parse | Frontend | Preparation¹ | Native | Compiler total | Whole process |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Control 1 | 2.486 s | 7.608 s | 10.737 s | 29.511 s | 39.710 s | 40.931 s |
| Candidate 1 | 2.675 s | 7.463 s | 8.935 s | 27.991 s | 38.223 s | 39.411 s |
| Candidate 2 | 2.450 s | 6.693 s | 8.957 s | 27.889 s | 37.113 s | 37.355 s |
| Control 2 | 2.450 s | 6.828 s | 10.841 s | 29.857 s | 39.236 s | 39.437 s |

¹ Preparation is included in native time. All observations use the same 52
flags, source and toolchain. Builds, tests, fixture comparisons and sampling do
not overlap primary timing. Compiler bootstrap takes **453.05 seconds**, excluded
from program compilation; restoring the preceding rejected Core trial causes
dependencies to rebuild as well as the modified GPU compiler.

Both complete elapsed pairs improve. Four observations do not establish a
statistical population. The first candidate's adverse parsing interval remains,
and first launches of both subjects have longer frontend and startup intervals.
Frontend variation is outside the modified preparation code and is not attributed
to this change. Candidate maximum RSS is **7,668,465,664–7,707,688,960 bytes**,
compared with **7,506,395,136–7,756,759,040 bytes** for control. Its mean is higher
and ranges overlap; no whole-compiler memory saving is established.

All four observations preserve source, canonical bytecode, native Object and
complete metadata byte-for-byte. Object SHA-256 remains
`4ddc16b053bda6256656e0d5cbf95ba60cab31b0911b9aadf526b66e1f3cab67`;
metadata remains
`0813625d0178e329bfd780c8a0d5f714bbd15a23727d22505172d1136bd0b44d`.
Generated instructions are unchanged; no new compression-runtime timing
campaign is claimed.

All **66 focused tests across six suites** pass. Exhaustive differential tests
make **552,960 query comparisons** against the previous ordered-set DFS across
every three-node graph, root, stop, avoided edge and target, with full, partial
and disabled row storage. Further cases cover cycles, parallel edges, distinct
contexts, latch membership and both storage limits. The standalone exact helper
also builds with warnings denied and passes its three tests. Six complete
additional programs preserve all four outputs. Their enum-state input follows
the maintained test assembly; this cohort has no fragment-only launch. The
copied assembly provenance retains a historical failure from the earlier study.

A prior retained-control preparation CPU sample motivated this change; its
elapsed is excluded. The packet retains that diagnostic and its output checks,
all primary observations, build/test commands, patch, source pins and licenses.
Its intermediate continuation-state snapshot does not describe final process
status.

The [compact result](../../benchmarks/mech-deflate/native-profile/numeric-reachability-compile-apple-m3-20261113.json)
and [receipt](../../benchmarks/mech-deflate/native-profile/numeric-reachability-compile-apple-m3-20261113.archive.json)
pin the [public evidence](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/level12-numeric-reachability-compile-apple-m3-20261113.tar.gz):
211 files, 5,828,158 compressed bytes, SHA-256
`07f64e7789c022644f6e3ad9ac7c170131cdabfd0c482f9eb3745dbc752912f1`.
Independent public retrieval verifies the manifest and reproduces the compact
result byte-for-byte with `python3 replay.py`. Compiler/test executables stay
local with their hashes; Object and metadata remain retrievable.

The preceding [dimension-remapping change](native-frontend-affine-compile-profile.md)
reduced frontend work. Full compilation now takes about 37–38 seconds;
the sub-10-second goal remains open.
