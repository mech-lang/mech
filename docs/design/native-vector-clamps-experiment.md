# R171: vector clamped-subtraction maps

Retain this general compiler change. All20 unchanged1.05 elapsed gates pass: 15 faster/5 slower medians; geometric elapsed improves 1.774875%. The source, bytecode, algorithm and complete evaluate_into boundary stay pinned. The comparison is retained R170, not R169.

| Input | Before ms | Current ms | Before / Roc | Current / Roc | Change |
|---|---:|---:|---:|---:|---:|
|random-32769|6.420562|5.692625|6.103196x|5.411240x|-11.337597%|
|repeated-32769|5.182688|4.889042|3.899690x|3.678737x|-5.665892%|
|33027-periodic-window|5.545812|5.274833|4.418974x|4.203054x|-4.886192%|
|reymont-prefix-32769|18.910729|18.626729|4.336329x|4.271206x|-1.501793%|
|reymont-prefix-16384|10.755021|10.784938|5.043386x|5.057415x|+0.278163%|
|16384-distribution-change|5.645958|5.600562|7.128735x|7.071417x|-0.804053%|
|eight-literals|1.357292|1.346021|4.696510x|4.657510x|-0.830404%|
|empty|0.115958|0.116333|below resolution|below resolution|+0.322960%|
|one-stored|0.119792|0.120021|below resolution|below resolution|+0.191165%|
|seven-stored-prefix|0.119750|0.119896|below resolution|below resolution|+0.121503%|
|eight-overlap|1.329542|1.298771|4.632551x|4.525333x|-2.314444%|
|320-overlap|1.318645|1.298271|4.508190x|4.438533x|-1.545108%|
|eight-literals-prefix|1.394583|1.423771|4.876166x|4.978219x|+2.092883%|
|128-biased-dynamic|1.152666|1.134125|3.947486x|3.883990x|-1.608532%|
|32-high-entropy-stored|1.248188|1.235437|4.395026x|4.350132x|-1.021481%|
|random-64|1.274417|1.257188|4.456003x|4.395760x|-1.351952%|
|random-256|1.344896|1.323687|4.558969x|4.487076x|-1.576962%|
|random-1024|1.249083|1.243208|4.068674x|4.049537x|-0.470345%|
|repeated-1024|1.436250|1.410771|4.392202x|4.314283x|-1.774030%|
|reymont-prefix-1024|1.657166|1.640500|5.021717x|4.971212x|-1.005723%|

This is one fresh balanced20-case cohort,3968 primary/4048 total complete calls;256 fresh reference observations per case. Every stream matches exact reference bytes and independently inflates, with current aliases, prefixes, capacities, source work and final managed bytes/nodes zero. Three tiny stored-only reference medians are zero; no ratio is invented. Largest input33027B. Historical R170 denominators and medians remain separate; gains are not compounded.

The existing source-derived collection proof already establishes a total, pure pointwise map and admits its current source/sink once. A new bounded instruction-selection proof recognizes the original unsigned scalar CFG x < threshold ? 0 : x - threshold, including checked unsigned conversions. It requires equal input/output widths, the same immutable threshold, exact zero/subtraction outcomes, and both selected paths. Unsupported operations, failing/unproved paths, mismatched expressions and cyclic/ambiguous visits decline. No benchmark names, source edits, serialized proof flags, ABI changes or new unsafe boundary are introduced. The canonical artifact reconstructs the same maintained proof.

The selected helper emits a64-byte vector prefix and retains the original checked scalar remainder. U8/U16 use portable unsigned saturating subtraction; U32/U64 use unsigned vector comparison/subtraction/selection. A wider current threshold is clamped to the element maximum before narrowing, preserving the all-zero result above that maximum. The original ownership, current admission, source charges, finite failures, scalar remainder and publication remain unchanged.

Actual ARM64 code selects four UQSUB.8H instructions per32 tree entries, matching the inspected reference tree-slide pattern. Exact diagnostic start/end markers delimit complete evaluate_into calls. Three calls per20 cases agree: random/repeated32KB execute helper18 and periodic/natural32KB helper19, each over262144 entries (8192 vector groups, no tail). Eight literals and distribution-change16KB do not execute these vector helpers. Their small whole-binary timing/counter changes are observed but are not attributed to this loop. The naive and threshold-clamped independent C witnesses both emit widening/comparison/narrowing vectors rather than UQSUB; neither is a C compressor timing or proof that C cannot match this code.

The main entry is byte-identical to R170 from its unchanged512-byte Object offset through the text-section end, verified by SHA-256. Only the two preceding collection helpers change; their growth fits alignment padding. Accounted generated text rises8974084 to8974316B (+232); entry stack stays15840B. The selected helper is still emitted with redundant uniform threshold preparation inside its vector body; this is recorded rather than tuned in this round.

| Input | Before instructions | Current instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
|random-32769|84142074.5|79866946.0|-5.080845%|-11.720019%|
|repeated-32769|83863714.5|80025331.5|-4.576929%|-6.065213%|
|33027-periodic-window|89739923.5|85828379.5|-4.358756%|-3.701163%|
|reymont-prefix-32769|251210571.5|246663160.5|-1.810199%|-2.429653%|
|reymont-prefix-16384|139239335.5|138741997.5|-0.357182%|-0.428908%|
|16384-distribution-change|72659482.5|72258585.5|-0.551748%|-0.527387%|
|eight-literals|12705262.0|12541760.5|-1.286880%|-1.508222%|

Separate counter cohorts retain 1 adverse instruction median (empty,+14 instructions) and 3 adverse cycle medians (one-stored,seven-stored-prefix,320-overlap). All20 allocation-request, requested-byte, reallocation and incremental live-heap-peak metrics are exactly equal. No copied-volume, streaming, live-memory reduction or per-function retired-instruction partition is claimed. Allocation counts include3354 for eight literals,5201 random32KB,3135 repeated32KB and6449 natural32KB. Hardware counters are independent from primary elapsed and time-sampling cohorts.

Fresh exact-primary-binary CPU profiles cover five cases and16277 complete calls, with cleanup zero. Main-entry source attribution uses independently proven byte identity; changed helpers use their own symbols. Raw Timer-Fired/Running samples place random74.35%, distribution56.88%, repeated75.19% and natural82.60% inside the entry, but outside-entry also includes other generated helpers and useful host algorithms. Unknown native origins remain28.56–37.12% on the four larger profiles. These samples are not hardware instruction counts and do not justify calling all outside-entry time removable host overhead. Separate current-boundary callback counts are4466 eight literals,4303 random32KB,8074 distribution16KB,3248 repeated32KB and9741 natural32KB. Counts establish frequency, not elapsed cost.

Validation:3 focused tests cover all U8/U16/U32/U64 widths, zero/short/exact/tail lengths, current thresholds including above-lane maximum, live aliases, exact work, ordinary/optimized original/decoded direct/restored execution, selection refusals and six finite resource axes with success/failure/retry and prior publication preserved. Final-source GPU qualification passes629 tests with the same7 inherited exclusions; all212 CI-selected names execute and pass. Existing unchanged component qualification passes236 components/8 composition/1 real-capacity path. All12 balanced original/decoded Bool/U64/I64/F64/loop controls pass unchanged1.05 and allocation gates. All132 workflow tests,15 architecture/catalog checks, runtime-only, formatting and warnings-as-errors pass. Expected values and allowances are unchanged.

Exact commands/features are in unit-final/build.json, unit-final/test.json, gpu-compatible-final.json, qualification/*.json and checks/receipts.json. Production opt-level3, GPU tests opt-level1; aot,canonical-native,canonical-native-program,canonical-native-program-runtime,canonical-native-runtime,jit. Pinned nightly-2026-03-03/aarch64, mimalloc and frozen R145 Core/Engine/math/dependencies. Component qualification predates the required module relocation; that relocation preserves every moved source byte and the exact compressor Object. Final-source focused/GPU tests and all primary compression/counter/profile/scalar cohorts use the final layout.

Repaired introduced setup faults are retained: prototype test type/dereference errors before execution; a W32 vector-zero lane mismatch rejected by the backend verifier; the module file/directory layout collision, repaired by an exact-byte move followed by fresh builds; a missing end marker in the first diagnostic callback census, detected by three-call consistency and fixed only in the diagnostic harness. All60 earlier outputs remain archived; its contaminated counts are excluded. The vector disassembly parser initially omitted the numeric lane suffix from opcode matching; correcting it verifies the four actual UQSUB instructions. No primary timing observation is discarded or rerun for these faults. No unresolved introduced functional failure remains.

Skipped: the same7 inherited GPU exclusions listed in gpu-compatible-final.json, fresh full-dependency/all-feature/minimal rebuilds, an additional AOT harness, multi-megabyte inputs and the separate ten-workload suite. Ordinary hosted CI is cancelled by standing instruction; review automation is preserved. PR973 remains draft pending green hosted CI and clean exact-head review. There is no green-hosted-CI claim.

Parent 9035d3ab3c34e802a652f2154c8d19ba0fae5597; baseline measured compiler 3a3ae5dca4aa4eda77b928a9073110949295770d; current measured compiler 9a0fb83a20ae9ce12f4251d209c03b754dbc82da. Object b0b71e766e86b3632173fd2892f496b6f87e980feee51ba18d261e9e79a01a55; primary worker dd3c88ac5706db8a8a80785ee5f40d3ecc06acfd10aace78f100cf4351094ee9. Source 331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9; MBC a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7. IR/work/ABI/MBC192/107/39/14. Documentation/evidence are folded into this one optimization commit, with all measured compiler-file hashes unchanged.

Preserve algorithm/tables/search/EOF/tie/block rules and source allocation sites:262144-entry U16 tree,1500513-word U32 cache,305257-word U64 path capacity,depth300/nice258,output13 clear/reuse and input aliases. Logical work is source accounting, not hardware instructions. Mech performs compression itself; the separate R165 source variant remains excluded.

Every timing case was sealed immediately. Receipt-only replay verifies all numerical results and reconstructs this report without compilation or compression. Evidence excludes native binaries, bytecode, raw trace stores, copied assignments and private environment metadata. One optimization commit in PR973. Parity remains an active goal. The next compiler investigation follows the new brief: establish dynamic weight for mutable-parent fact loss and current child admission before extending the existing analysis. Minimum-reduction tuning is deferred; source model/frequency/replay counts form a separate investigation.
