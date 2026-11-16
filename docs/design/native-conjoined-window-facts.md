# Selected compound facts for native child windows — R181

Retain this bounded compiler improvement. All 20 matched medians improve, with a geometric elapsed change of −0.910814%. Random32 improves 2.808356%, periodic33 1.003465%, eight literals 0.809978%, and natural32 0.216131%. Instruction counts are mixed: seven medians fall and thirteen rise. This is a measured latency improvement with explicit counter tradeoffs.

## The missed fact and the change

The pinned match-extension entry checks `(str-at + maximum <= length(input)) && (match-at < str-at)` before its child initialization. The existing fact adapter encoded the normalized conjunction as an opaque Boolean. Selecting its successful branch therefore retained neither operand, even in the same mutable-parent visit. The retained R180 diagnostic at initializer 1371 shows the checked sum and both comparisons, followed by opaque `Boolean(1360,1194)`, with the two operand relations absent.

The new generic source witness inserts that exact window assertion into the existing shared four-word comparison witness. On published parent 54280db43e, the regression fails its coverage assertion: the compiler proves only the earlier two order/lower-bound checks and no full window. The candidate passes. This was a missing compiler proof, not a previously incorrect compressed result.

The shared expression domain now retains AND/OR structure only when both operands are already normalized predicates. All eager operand computations remain in source order. A selected true AND establishes both operands; a selected false OR establishes both negations. False AND and true OR establish neither operand. An iterative DAG walk visits repeated nodes once under the existing term allowance. Opaque/raw values retain their prior conservative behavior.

A query-local sum rule consumes a selected mathematical sum bounded by this exact current array length. A sum of smaller current operands fits the same window and cannot overflow. Wrapping sums, unknown ordering, changed words and a different view grant no proof. This recovers actual smaller and full child windows without retaining ownership permission or exporting facts across fresh scopes or unknown returns. Original source assertions, residual shape/view checks, source prices, ownership, first-change handling, funding and totality remain independent.

Native image validation reconstructs these facts from the same canonical plan. IR advances to 201; work/ABI/MBC remain 107/39/14. The canonical plan, closed-tail recipes, prepared numeric regions, all 22 guarded-window selections, and prior 112 projection/627 read/54 arithmetic proof counts remain equal. No benchmark source, allocator, runtime ABI or dependency changes are included.

## Actual machine evidence

Current-argument admission proofs rise 12 → 60 across parent headers 837, 1279, 2624 and 3835: 48 additional Le/Fits predicates at twelve actual initialization sites. Independently compiled source-location labels reproduce the whole current Object byte-for-byte before any address is used. Every before/after disassembly uses its own Object coordinates.

| Parent / initializer / selected version | Tagged instructions before → after | Comparisons before → after | SP accesses before → after |
|---|---:|---:|---:|
| 837 / 1371 / None | 29 → 17 | 3 → 1 | 5 → 0 |
| 837 / 1780 / None | 29 → 22 | 3 → 3 | 5 → 0 |
| 837 / 2712 / None | 31 → 21 | 3 → 1 | 6 → 3 |
| 837 / 3121 / None | 32 → 25 | 3 → 3 | 6 → 1 |
| 837 / 3927 / None | 30 → 19 | 2 → 1 | 9 → 3 |
| 837 / 4336 / None | 33 → 26 | 3 → 3 | 7 → 2 |
| 1279 / 1371 / None | 20 → 11 | 2 → 0 | 4 → 2 |
| 1279 / 1780 / None | 17 → 14 | 1 → 1 | 2 → 1 |
| 2624 / 2712 / None | 16 → 11 | 2 → 0 | 0 → 2 |
| 2624 / 3121 / None | 15 → 14 | 1 → 1 | 0 → 1 |
| 3835 / 3927 / None | 17 → 11 | 2 → 0 | 1 → 2 |
| 3835 / 4336 / None | 16 → 13 | 1 → 1 | 1 → 0 |
| 837 / 1391 / 1372 | 32 → 41 | 6 → 8 | 0 → 5 |
| 837 / 2732 / 2713 | 32 → 38 | 6 → 8 | 0 → 2 |

The first inspected gate removes real comparisons and spills: 29 → 17 instructions, three → one comparisons, five → zero SP accesses. Two captured-version control gates grow to 41 and 38 instructions, with six → eight tagged comparisons. All adverse spans remain. Tags can include shared/hoisted provenance and funding work; these are static segments, not complete parent-to-child paths, executed visit counts or a per-function retired-instruction ledger. Forty-eight source predicates do not establish forty-eight removed machine compares. Global register allocation/layout also changes.

Reported generated text 8,661,492 → 8,659,628 B (-1,864); entry stack 17,216 → 17,152 B (-64).

## Complete-call timing

One unchanged balanced eight-block cohort contains 3,968 primary / 4,048 total complete calls, maximum 33,027 input bytes. All original 1.05 gates pass: 20 faster / 0 slower medians; geometric elapsed change -0.910814%. Paired adverse blocks remain, including four adverse natural32 and repeated32 blocks. Three zero reference medians remain unresolved. No primary observation is discarded or repeated, and independent rounds/source variants are not compounded.

| Input | Before / Roc | After / Roc | Elapsed change | Improving/adverse blocks |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 3.712139× | 3.704116× | -0.216131% | 4/4 |
| reymont-prefix-16384 | 4.457244× | 4.435999× | -0.476646% | 6/2 |
| random-32769 | 5.084448× | 4.941659× | -2.808356% | 7/1 |
| 16384-distribution-change | 6.446782× | 6.422283× | -0.380025% | 5/3 |
| repeated-32769 | 3.047536× | 3.036298× | -0.368775% | 4/4 |
| 33027-periodic-window | 3.562616× | 3.526867× | -1.003465% | 6/2 |
| eight-literals | 4.647733× | 4.610088× | -0.809978% | 5/3 |
| empty | unresolved (zero median) | unresolved (zero median) | -0.036065% | 5/3 |
| one-stored | unresolved (zero median) | unresolved (zero median) | -0.694610% | 6/2 |
| seven-stored-prefix | unresolved (zero median) | unresolved (zero median) | -0.826303% | 6/2 |
| eight-overlap | 4.623283× | 4.558991× | -1.390616% | 5/3 |
| 320-overlap | 4.236204× | 4.194329× | -0.988483% | 4/4 |
| eight-literals-prefix | 4.632261× | 4.618322× | -0.300912% | 4/4 |
| 128-biased-dynamic | 3.848274× | 3.813578× | -0.901613% | 5/3 |
| 32-high-entropy-stored | 4.233538× | 4.179633× | -1.273299% | 7/1 |
| random-64 | 4.403619× | 4.342230× | -1.394073% | 8/0 |
| random-256 | 4.385809× | 4.336555× | -1.123038% | 5/3 |
| random-1024 | 4.007430× | 3.964237× | -1.077831% | 5/3 |
| repeated-1024 | 4.113952× | 4.092426× | -0.523238% | 5/3 |
| reymont-prefix-1024 | 4.852505× | 4.775563× | -1.585608% | 6/2 |

Every full evaluate_into call verifies exact pinned raw bytes, independent inflation, equal logical source work, input aliases, prefixes and capacities. Both primary workers close with managed bytes/nodes zero. Image restoration, harness clear/rebind, validation and final cleanup retain their original placement outside the timer. Current ratios use the same fresh reference for each before/after pair. Cross-round drift of an unchanged baseline binary is not a compiler regression or a compounded gain.

## Independent retired counters and heap

| Input | Instructions before | Instructions after | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 221375765.5 | 222005002.5 | +0.284239% | -1.816656% |
| reymont-prefix-16384 | 125423630.0 | 125984211.5 | +0.446950% | -0.567567% |
| random-32769 | 72119752.5 | 72495835.5 | +0.521470% | +0.375050% |
| 16384-distribution-change | 66213636.0 | 66517704.0 | +0.459223% | -0.109274% |
| repeated-32769 | 71969188.0 | 71775131.0 | -0.269639% | +0.412918% |
| 33027-periodic-window | 77704648.0 | 77508380.0 | -0.252582% | +0.034743% |
| eight-literals | 12493222.0 | 12494334.0 | +0.008901% | -0.996676% |
| empty | 1061741.0 | 1061783.5 | +0.004003% | -0.563564% |
| one-stored | 1114250.0 | 1114211.5 | -0.003455% | -0.914860% |
| seven-stored-prefix | 1114024.0 | 1113977.5 | -0.004174% | +0.319294% |
| eight-overlap | 11689747.0 | 11691439.5 | +0.014478% | +0.292414% |
| 320-overlap | 11838373.5 | 11838215.5 | -0.001335% | -0.025649% |
| eight-literals-prefix | 12494823.0 | 12494748.0 | -0.000600% | -1.635997% |
| 128-biased-dynamic | 10452084.5 | 10455171.0 | +0.029530% | -1.045133% |
| 32-high-entropy-stored | 11071426.5 | 11073534.0 | +0.019035% | +0.103034% |
| random-64 | 11229186.5 | 11231339.0 | +0.019169% | -5.667969% |
| random-256 | 11762186.0 | 11765490.0 | +0.028090% | -1.038660% |
| random-1024 | 11975783.5 | 11987948.5 | +0.101580% | -0.132897% |
| repeated-1024 | 13510102.5 | 13504238.0 | -0.043408% | -0.048944% |
| reymont-prefix-1024 | 16298485.5 | 16316395.5 | +0.109888% | +0.758899% |

Seven instruction medians fall and thirteen rise; seven cycle medians rise. Adverse instruction cases: empty, eight-literals, eight-overlap, 128-biased-dynamic, 32-high-entropy-stored, 16384-distribution-change, random-64, random-256, random-1024, random-32769, reymont-prefix-1024, reymont-prefix-16384, reymont-prefix-32769. Adverse cycle cases: seven-stored-prefix, eight-overlap, 32-high-entropy-stored, 33027-periodic-window, random-32769, repeated-32769, reymont-prefix-1024. Natural32 instructions rise 0.284239% despite primary elapsed improving; random32 instructions rise 0.521470% and its separate cycle median 0.375050%. Repeated32 and periodic33 instructions fall approximately 0.27% and 0.25%, but their cycle medians rise. No broad instruction-reduction claim is made.

All 20 allocation-request, requested-byte, reallocation and incremental usable-live-heap peak medians are exactly equal. Live heap before/after and peak have a fixed +16 B offset on every case. Its cause is not established; it is not attributed to invocation construction or presented as a memory saving. Eight literals retain 3,354 allocation requests, 17,643,969 requested bytes, 68 reallocations, and a 15,279,752 B incremental usable peak. Requested capacity is not initialized bytes copied or evidence of streaming.

| Input / cohort | RSS before → after | Physical footprint before → after |
|---|---:|---:|
| eight-literals / cpu | 323190784.0 → 323305472.0 | 233309888.0 → 233375424.0 |
| eight-literals / memory | 305905664.0 → 323534848.0 | 233490176.0 → 233441024.0 |
| reymont-prefix-32769 / cpu | 324485120.0 → 324648960.0 | 233309888.0 → 233375424.0 |
| reymont-prefix-32769 / memory | 308330496.0 → 324780032.0 | 233490176.0 → 233441024.0 |

CPU-cohort RSS/physical changes are small; the separate heap-probe cohort has a larger RSS difference with slightly lower physical footprint. Both cohorts are retained. They measure different sessions and instrumentation, and establish no physical-memory reduction. No allocator purge or additional RSS confirmation cohort is used.

## Profiles and reference binary

Five recordings of the exact primary worker cover 17,545 validated complete calls. Whole-Object identity precedes mapping. The current map has 115,828 tags / 279,254 spans. Outside-main time includes generated helpers, useful runtime work and validation; CPU samples are separate from retired events and do not establish a removable-host fraction. Unknown native attribution stays explicit.

| Input | Complete calls | Samples | Main entry | Unknown native | Tagged child gates |
|---|---:|---:|---:|---:|---:|
| eight-literals | 9136 | 5496 | 10.225619% | 4.894469% | 0.000000% |
| random-32769 | 2322 | 5480 | 69.489051% | 34.543796% | 0.000000% |
| 16384-distribution-change | 2368 | 5570 | 51.328546% | 27.360862% | 0.807899% |
| repeated-32769 | 2972 | 5431 | 69.913460% | 28.190020% | 2.761922% |
| reymont-prefix-32769 | 747 | 5395 | 79.851715% | 38.368860% | 5.227062% |

The exact pinned natural32 reference binary is hash-verified and disassembled. Its eighteen opaque-hash generated-function intervals span 122,456 B and decode to 30,614 instruction lines, with 49 static calls to sixteen generated targets. The Mech Object has the main entry plus two small word-collection text symbols; its reported generated text is 8,659,628 B. Boundaries differ and may include padding/data. These are static organization measurements, not executed instruction counts, resolved source-function names or a predicted speedup. They support a later bounded numeric-phase outlining investigation; no outlining implementation is part of this round.

## Scalar protection and qualification

All 12 original/decoded Bool/U64/I64/F64 and 0/64-loop controls pass their original 1.05 latency gate with unchanged requests/bytes and no iteration-dependent allocation slope. Ratios range 0.989905806–1.000756213. Adverse blocks/controls remain.

Five new regressions (two domain / three source tests) pass alongside the five previous focused controls: ten focused tests total. Full compatible GPU qualification passes 649, with the same seven inherited exclusions. All 232 CI-selected names are listed and verified executed; the existing inventory guard changes only 227 → 232 for these five new tests. Fresh unchanged 236 component / eight composition / one real-capacity path checks pass. All 15 architecture/catalog checks, runtime-only, formatting, warnings-as-errors and 132 workflow tests pass.

Controls cover forced/unforced AND/OR results, NOT and repeated DAG nodes, opaque values, checked versus wrapping sums, current/different views and overwritten operands. Actual source tests cover both false conjuncts rejecting before an overflowing start, overflow while evaluating the assertion, overflow after a successful assertion, a second recursive visit with reversed coordinates, fresh mismatches/matrix shapes, unselected zero visits, exact finite prefixes, aliases, prior publication, cleanup and retry in original/decoded and direct/restored forms.

A separate public-API full-native before/after witness verifies twenty original/decoded rows: work, literal outputs and error origins agree, with proof counts 2 → 10. Resident success outputs and rejections agree. Live aliases and prior publication remain unchanged on failures, retries succeed, and final managed bytes/nodes are zero. No work comparison across disabled/full-native strategies is used.

Exact commands/features are retained in unit-candidate/build.json and test.json, gpu-compatible-candidate*.json, ci-inventory.json, checks/receipts.json, qualification/*.json, work-probe-receipts.json and every timing/counter/scalar/profile specification. Frozen R145 Core/Engine/math/dependencies; nightly-2026-03-03 aarch64, GPU opt-level 3, unit/component opt-level 1, canonical-native/program/runtime plus jit/aot, pinned mimalloc 2.1.7. The real compressor executes through its linked Object.

## Failures, recovery and exclusions

The clean-parent missing-window coverage failure is the intended negative regression. External wrapper failures are initial final-versus-candidate receipt/packet filenames and a retained before/after specification read as a flat command; both fail before any affected compilation/measurement and are corrected. A patch application used a stale identity-constant spelling and applied nothing; the exact maintained version constant is then updated. No source expectation, allowance, warning or funding policy is weakened.

An environmental ENOSPC occurs after all five profile recordings and their 17,545 calls finish, during the final XML export. All five recorder exits are zero, observations validate, and CASE_CLEANUP bytes=0 nodes=0 is retained for every case. The driver receipt write also fails and leaves an empty file, preserved separately. Only the existing last trace is exported after cleanup; no compressor, primary, counter, scalar or profile-recording cohort is rerun. The combined profiling worker is terminated by the harness on export failure, so its final graceful-Q receipt is absent. This is explicit; per-case profile cleanup and the primary/counter/generic full-session cleanup are independently observed.

Cleanup removes 581,097,249 B of already verified duplicate public replay downloads/overlays and 1,504,692,152 B of obsolete task-owned compiled builds. Hash inventories remain. Both measurement workers, exact before/after compiler libraries and Objects, counter/scalar executables, the current qualified test binary, every source/evidence file and frozen R145 dependencies are preserved. All recorded profiles and three analyses recover successfully.

The seven inherited exclusions are:

- batched::aot::tests::aot_rejected_updates_preserve_backing_buffers_before_dispatch
- batched::aot::tests::concurrent_aot_builds_publish_complete_isolated_libraries
- batched::aot::tests::isa_features_shared_flags_and_abi_change_cache_identity
- batched::axis_tests::gpu_matrix_solve_rejects_singular_inputs_without_publishing
- batched::axis_tests::legacy_matrix_multiply_artifacts_remain_gpu_lowerable
- batched::jit::tests::jit_rejected_updates_preserve_backing_buffers_before_dispatch
- native_artifact::program::runtime::collections::text_join::compiled_map::owned_tests::complete_unchanged_knucleotide_body_uses_the_owned_emitted_map

Published parent `54280db43e2d26e2554ff2651f24e929e11571c4`; measured source `ebedd713669e8e2a424ac35163c2eb480c368be0`; baseline compiler source `bcec44b0c2dab03e228fbdec5fdaa142552ff1ed`. All nine changed compiler/test/CI files reconstruct from the published parent and exact patch. Library/Object/unit builds record parent+patch before the Git commit; compiled source hashes match the clean committed files. CI is checked separately against 232 actually executed names and workflow tests. An independent map compiler repeats the whole Object byte-for-byte. A redundant full compiler build solely for Git metadata is avoided.

Object `319f7bfa3a38a4f72e5d7c21ecb9efcf526ee3880b77b1ebe9c5cf96bb935d08`; worker `e2f0d34eeddf4f2a8a069a9c1bdde98939caa34d06dd0c0de0cf78963954691a`; source `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; MBC `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. IR/work/ABI/MBC 201/107/39/14. Preserve original algorithm/tables, depth 300/nice 258, near-EOF limits/ties, 262144 U16 tree, 1500513 U32 cache, 305257 U64 path capacity, source allocation sites, output capacity 13 clear/reuse, input aliases, allocator and full API boundary.

Fresh all-feature/dependency builds, another standalone AOT harness, multi-megabyte inputs and the separate ten-workload suite are skipped. Ordinary hosted CI is cancelled by standing instruction; review automation is preserved and existing #973 remains draft pending green CI and clean exact-head review. One optimization commit in that PR; no new PR is opened.

Immutable packets retain every timing case, contrary observations, raw counters, scalar controls, source/fixture pins, negative-parent evidence, actual machine maps/disassembly and environmental recovery. Native binaries, bytecode, raw trace containers, private environment exports and copied user briefs are excluded from public archives. The receipt-only checkpoint reconstructs the report and results without compiling or compressing.

This completes the selected compound-window experiment. The broader mutable-parent/child brief remains partial at opaque returns and fresh captured-version handoffs, which retain their checks. The new binary geometry also identifies phase outlining as a structural research candidate. Inspect remaining current costs and exact-head feedback before the next bounded implementation. The 1× objective remains active.
