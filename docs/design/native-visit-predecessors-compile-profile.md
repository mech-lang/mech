# Prepared-owner reverse edges: compiler qualification

The matched level-12 compressor runs take **23.087–23.092 seconds for a fresh
complete source-to-Object process**, compared with 23.298–23.728 seconds for the
control in this cohort. Compiler time falls 1.87%, numeric generation 11.39%,
and retired instructions 3.34%. Source, MBC, Object and complete metadata bytes
remain identical. The full ten-second compilation goal remains open.

Numeric traversal previously scanned all 41,039 program visits for each scope
to detect an outside edge entering below its fresh header. The prepared owner
now builds reverse control edges once, and each query checks the current scope
membership and header against those edges. The index stores topology. It
preserves the original first offending source and diagnostic, and captured
windows retain their existing separate entry contract. It caches no validity,
ownership or current-visit proof.

The index belongs to the exact immutable prepared owner and is compiler-only.
It changes no serialized metadata, runtime instruction emission, language or
grammar. Direct Bool/U64/I64/F64 emission, checked errors, finite admission,
ownership checks and backend optimization levels retain their existing behavior.

Source commit: `40c33f0c75865483fa6bc140fb9b4413d23d7cd0`. Base: `d36a53e9e2c51e197eafd2d9c7c44cf6ee5a8f0e`.
The frozen control comes from source `7d0c06d002758d029839c81a774d5e704e56d440`;
its production sources are identical to the documentation-only base. Both
subjects run with the same 55 flags, including `MECH_TERMINAL_BYTE_APPENDS=1`.
Hardware: Apple M3 Air, 8 cores (4P+4E), 16 GB, macOS 26.2.

| Measurement | Control observations | Candidate observations | Mean change |
| --- | --- | --- | ---: |
| Compiler, s | 23.125766, 23.647642 | 22.898647, 23.002104 | -1.87% |
| Whole process, s | 23.298451, 23.728103 | 23.086591, 23.091953 | -1.80% |
| Parse, s | 2.291869, 2.454869 | 2.453489, 2.438200 | +3.05% |
| Frontend, s | 2.894852, 3.011566 | 2.981054, 2.989941 | +1.09% |
| Native, s | 17.866778, 18.106835 | 17.388709, 17.497644 | -3.02% |
| Canonical lowering, s | 0.198374, 0.209477 | 0.205327, 0.208458 | +1.45% |
| Preparation, s | 3.640662, 3.764207 | 3.736443, 3.767534 | +1.34% |
| Numeric generation, s | 4.549605, 4.664156 | 4.076197, 4.088381 | -11.39% |
| Source CFG, s | 2.503935, 2.446205 | 2.447775, 2.443169 | -1.20% |
| Cleanup, s | 4.108586, 4.089428 | 4.035936, 4.067234 | -1.16% |
| Retired instructions | 362,269,413,210, 362,015,884,245 | 350,053,209,568, 350,065,251,776 | -3.34% |
| Peak RSS, bytes | 5,993,742,336, 6,007,488,512 | 5,993,480,192, 5,993,365,504 | -0.12% |
| Peak footprint, bytes | 5,962,241,304, 5,975,987,480 | 5,962,028,360, 5,961,897,264 | -0.12% |

Four runs were collected serially in control/candidate/candidate/control order.
Compiler builds, tests, both screening compiles, fixtures and packet work did
not overlap primary timing. Both candidates fall below both controls for
compiler time, whole-process time, numeric generation and retired instructions.
The roughly 0.12% memory difference is small. Parse, frontend, canonical
lowering and preparation are slower in the observed candidate means; those
unchanged-stage differences are retained without causal attribution. Four
observations describe this cohort, not a population estimate.

All four production-module tests pass. They include **7,464,960** exhaustive
graph/scope/header comparisons against the original ordered scan, a 65,536-node
sparse graph, changed scopes and headers, separate owner topology, duplicate
edges and out-of-range targets. Fourteen successful fixture pairs cover six
complete array/enum programs and eight checked scalar programs; all four
outputs match between subjects for every pair. Both screens and every primary
compile also match the previously qualified complete compressor outputs.

The test command rebuilt compiler and tests in 86.911 seconds. The subsequent
0.114-second Cargo invocation reused that compiler and located its executable;
it is recorded as a cached build, with no cold bootstrap claim. These build
times are separate from the measured source-to-Object compiler runs.

No new runtime latency campaign is claimed. The generated machine code and
complete metadata are byte-identical, preserving the previously qualified
program. The prior [CPU profile](native-constant-bounds-compile-profile.md)
identifies numeric traversal as a remaining hotspot; its samples are in that
earlier packet and are not presented as new measurements of this candidate.

| Output | Bytes | SHA-256 |
| --- | ---: | --- |
| metadata.json | 2,292,704 | `0f6caeb3b18c65552dcda3e5fa460e1796884e4131e275473cfd2858339382cf` |
| program.mecb | 4,138,627 | `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7` |
| program.o | 1,238,328 | `38b194ec2c931fbd5d584ace06bdaba1efb59381e64eef1d662a5f2f12bd96b4` |
| source.mec | 160,857 | `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9` |

Compiler executable pins:

- Control: `e359f3ae5c65e7841ccc29531912be2cf54da90ccde601d12e1cfa977732f99f`.
- Candidate: `7e6464ea83e3479fbc69386375560d011a38bd67a5d6c8911e19fce5e7d9af4c`.
- Measured source patch: `9ceee4508f451a238ee6f17f021829251d8428044cbec9d62e7bd35f52ec6ec3`.

The complete [360-file evidence archive](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/mech-visit-predecessors-20261114.tar.gz)
is tracked in benchmarks at the user's request. It includes all raw commands,
host counters and phase timestamps, both screens, every fixture output, source
snapshots and patch, all test logs, licenses and the pure offline replay script.
Compiler executables are excluded and hash-pinned. The archive SHA-256 is
`6ec4230f07deb3880ad2c57fe4bcf45aff897ee31d2d6a21ed117a9c4da6a8e7`. The
[receipt](../../benchmarks/mech-deflate/native-profile/visit-predecessors-compile-apple-m3-20261114.archive.json)
pins the manifest and verified replay; the
[compact result](../../benchmarks/mech-deflate/native-profile/visit-predecessors-compile-apple-m3-20261114.json)
contains full-precision observations and arithmetic. The repository copy was
extracted and verified, and its offline replay reproduced the summary byte for
byte. See the [evidence index](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
for checkout-only verification commands. Earlier packets remain unchanged.
