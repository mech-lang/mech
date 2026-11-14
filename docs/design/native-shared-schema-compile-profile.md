# Shared pending frontend schemas: compilation qualification

Complete fresh level-12 compilation takes **21.196–21.415 seconds** on the Apple
M3 Air. Eight matched runs show **2.46% lower whole-process time, 2.37% lower
compiler time, 18.85% lower frontend time and 3.27% fewer instructions**. Peak
RSS falls 1.38%. Source, MBC, Object and complete metadata bytes remain identical.
The full ten-second compilation goal remains open.

The frontend previously copied complete dimension-declaration arrays when it
cloned pending schemas and repeated pure canonicalization on immutable drafts.
Its private pending values now share an immutable schema owner. A mutable access
makes the draft private and invalidates its cached canonical schema or error,
including the case where the owner was already unique. Read-only annotation and
output-schema queries borrow drafts; explicit owned boundaries still materialize
them. Core's public `SchemaDraft` and runtime validation are unchanged.

Only pure schema data and its deterministic validation result are cached. Current
shape, factory, visit, resource and ownership proofs are checked as before. Errors
are still anchored at the caller. Unused malformed dimension declarations remain
validated. No grammar or language change, interpreter fallback, backend setting
change or runtime optimization is part of this trial.

Source `610f3692c0ad0b894d6537f206a79a4de8833ede`, parent `f18b1b20c25fd20efc8ce06c8664c72197e7cd52`.
The frozen control comes from `7306a0097a2a8e52bf263bb9d600a5e4ed8b417d`; its production
source equals the documentation-only parent. Both subjects use the same opt-in
`compiler` build profile, full fresh source-to-Object pipeline and 60 runtime
flags. Hardware: Apple M3 Air, 8 cores (4P+4E), 16 GB, macOS 26.2.

| Measurement | Control range (4 runs) | Candidate range (4 runs) | Mean change |
| --- | --- | --- | ---: |
| Compiler, s | 21.618094–21.779042 | 21.063822–21.323196 | -2.37% |
| Whole process, s | 21.841207–21.843376 | 21.195902–21.415386 | -2.46% |
| Parse, s | 2.410421–2.431071 | 2.417481–2.432017 | +0.06% |
| Frontend, s | 3.128764–3.150851 | 2.536188–2.563476 | -18.85% |
| Native, s | 16.004934–16.134203 | 16.021533–16.284809 | +0.46% |
| Canonical lowering, s | 0.204575–0.205029 | 0.205544–0.210076 | +1.08% |
| Preparation, s | 3.556269–3.573137 | 3.545895–3.630565 | +0.35% |
| Numeric generation, s | 3.842132–3.867092 | 3.846229–3.873264 | +0.16% |
| Source CFG, s | 2.338791–2.344012 | 2.328233–2.417054 | +0.53% |
| Cleanup including compaction, s | 3.390652–3.423133 | 3.387255–3.459419 | +0.18% |
| Compaction, s | 0.165400–0.172501 | 0.165397–0.168160 | -0.18% |
| Retired instructions | 330,357,322,049–330,480,082,922 | 319,379,449,237–319,679,309,506 | -3.27% |
| Peak RSS, bytes | 5,991,743,488–5,993,512,960 | 5,909,102,592–5,911,216,128 | -1.38% |
| Peak footprint, bytes | 5,961,405,720–5,963,175,192 | 5,878,764,728–5,880,878,264 | -1.39% |

Runs are serial control/candidate/candidate/control then
candidate/control/control/candidate. Both blocks improve compiler time (2.61%
and 2.13%). Every candidate compiler, process, instruction and peak RSS observation
falls below every control. Native time is an adverse 0.46% higher; parse and
several native substage means also increase. All observations remain. These finite
cohorts are descriptive, not population estimates. Both subjects include retired
SSA value compaction and its measured cost.

Builds, tests, screens, fixture compiles, hashing, copying and packet work do not
overlap primary timing. The observed 401.050-second optimized compiler rebuild
uses existing Cargo caches, is excluded from Mech compilation timings and is not
a matched cold-bootstrap comparison. Both screens and all eight primary runs
reproduce every qualified output byte. Fourteen successful fixture pairs cover
six complete array/enum programs and eight checked scalar programs, also matching
all four output files. Generated program code and metadata are byte-identical;
no new compressor-runtime timing campaign or CPU sampling is claimed.

The final source passes **64 frontend tests, including four owner tests, and two
codec tests: 66 unique tests**. The new tests use the production owner module and
cover copy-on-write/cache invalidation, cached errors and repair, unused invalid
declarations, 512 seeded drafts with four mutation branches, owned materialization
and concurrent readers. An earlier standalone four-test pass duplicates the owner
tests and predates four test-fixture constructor repairs; it is not counted again.
Type-integration failures and the failed full-suite build remain in the packet.

The [compact result](../../benchmarks/mech-deflate/native-profile/shared-schema-compile-apple-m3-20261114.json)
and [receipt](../../benchmarks/mech-deflate/native-profile/shared-schema-compile-apple-m3-20261114.archive.json)
pin the complete [benchmark evidence packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md).
It includes exact source/base snapshots and patches, flags, commands, raw timings,
all output files, test logs, corrections, compiler executable hashes and pure
offline replay. Compiler executables are excluded. Python 3.11+ replay reconstructs
the summary without compiling, running benchmarks or contacting an external service.
