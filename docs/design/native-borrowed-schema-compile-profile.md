# Borrow schema environments during finalization

Borrowing the dimension environment reduces frontend time **24.33%**, full
compiler time **4.08%**, complete-process time **4.19%** and maximum RSS **7.62%**
in the matched level-12 compressor cohort. Complete fresh source-to-Object
processes take **28.764–28.769 seconds**, versus **29.822–30.228 seconds** for
control. All source, bytecode, Object and complete metadata bytes match exactly.

Source `2f4134024e19ed316437334331a837874b1701b5` changes schema finalization to
consume an owned body while borrowing its dimension declarations. The existing
consuming `SchemaDraft::finalize` and new `finalize_ref` / `finalize_body` methods
share the same canonicalizer and validation order. Immediate clone/finalize
callers and two component helpers avoid cloning the entire environment. The
patch retains all 37 migrated call sites, including existing test code.

Every declaration is still validated, including unused malformed declarations.
Name/keyability checks, normalization, dimension-reference collection, complete
environment canonicalization, body rewriting and final validation keep their
original order. No schema result, semantic proof or factory selection is cached.
Mutable drafts expose their current fields on every call. Grammar, language,
current-visit ownership checks, finite admission and backend optimization level
are unchanged.

## Matched complete compilation

Four serial ABBA compiles use frozen control and candidate binaries, the same
54 existing runtime flags and identical host-statistics access. Control source
`342937b4cba43ed57d08ca78648dde4b188d82b7` is production-identical to the
documentation-only base `0096434d1a487c73aa879a29ffc5b94cc56e1fd3`. It includes
the independently qualified nested-reduction runtime transformation. Compiler
bootstrap, tests, fixtures, screen, diagnostic census and archive work are
excluded; none of this campaign's competing work overlaps primary timing.

| Measurement | Control | Candidate | Mean reduction |
| --- | ---: | ---: | ---: |
| Compiler source through Object | 29.671, 29.969 s | 28.666, 28.541 s | 4.08% |
| Complete process | 29.822, 30.228 s | 28.769, 28.764 s | 4.19% |
| Frontend semantic analysis | 6.787, 6.520 s | 5.021, 5.048 s | 24.33% |
| Native compilation (unchanged work) | 20.510, 20.908 s | 21.115, 20.961 s | -1.59% |
| Retired instructions | 453.644, 453.082 billion | 431.166, 431.147 billion | 4.90% |
| Maximum RSS | 6.935, 7.183 GiB | 6.522, 6.521 GiB | 7.62% |

Both candidates have lower frontend/compiler time, retired instructions and RSS
than both controls. Native-stage observations are slower on average; all such
contrary observations remain in the summary, without attributing unchanged-stage
variation to this frontend edit. Four compiles are descriptive observations,
not a population estimate. Compiler bootstrap separately takes 454.041 seconds.
Full fresh compilation remains above ten seconds.

## Qualification and evidence

All **357 tests** pass: 188 Core unit tests, 105 contract tests, two explicit
serde-enabled canonical-vector tests, 60 frontend tests and two codec tests.
Core no-std checking passes. The initial no-default-feature canonical-vector
target runs zero tests because of its serde gate; that run is retained and does
not count as vector coverage. Added contracts check component axis identity,
renumbering and bounds, parent immutability, original error precedence, unused
invalid declarations and mutations between calls.

All four outputs also match between subjects for six complete array/enum
fixtures and eight checked Bool/U64/I64/F64 scalar programs. The complete state
fixture's assembly provenance retains earlier attempts. Every primary compile
and the separate screen reproduces Object
`8f33516fb93e1614a02f64d6dec1bd92f68abc6c4824b468c71840da494141a7`
and the current qualified metadata. No new program runtime-latency campaign is
claimed; the emitted program is byte-identical.

A separate frozen-Object census observes 358 functions, 234 distinct machine
byte/raw-relocation groups and 124 duplicate functions beyond the first. It
includes inter-function alignment. This is a future backend inspection target,
not an IR-equivalence proof or validated cache key. This trial uses no cached
compilation or cached Object.

The [compact result](../../benchmarks/mech-deflate/native-profile/borrowed-schema-compile-apple-m3-20261114.json),
[archive receipt](../../benchmarks/mech-deflate/native-profile/borrowed-schema-compile-apple-m3-20261114.archive.json)
and [complete 371-file packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/mech-borrowed-schema-20261114.tar.gz)
are published in benchmarks. The repository archive verifies every hash/Git blob
and replays the compact result byte-identically offline. The [evidence index](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
provides extraction and replay commands for this and all four preceding packets.
