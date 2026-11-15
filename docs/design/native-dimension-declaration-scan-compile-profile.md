# Fused dimension declaration scans

Combining three declaration scans improves complete Deflate source-to-executable
compilation **15.120963 to 14.971381 seconds (0.99%)**, with both balanced blocks
faster and every executable byte identical. Frontend time falls **11.72%**.
This source change is retained. The new source has standard-compiler
qualification; fresh PGO qualification remains pending. The separate best
Deflate-trained native-CPU/PGO result is still 13.596783 seconds on earlier
source `0c949505cc`. Compilation below ten seconds remains open.

Qualified source `ef5e255e817eb5bbc6df61b094a3741b31c49da2` has exactly the
measured one-file diff from `74d50336ff80175921d45a627d354782ec5963ed`:
SHA-256 `57d00ade5de3607bddebefce7e823357bbd8a79cb4a668b138b8ca6df0b46423`.
The source commit precedes this evidence publication; its entire patch,
parent, path and source snapshots are retained and checked.

## Removed work and preserved validation

The constant-bound canonicalization path previously scans declarations for
valid IDs, then for constant bound kinds, then for valid lifetimes. One fresh
scan now validates IDs and collects the two pure classification facts. The
constant path can use those facts without the other two scans. There is no
cache or inventory pruning. Every declaration, including unused declarations,
still participates in the original validation demand.

A malformed declaration ID anywhere still precedes all bound/lifetime errors.
For constant bounds, a compile-time declaration anywhere still precedes an
unknown root. For mixed bounds, the original per-declaration lifetime and bound
normalization sequence remains, including earlier malformed bounds preceding
later lifetime failures. Constant selection, explicit/inferred parameter order,
root ordering, remapping, dependencies and cycle/forward-reference checks keep
their original code. Local facts do not replace permission or admission proofs.
No grammar, public type layout, identity limit or emitted-program change is
introduced.

The source audit checks the exact original ID-check sequence, complete mixed-
bound/rootless paths, constant selection/mapping and every other production
token. The original reference canonicalizer remains byte-identical. Existing
ID-validation tests project the new helper's classification result to unit;
their original error comparisons remain. A new test adds **24,576 comparisons**
against the reference with competing ID, lifetime, lower/upper bound and root
failures.

Selection uses the current `6fc6db8164` diagnostics already published in packet
46: 89 and 78 named leaf observations in dimension canonicalization. The packet
keeps their configuration, analysis and publication receipt. No new sample is
run here; sampled thread-stack counts are not exclusive CPU percentages.

## Complete-build qualification

Both subjects use the standard ThinLTO compiler profile, pinned compiler-only
mimalloc, the same 61 program flags and two workers on the Apple M3. Control
`6ce7a53b36` is compared with candidate `3c09f35890`. No native-CPU or PGO build
overrides are involved, and earlier percentages are not added.

Eight complete builds run serially in ABBA/BAAB order, without overlapping
heavy build, test, sampling, hashing or archive work. The boundary includes
fresh source assembly through final native link completion. Runtime/entry
adapters and fixed native inputs are cached and hash-pinned; historical first-
use adapter construction of 11.882560208 seconds is excluded.

| Boundary | Control | Fused scan | Change |
| --- | ---: | ---: | ---: |
| Complete source to executable | 15.120963 s | 14.971381 s | -0.99% |
| Source-to-Object process within those builds | 14.988616 s | 14.838702 s | -1.00% |
| Final native link | 0.131400 s | 0.131710 s | +0.24% |

The complete-build blocks improve 0.76% and 1.22%. Object handoff increases
2.38%, about 0.0225 ms. Link inputs and implementation are unchanged; their
movement is observational.

A separate eight-run Object cohort uses the same configuration and order:

| Work | Control | Fused scan | Change |
| --- | ---: | ---: | ---: |
| Internal compiler phases | 14.993999 s | 14.801796 s | -1.28% |
| Complete source-to-Object process | 15.062133 s | 14.868820 s | -1.28% |
| Checked frontend | 1.611125 s | 1.422237 s | -11.72% |
| Instructions retired | 220.907616 billion | 220.389810 billion | -0.2344% |
| Source CFG construction | 2.218066 s | 2.265167 s | +2.12% |
| Maximum RSS | 4,488,896,512 B | 4,495,032,320 B | +0.1367% |

Both Object compiler blocks improve (1.91%, 0.65%), as do frontend (11.86%,
11.59%) and instructions (0.2362%, 0.2326%). Footprint increases 0.1376%,
artifact writing 0.44% and encoding 0.13%. Cleanup including compaction falls
0.89%, compaction 4.62%, preparation 1.00%, helper generation 0.43% and canonical
lowering 1.37%. Those other stages retain their implementations; no separate
backend optimization is claimed. All observations, counters and adverse
fields remain in the
[complete result](../../benchmarks/mech-deflate/native-profile/dimension-declaration-scan-apple-m3-20261115.json).

## Evidence and verification

All **191 core unit tests**, **110 contract tests across ten suites**, and **two
serialization vectors** pass. The `no_std` library check passes. These compile
the actual current core crate with locked dependencies. The unit suite includes
the new competing-error cases and existing full canonicalization/reference
vectors, unused invalid declarations, repeated roots, cycles and forward
references. Test commands/logs, relevant source/configuration and hashes of all
12 executed test binaries are retained.

All source, bytecode, Object and **complete metadata** bytes match in every
observation and all 14 scalar/numeric fixture pairs. Two preliminary executable
gates, default-serial output, an excluded real-worker diagnostic and all eight
primary complete builds reproduce the entire 2,060,440-byte executable:
SHA-256 `d6c65cc3fb18acc4be77dce329962105bf0f2b56dd2623a12f466256463eca61`.
The worker diagnostic records 344 submitted jobs, including 48 cleanup jobs.
All six qualification stages and actual helper paths are audited. No new
emitted-program runtime campaign is needed to compare these identical images.

Compiler check/build take 25.708/439.632 seconds, outside Deflate compilation
timing. The native CLI check passes with `cranelift_build`; default-feature CLI
behavior is not changed or newly qualified. An initial edit-generator guard
stopped an overly broad reference-function replacement before any source write;
the corrected explicit patch preserves the reference oracle byte-for-byte.
There are no failures in the frozen build, test or measurement qualification.

The forty-eighth
[repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
contains source snapshots, patch and whole-commit proof, tests and original
reference audit, all commands/logs/observations/outputs, full executable bytes,
fixed native inputs and offline replay. Compiler/test executables are excluded
and hash-pinned. Fresh repository extraction verifies every manifest/file/Git
blob, then pure replay reconstructs every mean, block, source/test identity and
byte gate and reproduces the compact result exactly without rerunning timing.
