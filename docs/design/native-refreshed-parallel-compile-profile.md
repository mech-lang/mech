# Parallel compilation on the updated compressor

The parallel path produces identical executables, but establishes no useful
speedup on the updated workload. Eight complete builds average **19.166096
seconds serial and 19.196338 seconds with two workers**, a 0.16% slowdown with
mixed blocks. Default `MECH_COMPACT_COMPILE_THREADS=0` remains serial; two workers
remain opt-in.

The frozen source is `934bde961d47555db3fa45bc5c28d36ad1c0930a`, incorporating
PR #973's `9e34dc07e0` compressor/runtime updates. All seven worker implementation
files are byte-identical to `1090de011646c92e30c2d4b648ed0792c50f300d`.
Both subjects use one standard compiler binary and the same 61 program flags;
only scheduling `0` versus `2` differs. The host is the same Apple M3.

## Timing

| Boundary | Serial | Two workers | Change |
| --- | ---: | ---: | ---: |
| Complete source to executable | 19.166096 s | 19.196338 s | +0.16% |
| Source to Object within those builds | 19.030447 s | 19.063511 s | +0.17% |
| Final native link | 0.134686 s | 0.131730 s | -2.20% |

The first ABBA block is 1.06% slower with workers; the second BAAB block is
0.73% faster. All observations remain, including the faster first serial run.
Link movement is observational; there is no linker change. The complete native
boundary includes fresh source assembly/analysis, optimization/code generation,
Object/metadata writes, Object handoff and native link process completion.

All eight observations run serially after compiler/adapter construction, with
no concurrent heavy build, test, sampling, hashing or archival. The current
runtime/entry adapter and native inputs are pinned and cached. Its first-use
construction takes 11.883 seconds and is excluded from primary timings. Two
cached relinks reproduce its entire executable before primary measurement.
The separate compiler check/build take 6.804 and 253.459 seconds.

The updated program's bytecode grows from 4,138,627 to 4,733,696 bytes, Object
from 1,238,656 to 1,419,832 bytes, and descriptor from 2,292,881 to 2,519,706
bytes. This cohort is separate from the earlier smaller-workload
[17.657 to 17.379 second result](native-parallel-phases-compile-profile.md) and
from both PGO cohorts. No cross-revision compiler slowdown or additive gain is
inferred from those different program/runtime images.

## Equivalence and next work

Every source, bytecode, Object and complete metadata byte matches in all eight
builds. Each complete executable has 2,060,440 bytes and SHA-256
`d6c65cc3fb18acc4be77dce329962105bf0f2b56dd2623a12f466256463eca61`.
The fresh control also reproduces the updated PR's source, bytecode and Object
pins. No new emitted-program runtime campaign, scalar retirement, grammar or
language change, or weaker proof check is claimed.

Function construction and custom cleanup remain on the coordinator; existing
workers perform function-local machine compilation. Further work targets that
serial preparation/optimization cost. This requalification makes no new source
change and establishes no additional parallel saving. Complete compilation
below ten seconds remains open.

## Evidence

The thirtieth
[repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
contains all commands/logs, compiler/runtime/worker source snapshots, complete
program outputs/executables, fixed native inputs and pure replay. Compiler
binaries and dependency libraries/toolchain are excluded and hash-pinned.
Internal phase clocks are retained; no new instruction-counter cohort or
exclusive cleanup measurement is claimed. A fresh extraction verifies every
manifest/file/Git blob, then reconstructs all means/blocks and checks every
output byte against the compact result without rerunning compilation or timing.
