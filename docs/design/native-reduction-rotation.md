# Review of nested reduction rotation

The compact compiler puts the current equality test on the seed and recurring
edges of an existing nested scalar reduction. The ordinary entry still tests
the initial cursor, including zero trips. Each recurring edge tests the next
cursor once after the original selected work. A successful completion sends
the final current accumulator lanes to the original exit as typed block
arguments.

The transformation requires exactly one initial jump and one recurring jump,
an invariant stop available on both edges, and an equality test reached through
a corridor of total scalar operations. It refuses other entries, observations
and effects before the test, unavailable operands, or live-outs that cannot be
forwarded as current parameters. The original selected record reads, arithmetic,
capacity proof, update order, reject origins and output handling are preserved.
The compact diagnostic keeps its existing scope. The ordinary managed entry
retains its resource and recovery contract.

| Cohort | Cases | Validated timed calls | Elapsed / control | Faster than Roc | Faster than libdeflate | Faster than both |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Screen | 16 | 154,752 | 0.973748× | 4 → 9 | 13 → 14 | 1 → 7 |
| Full | 81 | 245,664 | 0.993688× | 64 → 72 | 34 → 35 | 17 → 26 |
| Full repeat | 81 | 245,664 | 0.996321× | 64 → 73 | 34 → 34 | 17 → 27 |
| Adverse confirmation | 10 | 105,120 | 0.994427× | 4 → 4 | 6 → 6 | 0 → 0 |

Both existing 1.05 timing gates pass on each case in all four cohorts. Each
actor runs in 24 balanced blocks per case at level 12 with the same runtime
input, allocation boundary, output ownership and validation placement. The
unaltered benchmark source and strict revision-14 bytecode give exact raw bytes,
independent inflation, matching retained capacity and input/output alias
checks. Paired ratios come from balanced block timings. Single-call medians
are retained separately. Every cohort is reported individually.

| Input | Trial / control, full and repeat | Trial / Roc, full and repeat | Trial / libdeflate, repeat |
| --- | ---: | ---: | ---: |
| Eight literals | 0.997611× / 1.002615× | 0.165117× / 0.162580× | 2.056499× |
| Natural 128 KiB | 0.973721× / 0.973155× | 0.995368× / 0.991366× | 0.816704× |
| Natural 256 KiB | 0.972863× / 0.970615× | 0.994481× / 0.993705× | 0.793710× |
| Natural 1 MiB | 0.975548× / 0.972993× | 1.007860× / 1.006525× | 0.797091× |
| Held-out slides | 0.978501× / 0.978658× | 1.014728× / 1.017101× | 0.889273× |
| Random 1 MiB | 1.001677× / 1.029061× | 0.990158× / 1.014238× | 1.414380× |
| Periodic 4 KiB | 0.987891× / 1.031234× | 0.334218× / 0.349895× | 1.451393× |

Random 1 MiB produces the only lost Roc crossing in the repeat. Its two
order subsets measure 1.057837× and 1.001068×. The fresh confirmation gives
0.998957× paired control and 0.988357× Roc. Periodic 4 KiB measures 1.009029×
paired control in confirmation. Periodic 8 KiB is 1.020483× in confirmation
with 1.000508× and 1.040858× in the two order subsets. Both timing gates
pass. Natural 32,767 and 32,768 bytes beat Roc narrowly in the first run and
are just above in the repeat. The single-input runs and all regressions
remain in the receipts. In the repeat, the normal worker wins against at
least one peer on 80/81 cases.

The disassembly verifies that 4 of 357 recorded bodies change. Each grows
from 1,168 to 1,184 bytes with a 224-byte frame. The large root and the
matcher are byte-identical. All 1,270 named addresses remain fixed. The
initial check can fall through to the next record. The recurring ARM64
comparison now uses the advanced cursor directly and branches straight to
the existing checked-read block. The cursor bound, index check and older
zero-default path are all retained.

Two optimized JIT controls run 288 compiled calls, including wrapped
induction, zero trips, selected visits, exact stores and typed completion.
They also establish that a changing stop, a store before the test and a
computed exit value decline the rewrite. Original source and decoded
revision-14 input produce identical Objects and each pass 162 linked
checks using their emitted descriptor. Direct native controls pass 68,096
positions and 288 failure/retry pairs at the same source origin 15525.
Publication and cache data remain unchanged on rejection.

The focused build uses nightly 2026-03-03, -D warnings, optimization level 3
and the pinned dependency closure with aot, canonical-native,
canonical-native-program, canonical-native-program-runtime,
canonical-native-runtime and jit. The verified rebuild selects the rotation
through the existing MECH_CARRY_REDUCTION_BOUND configuration. The design
checks cover compiler planning, interactive architecture, host and source
catalogs, shipping resident routing, formatting and whitespace. The
repository-wide unsafe audit and warning-policy scanner report inherited
diagnostics in benchmarks/related-post-gen/check-measurement.rs and
benchmarks/mech-deflate/native-profile/old_value_reads_tests.rs respectively.
Their paths are unchanged by the patch. The warnings-as-errors compile
passes. Hosted CI and production-runtime benchmarks are outside these
measurement receipts. The first preflight build found one unused import
and a borrow-lifetime error in the pass. Both are corrected. The initial
focused invocation selected zero tests owing to its name filter. The
corrected invocation executes both tests.

The timings use parent 75e2e8bbec1d362d2f72746e858901e3375ff4bb plus the
rotation. The review patch is based on ba7924b4ee96eb5fa8b35ddc1c399dd1cef930fa.
The intervening change reuses traversal visitation storage. The rebuilt
candidate produces the same Object on both forms. The benchmark source
SHA-256 is 331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9.
The bytecode SHA-256 is 51de51658c1d8d0be66567400e382ef5c0f5789baf317c53a8cd56d1df2dd0ca.
The emitted Object is 8f33516fb93e1614a02f64d6dec1bd92f68abc6c4824b468c71840da494141a7.
The timed worker is 29cae80726fda36f7bee459352be9e13fce2b79d8cc7966b95b2d6495d4870a7.

Use the command receipts alongside complete_timing.py, screening.py,
generated-rotation/machine_audit.py, generated-rotation-rebased/run.py and
generated-rotation-verified/qualify.py to reconstruct the comparison.

[Paired observations, full tables, validation receipts and native code](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/native-compression-loop-rotation-8aa9e36c107c.tar.gz).
All 356 file hashes verify. Archive SHA-256
`8aa9e36c107ccd280fcfb0198ac1cff0730f5b0ddb1807220216c207c34babc0`.
