# Reject ownership-join SSA forwarding after runtime qualification

The provisional SSA construction change reduces full compiler time **8.13%**,
but repeated compressor runtime cohorts show slowdowns. It is **reverted before
updating PR 973**. The trial and revert remain in published Git ancestry; no SSA
forwarding change remains in the resulting source.

Source `d4efd4465c5a8689016abd5b42e16f73e475af63` seals array helper blocks
earlier and lets proven unchanged ownership diamonds forward incoming SSA
variable state. Different assignments, unknown predecessors, cycles and bounded
search failures retain ordinary SSA construction. Sixteen focused tests pass,
including an unsealed enclosing join and real one-arm assignments. This proves
neither generated-code identity nor runtime nonregression.

## Matched compiler observations

Both frozen subjects enable the same **54 existing runtime flags**, including
`MECH_NORMALIZE_DEFAULTED_READ_SETUP`. Earlier 53-flag screens remain diagnostic
and are excluded. The flag discovery, superseded guard-only trial and initial
failed test oracle are retained in the packet.

| Measurement | Control | Candidate | Mean reduction |
| --- | ---: | ---: | ---: |
| Compiler source through Object | 31.142, 31.948 s | 28.898, 29.060 s | 8.13% |
| Complete process | 32.310, 32.113 s | 29.797, 29.190 s | 8.44% |
| Main source CFG/SSA construction | 2.419, 2.493 s | 1.040, 1.048 s | 57.48% |
| Main cleanup | 4.793, 4.956 s | 4.312, 4.348 s | 11.17% |
| Retired instructions | 475.955, 475.463 billion | 442.386, 442.581 billion | 6.98% |

These are four serial ABBA observations, with bootstrap, diagnostics, fixtures
and runtime work excluded. Candidate maximum RSS is **3.20% higher on average**;
both candidate observations exceed both controls. Unchanged-stage variation,
including frontend time, is not causally attributed to SSA forwarding.

Source and MBC bytes match. Object bytes change from 1,237,696 to 1,235,264:
control `b125164d7e8ba9d4e98cb4b467b3194bb14a6d8cafe411600359c6f67943c703`,
candidate `66bface4f334fd291e6f8e048884e027e6134cdc9164c378938052697155cd87`.
The control Object exactly matches the previously qualified fast compressor.

## Correctness passes, runtime speed does not

Both Objects pass all 81 corpus cases: exact golden raw output, independent
inflation, unchanged borrowed aliases, capacity and six allocation/copy/zeroing
metrics. There are 486 checked smoke observations plus startup warmups.

The maintained `experiment.py measure` then runs all 81 cases with four process
groups and 12 alternating blocks per group. It retains **125,856 primary calls**
and **127,152 checked observations** including excluded initial/final calls.
Seventeen zero-duration short-call observations are retained; their block
medians are positive. Nine suspect/noisy cases receive a separate, predeclared
eight-group repeat, preserving the original cohort and every adverse sample.

| Case | Original paired-block median change | Disclosed repeat change |
| --- | ---: | ---: |
| Natural 16,384 bytes | +1.17% | +0.82% |
| Natural 65,535 bytes | +0.67% | +0.60% |
| Natural 65,536 bytes | +0.78% | +0.52% |
| Natural 65,537 bytes | +0.94% | +0.59% |
| Repeated 32,769 bytes | +4.80% | +2.28% |
| Repeated 33,027 bytes | +0.85% | +1.03% |
| Repeated 1,048,576 bytes | +0.79% | +0.70% |
| Held-out mixed 131,073 bytes | +0.58% | −0.42% |
| Reymont prefix 16,384 bytes | +0.66% | +0.77% |

All eight repeat group medians are above one for the three natural sizes near
64 KiB and repeated 33,027 bytes. Blocks within a process group are descriptive,
not independent experiments. No automatic tolerance grants acceptance; the
observed compiler saving does not justify these repeated runtime slowdowns.
The cause of the generated-program slowdown is not established by this trial.

Both workers share the maintained Heap runtime, the same freshly built pinned
`libmimalloc-sys 0.1.35` archive, compiler/runtime libraries, corpus and timing/
validation closure. No byte-identical historical allocator object or absolute
cross-cohort runtime comparison is claimed. The separate reference is the same
control program outside paired timing, not Roc or libdeflate.

The historical diagnostic boundary includes Heap construction, complete compact
execution, output transfer and private cleanup with prepared borrowed arguments.
Ordinary exact resource admission/accounting/publication recovery is outside
that boundary. Warmups, initial/final calls and validation are excluded from
primary runtime timing. No builds, tests or profiles overlap primary timing.

## Evidence

The [complete replayed result](../../benchmarks/mech-deflate/native-profile/ssa-forwarding-rejected-apple-m3-20261114.json)
and [archive receipt](../../benchmarks/mech-deflate/native-profile/ssa-forwarding-rejected-apple-m3-20261114.archive.json)
pin 4,114 files and all observations. The [complete archive](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/mech-ssa-forwarding-rejected-20261114.tar.gz)
is published in benchmarks. Its repository copy verifies every hash/Git blob and
replays the result byte-identically offline. Compiler executables are excluded
and hash-pinned. No grammar or language change is retained;
the full-compilation sub-ten-second goal remains open.
