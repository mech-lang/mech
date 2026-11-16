# Queue each reachability block once during cleanup

Queuing each block once in the identity-route reachability traversal improves
level-12 Deflate source-to-executable compilation **15.694847 to 15.463704
seconds (1.47%)**, with both balanced blocks faster and every executable byte
identical. Cleanup including value compaction falls **5.45%**. The change is
retained. Compilation below ten seconds remains open.

Qualified source `d26cf2c65eac2f5978cabba257bb8bebbe4b0a80` has exactly the
measured one-file diff from `6b29f4a0fdacd5cbad2356144e402dae42323aff`:
SHA-256 `8a313dd1bf6550f9546e161bc5ccb0fc32ded92ca5e8879053936e2ff339f073`.
The source commit precedes evidence publication; its entire diff includes the
production traversal and its focused tests, with the comparison retained.

## Identified work and implementation

The two control-compiler stack diagnostics in
[packet 37](native-current-parallel-compile-profile.md) contain 43 and 42
sampled stacks copying under identity-route discovery. Their caller addresses,
normalized using each process's image base and the Mach-O preferred text
address, both resolve to `0x1022cba08` in the pinned `c5b0` compiler.
Bounded disassembly shows eight-byte successor entries being appended to the
pending worklist, followed by the pop/stop/visited loop. This identifies
`pending.extend(&self.successors[site])`. An earlier FIFO-eviction hypothesis
was incorrect; the 32-entry reachability cache remains unchanged. The original
module is identical in that diagnostic compiler and the current control.

The old traversal appends every successor and marks a block when popped,
discarding duplicate entries afterward. The new traversal marks each non-stop
block before queuing it, so each block enters the worklist at most once.
It returns the same complete boolean reachability bitmap. Start/stop behavior,
the existing cache contents and FIFO eviction, and previously returned shared
rows retain their semantics. Discovery still uses one immutable CFG before
applying any selected routes. Eligibility, current-visit proofs, errors and
finite execution accounting retain their existing paths.

The sampled counts identify a caller, not an exclusive CPU percentage or a
promise of removable time. The matched results below measure the benefit.

## Complete-build qualification

Both subjects use the retained compiler allocator and action-output inventory,
the standard ThinLTO compiler profile, current Deflate source/runtime, the same
61 program flags and two workers. The immutable control compiler SHA-256 begins
`9576`; the candidate begins `b72f`. PGO and native-CPU flags are absent.

Eight primary complete builds run serially in ABBA/BAAB order without overlapping
heavy build, test, sampling, hashing or archival. Each includes fresh source
assembly, parsing, checked frontend, preparation, optimization/code generation,
Object/metadata writes, Object handoff and final native link completion.
The fixed runtime/entry adapter and every native input are cached and pinned.
Its historical first-use construction of 11.882560208 seconds is excluded.

| Boundary | Control | Once-queued worklist | Change |
| --- | ---: | ---: | ---: |
| Complete source to executable | 15.694847 s | 15.463704 s | -1.47% |
| Source-to-Object process within those builds | 15.561865 s | 15.328919 s | -1.50% |
| Final native link | 0.132021 s | 0.133771 s | +1.33% |

The complete-build blocks improve 1.72% and 1.23%. Object handoff increases
5.53%, about 0.053 ms. Every observation remains. Link/handoff movement is
observational; their implementation and fixed inputs are unchanged.

A separate eight-run source-to-Object cohort uses the same settings and order:

| Work | Control | Once-queued worklist | Change |
| --- | ---: | ---: | ---: |
| Internal compiler phases | 15.470587 s | 15.255000 s | -1.39% |
| Complete source-to-Object process | 15.539207 s | 15.321923 s | -1.40% |
| Cleanup including value compaction | 3.140482 s | 2.969280 s | -5.45% |
| Instructions retired | 222.655 billion | 221.751 billion | -0.41% |
| Maximum compiler RSS | 4,489,752,576 B | 4,489,883,648 B | +0.0029% |
| Peak memory footprint | 4,467,064,504 B | 4,467,183,294 B | +0.0027% |

Both Object blocks improve compiler time, cleanup and instructions. Parsing
increases 0.18%, catalog time 3.81%, preparation 0.34%, source CFG construction
0.56% and canonical lowering 0.013%. All stage values and adverse observations
are in the
[complete result](../../benchmarks/mech-deflate/native-profile/reachability-worklist-apple-m3-20261115.json).
These cohorts remain separate from earlier source changes and parallel/PGO
comparisons; their percentages and timings are not pooled or added.

## Correctness and evidence

Every source, bytecode, Object and complete metadata byte matches in all
qualification observations. Fourteen paired numeric/scalar fixtures pass.
The two preliminary complete-executable gates, eight primary complete builds
and separate default-serial gate reproduce the entire 2,060,440-byte executable:
SHA-256 `d6c65cc3fb18acc4be77dce329962105bf0f2b56dd2623a12f466256463eca61`.
The default-serial observation is an unpaired byte gate, excluded from timing
means. No new emitted-program runtime timing campaign is claimed.

All three focused production-module tests pass. They cover every start/stop
pair in all 512 three-node graphs, cache reuse/eviction and retained old rows,
and parallel edges/self-cycles/stopped cycles on 1, 2, 17 and 65 nodes. A
separate BTreeSet traversal supplies the reference results.

Compiler checking takes 6.582 seconds, rebuilding 250.462 seconds and the real
native CLI check with `--features cranelift_build` 8.968 seconds. Focused test
building takes 2.166 seconds and execution 0.395 seconds. All are outside program
compilation timings. There is no failed build or test setup in this trial.

The thirty-ninth
[repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
contains all commands/logs, sources/patch, outputs/executables, native inputs,
tests and observations. Selection evidence includes both raw/demangled stacks,
the symbol bridge and analysis, address calculations, Mach-O headers and bounded
disassembly. Compiler/test binaries are excluded and hash-pinned. Fresh
extraction verifies every manifest/file/Git blob; pure replay reconstructs the
summaries and stack analysis, checks the address proof and every output, and
reproduces the compact result without rerunning compilation or timing.
