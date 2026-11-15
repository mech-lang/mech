# Stable batch phi-parameter removal trial

Stable batch removal is **rejected**. Eight complete Deflate builds take
**15.414733 seconds with the original code and 15.444185 seconds with batch
removal (+0.19%)**, with mixed balanced blocks. The separate Object cohort
reduces internal compiler time 0.66%, cleanup including compaction 2.58%, and
instructions 0.82%, but does not establish a useful complete-build saving.
The original source and standard compiler are restored. The ten-second goal
remains open.

## Frozen change and evidence

Base source is `5d0b9cec8b8dd1bbc367db97a51268199accf1a1`. The entire two-file
trial patch is SHA-256
`b0ae8952b0d92628b581892aaf50701b0eb67ab2acf8a1d7a306b94a0a4a3eac`.
It adds `DataFlowGraph::remove_block_params` and uses it only for constant-phi
formal removal. The existing solver, selected formals and replacements,
reverse alias insertion and incoming argument filtering remain unchanged.
No local-phi, pass-order, CFG-validity or grammar change is included.

The old loop removes each selected formal separately, shifting its surviving
suffix and updating all later ordinals repeatedly. The candidate scans from
the first selected ordinal, retains survivors in order, updates each ordinal
once, and truncates once. Both complete functions and every alias/ordinal
remain identical. This tests the batch-removal proposal independently of the
previous rejected constant-phi worklist algorithms.

The current-PGO stack diagnostics retained in
[packet 41](native-dense-hole-lookups-compile-profile.md) still show the
constant-phi pass. Both reference modules are byte-identical in that diagnostic
source d26cf2c65e and this trial's baseline; the comparison is retained. Those
thread-stack samples do not isolate parameter-editing cost or predict savings.
No new sampling is mixed into primary observations.

## Measurements

Both subjects use the standard ThinLTO compiler profile, pinned compiler-only
mimalloc, the same 61 program flags and two machine-code workers on the Apple M3.
Control compiler SHA-256 begins `dd7d`; candidate begins `7a5d`.
No PGO or native-CPU flags are added. The selected 5a1687 PGO artifact remains
unchanged on its separately qualified d26 source; no new combined saving is
claimed.

Eight primary complete builds run one at a time in ABBA/BAAB order. No heavy
build, test, sampling, hashing or packaging overlaps them. Each includes fresh
source assembly through final native link completion. The fixed runtime/entry
adapter and native inputs are cached and hash-pinned. Historical first-use
adapter construction of 11.882560208 seconds is excluded.

| Boundary | Control | Batch removal | Change |
| --- | ---: | ---: | ---: |
| Complete source to executable | 15.414733 s | 15.444185 s | +0.19% |
| Source-to-Object process within those builds | 15.280258 s | 15.318266 s | +0.25% |
| Final native link | 0.133378 s | 0.124757 s | -6.46% |

The first complete-build block is 0.80% slower; the second is 0.41% faster.
Object handoff increases 6.03%, approximately 0.066 ms. All observations remain.

The separate eight-run Object cohort uses the same configuration and order:

| Boundary | Control | Batch removal | Change |
| --- | ---: | ---: | ---: |
| Internal compiler phases | 15.256608 s | 15.155633 s | -0.66% |
| Complete source-to-Object process | 15.324604 s | 15.223252 s | -0.66% |
| Cleanup including compaction | 2.995330 s | 2.918005 s | -2.58% |
| Cleanup excluding compaction | 2.852857 s | 2.771308 s | -2.86% |
| Value compaction | 0.142473 s | 0.146697 s | +2.97% |

Both internal-compiler blocks improve, by 0.63% and 0.69%. Retired instructions
fall from 220.643142 to 218.844004 billion (-0.82%). Maximum RSS falls 0.0110%
and peak footprint 0.0102%. Adverse stage movement includes parsing +0.28%,
frontend +0.16%, preparation +0.13%, canonical lowering +0.18% and compaction
+2.97%. Every phase and counter remains in the
[complete result](../../benchmarks/mech-deflate/native-profile/batch-phi-params-apple-m3-20261115.json).
The two cohorts remain separate; the favorable work counters do not override
the complete-build result.

## Correctness, restoration and replay

Three focused tests pass against the actual candidate DFG library. The two
new DFG test bodies are copied exactly from the frozen candidate module. They
compare batch removal with existing repeated removals for all 2,047 subsets
on zero-to-ten parameters and 20 cases around list-pool and ordinal boundaries,
including 65,535 parameters. Types, attached/detached states, ordinals, untouched
blocks and subsequent append identities match.

The third test compiles both complete original/candidate constant-phi modules.
It passes 128 paired loop/merge/unreachable cases with widths through 512 and
alias chains. Complete post-pass IR, every value's immediate/resolved alias,
type and definition, ordered block parameters and AArch64 speed-optimized machine
bytes are identical. The original verifier passes before and after both passes.

All four outputs and **complete metadata** match in the main observations and
14 paired scalar/numeric fixtures. Both preliminary executable gates and all
eight primary complete builds reproduce every 2,060,440 executable byte:
SHA-256 `d6c65cc3fb18acc4be77dce329962105bf0f2b56dd2623a12f466256463eca61`.
No additional candidate default-serial gate or emitted-program runtime campaign
is run after rejection. Existing direct scalar emission, checked errors,
current-visit/ownership proofs and finite admission paths remain intact.

Compiler check/build take 13.610/263.069 seconds; the real native CLI check
with `--features cranelift_build` takes 15.838 seconds. Focused build/execution
take 20.124/0.677 seconds. All pass outside program timings. This trial has no
failed build, test or measurement setup.

Restoration checks both production-file hashes against the frozen reference
and leaves a clean working tree. The immutable dd7 standard compiler is restored;
the selected PGO tool is untouched. The temporary Cargo dependency cache still
contains candidate rlibs until the next genuine rebuild; restored source mtimes
force recompilation before that cache is used as a new baseline.

The forty-second [repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
keeps the rejected source/patch, tests, commands/logs, every observation/output,
complete executables, fixed native inputs, previous diagnostic selection evidence
and restoration receipt. Compiler/test binaries are excluded and hash-pinned.
Fresh extraction verifies every manifest/file/Git blob, then pure replay checks
source/test/byte identities, reconstructs all numerical summaries and stack
analysis, and matches the compact result byte-for-byte without rerunning timing.
