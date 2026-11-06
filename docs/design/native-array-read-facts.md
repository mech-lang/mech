# R160: current immutable-array read facts

Native numeric lowering now consumes the maintained traversal analysis's current length, selected predicate and exact index identities. A source guard `at < length(view)` can justify that same view's one-based `at + 1` read after a checked addition succeeds or wrapping addition is proved nonwrapping. The successful read omits its duplicate emitted bounds branch and cold failure snapshot. Source get-or-zero predicates, geometry, actual loads, source arithmetic, source visits/prices and all unproved checks remain. Writes remain checked.

The numeric adapter rejects an entire local traversal containing writes, appends, unsupported actions or cold boundaries. Array-copy actions are admitted only by the existing equal-provenance rule. No fact crosses a latch; outside edges can enter only at the fresh header, and overwritten words and joins lose facts. Literal-case reasoning propagates truth only for an independently normalized 0/1 expression. Raw Bool/tag remainder paths remain possible. Original/decoded image construction independently rederives these facts; metadata is descriptive, never authority.

This is a general same-traversal window foundation, not whole-loop entry versioning, totality, settlement or SIMD. The source helper predicates still execute. There is no source, algorithm, allocation, capacity, allocator or timing-boundary change.

Source SHA256 331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9; bytecode SHA256 a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7; allocator SHA256 38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874. The real262144-entry U16 tree,1500513-word U32 cache,305257-entry U64 path capacity, depth300, nice258 and original EOF limits remain pinned.

Published base #986:8ef1693a558b9a46544ce425ca2f222e10e3dddc. Before compiler:09853c8157a7cc27448791543f976348a0158868. Compiler source:4fe3888f4d98d1ea159cd3186a8bc0f3b81ef773. Test-only parent:d19839958d3d34f77819e0e6f69087b47113758a. Semantic implementation:a3c72d8528d0a068423571017179b854f3266ba6, followed by the small-module split at4fe3888. Separate CI inventory commit:2b2927a511cf5cf87fbbca1a23fdab1b50c3f062. IR/work/ABI/MBC are182/107/39/14; older native images remain rejected.

The exact test-only parent passes both existing behavior/finite controls and fails the emitted-read-proof expectation with zero proofs. The candidate passes all three. The independent source witness uses forward and reverse reads, default-zero short/empty inputs, wrapping and zero-trip boundaries, strict tie selection, current nested/repeated activations, an arithmetic failure between reads, unchanged published outputs and live input aliases, recovery and zero cleanup. Original/decoded and direct/restored native executions agree with the ordinary path and an independent Rust cost oracle. Unit controls additionally reject another array, overwritten indices, a one-arm join, an earlier traversal and a raw Boolean remainder. Six finite-resource boundary dimensions retain original error prefixes and retry behavior.

The actual compressor Object has109 static scalar-read proofs, including both reads in each of the four path-loop clones. Dynamic eligible/executed counts were not instrumented. All36 earlier projection proofs remain. Source plan, complete numeric recipes and source prices equal the before image. Source-location instrumentation independently rebuilds a whole Object byte-identical to the actual primary Object. At the profiled clone's two reads, source-attributed emitted bytes fall456 to8 and444 to4. Those byte totals include cold source-failure machinery; they are not executed-instruction counts. ARM64 disassembly retains the original source predicates and U32/U64 loads while removing their second comparisons and failure paths.

The predeclared final20-case cohort retains3,968 primary observations (4,048 including warmups). Fourteen medians improve and six worsen. No observations are trimmed, and the separate four-case development screen is never pooled into the final cohort.

| Case | Before ms | After ms | Before / Roc | After / Roc | Change |
|---|---:|---:|---:|---:|---:|
|eight-literals|1.385875|1.382605|4.854203x|4.842748x|-0.235988%|
|16384-distribution-change|5.998979|5.993667|7.584044x|7.577329x|-0.088548%|
|33027-periodic-window|8.935249|8.565563|7.131085x|6.836043x|-4.137400%|
|random-32769|6.735771|6.683896|6.396743x|6.347479x|-0.770142%|
|repeated-32769|8.548792|8.132291|6.422834x|6.109911x|-4.872034%|
|reymont-prefix-32769|28.177063|27.107271|6.470792x|6.225117x|-3.796679%|
|reymont-prefix-16384|15.451791|14.870583|7.256066x|6.983134x|-3.761428%|
|random-1024|1.250313|1.259479|4.126444x|4.156696x|+0.733137%|
|repeated-1024|1.533708|1.520728|4.763068x|4.722759x|-0.846282%|
|reymont-prefix-1024|1.819020|1.794312|5.429912x|5.356157x|-1.358313%|
|320-overlap|1.345584|1.336042|4.397332x|4.366149x|-0.709135%|
|random-256|1.319063|1.318688|4.517337x|4.516053x|-0.028429%|
|128-biased-dynamic|1.140792|1.151625|3.988781x|4.026661x|+0.949648%|
|random-64|1.260833|1.257396|4.362746x|4.350851x|-0.272637%|
|32-high-entropy-stored|1.237791|1.241105|4.327942x|4.339526x|+0.267654%|
|eight-overlap|1.311125|1.309104|4.459609x|4.452735x|-0.154142%|
|eight-literals-prefix|1.349000|1.349355|4.684028x|4.685259x|+0.026279%|
|seven-stored-prefix|0.120313|0.120833|below resolution|below resolution|+0.433039%|
|one-stored|0.119896|0.120292|below resolution|below resolution|+0.330705%|
|empty|0.116375|0.116229|below resolution|below resolution|-0.125456%|

Geometric after/before is0.990643204157 (-0.935680%); worst+0.949648%. All unchanged1.05 gates pass. Each pair uses its own fresh reference denominator. Historical absolute times and ratios are not a matched comparison; do not compound independent rounds. Eight bytes changes-0.235988% in this matched cohort.

All20 exact raw-byte, independent-inflate, logical-work, input-alias, output-prefix/capacity/clear-reuse and zero-final-managed-storage checks pass. All20 heap requests, requested bytes, reallocations and incremental peak live usable bytes are equal. RSS/physical/lifetime peaks remain separately archived and include image/harness residency; requested bytes do not measure copied or resident bytes.

| Separate CPU cohort | Before instructions | After instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
|empty|1072205.5|1070887.5|-0.122924%|-0.749593%|
|one-stored|1124937.5|1123615.5|-0.117518%|-1.438471%|
|seven-stored-prefix|1124612.5|1123380.0|-0.109593%|-1.141284%|
|eight-literals|12701240.5|12684831.0|-0.129196%|-4.021698%|
|eight-overlap|11892831.5|11876074.0|-0.140904%|-0.534695%|
|320-overlap|12607653.0|12559139.0|-0.384798%|-5.255181%|
|eight-literals-prefix|12702105.0|12686257.5|-0.124763%|-0.012522%|
|128-biased-dynamic|10679886.0|10662553.0|-0.162296%|-3.769782%|
|32-high-entropy-stored|11289734.0|11274496.0|-0.134972%|-1.221754%|
|16384-distribution-change|76726754.0|76086765.0|-0.834115%|-1.531632%|
|33027-periodic-window|155404774.0|152597876.5|-1.806185%|-1.172479%|
|random-64|11468099.0|11452255.0|-0.138157%|-2.463719%|
|random-256|12119282.5|12103840.0|-0.127421%|-1.884131%|
|random-1024|12576110.5|12558615.0|-0.139117%|-3.120600%|
|repeated-1024|16074297.5|15954019.5|-0.748263%|-2.640986%|
|random-32769|90533595.5|90168180.0|-0.403624%|+0.083776%|
|repeated-32769|149141797.5|146355166.0|-1.868444%|-1.289883%|
|reymont-prefix-1024|18572918.0|18296598.5|-1.487755%|-4.378406%|
|reymont-prefix-16384|177391768.0|169983664.5|-4.176126%|-5.376796%|
|reymont-prefix-32769|323530562.0|309945288.0|-4.199070%|-5.258421%|

Primary elapsed, CPU counters, heap counters and CPU-time sampling are separate cohorts. Contrary results stay in these tables. No sample percentage is multiplied by retired instructions, and there is no per-function retired-instruction attribution claim. Twelve balanced original/decoded scalar controls retain the unchanged1.05 gates, pure-scalar allocations/bytes and zero loop allocation slope.

| Current executable | Generated code exclusive | Host callbacks/cleanup | Outside/unrecognized |
|---|---:|---:|---:|
|eight-literals|11.09%|78.53%|10.38%|
|reymont-prefix-32769|86.16%|12.71%|1.13%|
|repeated-32769|84.99%|12.62%|2.39%|
|33027-periodic-window|82.24%|15.50%|2.26%|

Generated text falls9,110,288 to9,024,768 bytes (-85,520); entry stack falls15,872 to15,808 bytes (-64). Final image restoration is37.577111541/37.683947125 seconds outside compression timing, descriptive observations rather than a matched startup gain. Main Object generation is135.417 seconds outside timing; build/qualification overlap is recorded and no matched compilation-speed claim is made.

Passing current-checkout receipts:595 GPU tests with the same seven inherited exclusions;15 focused;174 CI-selected tests inventoried and actually executed;236 unchanged components, eight composition and one real-capacity path;132 workflow tests;15 architecture/catalog checks; runtime-only, formatting and warnings as errors. GPU opt3 and unit/root qualification opt1 use nightly-2026-03-03 aarch64 with frozen R145 Core/Engine/math dependencies. CI-selected tests execute in the retained aot/jit/canonical-native feature superset, not a freshly rebuilt minimal feature graph. Exact commands/features are in unit-spec.json, library-spec.json, checks/receipts.json and qualification/*.json.

An inherited harness defect was corrected: the first runtime-only metadata build still selected an older checkout, and architecture commands had stale manifest environment values. Its original receipts are retained in checks-inherited-paths. The corrected current-source runtime-only build and all checks pass, and the component harness's manifest resolution is also corrected. Primary worker/Object pins were always current and unaffected. The first local disassembly request spanned distant cold origins and produced a needlessly large dump; it was replaced with bounded contiguous excerpts, retaining all exact source spans in JSON. No measurement was affected. No functional failure remains. Seven inherited GPU exclusions and unrun fresh full-math/all-feature/minimal graphs stay explicit. Actual restored linked compression passes all20 cases; no additional standalone linked-suite claim is made. Hosted CI is cancelled under instruction, with no green claim; the PR stays draft.

Decision: retain this current-view read foundation under the unchanged gates, with six adverse small-case medians explicitly preserved. Next establish the complete current child read window, remove the remaining masked narrowing/control checks using maintained unsigned semantics, prove whole-body totality, and settle exact source visits against the existing prepaid loan. Ordered reduction/unrolling/SIMD and a separate pinned source variant remain unfinished. Parity is not achieved.

Numerical replay: checkpoint.py INPUT OUTPUT reconstructs results.json and REPORT.md from hash-verified receipts without compiling, compressing or changing a worktree. --write-doc is optional and explicit. Evidence assets exclude native binaries, bytecode and raw trace containers.
