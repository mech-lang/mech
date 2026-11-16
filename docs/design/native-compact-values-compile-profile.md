# Retired SSA value compaction: compiler qualification

The latest matched level-12 compressor takes **22.672–22.879 seconds for a fresh
complete source-to-Object process**. Post-producer value compaction saves 2.21%
of compiler time, 9.44% of cleanup including its own cost, and 1.95% of retired
instructions across eight current runs. Generated source, MBC, Object and complete
metadata bytes remain identical. Peak RSS is 0.04% higher, and elapsed ranges
overlap. The full ten-second compilation goal remains open.

After SSA construction, the main function contains **20,787,570** value IDs.
Most are retired aliases. Compaction retains **1,241,393** real definitions,
preserves their relative order, and remaps every use before releasing the old
table. Instructions, blocks, layout, types, effects and errors keep their existing
identities and behavior. The builder has finished and discarded all external
Value handles before this pass runs. No semantic, ownership, current-visit or
factory-selection proof is cached.

The unchanged alias-chain resolver is shared with the existing alias pass.
Compaction keeps resolved targets available until it has rewritten shared pooled
arguments, unused jump and exception tables, exception context items, union
inputs and debug labels. Two idempotent sweeps use a disjoint temporary ID
namespace, so shared storage cannot be renumbered twice. Extremely large graphs
retain the ordinary path if the namespace would exceed BlockArg payload bits.

PR 973 advanced during qualification. The final source is
`57376165608ec32729ff99a04b601d52d8f64950`, integrating the independently qualified
short-reduction change from control source `1c09216698a3f4c3d5c33da150d1a9b3d1b48eaa`.
Both current subjects use identical **60 switches**, including the five new
short-reduction switches. The control reproduces its author-qualified Object
`8a4c2b172798d06252055b20398cdbf9517b4b5be76730ea1824435ef89bbba4`.
The candidate retains that exact Object and descriptor. The earlier 55-switch
qualification is preserved separately below; different configurations and
generated programs are never pooled.

Hardware: Apple M3 Air, 8 cores (4P+4E), 16 GB, macOS 26.2. Eight current runs use consecutive
control/candidate/candidate/control then candidate/control/control/candidate
blocks. Builds, tests, screens, fixtures, profiling and packet work did not
overlap primary timing. Every compiler total includes compaction.

| Current measurement | Control range (4 runs) | Candidate range (4 runs) | Mean change |
| --- | --- | --- | ---: |
| Compiler, s | 22.515432–23.384138 | 22.480996–22.683971 | -2.21% |
| Whole process, s | 22.683263–23.506979 | 22.671849–22.878756 | -2.03% |
| Parse, s | 2.305657–2.461839 | 2.438279–2.450127 | +1.11% |
| Frontend, s | 2.945993–3.038496 | 3.001310–3.042812 | +0.27% |
| Native, s | 17.189593–17.827225 | 16.955267–17.164918 | -3.09% |
| Canonical lowering, s | 0.197929–0.210110 | 0.204993–0.210291 | +0.93% |
| Preparation, s | 3.639002–3.769717 | 3.754945–3.772676 | +0.82% |
| Numeric generation, s | 3.997212–4.126623 | 4.104173–4.131614 | +0.63% |
| Source CFG, s | 2.445730–2.544640 | 2.463146–2.551004 | -0.35% |
| Cleanup including compaction, s | 4.043574–4.198300 | 3.670436–3.761528 | -9.44% |
| Cleanup after compaction, s | 4.043574–4.198300 | 3.504521–3.594811 | -13.51% |
| Compaction, s | 0.000000–0.000000 | 0.165916–0.168445 | Added cost |
| Retired instructions | 351,733,896,677–352,030,373,059 | 344,848,271,316–345,289,454,468 | -1.95% |
| Peak RSS, bytes | 5,993,791,488–5,997,395,968 | 5,993,971,712–6,001,590,272 | +0.04% |
| Peak footprint, bytes | 5,962,257,688–5,965,878,576 | 5,962,470,680–5,970,105,648 | +0.04% |

The two current block means improve compiler time by 1.60% and 2.81%.
The first control is faster than some candidates; the complete timing ranges
overlap. The small adverse RSS and footprint observations remain, along with
slower parse, frontend, canonical-lowering, preparation and numeric-generation
means. Those unchanged-stage differences have no causal attribution to this
pass. These finite cohorts are descriptive, not population estimates.

The separate historical 55-switch base uses source commit `19c1fd7aaf` on
parent `4f90b1b4f0`, with compiler binary pins retained in the compact result.
Its eight runs show 2.12% less compiler
time, 8.63% less cleanup including compaction and 1.67% fewer instructions.
Complete process observations are 22.047–22.890 seconds. Both historical blocks
improve their mean compiler time (0.78% and 3.44%); timing ranges also overlap.
Its older Object remains pinned separately. No historical observation is used
in the current means above.

All seven production tests pass on both bases. Coverage includes 65 exact
supported native-code pairs per base, 64 branching programs, forward and backward
aliases, holes, 1,000,003 aliases, stable result positions, shared argument pools,
unused tables, exception pseudo-arguments and context items, union inputs and
debug labels. The multi-result fixture is verified as IR: this backend does
not lower its I32 isplit. Each base also passes fourteen successful fixture
pairs covering complete array/enum programs and checked Bool/U64/I64/F64 cases.
Every source, MBC, Object and complete metadata byte matches within each pair;
both screens and all primary compressor outputs match within each base.

The first prototype passed five tests and preserved compressor output bytes.
Tightened tests exposed alias references left in unused metadata, and that
prototype was corrected before retention. Both failing tests, their patch and
the original screen remain. An unused-binding build correction, an invalid
I128-constant fixture and the mistaken unsupported split-codegen invocation
also remain, with their original commands and logs. No failed observation is
counted as successful coverage. Rebuilds and subsequent cached executable
lookups are recorded separately; no cold bootstrap timing is claimed.

Two separate CPU profiles retain all raw and demangled stacks, self-count
tables, executable pins, existing demangler source and output bytes. Each has
two completed sample files and a third unsuccessful attempt (exit 255). They
are excluded from primary timing. In the current first window, SSA nonlocal
variable lookup has 160 self samples, dimension canonicalization 158, liveness
147, declaration-array cloning 116 and traversal analysis 111. Main-thread
waits are excluded. Samples span stages; these counts are not stage-exclusive
elapsed time or exact source-line attribution. SSA construction, frontend
copying and liveness remain substantial opportunities after this pass.

No new program-runtime latency campaign is claimed. Exact generated machine
code and complete metadata preserve the previously qualified program on each
base. Grammar, language, direct scalar emission, checked errors, finite
admission, current ownership checks and backend optimization levels are unchanged.

| Current output | Bytes | SHA-256 |
| --- | ---: | --- |
| metadata.json | 2,292,704 | `0f6caeb3b18c65552dcda3e5fa460e1796884e4131e275473cfd2858339382cf` |
| program.mecb | 4,138,627 | `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7` |
| program.o | 1,238,328 | `8a4c2b172798d06252055b20398cdbf9517b4b5be76730ea1824435ef89bbba4` |
| source.mec | 160,857 | `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9` |

Current compiler executable pins:

- Control: `0efd7bb64afc39f07065aa3e079dfd30bb4b82a8755a0b38331e7e4c9060072c`.
- Candidate: `fa4e15f616f0bed889ef300b5907048db12f5db808c4c4688b8698699679d3e2`.
- Optimization patch: `acf33e5a9894ea13dfe957d29681607748067b443aa39e635d8e61d0a6cdcd5f`.

The complete [907-file evidence archive](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/mech-compact-values-20261114.tar.gz)
is tracked in benchmarks at the user's request. It retains all commands, logs,
phase timestamps, resource counters, source snapshots and patches, output bytes,
fixture comparisons, corrections, profiles and pure offline replay. Executables
are excluded and hash-pinned. Archive SHA-256:
`494832ac6beca385ced77297abb3b3d0814f0270368310db9a697dda0c37d0da`.
The [receipt](../../benchmarks/mech-deflate/native-profile/compact-values-compile-apple-m3-20261114.archive.json)
pins the manifest and verification; the
[compact result](../../benchmarks/mech-deflate/native-profile/compact-values-compile-apple-m3-20261114.json)
retains every full-precision observation and reconstructed calculation. The
repository copy was extracted and verified, then replayed byte-identically.
See the [evidence index](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
for checkout-only commands. Earlier archives remain unchanged.
