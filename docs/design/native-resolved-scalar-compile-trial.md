# Scalar constructor trial rejected

The scalar-only schema constructor trial is **rejected and fully removed**.
Fresh level-12 source-to-Object compilation takes **39.914–39.982 seconds**,
against **39.184–40.108 seconds** for its retained control on the Apple M3 Air.
Mean frontend time rises **5.85%**, complete compilation **0.76%** and retired
instructions **1.72%**. The retained
[inline dimension-remapping compiler](native-frontend-affine-compile-profile.md)
remains unchanged.

The candidate validates borrowed dimension bounds before resolving a builtin
scalar with an empty environment, avoiding a temporary declaration clone and
unreachable dependency graph. It also extracts the general canonicalizer's
bound-normalization loop into a shared helper. The combined trial does not
isolate those two changes or establish the cause of the regression. It does
not rule out a differently scoped optimization of unused dimension contexts.

| Balanced run order | Parse | Frontend | Native | Compiler total | Whole process |
| --- | ---: | ---: | ---: | ---: | ---: |
| Control 1 | 2.481 s | 7.664 s | 29.881 s | 40.108 s | 41.318 s |
| Candidate 1 | 2.507 s | 7.726 s | 29.596 s | 39.914 s | 41.128 s |
| Candidate 2 | 2.441 s | 7.786 s | 29.677 s | 39.982 s | 40.093 s |
| Control 2 | 2.438 s | 6.991 s | 29.650 s | 39.184 s | 39.257 s |

One complete-compiler pair improves and the other regresses. Both candidate
frontend intervals exceed both controls. Four observations do not establish a
statistical population. All use the same 52 flags, source and toolchain, with
no builds, tests, fixtures or sampling overlapping primary timing. The
**453.69-second** compiler rebuild is excluded. Initial launches of both subjects
retain about one second of extra startup beyond the compiler's phase timers.

All source, canonical bytecode, native Object and complete metadata bytes match
the retained output. Object SHA-256 remains
`4ddc16b053bda6256656e0d5cbf95ba60cab31b0911b9aadf526b66e1f3cab67`.
The candidate passes **190 Core unit tests and 81 type-system contract tests**.
These include all 17 builtin scalar kinds, complete normalized evidence,
first-error ordering and 4,096 seeded dimension environments compared with the
original constructor. A Rust 2024 formatting check passes. The initial offline
`no_std` check cannot unpack a cached dependency under the filesystem sandbox;
the permitted offline retry passes. Both observations remain in evidence.
No further fixture or runtime campaign is run after rejection.

A separate **retained-control** compile takes 40.016 seconds and preserves all
four outputs. Three diagnostic CPU windows cover frontend, preparation and
CFG cleanup. Their elapsed is excluded from the balanced cohort. The frontend
still spends substantial work cloning dimension contexts and canonicalizing
environments. Preparation repeatedly traverses the same forest for immutable
guard edges in `numeric_exits::required_predicates`, builds ordered visited
sets, and scans loop inventories in `collapsed_successors`. Cleanup still
spends work inserting block sets, simplifying phis and cloning range maps.
Top-of-stack rows are self samples; the total-in-stack table includes callers.
The main thread's join wait is not compiler CPU work.

The next candidate is exact reachability reuse within one immutable forest:
retain the root, latch stop and avoided edge in its scope, and share the
reachable set across continuation targets. Predicate computation and emitted
branches must retain their existing behavior. This candidate has not yet been
implemented or timed in the archived trial.

Base `44c7222384945bb3c9fefe5a4ca6a4277f29e1e7` plus archived uncommitted patch
`4d12731bfbee19ddd118221fc39534686c5a3c686f7f5caa34c8a02b12eca5dd`
identifies the rejected compiler. Its complete changed source and all raw
observations remain retrievable; no rejected production source is committed.
The [compact result](../../benchmarks/mech-deflate/native-profile/resolved-scalar-trial-apple-m3-20261113.json)
and [receipt](../../benchmarks/mech-deflate/native-profile/resolved-scalar-trial-apple-m3-20261113.archive.json)
pin the [public evidence](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/level12-resolved-scalar-trial-apple-m3-20261113.tar.gz):
116 files, 6,143,478 compressed bytes, SHA-256
`1e9828d29ced2e01e087cda2184ae533ff7fef745f8330cb7f22d4f423a97449`.
Independent download verifies the complete manifest and reproduces the compact
result byte-for-byte with `python3 replay.py`. Executables stay local with hashes.
The retained compiler remains about 39–40 seconds; the sub-10-second goal is open.
