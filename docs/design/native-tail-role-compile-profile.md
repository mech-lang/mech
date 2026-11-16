# Checked tail-finish role lookup

Fresh level-12 source-to-Object compilation takes **36.728–36.876 seconds**,
compared with **37.340–37.470 seconds** for its matched control on the Apple M3
Air. Mean native preparation falls **4.27%**, native time **1.15%**, complete
compiler time **1.61%** and retired instructions **1.05%**. Source change
`b0b9b9a2f4f4ec87e87287ac7b4f17cc7974f249` preserves all emitted program bytes.

The current-visit validator already checks every loop's roles, edges, types and
ownership. It uniquely records each tail-loop finish as role 4. Later, both the
incoming-definition proof and degree-decrement traversal repeatedly scanned all
loops to recognize those same finish blocks. They now consult the checked role
table directly. All role validation completes before either lookup. No proof
result is cached, no checks are skipped, and no scratch allocation is added.

| Balanced run order | Parse | Frontend | Preparation¹ | Native | Compiler total | Whole process |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Control 1 | 2.398 s | 7.461 s | 8.839 s | 27.529 s | 37.470 s | 38.595 s |
| Candidate 1 | 2.489 s | 6.855 s | 8.533 s | 27.301 s | 36.728 s | 37.963 s |
| Candidate 2 | 2.443 s | 6.779 s | 8.547 s | 27.576 s | 36.876 s | 37.144 s |
| Control 2 | 2.448 s | 6.821 s | 9.003 s | 27.985 s | 37.340 s | 37.589 s |

¹ Preparation is included in native time. All observations use the same 52
flags, source and toolchain. Builds, tests, fixtures and CPU sampling do not
overlap primary timing. Compiler bootstrap takes **244.37 seconds**, excluded
from program compilation. The separate engine unit-test configuration builds
in **251.86 seconds**; primary timing uses the already frozen compiler binaries.

Both matched compiler and native elapsed pairs improve. These four observations
do not establish a statistical population. The first control's adverse frontend
interval and first candidate's adverse parsing interval remain. Frontend
variation accounts for part of the observed full-compile reduction and is not
attributed to this validation change. First launches of both subjects retain
longer startup beyond the phase timers. Candidate maximum RSS is
**7,758,872,576 bytes** in both observations, compared with control's
**7,463,976,960–7,964,065,792 bytes**. Its mean is higher, ranges overlap, and no
whole-compiler memory saving is established.

All four level-12 observations and six complete additional programs retain
byte-identical source, canonical bytecode, native Object and full metadata.
Object SHA-256 remains
`4ddc16b053bda6256656e0d5cbf95ba60cab31b0911b9aadf526b66e1f3cab67`;
metadata remains
`0813625d0178e329bfd780c8a0d5f714bbd15a23727d22505172d1136bd0b44d`.
Runtime instructions are unchanged; no new compression-runtime timing campaign
is claimed. Grammar and language semantics remain unchanged.

All **14 existing engine iteration-proof tests** pass. They cover original and
decoded tail plans, forged roles and edges, stale returns, nested counted and
collection loops, scope errors and admission limits. An initial Cargo command
cannot select dependency features from the profiling workspace. The corrected
engine manifest builds successfully, but the first module filter selects zero
tests. That run does not qualify the change. The corrected filter runs all 14
tests without rebuilding. Every attempt and its exact command remains archived.
The fixture enum-state input uses the maintained complete test assembly; this
cohort has no fragment-only launch. Its provenance retains an older failure.

The prior preparation CPU sample identifies repeated collapsed-successor work.
It uses retained compiler `5d856d…`, before the separately qualified reachability
rows, and is diagnostic only; its elapsed is excluded from this cohort.

The [compact result](../../benchmarks/mech-deflate/native-profile/tail-role-compile-apple-m3-20261113.json)
and [receipt](../../benchmarks/mech-deflate/native-profile/tail-role-compile-apple-m3-20261113.archive.json)
pin the [public evidence](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/level12-tail-role-compile-apple-m3-20261113.tar.gz):
212 files, 5,848,379 compressed bytes, SHA-256
`574728a392649890045c9e00825be46701781b5760dddae5f75aad00c6c79170`.
Independent public retrieval verifies the manifest and reproduces the compact
result byte-for-byte with `python3 replay.py`. Compiler/test executables remain
local with hashes; Object and metadata remain retrievable.

The preceding [numeric reachability change](native-numeric-reachability-compile-profile.md)
reduced repeated graph work. Full compilation now takes about 36.7–36.9 seconds;
the sub-10-second goal remains open.
