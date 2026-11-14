# R198: invariant addresses in captured native reads

**Retain this bounded compiler change for the repeated32 and periodic33 gains, with the mixed results below explicit.** The fastest retained compressor source remains unchanged; the state-machine source stays experimental. Parity is incomplete.

Base: `681cf6b5e35cd905c17ef651195d3810fbf1f8c3`, reviewed in comment6714324714 on existing draft#973. Native IR/work/ABI/MBC: **214/107/39/14**. Nine compiler/test source files are hash-pinned; the native image is built from the final formatted optimized library. No new PR.

## Complete compression timings

All 20 original1.05 gates pass, with 9 lower/11 higher elapsed medians. Geometric elapsed changes **-0.183679%**. Every paired block and contrary result is retained. There are 3,968 primary observations and 4,048 total complete calls. Before/after share the same fresh reference denominator for each input; independent rounds are not compounded.

| Input | Before / Roc | Current / Roc | Elapsed change |
|---|---:|---:|---:|
| reymont-prefix-32769 | 3.331486× | 3.325313× | -0.185283% |
| random-32769 | 4.724028× | 4.769134× | +0.954816% |
| 16384-distribution-change | 6.205120× | 6.203092× | -0.032690% |
| repeated-32769 | 3.197971× | 2.923528× | -8.581792% |
| 33027-periodic-window | 3.743487× | 3.579081× | -4.391806% |
| eight-literals | 4.805842× | 4.871062× | +1.357097% |
| reymont-prefix-16384 | 4.030242× | 4.025633× | -0.114363% |
| empty | unresolved (zero reference median) | unresolved (zero reference median) | +0.564779% |
| one-stored | unresolved (zero reference median) | unresolved (zero reference median) | +0.382329% |
| seven-stored-prefix | unresolved (zero reference median) | unresolved (zero reference median) | +0.134367% |
| eight-overlap | 4.314458× | 4.420587× | +2.459844% |
| 320-overlap | 4.090152× | 4.086020× | -0.101029% |
| eight-literals-prefix | 4.909725× | 4.908817× | -0.018486% |
| 128-biased-dynamic | 3.784118× | 3.833186× | +1.296681% |
| 32-high-entropy-stored | 4.172223× | 4.162353× | -0.236569% |
| random-64 | 4.185023× | 4.219843× | +0.832013% |
| random-256 | 4.256279× | 4.272332× | +0.377162% |
| random-1024 | 3.815371× | 3.814561× | -0.021240% |
| repeated-1024 | 3.987360× | 4.029980× | +1.068888% |
| reymont-prefix-1024 | 4.487842× | 4.538978× | +1.139442% |

Eight literals rise1.357097%, eight overlap2.459844% and random32 rises0.954816%. These costs remain accepted tradeoffs within the unchanged gates; the cause is not dismissed as noise. The three stored-prefix cases with zero reference medians retain their actual Mech comparisons and unresolved reference multiples.

The source hash remains `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; bytecode remains `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. The full level 12 computation, real 262,144 U16 tree / 1,500,513 U32 cache / 305,257 U64 path capacity, depth 300, nice length 258, tables, near-end limits and allocation sites remain. Output capacity, clear/reuse, aliases and the pinned mimalloc2.1.7 remain. Only evaluate_into is timed; image restoration, seed/clear/prefix/rebind, validation/drop and output-slot handling are outside that boundary. Image restoration is retained separately in primary/startup.json.

## Emitted mechanism

The existing captured-window proof already binds each packed read to the current stable buffer/base/induction and proves all recursive next-state alternatives. At the canonical child initializer, private machine SSA now stages buffer pointer plus captured base. A complete qualified packed read adds its current induction to that staged address. Every activation refreshes the staged value. No new range, trip, ownership, funding, read permission or source operation is added.

The old current shape/readable-window/funding gates, source arithmetic, actual visit debits, snapshots, bounds failures, fallback, publication, cleanup and retry remain. Refused gates use checked source execution. Speculative machine integer address additions do not read memory. Addresses never become retained source values, caches or account authority. Original/decoded image reconstruction derives the same selection independently. No allowance or expected value increases.

There are36 actual emitted staged-read occurrences across four native regions, including existing duplicated versions. This is a static count. The exact hot matching eight-byte loop falls **19→15 machine instructions**, and **three stack reloads→zero** per matching step; data loads remain two. It still carries source guards and actual debit/count work. The pinned reference binary keeps invariant address setup outside similar tight comparison loops. The full current instruction section is independently mapped only on the baseline, after verifying every instruction byte against the R191 mapped Object; no stale map is applied to the candidate.

Reported native text is8,152,248→8,152,436 bytes (**+188B**); stack16,672→16,480 bytes (**−192B**). All scalar direct emission and existing native selection paths remain.

## Independent counters and memory

All 20 inputs have separate16-observation-per-side CPU and heap cohorts. **16 instruction medians and 3 cycle medians are higher**; preserve them. CPU samples are a different cohort and are never multiplied by retired instructions.

| Input | Instructions before | Current | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 208,808,249.5 | 208,787,991.5 | -0.009702% | -0.518611% |
| random-32769 | 71,209,425.5 | 71,218,378.0 | +0.012572% | -1.577583% |
| 16384-distribution-change | 64,819,153.5 | 65,118,657.5 | +0.462061% | -0.364283% |
| repeated-32769 | 70,908,454.5 | 66,996,969.0 | -5.516247% | -2.020822% |
| 33027-periodic-window | 76,587,675.0 | 72,730,739.0 | -5.035975% | -1.127002% |
| eight-literals | 12,452,709.0 | 12,607,533.0 | +1.243296% | -1.104458% |

Allocation requests, requested bytes, reallocations and incremental usable peak are exactly equal in all 20 cases. Eight literals retain 3,354 requests/17,643,969 requested bytes/15,279,752 incremental usable peak bytes. Repeated32 retains3135 requests; periodic33 retains3679. This patch saves neither heap allocations nor input-copy volume. Requested capacity is not copied or live bytes; raw RSS/physical footprint include image/setup and remain in the receipts.

A separate forwarding-wrapper diagnostic finds two actual root entries on every observed call. Its disjoint setup/entry-with-callbacks/between-entry/cleanup windows reconcile per observation. On eight literals, the entry-with-callbacks median rises155,241 instructions while setup falls1,326.5 and cleanup95.5. This localizes the adverse cost but does not separate generated source work from callback code. Repeated and periodic entry medians fall3,911,913 and3,857,523.5. Raw64 read-pair calibrations per side have equal5006-instruction medians; nothing is subtracted. These are diagnostics, not primary timings or an additive ledger of medians.

## Fresh profiles and next target

The exact current executable is sampled on periodic33/repeated32/natural32/eight literals in a separate 6,472-call cohort, matched by the same-sized retained-binary cohort. All calls validate and finish at zero managed bytes/nodes. Exclusive child-subtracted wall-stack weights follow; these are not instruction-event samples, callback counts or removable-overhead estimates.

| Input | Generated before | Current | Host below entry before | Current |
|---|---:|---:|---:|---:|
| 33027-periodic-window | 67.11% | 64.48% | 28.71% | 30.77% |
| repeated-32769 | 71.90% | 70.65% | 23.45% | 24.77% |
| reymont-prefix-32769 | 78.59% | 79.42% | 19.02% | 17.96% |
| eight-literals | 9.35% | 8.94% | 78.27% | 79.66% |

The next bounded investigation should explain the small-input entry increase before broadening address staging: distinguish generated copies/spills from changed callback machine code, and inspect immutable geometry still repeated at the natural-text reduction entry. The current hot word loop also retains actual fuel/count updates. Any change must preserve the existing finite prefix/recovery contract and beat this retained baseline across all 20 gates. No next semantic change is included here.

## Qualification and failures

**675 native tests, two new maintained tests, nine focused checks and all 258 actual CI selections pass**, with the same seven inherited exclusions named in native-command.json. The exact new optimization witness executes one test and fails on the clean parent. It passes on the candidate. Original/decoded/restored controls preserve fresh nested offsets, alternating captures, changing row/column/matrix geometry, overflow, unselected overflow, aliases, prior publication, cleanup and recovery. Existing exhaustive captured-window and current-account tests remain.

Unchanged 236 component/eight composition/one real-capacity qualifications and 1,724 native real-match controls pass. Every native match result agrees with the independent byte oracle. The retained source preserves 71 disclosed resident default-work denials per artifact form without allowance increases. Generic state controls and fifteen architecture/catalog checks,132 workflow tests, runtime-only and formatting pass; warnings remain errors. New tests already fall under the maintained composition_tests CI selection.

The Python shim initially lacked yaml even for unchanged baseline modules. Cached PyYAML6.0.3 was restored only into this task temporary directory with an explicit PYTHONPATH; all 132 tests then pass. An initial zero-count parent filter, atomic patch context failure, mapping-summary output-path error and one-entry bracket-analysis assumption are retained as corrected setup errors. No failed command is relabeled passed. The first unit root precedes whitespace-only formatting; the final optimized production library/producer is rebuilt from the exact formatted pinned source. All measured native images use that final library.

GPU features: aot, canonical-native, canonical-native-program, canonical-native-program-runtime, canonical-native-runtime, jit. Libraries opt3, qualification roots opt1, rustc1.96-nightly2026-03-03/aarch64, AppleM1/8GiB, pinned mimalloc2.1.7. R196 Engine/Compute/Runtime/thin selected Stdlib and frozen R145 Core/Math/Syntax/external libraries are reused; the large math library is not rebuilt. Exact commands, current source pins, executable hashes and feature configurations are retained.

Skipped: fresh full Cargo/all-features/dependency/registry-project runs, multi-megabyte timings, the separate ten-workload suite, a new full state-source matrix and standalone scalar-latency controls. Hosted ordinary CI is cancelled by instruction and not awaited; #973 stays draft and review is requested on the published head. No green hosted-CI claim.

## Reproduction

Run heap-summary.py, then analyze.py after unpacking. They validate raw observations, controls and aggregates without running a compiler or compressor. All 20 timing rows and contrary blocks, counter/heap observations, current source files/patch, actual disassembly excerpts, profile aggregates and corrected failures are retained. Native binaries, bytecode, executable descriptors and raw trace stores are excluded from public evidence.

[Immutable evidence and numerical replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r198-captured-read-addresses-fd6abc7a619a.tar.gz) — SHA-256 `fd6abc7a619a8ded3dfe9b0f62f018e54872b88ca67742ca45b003a1ddf281ef`, 2,609,780 bytes / 444 hash-listed files.

The earlier packet `r198-captured-read-addresses-5a7c1b573c97.tar.gz` is superseded solely to classify emitted word-collection helpers as native code in profile totals. Raw measurements and performance results are unchanged; the correction is retained in evidence.
