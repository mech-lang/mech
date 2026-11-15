# SSA descending-chain lookup trial

The descending-chain fast walk is **rejected**. Complete source-to-executable
builds increase **15.126993 to 15.195366 seconds (+0.45%)**, with both balanced
blocks slower. A separate Object cohort increases compiler time 0.78%, source
CFG construction 2.55% and instructions 0.0164%. Correctness checks pass, but
the candidate establishes no useful work or time saving. The original source
and standard compiler are restored. The separately qualified Deflate-trained
native-CPU/PGO configuration remains 13.596783 seconds; compilation below ten
seconds remains open.

## Selection and frozen implementation

Two fresh stack diagnostics run the current `6fc6db8164` PGO compiler on retained
source `0c949505cc`. Their callgraph-only analysis records 261 and 252 leaf
observations in `SSABuilder::run_state_machine`. The packet retains the raw
samples, address/disassembly, symbol bridge and analyzer. These are sampled
thread-stack observations, including waiting stacks, rather than exclusive CPU
percentages. Sampling runs and their complete executable gates are excluded
from primary timings.

The trial starts at evidence commit
`db1498fe94873117fd5b9e5e8b4a7864efe0aa2d`. Its entire one-file patch is SHA-256
`b08f0fccdfd3a222d3ce6036980372dc868da34d279878b40f759ea1b90e1882`.
It changes `SSABuilder::find_var` in `vendor/cranelift-frontend/src/ssa.rs` and
adds two focused tests. A read-only walk follows strictly decreasing
single-predecessor block IDs. Such a walk cannot cycle, so it can return a known
definition or reach the original parameter-creation path without initializing
or writing cycle marks. On any nondecreasing edge, it restarts the exact
original traversal at the original block. No mutation precedes that restart,
preserving the original repeated-block sentinel and value-allocation order.

A token audit independently checks the original fallback loop, parameter-
creation/sealed-lookup tail, function signature and all remaining production
code. SSA storage/layout, sealing, IDs, permission paths and grammar retain
their existing implementation. The trial may repeat lookups before falling
back. The actual frequency of fast returns, avoided mark writes and fallback
rescans on Deflate is unmeasured; the slowdown is not assigned to one cause.

## Matched measurements

Control `6ce7a53b36` and candidate `2fc734830c` use the standard ThinLTO compiler
profile, pinned compiler-only mimalloc, the same 61 program flags and two
workers on the Apple M3. Native-CPU and PGO settings are not added. Eight
complete builds run serially in ABBA/BAAB order without overlapping heavy
build, test, sampling, hashing or archive work. The boundary includes fresh
source assembly through final native link completion. Runtime/entry adapters
and fixed native inputs are cached and hash-pinned; historical first-use
adapter construction of 11.882560208 seconds is excluded.

| Boundary | Control | Candidate | Change |
| --- | ---: | ---: | ---: |
| Complete source to executable | 15.126993 s | 15.195366 s | +0.45% |
| Source-to-Object process within those builds | 14.997104 s | 15.063238 s | +0.44% |
| Final native link | 0.128787 s | 0.131183 s | +1.86% |

The two complete-build blocks are 15.194901 to 15.198605 seconds (+0.024%) and
15.059085 to 15.192126 seconds (+0.883%). Object handoff falls 14.29%, about
0.157 ms. Fixed native link inputs and implementation are unchanged; link
movement is not credited to the SSA change.

A separate eight-run Object cohort uses the same configuration and order:

| Boundary | Control | Candidate | Change |
| --- | ---: | ---: | ---: |
| Internal compiler phases | 14.891352 s | 15.007986 s | +0.78% |
| Complete source-to-Object process | 14.957699 s | 15.074433 s | +0.78% |
| Source CFG construction | 2.217523 s | 2.274111 s | +2.55% |
| Cleanup including compaction | 2.974231 s | 2.982753 s | +0.29% |
| Cleanup excluding compaction | 2.826168 s | 2.839167 s | +0.46% |
| Value compaction | 0.148063 s | 0.143586 s | -3.02% |

Both Object compiler-time blocks are slower (+1.37%, +0.19%), as are both
source-CFG blocks (+2.57%, +2.54%) and instruction blocks (+0.020%, +0.013%).
Instructions increase 220.639928 to 220.676129 billion. Helper generation
increases 0.64%, catalog work 1.79%, frontend 0.037%, native phases 1.06%, RSS
0.0463% and footprint 0.0465%. Parsing falls 0.26%, preparation 0.49%, canonical
lowering 0.22%, encoding 0.59% and artifact writing 0.28%. Every observation,
stage and adverse counter remains in the
[complete result](../../benchmarks/mech-deflate/native-profile/ssa-descending-chain-apple-m3-20261115.json).
The cohorts remain separate. No new emitted-program runtime campaign is run.

## Correctness and restoration

All 32 focused units pass: 15 reference tests and 17 candidate tests. The two
new tests cover a 64-block decreasing chain with no generation/mark writes and
a nonmonotone chain that restarts after a decreasing prefix. Differential
tests pass 328 paired cases and 656 verified functions. They compare complete
raw and compacted IR, queries, definitions, value counts, aliases, types,
effects, enumerations and AArch64 machine bytes. The 24 additional cases cover
offset block IDs, four/16/65-block chains, nonmonotone chains and actual sealed
unreachable cycles. All corresponding bytes and state match.

All source, bytecode, Object and **complete metadata** bytes match in the main
observations and all 14 fixture pairs. Two preliminary executable gates, the
candidate default-serial gate, one excluded two-worker diagnostic and all eight
primary complete builds reproduce every byte of the 2,060,440-byte executable,
SHA-256 `d6c65cc3fb18acc4be77dce329962105bf0f2b56dd2623a12f466256463eca61`.
The two separate selection profiles also reproduce that executable. The worker
diagnostic records 344 submitted jobs, including 48 cleanup jobs. The actual
qualification driver's six-stage order and helper paths are audited before
execution and again during replay.

Compiler check/build take 7.169/254.470 seconds; the native CLI check with
`--features cranelift_build` takes 9.041 seconds. Focused build/execution take
3.271/0.680 seconds. All pass outside program timings. No failed build, test or
measurement setup occurs in this trial.

The production file is restored byte-for-byte. The canonical standard compiler
is restored to immutable `6ce7a53b36`; selected PGO `6fc6db8164` is untouched.
Temporary Cargo build-profile rlibs remain from the rejected candidate until
the next genuine Cargo rebuild. The restored source has a fresh mtime, forcing
affected crates to rebuild before treating cached libraries as a reference.
Retained production source remains `0c949505cc`.

The forty-sixth
[repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
retains the rejected source/patch, exact tests and source audit, fresh selection
profiles, every command/log/observation/output, all complete executable bytes,
native inputs and restoration receipt. Compiler/test binaries are excluded
and hash-pinned. Fresh repository extraction verifies all 833 files and Git
blobs; pure replay reconstructs tests, audits, summaries, blocks and restoration
and reproduces the compact result byte-for-byte without rerunning timing.
