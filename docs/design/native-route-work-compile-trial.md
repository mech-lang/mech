# Empty repair traversal trial and current worker profiles

The repair traversal shortcut is rejected and production source is restored.
Its eight source-to-Object observations increase compiler phases **0.59%** and
main cleanup **0.45%**, with both balanced blocks slower. The tiny instruction
reduction does not establish a useful saving. Complete compilation below ten
seconds remains open.

The trial starts at `aaef2a932bf81c7638c522dc777a2b78f0abcb5d` and compares
against the qualified compiler from
`1090de011646c92e30c2d4b648ed0792c50f300d`. Both subjects use the same 61 program
flags and `MECH_COMPACT_COMPILE_THREADS=2`, without PGO. The original constant-phi
solver is present in both subjects.

## Trial and results

The candidate returns from threaded-edge operand rewriting when every transport
replacement map is empty. Ordinary and exceptional edge checks still run before
route discovery. No route proof, ownership check or language rule is changed.
The original helper is restored after qualification; no shortcut is retained.

Eight primary compiles run serially in ABBA / BAAB order, four per subject, with
precise process-completion timing and macOS counters. Builds, tests, sampling,
hashing and archival do not overlap primary timing. Preliminary screens, focused
tests, compiler-tool construction and cached executable gates are separate.

| Boundary or counter | Control mean | Candidate mean | Change |
| --- | ---: | ---: | ---: |
| Complete source-to-Object process | 16.959669 s | 17.074154 s | +0.68% |
| Internal compiler phases | 16.907414 s | 17.006988 s | +0.59% |
| Main cleanup including compaction | 3.335976 s | 3.351121 s | +0.45% |
| Retired instructions | 255.708222 G | 255.598803 G | -0.04% |
| Maximum RSS | 5,967,429,632 B | 6,025,953,280 B | +0.98% |

Compiler phases increase 1.08% and 0.10% in the two blocks; cleanup increases
0.36% and 0.54%. Instructions fall 0.01% and 0.08%. Frontend (+0.95%), parsing
(+1.65%), source CFG construction (+0.90%) and preparation (+0.35%) are also
adverse. Every stage and observation remains in the
[compact result](../../benchmarks/mech-deflate/native-profile/route-work-trial-apple-m3-20261115.json).
The source-to-Object boundary is not a complete executable build. No balanced
complete-executable cohort is run for this already rejected candidate.

## Equivalence and tests

One focused differential test passes 128 complete IR, alias-definition and
machine-byte comparisons. It compiles the exact original and candidate helper
bodies, changing only their names in the harness. Cases cover shared pooled
branch-argument storage, loops, absent/empty replacement maps and scoped integer
and floating-point replacements. The test uses pinned Cranelift dependencies
from the earlier control build, independently of the default build target.

The first harness build fails warnings-as-errors on unused imports before any
test executes. Removing those harness imports changes no production source;
the failure, correction and successful command remain. The compiler check takes
13.929 seconds and rebuild takes 263.644 seconds, excluded from program timings.

Both screens, fourteen paired fixtures and every primary observation reproduce
source, bytecode, Object and complete metadata. Both cached native link gates
reproduce the complete 1,878,568-byte executable with SHA-256
`557d0f5ef2b979e01851ae23b5b3944453a0bd9e772d68161be198916ca06c2c`.
The fixed adapter and native inputs are pinned and cached; adapter first-use
construction is excluded. No fresh emitted-program runtime campaign is claimed.

## Current worker samples

Two separate stack diagnostics run the qualified worker compiler with the same
61 flags and two workers. Each successful external sample preserves all four
program outputs. Sampled process durations and counters are excluded from
primary means. Launcher/worker waiting samples are not CPU-time percentages,
and no exclusive phase-time conclusion is drawn from inclusive stacks.

The stacks show range-map copying in threaded-edge route discovery. Source
inspection confirms that discovery currently clones the full source range map
for each edge, then changes at most one source-edge condition before querying
it. A borrowed base plus one scoped override is a subsequent investigation;
it is not implemented or qualified by this packet.

Helper preparation/cleanup also remains on the coordinator. It can introduce
nested admissions, functions and data before later choices, so moving it to
workers requires preserving those ordered dependencies. The existing
[worker result](native-parallel-phases-compile-profile.md) remains the qualified
parallel saving; these samples establish no additional speedup.

## Evidence

The twenty-eighth
[repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
contains the exact trial source/patch, tests, initial harness failure, all raw
build/timing logs, output and executable bytes, frozen native inputs, both stack
diagnostics and pure replay. Compiler/test executables and intermediates are
excluded and SHA-pinned. The repository archive is freshly extracted, every
manifest/file/Git blob is verified, and replay reproduces the compact result
byte-for-byte. Production source and the qualified profiling executable are
restored; no compiler change is retained from this trial.
