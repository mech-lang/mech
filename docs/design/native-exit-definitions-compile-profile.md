# Selected exit-query definitions

The latest complete fresh level-12 source-to-Object process takes
**18.690–19.107 seconds** on the Apple M3 Air,
compared with **18.685–19.107 seconds** for the matched
control. Total compiler time is essentially flat (**-0.07%** mean):
two balanced blocks are slower and two faster. An overall compiler-wall
improvement is unestablished. The mean process change is
**-0.27%** and
preparation changes **-0.60%**. The compiler instruction
count changes **-0.29%**. Complete compilation remains above
the ten-second goal.

The former exit liveness query checked its exact graph and read sets, then
scanned every plan value (22,460 for this compressor), looking up definitions
even for values never read by that query. The first lookup already builds and
validates the entire immutable definition inventory. The new selection preserves
that first demand, then visits only sorted distinct valid read IDs. It removes
the full-plan read bitmap and value scan from each query.

The source is `414693ce793b1ea1e99a6dcac34cb2533d9c856c`, measured against production control
`4a451c748295164aa300d4e03ff9b6a4fdbd740f`, with build base `9b9fb4ac13d42d17d392fc026d76d94d241376a0`. The exact
source patch and executable hashes are in the compact result and full packet.
There is no new engine API, grammar production, language change, serialized
format, global cache or persistent/current-visit proof.

## Matched full compilation

Sixteen serial observations use ABBA then BAAB order twice, the same 60 runtime flags,
toolchain, optimized `compiler` profile and full source pipeline. Both subjects
include SSA value compaction and its elapsed cost. Each run starts a new process,
parses the full source, constructs the canonical artifact, emits bytecode and
produces native Object and complete metadata. No cached Object or decoded-only
substitute is used. Builds, tests, screens, fixtures, sampling, hashing, copying
and archive work do not overlap the primary observations.

The table contains arithmetic means of all aggregate comparison fields. Phase
and nested-stage intervals overlap and are not additive. All primary observations
and full phase traces remain in the raw packet. Overlapping ranges and sixteen
observations do not establish a population estimate or isolate unrelated phases.

| Measurement | Control mean | Candidate mean | Change |
| --- | ---: | ---: | ---: |
| Complete process | 18.9211 s | 18.8697 s | -0.27% |
| Compiler phases | 18.7293 s | 18.7157 s | -0.07% |
| Parsing | 2.4198 s | 2.4222 s | +0.10% |
| Catalog | 0.0366 s | 0.0382 s | +4.42% |
| Artifact | 0.0259 s | 0.0262 s | +1.15% |
| Bytecode encoding | 0.0092 s | 0.0092 s | -0.23% |
| Frontend | 2.5060 s | 2.5103 s | +0.17% |
| Native | 13.7310 s | 13.7090 s | -0.16% |
| Canonical lowering | 0.0981 s | 0.0989 s | +0.82% |
| Preparation | 3.0209 s | 3.0029 s | -0.60% |
| Numeric helper generation | 2.1275 s | 2.1262 s | -0.06% |
| Source CFG construction | 2.3469 s | 2.3701 s | +0.99% |
| Cleanup including compaction | 3.4420 s | 3.3936 s | -1.40% |
| CFG cleanup | 3.2729 s | 3.2241 s | -1.49% |
| Value compaction | 0.1690 s | 0.1696 s | +0.31% |
| Retired instructions | 277.6863 G | 276.8887 G | -0.29% |
| Maximum RSS | 5.9118 GB | 5.9120 GB | +0.0029% |
| Peak footprint | 5.8813 GB | 5.8815 GB | +0.0033% |

All four balanced block results and all individual values are retained in
the [compact result](../../benchmarks/mech-deflate/native-profile/exit-definitions-compile-apple-m3-20261114.json).
The first eight observations suggested a small mean improvement, but their
compiler-time block directions disagreed (+0.18% and -0.97%). Eight additional
observations tested that uncertainty without replacing any run. Their block
changes are +0.67% and -0.16%. The initial and repeated cohorts, reason for
repetition, all values and the aggregate are retained. Every candidate retires
fewer instructions than every control, while preparation ranges overlap. The
retained claim is a small work reduction, with total compiler time effectively
flat.

The separate observed compiler-executable rebuild takes **164.991
seconds**, using existing Cargo caches. It is excluded from Mech compilation
timing and is not a matched cold-bootstrap comparison. The test build timings,
including the failed initial setup, are separately recorded in the raw packet.

## Validation and program speed

All source, MBC, Object and complete metadata bytes match the qualified current
program in both screens and every primary observation. Fourteen successful
fixture pairs also reproduce all four files, including numeric phases, nested
and changing captures, array windows, and Bool/U64/I64/F64 scalar controls.
The generated instructions and metadata are identical; no new compressor-runtime
latency campaign is claimed. No scalar execution machinery is retired.

All **17 focused Rust tests pass**: four production
selection tests and thirteen existing liveness tests. There are **15,360**
original/decoded random graph/subset comparisons against the previous full-value
loop, plus explicit cases for malformed unused producers, empty read/wanted
sets, invalid read/wanted IDs, zero-value plans, duplicate records, site-less
inputs and unselected/out-of-range definition sites. Independent facts owners
prevent one side from initializing the other's inventory.

The first definition lookup still validates all producers even if the read set
is empty. When the previous loop made no demand (no values, an empty wanted set,
or only wanted IDs outside the value range), the new selection makes none.
Byte comparisons verify that graph traversal/read validation and remaining
exit/liveness logic are unchanged. Current shape/factory/visit/ownership checks,
finite admission, checked errors and runtime accounting retain their existing
execution paths. Earlier bare runtime-only lint failures keep their original
source pins in the helper-liveness packet; they were not rerun or waived here.

Both new Rust files pass `rustfmt --check`. The existing caller file fails the
same check on parent and candidate due to its unchanged long liveness-call line;
the commands and both outputs are recorded. No formatting mutation changes the
measured source.

The initial integration test build failed with E0599 because the harness called
a nonexistent plan-loading method. The corrected test uses the existing confined
image loader with the exact source artifact schema table. The failed command,
source patch and logs remain in the packet. This correction changes test setup.

## Reproduce the results

The [raw packet index](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
and [receipt](../../benchmarks/mech-deflate/native-profile/exit-definitions-compile-apple-m3-20261114.archive.json)
pin the complete repository archive: every observation, command, flag set,
fixture, source snapshot and patch, correction, raw output, hash and license.
Compiler executables are excluded and hash-pinned. The maintained `experiment.py`
verifier checks the published repository copy, its manifest, safe paths and every
file; pure offline `python3 replay.py` reconstructs this result byte-for-byte
without a compiler, benchmark, original scratch path or network.

The next parser candidate is applying existing bounded predictions to nested
base-rule calls while preserving logical work, Unicode, limits, resumptions,
trees and diagnostics. It has no measured saving in this packet.
