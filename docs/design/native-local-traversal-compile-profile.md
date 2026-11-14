# Proof-local numeric traversal: compilation qualification

Complete fresh level-12 compilation takes **19.741–20.163 seconds** on the Apple
M3 Air. Eight matched runs show **2.77% lower whole-process time, 2.80% lower
compiler time, 18.01% lower numeric-helper generation time and 4.88% fewer
instructions**. Source, MBC, Object and complete metadata bytes remain identical.
Peak RSS is an adverse 0.028% higher. The ten-second compilation goal remains open.

Numeric traversal analysis previously enumerated every program iteration for each
helper and repeatedly filtered that helper's sites, even for unrelated loops.
It now groups only the current proof's sites, preserving their relative order and
the original ascending iteration order. Loops whose header is absent still get
no scope or proof.

Supported-prefix analysis also scanned every prepared visit to find edges from
omitted predecessors. It now queries the prepared owner's existing reverse-edge
index against the fresh current set and header. The same descendant pruning then
runs. Seed order and duplicates cannot change this monotone removal; discovery
already checked every retained site's successor shape. No scope-validity or guard
answer is cached. The original first-refusal query also keeps its source ordering.
These two compiler-only changes are qualified together; the measurements do not
isolate their individual savings.

Source `1774ca003111a511d9f961fded0005412e1f39df`, parent `a0832dbe7ad978600f0dac8525ad1bb32d60d103`.
The frozen control comes from `7feb94669905d3413d7a79fb9f8b89b851027226`; its production
source equals the documentation-only parent. Both subjects use the same opt-in
`compiler` profile, full fresh source-to-Object pipeline and 60 runtime flags.
Hardware: Apple M3 Air, 8 cores (4P+4E), 16 GB, macOS 26.2.

| Measurement | Control range (4 runs) | Candidate range (4 runs) | Mean change |
| --- | --- | --- | ---: |
| Compiler, s | 20.359407–20.464804 | 19.684034–20.015941 | -2.80% |
| Whole process, s | 20.559353–20.580938 | 19.740986–20.163215 | -2.77% |
| Parse, s | 2.419707–2.447224 | 2.451712–2.467365 | +1.22% |
| Frontend, s | 2.499307–2.539825 | 2.507594–2.601412 | +0.92% |
| Native, s | 15.329804–15.451163 | 14.627504–14.907999 | -4.06% |
| Canonical lowering, s | 0.097128–0.098284 | 0.098033–0.100058 | +1.30% |
| Preparation, s | 2.998239–3.008256 | 2.968390–3.051622 | +0.20% |
| Numeric generation, s | 3.851237–3.860131 | 3.151239–3.177989 | -18.01% |
| Source CFG, s | 2.329533–2.343723 | 2.346925–2.422133 | +1.95% |
| Cleanup including compaction, s | 3.376956–3.479152 | 3.351497–3.455739 | -0.63% |
| Compaction, s | 0.168927–0.172044 | 0.165434–0.171519 | -1.93% |
| Retired instructions | 311,603,585,915–311,705,930,111 | 296,320,973,102–296,629,603,180 | -4.88% |
| Peak RSS, bytes | 5,911,216,128–5,911,511,040 | 5,912,002,560–5,915,246,592 | +0.03% |
| Peak footprint, bytes | 5,880,911,056–5,881,189,560 | 5,881,681,104–5,884,925,136 | +0.03% |

Runs are serial control/candidate/candidate/control then
candidate/control/control/candidate. Both block means improve compiler time
(2.94% and 2.67%). Every candidate compiler, process, native, helper-generation and
instruction observation falls below every control. Every candidate RSS and peak
footprint observation is slightly above every control; no memory saving is claimed.
Parse, frontend, preparation, source CFG and canonical-lowering means are adverse
and retained. These finite cohorts are descriptive, not population estimates.
Both subjects include SSA value compaction and its measured cost. Native substages
are nested and must not be added to whole-process time.

Builds, tests, screens, fixtures, CPU sampling, hashing, copying and packet work do
not overlap primary timing. The observed 273.093-second optimized compiler rebuild
uses existing Cargo caches, is excluded from Mech compilation timings and is not
a matched cold-bootstrap comparison. Both screens and every primary output match
the qualified program byte for byte. Fourteen successful fixture pairs cover six
array/enum programs and eight checked scalar programs; all four files match.

Five tests compile the exact production reverse-edge module. The new target query
matches the original full scan in **7,464,960 graph/scope/header contexts**, including
sparse and out-of-range targets. **196,608 seeded pruning contexts** compare final
sets through branches, duplicate edges, cycles and latch stops. Existing owner
identity, changed-scope and first-refusal checks remain. These are topology tests
and compiler-output checks; GPU/runtime unit suites were not rerun for this trial.
No execution machinery is retired. Grammar, language, checked errors, direct
scalar emission, fresh shape/factory/visit/ownership proofs, admission limits,
runtime accounting and backend optimization settings retain their behavior.
Generated machine code and complete metadata are identical; no new compressor
runtime timing campaign is claimed.

A separate diagnostic compile samples one requested one-second helper window and
preserves all generated bytes. It retains raw/demangled stacks, commands, self
tables and the existing demangler's source/pin. The window includes 111 self
samples in tree-set insertion, 55 in definition-inventory initialization and 50
in word-level `required` liveness. Coordinator waits are excluded. These counts
are neither phase-exclusive elapsed measurements nor exact source-line attribution.
The next recorded candidates are packed word-level liveness and sharing descriptive
definition inventories within one immutable compilation. Neither is applied or
credited here. Earlier CPU profiles keep their original source pins.

The [compact result](../../benchmarks/mech-deflate/native-profile/local-traversal-compile-apple-m3-20261114.json)
and [receipt](../../benchmarks/mech-deflate/native-profile/local-traversal-compile-apple-m3-20261114.archive.json)
pin the complete [benchmark packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md).
It contains exact source/base snapshots and patches, commands, flags, tests, raw
cohorts, outputs, the separate CPU profile and pure offline replay. Compiler/test
executables are excluded and hash-pinned. Python 3.11+ replay needs no compilers,
benchmark execution, original scratch paths or external services.
