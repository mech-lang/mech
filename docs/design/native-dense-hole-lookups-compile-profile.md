# Skip redundant sparse SSA probes inside the dense prefix

Returning absence directly for an empty dense-prefix slot reduces level-12
Deflate source-to-executable compilation **15.500449 to 15.421547 seconds
(0.51%, 0.079 seconds)**. Both complete-build blocks improve and every executable
byte matches. Source CFG construction falls 4.31% and retired instructions 0.47%.
This is a small retained work reduction; compilation below ten seconds remains open.

Qualified source `931b63c878622ea397aac3b0b5da9755bce8f3ff` contains exactly
the measured one-file patch from `55016698865eb95ec14a38ef35342b3b86bf4ba3`:
SHA-256 `8158330e494efff74b5bdd0e6b73515590563d7d103a4046025d00a6d993a9cd`.
The entire source commit, including the focused unit, is compared with that
frozen patch before the separate evidence publication.

## Identified work and change

Two fresh stack diagnostics use the separately qualified current native-CPU/PGO
compiler, SHA-256 beginning `5a1687`, on source d26cf2c65e. They contain 262 and
280 named SSA state-machine leaf samples. The observed address at function
offset 456 in both processes normalizes to `0x100133cf4`: the step that advances
an unsuccessful hash-table group probe, followed by its loop back edge. The
raw/demangled stacks, 742-symbol bridge, analysis and bounded disassembly remain.
These thread-stack counts identify work; they are not exclusive CPU percentages
or predicted savings. Both sampled compiles and their complete executable gates
match every byte and are excluded from primary means.

`VariableDefs` already stores a dense prefix and a sparse tail. Inserting a key
inside that prefix always updates the dense slot. Growing the prefix moves
**every** covered sparse key into the dense vector and removes it from the map.
Consequently an empty dense slot proves absence, including when the sparse tail
is nonempty. The old chained lookup still probes that sparse map. The new match
returns the empty slot immediately and consults the map only beyond the prefix.
A present `PackedOption::None` remains a present record.

Storage layout and thresholds, insertion/promotion and enumeration, predecessor
walks, sealing schedule, value creation/order, errors and side effects retain
their existing implementation. Grammar, fresh proofs, whole-inventory demand,
first refusal and finite admission/accounting retain their paths. No runtime
program code or program optimization selection changes.

## Measured complete builds

The actual source comparison uses the **standard** compiler profile, without
PGO or native-CPU flags: immutable B72 control versus dd7 candidate. Both include
the retained allocator, owner/inventory/reachability changes and use the same
61 program flags plus two workers on the Apple M3. The current PGO artifact is
kept separately. The new source has not received combined PGO qualification;
its result must not be added to the separate 14.027481-second result or any
previous parallel saving.

Eight complete builds run serially in ABBA/BAAB order without overlapping heavy
build, test, sampling, hashing or archival. The boundary includes fresh source
assembly, parse, checked frontend, native preparation/optimization/code generation,
Object and complete metadata writes, Object handoff and final link completion.
The fixed runtime/entry adapter and native inputs are cached and hash-pinned;
historical first-use adapter construction of 11.882560208 seconds is excluded.

| Boundary | Control | Dense-hole lookup | Change |
| --- | ---: | ---: | ---: |
| Complete source to executable | 15.500449 s | 15.421547 s | -0.51% |
| Source-to-Object process within those builds | 15.369784 s | 15.280378 s | -0.58% |
| Final native link | 0.129593 s | 0.140223 s | +8.20% |

The two complete-build blocks improve 0.84% and 0.18%. Final linking is adverse
by 8.20%; Object handoff falls 11.73%, about 0.126 ms. Those implementations and
inputs are unchanged. Every observation remains.

A separate eight-run source-to-Object cohort uses the same configuration/order:

| Work | Control | Dense-hole lookup | Change |
| --- | ---: | ---: | ---: |
| Internal compiler phases | 15.323233 s | 15.278015 s | -0.30% |
| Complete source-to-Object process | 15.390831 s | 15.346430 s | -0.29% |
| Source CFG construction | 2.359747 s | 2.258101 s | -4.31% |
| Cleanup including compaction | 2.976365 s | 3.009276 s | +1.11% |
| Numeric helper generation | 2.051417 s | 2.061674 s | +0.50% |
| Instructions retired | 221,689,815,187.75 | 220,653,133,247.50 | -0.4676% |
| Maximum compiler RSS | 4,489,883,648.00 | 4,489,756,672.00 | -0.0028% |
| Peak memory footprint | 4,467,183,294.00 | 4,467,060,420.00 | -0.0028% |

The Object cohort's internal compiler mean improves 0.30%, with **mixed blocks**:
the first increases 0.60%, the second falls 1.18%. Source CFG construction
improves in both blocks; cleanup including compaction has mixed blocks and an
adverse overall mean.
The complete-build cohort above supplies the favorable paired process result.
Numeric helper generation increases 0.50%; all phase values and adverse movement
are preserved in the [complete result](../../benchmarks/mech-deflate/native-profile/dense-hole-lookups-apple-m3-20261115.json).

## Correctness and retained evidence

All 29 production-module units pass: 14 reference and 15 candidate. Only the
test import is redirected to each copied module. The existing 50,000-operation
independent BTreeMap oracle covers mixed inserts, promotion, empty/present-None
records and enumeration. The new unit queries every dense hole and the bounds
around a nonempty, distant sparse tail.

A separate comparator passes 304 paired cases and verifies 608 functions. It
compares raw IR, query IDs, variable definitions and value counts, then compacted
IR, aliases/types/definitions, query answers, all side effects/enumeration and
actual AArch64 speed-optimized machine bytes. Every comparison is identical.
No prior early-sealing or known-origin prototype is enabled.

Every source, bytecode, Object and **complete metadata** byte matches in the main
observations and 14 numeric/scalar fixture pairs. The two preliminary complete
executable gates, eight primary complete builds and a separate default-serial
gate reproduce all 2,060,440 executable bytes, SHA-256
`d6c65cc3fb18acc4be77dce329962105bf0f2b56dd2623a12f466256463eca61`.
The serial gate is unpaired and excluded from primary means. No new emitted
program runtime campaign is claimed.

Compiler check/build take 7.211/253.772 seconds; the real native CLI check with
`--features cranelift_build` takes 10.507 seconds. Unit build/execution take
3.448/0.832 seconds; IR comparator build/execution take 20.263/1.353 seconds.
All pass outside program compilation timings. No build, test or measurement
setup failure occurs in this trial.

The forty-first [repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
contains commands/logs, exact source/patch and whole-commit proof, tests,
observations, all output/executable bytes, fixed native inputs and the separate
PGO selection diagnostics. Compiler/test binaries are excluded and hash-pinned.
Fresh repository extraction verifies every manifest/file/Git blob. Pure replay
checks artifacts, address normalization, source/patch identity and test receipts,
reconstructs the timing summaries and stack analysis, and reproduces the compact
result byte-for-byte without rerunning compilation or timing.
