# Rootless dimension normalization

Fresh level-12 source-to-Object compilation takes **36.097–36.394 seconds**,
compared with **37.102–38.072 seconds** for its matched control on the Apple M3
Air. Mean frontend time falls **16.07%**, complete compiler time **3.57%** and
retired instructions **1.46%**. Source change
`6d7390e59616e52c5d1dce30f7514cf89167731a` preserves all emitted program bytes.

When a type retains no dimension parameters, canonicalization still constructed
normalized-bound and dependency tables, then walked empty roots and discarded
everything. It now streams the same declaration-ID, lifetime and lower/upper
bound validation in the original order, without retaining those unused tables
or constructing DFS state. Invalid unused bounds still report their original
first error. Unused cycles retain their original acceptance; reachable cycles
still fail. The path with roots retains its original algorithm.

| Balanced run order | Parse | Frontend | Native | Compiler total | Whole process |
| --- | ---: | ---: | ---: | ---: | ---: |
| Control 1 | 2.482 s | 7.711 s | 27.795 s | 38.072 s | 39.378 s |
| Candidate 1 | 2.482 s | 6.248 s | 27.281 s | 36.097 s | 37.138 s |
| Candidate 2 | 2.450 s | 6.173 s | 27.694 s | 36.394 s | 36.520 s |
| Control 2 | 2.446 s | 7.087 s | 27.486 s | 37.102 s | 37.375 s |

All observations use the same 52 flags, source and toolchain. Builds, tests,
fixtures and CPU samples do not overlap primary timing. Compiler bootstrap
takes **461.84 seconds**, excluded from program compilation; its build overlaps
the separate Core test builds. Both are terminal before primary timing.

Both matched frontend and complete elapsed pairs improve. Four observations do
not establish a statistical population. The first control's adverse frontend
interval remains and enlarges the mean improvement; the second control still
exceeds both candidates. The second candidate's native interval is adverse
against its matched control. Preparation mean rises **0.20%**; native variation
is not attributed to this normalization change. First launches retain longer
startup beyond compiler phase timers. Candidate maximum RSS is
**7,757,414,400–7,759,708,160 bytes**, compared with control's
**7,487,602,688–7,736,623,104 bytes**. Its mean is higher and this cohort's ranges
do not overlap; no whole-compiler memory saving is established.

All **188 Core units and 81 type-system contract tests** pass, as does the
`no_std` check. A new **4,096-case differential test** compares empty-root
behavior with the full reference algorithm, including malformed IDs, lifetimes,
holes, arithmetic overflow and unused cycles. The existing 16,384-case test
continues to compare rooted and unrooted environments with the ordered-set
reference. Six complete additional programs preserve all four outputs. The
enum-state program uses the maintained complete assembly; copied provenance
retains an older fragment failure, without a fragment-only launch in this cohort.

All four level-12 observations retain byte-identical source, canonical bytecode,
native Object and full metadata. Object SHA-256 remains
`4ddc16b053bda6256656e0d5cbf95ba60cab31b0911b9aadf526b66e1f3cab67`;
metadata remains
`0813625d0178e329bfd780c8a0d5f714bbd15a23727d22505172d1136bd0b44d`.
Grammar, language semantics and generated runtime instructions are unchanged.
No new compression-runtime timing campaign is claimed.

The [compact result](../../benchmarks/mech-deflate/native-profile/rootless-dimension-compile-apple-m3-20261113.json)
and [receipt](../../benchmarks/mech-deflate/native-profile/rootless-dimension-compile-apple-m3-20261113.archive.json)
pin the [public evidence](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/level12-rootless-dimension-compile-apple-m3-20261113-v2.tar.gz):
220 files, 6,192,054 compressed bytes, SHA-256
`ae5358906aa28c364a69d5f7b2f7b9999018c3af310b6de0f60cd920fe30e4f0`.
Independent public retrieval verifies the manifest and reproduces the compact
result byte-for-byte with `python3 replay.py`. Compiler/test executables remain
local with hashes; Object and metadata remain retrievable.

The initial public packet inherited an inaccurate overlapping-RSS-ranges
sentence. Its raw measurements were correct. This replacement corrects that
narrative and preserves the original summary, replay and public archive receipt.
The original archive remains available; no measurements are removed.

The new current/peer process profiles form a separate experiment. Full
compilation remains about 36.1–36.4 seconds; the sub-10-second goal remains open.
