# Parallel cleanup and machine generation for eligible numeric phases

The expanded worker path is retained. On the standard compiler, eight matched
complete Deflate builds improve **15.398425 to 15.130134 seconds (1.74%)**,
with both balanced blocks faster and every executable byte identical. Numeric
helper generation improves **12.68%**.

A separate same-binary comparison isolates enabling the current parallel path:
**15.553768 seconds serial versus 15.218166 seconds with two workers (2.16%,
0.336 seconds saved)**, again with both blocks faster. Default
`MECH_COMPACT_COMPILE_THREADS=0` remains serial; `=2` enables the measured mode.
These two experiments remain separate. At publication, the separately qualified
native-CPU/PGO artifact on d26 source measured 14.027481 seconds. The later
[Deflate-trained PGO qualification](native-parallel-pgo-compile-profile.md) on this
exact expanded-worker source measures 13.596783 seconds, with a separate
same-PGO-binary serial/two-worker comparison. Its seven small heldouts do not
establish broad generalization. Compilation below ten seconds remains open.

Qualified source `0c949505cc708c233e27c094aa67e105068d6823` contains exactly
the measured three-file diff from `7c4d62ee9a45992246e2e06e843bdf235351c222`:
SHA-256 `2a8628d3fb44b3b977affa65d7241dbf236154daa09125ffe2cd748c9a06b909`.
The source commit precedes evidence publication; the entire commit diff is
compared with the frozen patch, including tests and the common report factory.

## Implemented architecture

The existing bounded worker pool now accepts typed owned work and returns its
result through a per-job ticket. Ordinary machine-only submissions retain their
previous compilation and ModuleError/backend conversion. Eligible general
numeric phases submit their existing cleanup followed by machine generation,
overlapping the coordinator's preparation of the later vector variant.

Each broader job owns its Context and the **same** local CompilationControls
already used during emission, plus an Arc of that function's immutable emission
report. Local selection budgets retain their state. PrivatePhase's raw proof
identity stays on the coordinator. Pure cleanup previously used only
`private.is_some()`; the job receives that exact presence boolean. The private
contract and fresh proof checks in emission and postprocessing retain their
original implementation. Rust's thread-safety checks pass with no unsafe Send
implementation and no sharing of PreparedNativeProgram or raw proof pointers.

The coordinator still emits and verifies IR, derives entry bounds from the same
report/proof, allocates symbols, handles all Module effects and completes
metadata. Cases whose remaining general preparation could create a grouped-
histogram helper retain serial preparation. Vector/composed cases and diagnostic
file modes also retain their existing preparation paths. This is a conservative
scheduling boundary; optimization eligibility and output choices are unchanged.

The existing recorded vector-Module ledger stays in place. The general result
is awaited and published before replaying **any** later declaration/data/function
event or surfacing a later vector failure. Error conversion and first-refusal
ordering are retained. Parsing, checked frontend, native preparation, helper IR
construction and main construction/cleanup remain on the coordinator.

The architecture audit independently compares source tokens: phase IR emission
is unchanged; the pure cleanup body changes only the private-presence parameter;
the complete finish-report closure is identical; and the coordinator publication/
replay/error sequence is unchanged. The supporting proof/control/report source
is retained. No grammar, runtime allocator, scalar entry, fresh shape/factory/
current-visit check, whole-inventory demand or finite admission path is changed.

## Full-build qualification

Both subjects use the standard ThinLTO compiler profile, compiler-only pinned
mimalloc, the same 61 program flags and two workers on the Apple M3. Control
compiler SHA-256 begins `dd7d`; candidate begins `6ce7`. PGO/native-CPU flags are
absent. This compares the previous machine-worker compiler with the expanded
cleanup-worker compiler.

Eight complete builds run one at a time in ABBA/BAAB order, without overlapping
heavy build, test, sampling, hashing or packaging. Each includes fresh source
assembly, parse/frontend/preparation/optimization/code generation, Object/full-
metadata writes, handoff and final native link process completion. The fixed
runtime/entry adapter and all native inputs are cached and hash-pinned;
historical first-use adapter construction of 11.882560208 seconds is excluded.

| Boundary | Control | Expanded workers | Change |
| --- | ---: | ---: | ---: |
| Complete source to executable | 15.398425 s | 15.130134 s | -1.74% |
| Source-to-Object process within those builds | 15.264104 s | 14.998706 s | -1.74% |
| Final native link | 0.133345 s | 0.130486 s | -2.14% |

The complete-build blocks improve 1.24% and 2.23%. Handoff improves 3.39%, about
0.033 ms. Fixed linker inputs and implementation are unchanged; link movement
is observational. No earlier source/parallel/PGO percentages are added.

The separate eight-run Object cohort uses the same configuration and order:

| Boundary | Control | Expanded workers | Change |
| --- | ---: | ---: | ---: |
| Internal compiler phases | 15.169613 s | 14.926082 s | -1.61% |
| Complete source-to-Object process | 15.237227 s | 14.992365 s | -1.61% |
| Numeric helper generation | 2.042652 s | 1.783652 s | -12.68% |
| Main cleanup including compaction | 2.967462 s | 2.976254 s | +0.30% |

Both Object compiler blocks improve, by 1.91% and 1.30%. Instructions increase
0.0352% (220.628929 to 220.706639 billion): concurrency reduces wall time while
preserving the work, with small extra scheduling/report overhead. Maximum RSS
falls 0.0053% and peak footprint 0.0069%. Adverse stage movement includes parsing
+0.27%, catalog +4.16%, frontend +0.14%, preparation +0.26%, source CFG +0.58%,
cleanup including compaction +0.30% and compaction +3.97%. Every value remains in
the [complete result](../../benchmarks/mech-deflate/native-profile/parallel-phase-cleanup-apple-m3-20261115.json).

## Isolated current serial/parallel comparison

The following separate ABBA/BAAB cohort changes **only worker count 0/2**, using
one immutable candidate executable, the same 61 flags, source/runtime and fixed
native inputs. It isolates enabled parallel execution from the binary refactor.
No heavy work overlaps its eight complete builds.

| Boundary | Serial | Two workers | Change |
| --- | ---: | ---: | ---: |
| Complete source to executable | 15.553768 s | 15.218166 s | -2.16% |
| Source-to-Object process within those builds | 15.417083 s | 15.081477 s | -2.18% |
| Final native link | 0.135717 s | 0.135735 s | +0.01% |

Its blocks improve 1.30% and 3.01%. Native linking increases 0.013%, effectively
flat; handoff falls 1.49%, about 0.014 ms. These observations remain separate
from the source comparison, Object cohort and historical parallel results.

## Identity, tests and real job use

All four focused worker suites pass. Existing tests provide 24 ordered Object
comparisons over one/two/four workers, both submission orders, libcall symbols,
anonymous IDs and interleaved data/functions. The new typed cleanup test adds
12 coordinator/worker IR and Object comparisons, including phi removal before
machine generation. Earlier publication errors still precede invalid later data;
typed cleanup errors are returned through their corresponding ordered tickets.
Tests use the actual candidate GPU NativeCompileError and backend conversion.

Before primary timing, both preliminary complete-executable gates, a separate
default-serial byte gate and a real worker diagnostic pass. The diagnostic
records **344 submitted function jobs, including 48 cleanup-and-machine jobs**
with two workers, and reproduces every artifact/full executable byte. It is
excluded from primary means. Thus the broader path runs on the measured
compressor. The other jobs retain machine-only worker compilation.

All main observations, 14 scalar/numeric fixture pairs, both modes and all
16 primary complete builds reproduce the entire source, bytecode, Object and
**complete metadata**. All 20 retained executable gates/builds reproduce every
2,060,440 executable byte, SHA-256
`d6c65cc3fb18acc4be77dce329962105bf0f2b56dd2623a12f466256463eca61`.
No new emitted-program runtime timing campaign substitutes for that identity.

Compiler check/build take 6.660/251.020 seconds; the real native CLI check with
`--features cranelift_build` takes 8.714 seconds. Focused build/execution take
22.940/1.282 seconds. All pass outside Mech compilation timings. No failed build,
test or measurement setup occurs in this trial.

The forty-fourth [repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
keeps commands/logs, source/patch and entire-commit proof, tests/supporting source,
architecture audit, all observations/outputs/executables, fixed native inputs
and both scheduling-mode results. Compiler/test binaries are excluded and
hash-pinned. Fresh extraction verifies every manifest/file/Git blob. Pure replay
checks source-token and test identities, real-job counts, every output/executable,
and reconstructs every numerical summary byte-for-byte without rerunning timing.
