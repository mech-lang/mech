# Canonicalize independent constant bounds without dependency arrays

Independent constant-bound environments reduce full compiler time **8.51%**,
frontend time **42.09%**, retired instructions **10.21%** and maximum RSS
**14.41%** in the matched level-12 cohort. Complete fresh source-to-Object
processes take **23.508–23.517 seconds**, versus **25.195–26.034 seconds** for
control. All source, bytecode, Object and complete metadata bytes match exactly.

Source `7d0c06d002758d029839c81a774d5e704e56d440` changes only
`src/core/src/dimension.rs` on base
`6176af3925b0f0f8aef173c5deadc63576bb8994`. The control's production source is
identical to retained graph-indexing source
`619386076692d819778fdc5c53579b3b7604b062`.

## The remaining frontend work

A separate fresh complete-compile CPU profile on that qualified control retains
two sampling windows. The first window spans parser, frontend, preparation,
helper generation and source-SSA work. These are self-sample counts from that
window, not stage-exclusive elapsed times or exact source-line attribution:

| Function | Self samples |
| --- | ---: |
| Dimension-environment canonicalization | 319 |
| Numeric traversal analysis | 211 |
| Nonlocal SSA variable lookup | 168 |
| Selected-graph liveness | 132 |
| Dimension-declaration array cloning | 112 |
| Dimension-expression normalization | 97 |

Main-thread join waits are excluded from this CPU-work table. Both raw call
graphs, all symbols, demangled stacks, extraction tables and the existing
demangler source/pin remain. The profiled compile preserves every output byte
and is excluded from primary timing.

## Avoid arrays that cannot contain dependencies

The generic canonicalizer normalizes each declaration, stores all normalized
bounds and dependency lists, walks the reachable dependencies, renumbers retained
parameters, then rewrites and normalizes their bounds. A constant bound is
already canonical and has no dependency edge.

The new path applies only when every lower bound and optional upper bound is a
constant. It validates all declaration IDs first, then every lifetime in the
original order, including unused declarations. Unknown-root errors still follow
those checks. Selected explicit parameters retain declaration order; inferred
parameters retain first occurrence in the roots. The old-to-new map keeps its
original length and exact contents. No declaration or current draft is assumed
valid because of an earlier call, and no schema/proof result is cached.

Expressions, parameter references and holes retain the existing generic path
and its original validation, normalization and DFS behavior. This includes
unused invalid expressions, overflow, multiply-by-zero with an invalid child,
forward references and unused/retained cycles. The optimization adds no grammar,
language rule, new validation or relaxed error condition.

## Matched complete compilation

Four serial ABBA observations use frozen compilers, identical **55 runtime
flags** and equal host-statistics access. Both subjects use the current qualified
terminal-append program. Compiler builds, 359 tests, both screens, 14 fixture
pairs, CPU sampling and archiving are excluded and do not overlap primary timing.
No cached Object or decoded-only substitute is used.

| Measurement | Control | Candidate | Mean reduction |
| --- | ---: | ---: | ---: |
| Compiler source through Object | 25.057, 25.904 s | 23.255, 23.372 s | 8.51% |
| Complete process | 25.195, 26.034 s | 23.517, 23.508 s | 8.21% |
| Frontend semantic analysis | 4.957, 5.096 s | 2.913, 2.908 s | 42.09% |
| Retired instructions | 401.827, 402.643 billion | 361.019, 361.330 billion | 10.21% |
| Maximum RSS | 6.520, 6.522 GiB | 5.581, 5.582 GiB | 14.41% |
| Peak footprint | 6.491, 6.494 GiB | 5.552, 5.553 GiB | 14.48% |

Both candidates are below both controls for compiler/frontend time, retired
instructions, RSS and footprint. Parse and preparation elapsed observations
are slower on average; main cleanup is essentially flat and slightly slower.
All adverse counters and unchanged-stage variation remain without causal
attribution. Four compiles are descriptive observations, not a population
estimate. The separate compiler bootstrap takes 453.248 seconds and is not
part of source-program compilation. Full fresh compilation remains above ten
seconds.

## Qualification and evidence

All **359 tests** pass: Core 190, contracts 105, explicit serde canonical
vectors two, frontend 60 and codec two. Core no-std checking passes. The gated
no-default-feature vector target runs zero tests and is not counted as vector
coverage. Existing 16,384 seeded canonical/reference comparisons remain. Added
coverage checks 256 larger constant-bound environments, mixed origins, duplicate
and unknown roots, unused compile-time declarations, first-error precedence
and transitions to the unchanged generic path.

All four outputs also match between subjects for six complete array/enum and
eight checked Bool/U64/I64/F64 fixtures. Every primary compile and both screens
reproduce Object
`38b194ec2c931fbd5d584ace06bdaba1efb59381e64eef1d662a5f2f12bd96b4`
and complete metadata
`0f6caeb3b18c65552dcda3e5fa460e1796884e4131e275473cfd2858339382cf`.
No new program-runtime latency campaign is claimed. Direct native scalar
emission, checked errors, finite admission, current ownership/visit checks and
backend optimization remain unchanged.

The [compact result](../../benchmarks/mech-deflate/native-profile/constant-bounds-compile-apple-m3-20261114.json),
[archive receipt](../../benchmarks/mech-deflate/native-profile/constant-bounds-compile-apple-m3-20261114.archive.json)
and [complete 404-file packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/mech-constant-bounds-20261114.tar.gz)
are published in benchmarks. The repository copy verifies every manifest/file
hash and Git blob identity and replays the compact result byte-identically
offline. The [evidence index](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
provides extraction and replay commands, including all six earlier packets.
