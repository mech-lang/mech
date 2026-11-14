# Resolve immutable lexical sites once

The compiler repeatedly walked a library operation's complete lexical declaration
tree to recover its operand schemas, both during native lowering and later binding.
An artifact-owned index now resolves that descriptive metadata once per owner.
The level-12 compressor's full compiler time falls **12.6%**, from
**35.71–36.36 seconds to 31.49–31.53 seconds**, with identical source, bytecode,
native Object and complete metadata. Complete process time falls **11.0%**.
The sub-ten-second full-compilation goal remains open.

The [preceding process profile](native-compiler-process-profile.md) identifies
repeated `control_source_schema` calls, temporary schema vectors and lexical tree
traversal in native lowering and preparation. This change removes that repeated
work; it changes neither parser grammar nor the emitted program.

## Current PR and measurement boundary

Base/control is `60f25c091106a9372e5b43165a06ac3fbaf7a467`, including the freshly
pulled PR 973 reduction-bound commit `79e0649f0088cee334ed7d071f68ad085b112525`.
The index source is `89f84855d203cd40a133b0c866b8c727fd0d9ffc`.
Both primary subjects enable the same **53 flags**, including
`MECH_CARRY_REDUCTION_BOUND=1`. That preexisting runtime optimization is independent
of indexing. Its enabled Object differs from the preceding 52-flag profile; only
the matched 53-flag control/candidate comparisons qualify the index.

A separate rebuilt-control integration compile with the preceding 52 flags
reproduces every previously pinned output byte. Its first timing wrapper returns
1 because sandboxing denies macOS clockrate statistics, after all four compiler
outputs are written and match. That attempt is retained and excluded from primary
timing. The identical integration retry succeeds with host statistics access.
Both primary subjects then use the same access. The integration observations do
not enter the four-observation cohort.

All measurements use the same Apple M3 Air, 8 cores (4P+4E), 16 GB, macOS 26.2.
Primary observations are serial ABBA full source-to-Object compiles. No builds,
tests, fixtures or profiling overlap them. Compiler bootstrap, source/tool
retrieval and host linking are excluded. These four observations describe this
cohort; they are not a statistical population.

## Qualified observations

| Observation | Compiler s | Complete process s | Native lowering s | Preparation s | Retired instructions |
| --- | ---: | ---: | ---: | ---: | ---: |
| Control 1 | 35.707 | 35.900 | 3.692 | 8.454 | 572,732,547,658 |
| Candidate 1 | 31.488 | 32.740 | 0.210 | 7.342 | 474,945,572,741 |
| Candidate 2 | 31.529 | 31.713 | 0.211 | 7.377 | 474,960,784,869 |
| Control 2 | 36.361 | 36.539 | 3.732 | 8.517 | 573,459,966,289 |

Arithmetic-mean reductions are **12.5592% compiler elapsed**, **11.0245% complete
process elapsed**, **94.3261% native canonical lowering**, **13.2684% preparation**,
**16.5704% native total** and **17.1251% retired instructions**. The first candidate
has a longer interval outside the compiler's phase timers; it remains in the
complete-process result.

Unchanged stages retain their adverse observations. Candidate parsing averages
2.35% higher, numeric helper generation 0.99% higher, source CFG construction
0.015% higher and cleanup 0.73% higher. They are not attributed to this index.
The second control's frontend interval is higher than the other observations.
All raw elapsed and instruction values remain available.

Maximum RSS is 7,757,922,304 and 7,572,865,024 bytes for controls, and
7,737,606,144 and 7,740,981,248 for candidates. Candidate mean RSS is **0.964% higher**,
with overlapping ranges. Peak footprint is also slightly adverse. There is no
whole-compiler memory-saving claim.

The current emitted `__text` remains **1,101,332 bytes**. The latest
[size/compile comparison](native-compressor-size-compile-comparison.md) retains
fresh libdeflate 0.408–0.484-second and Roc 2.347–2.500-second clean process
observations, with their different implementation and optimization scopes.

## Index boundaries and checks

The cache belongs to one immutable `ProgramArtifact`. Its owner entries initialize
on demand. They retain operation references, contract IDs, input schema IDs,
output schema IDs and exact lexical operand identities. Clones of that immutable
artifact share descriptive records through `Arc`; separately finalized or decoded
artifacts start with an empty cache. The index contributes neither to the program
revision nor bytecode encoding. It uses no unsafe pointers or global/revision-keyed
lookup.

Every query still selects its actual factory against the current catalog and
checks its schemas and contracts. Current-visit, ownership, type, host-storage,
work and finite runtime admission proofs remain at their original boundaries.
No selected factory, semantic witness, proof or execution permission is cached.

Index construction follows the original ordered traversal and stops at its first
metadata failure. Previously encountered sites retain their results, so a later
invalid declaration cannot override an earlier query. Repeated block/local IDs
keep the first matching declaration. A comprehension operation is recorded before
its local is inserted, preserving the original queried-site result even if that
insertion subsequently detects a duplicate. Execution boundaries and recursion
back-edges retain their original traversal behavior.

**30 focused tests pass**: 14 template/image/index tests, 14 current-visit tests
and 2 codec tests. These include **114,688 differential descriptor queries** against
the original ordered walker over mixed control and comprehension scopes, malformed
references, duplicate identities and early/late failure cases. A lifecycle test
checks immutable clone sharing, fresh decoded caches and byte-identical encoding.
Six complete additional fixture programs preserve all four outputs between the
frozen compilers. The state helper uses the maintained full assembly.

The initial test build fails on an ambiguous integer type in the generated loop.
An explicit `u32` annotation corrects only `cfg(test)` source. The final confirmation
build reproduces the initial release executable hash, and every production patch
chunk is identical. The failed test log, original patch/build and correction remain.
The candidate compiler bootstrap takes 243.870 seconds; its unchanged-executable
confirmation takes 0.148 seconds. Control bootstrap takes 81.174 seconds. None is
program compilation time.

All four primary compiles and six complete fixture comparisons preserve every
source/MBC/Object/full metadata byte between control and candidate. Indexing
therefore preserves the generated runtime program; no new runtime-speed campaign
is claimed. Parser source and language are unchanged.

## Remaining measured opportunities and evidence

The candidate still spends about **6.3 seconds in the frontend**, **7.4 seconds in
preparation**, **4.7 seconds generating numeric helpers**, **2.45 seconds building
main CFG/SSA** and **5.0 seconds cleaning it up**. Parsing remains about 2.45 seconds.
The preceding main-entry census records 19.5 million distinct temporary aliases.
The shared guard emitter's fresh success blocks have a known single predecessor
but remain unsealed until later; earlier sealing is a concrete next experiment.
It is not part of this change or a claimed speedup. Repeated dimension cloning and
exact liveness/traversal queries remain additional measured targets.

The [compact result](../../benchmarks/mech-deflate/native-profile/lexical-index-compile-apple-m3-20261114.json)
and [archive receipt](../../benchmarks/mech-deflate/native-profile/lexical-index-compile-apple-m3-20261114.archive.json)
pin every command, flags, input/output hash, original failure, build/test log,
fixture, source snapshot and offline replay. Verify the public archive with the
maintained `experiment.py verify`, then run `python3 replay.py` in its extraction.
Replay requires no compiler, generated program execution or original scratch path.
