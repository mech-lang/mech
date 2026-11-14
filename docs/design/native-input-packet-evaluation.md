# Native argument reads: measured alternative — R235

**Keep R234 as the retained compiler.** The six-file candidate qualifies semantically and passes every unchanged 1.05 timing gate, but the two complete cohorts are both slower geometrically. Pooled elapsed rises **0.827%**. Natural32 improves **0.731%**, but periodic/repeated/random32 regress about **1.7–2.0%**, distribution16 **2.664%**, and twenty pooled block means rise. The complete candidate and all favorable/adverse measurements are archived; the patch is not discarded or silently adopted.

## Matched complete-call results

| Input | Retained R234 / Roc | Candidate R235 / Roc | Pooled blocks | Instructions | Cycles |
|---|---:|---:|---:|---:|---:|
| reymont-prefix-32769 | 2.255569× | 2.239070× | -0.731% | -0.443% | -0.769% |
| 33027-periodic-window | 1.670905× | 1.703978× | +1.979% | +0.152% | +1.803% |
| repeated-32769 | 1.584578× | 1.611664× | +1.709% | +0.152% | +1.495% |
| random-32769 | 2.787998× | 2.835677× | +1.710% | +0.199% | +2.570% |
| 16384-distribution-change | 2.758318× | 2.831807× | +2.664% | +0.065% | +1.949% |
| eight-literals | 0.493009× | 0.494791× | +0.310% | -0.013% | +2.410% |
| reymont-prefix-16384 | 2.200320× | 2.185142× | -0.690% | -0.427% | -0.733% |
| empty | 1.976190× | 1.976190× | +0.043% | +0.000% | +7.825% |
| one-stored | 1.976190× | 1.976190× | +0.178% | +0.000% | +9.256% |
| seven-stored-prefix | 2.000000× | 2.000000× | +0.426% | +0.000% | +0.333% |
| eight-overlap | 0.483096× | 0.489941× | +1.099% | -0.020% | -1.836% |
| 320-overlap | 0.555786× | 0.558414× | +0.792% | +0.059% | -0.564% |
| eight-literals-prefix | 0.494223× | 0.494936× | +0.430% | -0.008% | -5.676% |
| 128-biased-dynamic | 0.495789× | 0.500999× | +1.136% | +0.005% | -1.974% |
| 32-high-entropy-stored | 0.523628× | 0.528608× | +1.271% | -0.001% | -0.037% |
| random-64 | 0.553774× | 0.558600× | +0.281% | +0.003% | -3.141% |
| random-256 | 0.657299× | 0.665547× | +1.499% | +0.037% | -0.247% |
| random-1024 | 0.742857× | 0.753101× | +1.810% | +0.096% | -0.853% |
| repeated-1024 | 0.654102× | 0.669332× | +2.062% | +0.141% | +0.061% |
| reymont-prefix-1024 | 0.929510× | 0.936709× | +1.032% | -0.116% | +1.276% |
| heldout-window-32767 | 2.260931× | 2.242620× | -0.810% | -0.450% | -0.555% |
| heldout-window-32768 | 2.261575× | 2.246027× | -0.688% | -0.448% | -0.428% |
| heldout-slides-65537 | 2.330633× | 2.315817× | -0.636% | -0.433% | -0.799% |
| heldout-random-65537 | 3.095631× | 3.161056× | +2.113% | +0.202% | +1.943% |
| heldout-mixed-131073 | 2.527922× | 2.573842× | +1.816% | -0.006% | +1.735% |

Both columns use the same fresh reference denominators from these runs. They are point-median multiples; the separate elapsed percentage compares medians of balanced block means. Historical ratios are not code changes. The initial cohort's geometric regression is 0.591%, the second 0.753%, and the nonlinear pooled-block result 0.827%. No extra retry, changed expected value or increased allowance. All 25 point/block gates pass in both cohorts and pooled 48, while overall performance still rejects adoption.

Both sides run fixed level 12 with runtime inputs, the same Rust clock/validation library, equal 200 ms warmup, balanced order and held borrowed input/output aliases. One executable per variant handles all 25 inputs, including five held-outs through 131,073 B. Full compression, real scratch construction, output transfer and cleanup remain timed; preparation, validation/logging and old-output drop remain outside. All exact streams independently inflate and preserve aliases, prefixes/capacities and final zero nonreturned payload/owners. No builds, tests or profiles overlap primary or hardware/memory collection. 324,444 complete primary calls / 324,144 retained observations, 3,900 separate counter calls and 50 full-stream smokes remain.

## Checked internal packet span

Only local compact numeric callees gain a checked, current input-buffer certificate. Phase::emit fills each declared word after current owner/geometry gates; shared stack-slot reservation validates the span and keeps separate input/output storage alive until the synchronous call returns. Neither address is a source value or retained by array helpers. The next call rewrites its current arguments.

Entry::Compact now passes its exact word count. Boundary reconstructs scalar/view lanes, writable-owner lanes and fixed-capacity lanes, checks agreement, then creates a private Input. It checks byte arithmetic and the existing transport offset representation; each generated read checks its own word offset. An empty span is valid but has no readable word. Out-of-span reads and mismatched or overflowing extents refuse at compilation. This adds no policy allowance.

Only those loads gain notrap under the existing NontrappingHomes optimization. No readonly, aligned, alias-region or can_move flag is added. Standalone Diagnostic inputs retain default flags; ordinary entries and pointed-to source arrays retain their old contracts. Source checked arithmetic, bounds, lexical assertions, error/debit order and publication/retry stay unchanged. No address, permission or value is cached across calls or invocations. No source ID, field, program name, input size or compression-path selector decides eligibility.

All 134 existing phases retain identical input/output geometry, stack size, source bounds, private-store/read-reuse/widening reports and selection. 298 additional dead input loads retire across 89 routines. Root Object and 47 other numeric/map bodies remain byte-identical. Kernel text shrinks 1,224 B; root text 5,649,872 B and stack 55,088 B remain. The minimal same-index checked witness shrinks 492→488 B and removes one input load. The real component uses its inputs and stays 2,624 B; the complete compressor's four path routines each remove ten input loads and shrink 2,048→2,000 B. Actual disassembly also shows altered spill/code shape; do not equate static load count with whole-call instruction savings.

Full worker file size is 6623288→6623288 B. All 25 payload request/byte/copy/zero/private-peak fields and global request/byte/reallocation/incremental usable-peak fields remain equal. Whole-worker physical/RSS measurements remain separately reported; no streaming, allocation or whole-worker memory improvement is claimed.

## Qualification and negative result

696 maintained tests pass with the same single six-feature configuration (`aot`, `jit`, `canonical-native`, `canonical-native-runtime`, `canonical-native-program`, `canonical-native-program-runtime`) and `-D warnings`, with the same seven inherited exclusions. Four new tests cover exact/empty/mismatched/overflow/last-word spans, unproved and disabled-pass flags, actual IR elimination that retains a trapping source read, and fresh values after stores/across 40 native calls to two buffers. All four run focused and in the full suite. The existing six read-reuse tests and 1,057,536 admitted/declined native value controls run focused and in the suite with unchanged expectations.

All 7,064 original/decoded parent/candidate native/oracle source cases pass: 1,116 checked-read, 160 real path, 5,788 matching. 279 resident calls are fresh. Unchanged older path/matching rows retain explicit hashes and their 34 inherited default-work denials; no prior resident call is recounted. Errors, original origins, current recursive visits, unselected branches, live aliases, prior output on failure, cleanup and successful retry agree.

Fresh 25,184 wall-profile and 103,919 actual-PC-profile calls use this exact candidate executable/Object/symbol map. Eleven instruction medians fall, three tie and eleven rise; thirteen cycle medians fall and twelve rise. The apparently smaller static program is not uniformly cheaper dynamically. Placement, register allocation and changed hot instruction shapes are hypotheses, not proved explanations. Do not label an unisolated cause as established. No completed timing/counter/profile observation is discarded or repeated.

Maintained module-layout/native-build-analysis checks and formatting of all six changed files pass. The extra experimental audit still finds inherited compact/owner_flow.rs plus owner_flow/ layout debt: both parent and candidate fail with identical involved files. Its separate repair remains due before ordinary integration. No new compiler build/test/semantic failure or runtime/program resource denial occurs. Read-only path guesses are recorded. Completed source-control/test executables are hash-verified before deletion, and verified older archived observations/builds are freed without changing current R234 measurements or workers. CI is cancelled under the standing instruction; the existing PR remains draft with a fresh review requested.

## Next compiler target: demanded low-word arithmetic

The separate read-only audit finds real wrapping-U32 idioms in the retained source: optimal-add-cost/narrow32, the wrapping hash product and offset-slot subtraction. Actual retained ARM64 still uses 64-bit adds plus 32-bit masks in the scalar cost path. Local Clang and Rust controls for the same unsigned laws emit one `add w`, `sub w` or `mul w` plus return. Their exact sources, assembly and commands are in the packet. These are code-shape controls, not new elapsed or compressor performance measurements.

Start the next experiment from **R234**. Narrow pure wrapping I64 arithmetic feeding an exact low32 mask to I32 arithmetic plus zero extension. Keep other uses, checked-overflow/status logic, source checks, debits and ABI words. First establish high-bit/wrap, checked/multiuse negative, original/decoded and finite-prefix/error/retry witnesses; then all 25 timings/counters/current profiles. No compiler implementation or measured saving for this idea is claimed here. The vectorized cost path already exists; this target concerns scalar arithmetic. Do not silently combine the unadopted input-packet alternative.

## Pins and scope

- base: `eab2b19251c9ac6a221b862f1354bb5ec45a4d34`
- normal_compiler: `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`
- compiler_source_manifest_sha256: `241f947a2092f40dc954566557ff3ba48478cad96d4517acb33a7e071a75dcc0`
- compiler_patch_sha256: `528b084e8d0cc7eac0dc57f57a39dc506abe5cb0ecfc54d97039dcbad3b4d79f`
- source_sha256: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`
- bytecode_sha256: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`
- object_sha256: `e7cb459a803b3ee0bdfda2773bf2e68d6c7339ead587c89dda4d0b678963483b`
- parent_object_sha256: `1561d3a0d98b1db3e8176fb23d5725f543e9accf0b3b383d765c0d80855573fc`
- worker_sha256: `fd30f742a92bb449f77ad17e1d83a1af3a74d531b8c9db4be0278b4883b8876a`
- reference_worker_sha256: `ed1ab9175cd175d26d453afc26ba1fa7adb4827ec705aa1cf9bcd231ac84860c`
- allocator_sha256: `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`

Normal R215 remains IR/work/ABI/MBC215/107/39/14. These are standalone timings; its additional ordinary exact resource/admission/accounting/publication/recovery contract remains outside this experiment. The algorithm, tables, search limits, allocation sites/timing boundaries, 262,144 U16 tree, 1,500,513 U32 cache, 305,257 U64 path capacity, output capacity 13 clear/reuse, source/MBC pins and R225 separation remain. No LLVM, new operation class, branch or PR. Small original/decoded pairs rerun; the complete strict MBC14 image emits once. Multi-MB inputs, a second full-image emit and historical packet/write censuses are not newly claimed.

[Full source, six-file patch, both complete cohorts, counters, profiles, tests and C/Rust audit](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r235-native-inputs-2b977a6fa19e.tar.gz). SHA-256 `2b977a6fa19eafd5c4f78a4ded63457a3a39c8a10cd69915fa7e6a75aeb0fa50`; 6817912 B. All 1,653 public hashes and numerical replay verify. Native binaries/Objects, bytecode, full executable metadata/descriptors and raw traces/stack samples are excluded. R234 remains the retained baseline; parity is still open.

The initial archive remains immutable. Its one imprecise sentence about empty spans is corrected in this packet: zero-word spans are valid; every read is out of span. Compiler source and all numerical result hashes are unchanged. Publication verification resumed after preserving an existing download directory and a transient checksum-download HTTP500; all final hashes/replay verify, with no timing rerun.
