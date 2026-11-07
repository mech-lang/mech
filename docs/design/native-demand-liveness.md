# Remove unused native values before lowering — R182

Retain this small compiler latency/code-size improvement with explicit instruction tradeoffs. Sixteen of twenty matched elapsed medians improve and four worsen; geometric elapsed change is −0.527064%. Natural32 improves 1.159068% with all eight paired blocks improving. Eight literals worsen 0.204799%. All twenty separate instruction-count medians rise, ten cycle medians rise, and all twelve scalar-control medians rise within their original gate. This is not a hardware-instruction or physical-memory win.

## The missed mechanism and bounded change

The actual prior Object contains loads immediately overwritten before use. Merely marking ordinary private source homes nontrapping produced byte-identical text and was rejected before any performance run. Inspection of pinned Cranelift 0.131.3 showed that its lowering-side-effect predicate retains all loads, including already-nontrapping private homes. An explicit demand cleanup was missing.

The new generic pass starts demand at control and observable effects, then follows ordinary value operands and each live block formal to every current incoming argument. It removes only undemanded pure instructions, ordinary loads already marked nontrapping, and stack-slot loads. Calls, stores, trapping loads/arithmetic, branch conditions, every CFG edge, infinite loops and the declared function-entry ABI remain. Exception-table contexts and unsupported edge-only operands conservatively retain the original lowering. Interior unused block formals and corresponding arguments can be removed together; all six scalar widths are covered. The pass runs after existing local phi cleanup and before the single normal backend data pass.

The actual complete compressor loses 10,647 unused native IR instructions, including 8,878 IR loads, but zero real block parameters. The generic dead-cycle tests do remove parameters; that result is not attributed to the compressor. Source operations, logical prices, current admission, first-change handling, ownership and failure/publication contracts remain independent. No source, allocator, runtime ABI, dependency or extra backend optimization pass changes are included. IR advances to 202; work/ABI/MBC stay 107/39/14. Every prior numeric recipe, window selection and proof count remains equal.

Reported generated text 8,659,628 → 8,588,928 B (-70,700); entry stack 17,152 → 16,832 B (-320). An independent source-map compile reproduces the whole current Object byte-for-byte: 115,828 tags / 279,954 spans.

## Complete-call timing

One unchanged balanced eight-block cohort contains 3,968 primary / 4,048 total complete calls, maximum 33,027 input bytes. All original 1.05 gates pass: 16 faster / 4 slower medians, geometric elapsed change -0.527064%. Every paired adverse block is retained. Three zero reference medians remain unresolved. No primary observation is discarded or repeated, and independent rounds/source variants are not compounded.

| Input | Before / Roc | After / Roc | Elapsed change | Improving/adverse blocks |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 3.719808× | 3.676693× | -1.159068% | 8/0 |
| reymont-prefix-16384 | 4.452282× | 4.414359× | -0.851745% | 7/1 |
| random-32769 | 4.878450× | 4.846793× | -0.648915% | 4/4 |
| 16384-distribution-change | 6.413499× | 6.410778× | -0.042428% | 5/3 |
| repeated-32769 | 3.049449× | 3.048212× | -0.040569% | 5/3 |
| 33027-periodic-window | 3.769513× | 3.737136× | -0.858925% | 4/4 |
| eight-literals | 4.619655× | 4.629116× | +0.204799% | 4/4 |
| empty | unresolved (zero median) | unresolved (zero median) | +0.286041% | 4/4 |
| one-stored | unresolved (zero median) | unresolved (zero median) | +0.434319% | 3/5 |
| seven-stored-prefix | unresolved (zero median) | unresolved (zero median) | +0.416392% | 3/5 |
| eight-overlap | 4.473942× | 4.418957× | -1.228995% | 6/2 |
| 320-overlap | 4.326644× | 4.265398× | -1.415550% | 6/2 |
| eight-literals-prefix | 4.715132× | 4.683040× | -0.680601% | 5/3 |
| 128-biased-dynamic | 3.835105× | 3.812289× | -0.594933% | 6/2 |
| 32-high-entropy-stored | 4.308949× | 4.272554× | -0.844648% | 5/3 |
| random-64 | 4.324726× | 4.281830× | -0.991874% | 5/3 |
| random-256 | 4.425228× | 4.415812× | -0.212782% | 4/4 |
| random-1024 | 4.035733× | 3.991171× | -1.104209% | 5/3 |
| repeated-1024 | 4.238455× | 4.196401× | -0.992205% | 5/3 |
| reymont-prefix-1024 | 4.766980× | 4.758271× | -0.182711% | 6/2 |

Every full evaluate_into call verifies exact pinned raw bytes, independent inflation, equal logical source work, input aliases, prefixes and capacities. Both primary workers close with managed bytes/nodes zero. Image restoration, harness clear/rebind, validation and final cleanup retain their original placement outside the timer. Each before/after pair uses the same fresh reference. Cross-round reference drift is not a compiler regression or a compounded gain.

## Separate retired counters and heap

| Input | Instructions before | Instructions after | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 222019110.0 | 223343326.5 | +0.596443% | +0.327842% |
| reymont-prefix-16384 | 125985433.5 | 126724664.5 | +0.586759% | +0.130871% |
| random-32769 | 72496925.5 | 72770041.5 | +0.376728% | +0.155677% |
| 16384-distribution-change | 66517696.0 | 66967322.5 | +0.675950% | +1.771773% |
| repeated-32769 | 71778723.0 | 71995382.0 | +0.301843% | +0.942906% |
| 33027-periodic-window | 77511993.5 | 77751453.5 | +0.308933% | +0.629847% |
| eight-literals | 12493235.0 | 12579877.5 | +0.693515% | +0.495168% |
| empty | 1060379.5 | 1060410.5 | +0.002923% | -0.661379% |
| one-stored | 1112851.0 | 1113235.5 | +0.034551% | -0.078984% |
| seven-stored-prefix | 1112614.5 | 1113156.0 | +0.048669% | +0.605011% |
| eight-overlap | 11690370.0 | 11772661.0 | +0.703921% | -1.680492% |
| 320-overlap | 11836889.5 | 11917804.5 | +0.683583% | -0.309222% |
| eight-literals-prefix | 12494927.0 | 12581229.5 | +0.690700% | +0.584274% |
| 128-biased-dynamic | 10454405.0 | 10531292.5 | +0.735456% | -0.354082% |
| 32-high-entropy-stored | 11072405.0 | 11150470.0 | +0.705041% | -1.036671% |
| random-64 | 11230195.0 | 11307851.0 | +0.691493% | -1.663453% |
| random-256 | 11764336.0 | 11841755.0 | +0.658082% | -0.305700% |
| random-1024 | 11986795.5 | 12061024.0 | +0.619252% | -2.229923% |
| repeated-1024 | 13503030.5 | 13587284.0 | +0.623960% | +0.283099% |
| reymont-prefix-1024 | 16315515.0 | 16423578.5 | +0.662336% | -0.370322% |

All twenty instruction medians rise, from +0.002923482% to +0.735455533%. Ten cycle medians rise: seven-stored-prefix, eight-literals, eight-literals-prefix, 16384-distribution-change, 33027-periodic-window, repeated-1024, random-32769, repeated-32769, reymont-prefix-16384, reymont-prefix-32769. Natural32 instructions rise 0.596443% and eight literals 0.693515%, despite smaller static code. These independent cohorts are not multiplied together or substituted for primary elapsed. The causal split of instruction changes has not been established.

All twenty allocation-request, requested-byte, reallocation and incremental usable-live-heap peak medians are exactly equal. Live heap before/after and peak have a fixed −16 B offset on every case. Its cause is unproved; it is not presented as a memory saving. Requested capacity is not initialized bytes copied or evidence of streaming.

| Input | Requests | Requested bytes | Reallocations | Incremental usable peak |
|---|---:|---:|---:|---:|
| eight-literals | 3354.0 | 17643969.0 | 68.0 | 15279752.0 B |
| reymont-prefix-32769 | 6449.0 | 24793116.0 | 68.0 | 18722528.0 B |
| random-32769 | 5201.0 | 16018954.0 | 153.0 | 15314280.0 B |

| Input / cohort | RSS before → after | Physical footprint before → after |
|---|---:|---:|
| eight-literals / cpu | 323321856.0 → 323452928.0 | 233391808.0 → 233572096.0 |
| eight-literals / memory | 323633152.0 → 323502080.0 | 233522880.0 → 233604864.0 |
| reymont-prefix-32769 / cpu | 324665344.0 → 324665344.0 | 233391808.0 → 233572096.0 |
| reymont-prefix-32769 / memory | 324878336.0 → 324812800.0 | 233522880.0 → 233604864.0 |

CPU and heap-probe cohorts have separate instrumentation and sessions. Their RSS/physical differences establish no physical-memory reduction. No allocator purge or additional confirmation cohort is used.

## Actual machine evidence

The whole decoded text shrinks 2,164,905 → 2,147,227 words and static SP-relative accesses 598,202 → 596,072. Text words may include padding or literal data. They are static geometry, not executed instruction counts. Source-tagged spans retain adverse register-allocation/layout changes:

| Parent / initializer / selected version | Tagged instructions before → after | Comparisons before → after | SP accesses before → after |
|---|---:|---:|---:|
| 837 / 1371 / None | 17 → 15 | 1 → 1 | 0 → 0 |
| 837 / 1780 / None | 22 → 22 | 3 → 3 | 0 → 0 |
| 837 / 2712 / None | 21 → 18 | 1 → 1 | 3 → 2 |
| 837 / 3121 / None | 25 → 25 | 3 → 3 | 1 → 1 |
| 837 / 3927 / None | 19 → 17 | 1 → 1 | 3 → 2 |
| 837 / 4336 / None | 26 → 27 | 3 → 3 | 2 → 3 |
| 1279 / 1371 / None | 11 → 7 | 0 → 0 | 2 → 0 |
| 1279 / 1780 / None | 14 → 13 | 1 → 1 | 1 → 0 |
| 2624 / 2712 / None | 11 → 7 | 0 → 0 | 2 → 0 |
| 2624 / 3121 / None | 14 → 14 | 1 → 1 | 1 → 1 |
| 3835 / 3927 / None | 11 → 7 | 0 → 0 | 2 → 0 |
| 3835 / 4336 / None | 13 → 14 | 1 → 1 | 0 → 1 |
| 837 / 1391 / 1372 | 41 → 43 | 8 → 8 | 5 → 5 |
| 837 / 2732 / 2713 | 38 → 44 | 8 → 8 | 2 → 6 |

A tally source span at header 15737 / site 15815 grows 120 → 128 instructions and 10 → 18 SP accesses. The DP child gate at 15296 / 15529 grows 30 → 31 instructions and zero → one SP access. Its established reduction entry/body/backedge/finish remain 13/9/3/7 instructions. Captured gates also grow. These tagged spans include cold failure/materialization and shared/hoisted code; they are not complete hot paths or an instruction-cost partition. Adjacent-overwrite examples are bounded diagnostic observations, not blanket permission to remove potentially trapping reads.

## Exact-current profiles

All five recorder exits are zero and cover 17,413 validated complete calls from the exact primary worker/Object. Per-case cleanup and the final graceful-Q receipt are zero. No trace-export recovery or profile rerun is needed this round. Environment fields are scrubbed from archived XML; raw trace containers are excluded. CPU-time samples remain separate from retired events. Outside-main time includes useful generated helpers/runtime/validation, and unknown native attribution remains explicit.

| Input | Complete calls | Samples | Main entry | Unknown native | Tagged child gates |
|---|---:|---:|---:|---:|---:|
| eight-literals | 8988 | 5576 | 9.791966% | 4.644907% | 0.000000% |
| random-32769 | 2358 | 5365 | 70.624418% | 33.793103% | 0.000000% |
| 16384-distribution-change | 2354 | 5373 | 51.628513% | 24.734785% | 1.284199% |
| repeated-32769 | 2961 | 5539 | 71.944394% | 30.204008% | 1.263766% |
| reymont-prefix-32769 | 752 | 5333 | 80.048753% | 39.358710% | 5.044065% |

Natural32 remains about 80% in the generated main entry; random32 about 71%. The prior reference binary geometry identified separate generated functions, while this Object retains the large main entry. A bounded complete numeric-phase outline remains a research candidate for state pressure; no speedup is predicted from static size alone, and no outlining implementation is included here.

## Qualification and failure evidence

All twelve original/decoded Bool/U64/I64/F64 and 0/64-loop controls pass their original 1.05 latency/allocation gates with no iteration-dependent allocation slope. Every scalar median is adverse; ratios range 1.000768008–1.007803644. No scalar entry is retired, allowance increased or expectation changed.

Six focused tests pass: four new IR tests plus the real nested mutable traversal and callback-local pointer freshness controls. They cover I8/I16/I32/I64/F32/F64 unused loads/cycles, verifier/idempotence, retained trapping reads/division/calls/stores and effect order, two branch edges to the same target with different current arguments, and retained infinite-loop control. The real compressor removes no block parameters.

Full compatible GPU qualification passes 653 with the same seven inherited exclusions. All 236 CI-selected names are verified actually executed; the inventory guard changes only 232 → 236. Unchanged 236 component / eight composition / one real-capacity path checks pass. All fifteen architecture/catalog checks, runtime-only, formatting, warnings-as-errors and 132 workflow tests pass.

A separate twenty-row original/decoded public-API full-native witness preserves exact source work, literal outputs, failure origins, unselected paths, fresh recursion, live aliases, prior publication, cleanup and retry. Resident successes/rejections agree. Coverage detects 0 → 100 removed unused loads while the prior ten window predicates remain ten. The clean-parent emitter fails the new coverage assertion with exit 101; the candidate passes. No work comparison across disabled/full-native strategies is used.

Exact commands/features are retained in unit-dead-values/build.json and test.json, gpu-compatible-candidate*.json, checks/receipts.json, checks/ci-final-coverage.json, qualification/*.json, work-probe-receipts.json and all cohort specifications. Frozen R145 Core/Engine/math/dependencies; nightly-2026-03-03 aarch64, GPU opt-level 3, unit/component opt-level 1, canonical-native/program/runtime plus jit/aot, pinned mimalloc 2.1.7. The real compressor executes through its linked Object.

The initial source-home flags experiment is rejected for byte-identical text before any timing. It predates the now-working demand consumer: a future flags experiment must establish the initialized private-home contract and actual emitted-code effect anew. It is not a permanently exhausted hypothesis.

Development/environment failures are retained: Xcode has no llvm-objcopy (exit 72), replaced with validated direct Mach-O section parsing; the initial worker link has stale identity/descriptor includes (exit 1 before execution), corrected only to the current build; the first source-map archive guard rejects its harmless ancestor directory before extraction, followed by a missing script exit 2, then the narrowed guard/build succeeds. The initial patch digest omits two untracked new modules, although both compile and run; all seven source-file hashes are explicitly matched to unit and production builds and the final patch. No performance cohort is repeated for these repairs.

Proactive cleanup removes 292,835,756 B of verified duplicate R181 replay copies, 434,444,800 B of superseded R181 build executables, and 300,108,418 B of terminal R182 diagnostic build duplicates. Manifests retain their hashes; original receipts/source/evidence, essential before/after compilers/workers/Objects/counters/scalars and frozen dependencies remain. No disk exhaustion occurs in this round.

The seven inherited exclusions remain:

- batched::aot::tests::aot_rejected_updates_preserve_backing_buffers_before_dispatch
- batched::aot::tests::concurrent_aot_builds_publish_complete_isolated_libraries
- batched::aot::tests::isa_features_shared_flags_and_abi_change_cache_identity
- batched::axis_tests::gpu_matrix_solve_rejects_singular_inputs_without_publishing
- batched::axis_tests::legacy_matrix_multiply_artifacts_remain_gpu_lowerable
- batched::jit::tests::jit_rejected_updates_preserve_backing_buffers_before_dispatch
- native_artifact::program::runtime::collections::text_join::compiled_map::owned_tests::complete_unchanged_knucleotide_body_uses_the_owned_emitted_map

Published parent `65034db9ca9da4133355108e85784703cde7592b`; measured source `7f26d28e1dfa5b219c86f68736f3d7543a69b514`; baseline compiler source `ebedd713669e8e2a424ac35163c2eb480c368be0`. All seven compiler/test/CI files reconstruct from the published parent and exact patch and match compiled source bytes. Independent source-map compilation repeats the whole Object byte-for-byte; no redundant rebuild solely for amended Git documentation metadata is needed.

Object `e5f37967d1d097bc2ae80100f093387f6168453f00a2fc902e7be85e51c2048b`; worker `54690a56dba0a190883b081595bce598501fc462f168024e04997d88f803f1ca`; source `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; MBC `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. IR/work/ABI/MBC 202/107/39/14. Original algorithm/tables, depth 300/nice 258, near-EOF limits/ties, 262144 U16 tree, 1500513 U32 cache, 305257 U64 path capacity, source allocation sites, output capacity 13 clear/reuse, input aliases, allocator and full API timing remain.

Fresh all-feature/dependency builds, another standalone AOT harness, multi-megabyte inputs and the separate ten-workload suite are skipped. Ordinary hosted CI is cancelled by standing instruction; review automation is preserved and existing #973 remains draft pending green CI and clean exact-head review. One optimization commit in that PR; no new PR.

Immutable packets retain all timing cases, contrary observations, raw counters, scalar controls, source/fixture pins, clean-parent failure, actual machine maps/disassembly and repaired wrapper failures. Public archives exclude native binaries, bytecode, raw trace containers, private environment exports and copied user briefs. Receipt-only replay reconstructs this report and results without compiling or compressing.

This completes the demand-liveness experiment. The broader parent/child brief remains partial at opaque returns and fresh captured handoffs. Close publication/review of this checkpoint before the next implementation. The 1× goal remains active.
