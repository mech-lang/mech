# Combined compiler integration and resident-memory profile

PR 973 source `342937b4cba43ed57d08ca78648dde4b188d82b7` contains both qualified
compiler optimizations and the independently measured
[nested-reduction runtime change](native-reduction-rotation.md). A clean rebuild
reproduces that change's native Object exactly:
`8f33516fb93e1614a02f64d6dec1bd92f68abc6c4824b468c71840da494141a7`.
Both compiler optimizations therefore remain compatible with the advancing PR.

Source and MBC match our own previous complete profiling input. Metadata changes
with the independent runtime transformation and is recorded separately. No
bytecode identity with the separately authored runtime packet is claimed.

## The largest observed resident-memory spike is in the frontend

Two separate diagnostic processes use the frozen combined compiler and the same
54 flags. The first captures 373 asynchronous stage-boundary snapshots,
including 344 numeric-function boundaries. The second requests 100 ms waits
between `ps` calls and retains all 244 actual timestamps and active-stage labels.
All four generated outputs match byte-for-byte between these runs.

| Continuous trace: active stage | Highest observed RSS |
| --- | ---: |
| Frontend semantic analysis | 6.913 GiB |
| Native preparation | 2.737 GiB |
| Main source CFG construction | 2.244 GiB |
| Main CFG cleanup | 3.161 GiB |
| Main machine-code generation | 4.906 GiB |
| Host-reported process maximum RSS | 6.934 GiB |

The two largest continuous samples are during frontend semantic analysis. The
coarse boundary snapshot after that stage sees only 2.688 GiB, so boundary-only
sampling misses its transient peak. The parser-end boundary is 0.090 GiB.
These are resident snapshots, not allocation attribution: they cannot distinguish
live, retained, shared or compressed pages, or exclude peaks between samples.
Host peak-memory footprint counters are retained separately from RSS.

The prior [CPU profile](native-compiler-process-profile.md) identifies dimension
environment cloning, canonicalization and normalization in this frontend code.
The memory trace supports prioritizing that work. Borrowing an environment
during immediate finalization, while retaining the same validation order and
normalized result, is a concrete next experiment. No such memory fix is part of
this report.

Backend source also has reservations and arrays sized from historical DFG value
counts, including `VRegAllocator::with_capacity(num_values * 2)` and egraph
scoped-map capacities. These remain a separate hypothesis to inspect against
attached values; they are not established as the cause of the largest peak.

## Scope and evidence

These diagnostics are excluded from all primary timings. They establish no new
matched compile-time improvement or runtime-latency measurement. Compiler
bootstrap and process-monitor overhead remain separate. No compiler source
instrumentation or functional change is added. The latest matched compilation
range remains **29.61–30.03 seconds per complete process**, pinned to the
[selected-definition cohort](native-liveness-definitions-compile-profile.md)
before the independent runtime change. The sub-ten-second goal remains open.

The [compact result](../../benchmarks/mech-deflate/native-profile/current-pipeline-memory-apple-m3-20261114.json)
and [archive receipt](../../benchmarks/mech-deflate/native-profile/current-pipeline-memory-apple-m3-20261114.archive.json)
pin all 64 files, commands, source snapshots, input/output hashes, rebuild and
raw process observations. The local archive verifies every hash/Git blob and
replays byte-identically without a compiler or generated executable. Public
export awaits authorization; raw data and executables remain outside source Git.
