# R170: vector minima with directly selected rotations

Retain the corrected compiler-only change. Natural32KB improves1.879497% (4.430760x to4.347484x Roc), and natural16KB1.476092% (5.165119x to5.088877x). All20 unchanged1.05 gates pass:11 faster/9 slower medians,0.226285% geometric elapsed improvement. Repeated32KB rises0.477787%, periodic33KB0.172251% and eight literals0.264176%. This is a modest targeted gain with explicit costs. The separate histogram source variant is excluded.

| Input | Before ms | Current ms | Before / Roc | Current / Roc | Change |
|---|---:|---:|---:|---:|---:|
|reymont-prefix-32769|19.295959|18.933291|4.430760x|4.347484x|-1.879497%|
|reymont-prefix-16384|10.986208|10.824041|5.165119x|5.088877x|-1.476092%|
|repeated-32769|5.110541|5.134958|3.833864x|3.852182x|+0.477787%|
|33027-periodic-window|5.527687|5.537209|4.408044x|4.415637x|+0.172251%|
|eight-literals|1.356292|1.359875|4.693052x|4.705450x|+0.264176%|
|empty|0.116500|0.116584|below resolution|below resolution|+0.072103%|
|one-stored|0.120000|0.120250|below resolution|below resolution|+0.208333%|
|seven-stored-prefix|0.119917|0.120208|below resolution|below resolution|+0.243085%|
|eight-overlap|1.316479|1.315521|4.635489x|4.632116x|-0.072770%|
|320-overlap|1.308833|1.311022|4.277232x|4.284384x|+0.167210%|
|eight-literals-prefix|1.357563|1.357000|4.730183x|4.728223x|-0.041435%|
|128-biased-dynamic|1.151896|1.144187|3.978914x|3.952288x|-0.669158%|
|32-high-entropy-stored|1.239958|1.241208|4.358376x|4.362770x|+0.100810%|
|16384-distribution-change|5.648376|5.644917|7.140803x|7.136431x|-0.061230%|
|random-64|1.266042|1.275709|4.335759x|4.368865x|+0.763561%|
|random-256|1.318062|1.314792|4.498507x|4.487345x|-0.248129%|
|random-1024|1.252750|1.242687|3.989650x|3.957602x|-0.803273%|
|repeated-1024|1.423000|1.415875|4.365031x|4.343175x|-0.500703%|
|random-32769|6.432167|6.404354|6.108420x|6.082008x|-0.432397%|
|reymont-prefix-1024|1.660751|1.647958|5.032577x|4.993812x|-0.770284%|

Each source variant executes3968 primary/4048 total complete calls over the same20 inputs/sizes. Exact reference streams, independent inflate, aliases, prefixes, capacities, source work and final managed cleanup zero pass. Three tiny stored-only reference medians are zero; no ratio is invented. Largest input33027B. Reference denominators are observed separately; cohorts are not pooled and gains are not compounded.

The existing source-derived complete-child proof, current immutable read windows and exact price settlement remain unchanged. All descriptor fields other than code footprint, stack footprint and identity are exactly equal to R169. The patch changes only the result selection of an already-qualified four-lane U32 minimum. Broadcast the unsigned cost minimum, mask out nonwinning indices, then reduce the matching indices. A lane records an index only on strict improvement, preserving incoming-best ties. Reducing actual indices preserves the earliest global choice across groups even when it occupies a higher lane. Scalar remainders, the16-candidate threshold, cost/item expressions, source operations, finite charges, failure order and final publication remain on existing paths. No benchmark names, new callback, authority cache, ABI or unsafe boundary is introduced.

The first version is rejected. Experimental sourcea94f971223d8a38a520b8239d222cd86c5be4fca/IR190 passes3 focused tests and all20 thresholds, but geometric elapsed rises0.400464%; natural32KB+0.457466%, repeated32KB+1.200034%. Twelve instruction medians rise. Its complete favorable/adverse evidence is preserved in results.json and the first-variant report. R169 remains the comparison baseline for both sources.

Fresh disassembly exposed a specific lowering problem: the same-input rotation mask8..15,0..7 selected a literal-mask load, vector copy and TBL. Equivalent concatenated-input indices8..23 select one EXT. A minimal independent Object demonstrates both forms; the real corrected compressor confirms two EXT rotations. This uses the existing portable vector vocabulary, without a new global shuffle pass or architecture-specific inline assembly. A separate recorded C/NEON helper uses two horizontal unsigned-minimum instructions; it starts from available cost/index vectors and supplies a bounded instruction comparison, not compressor timing. The pinned reference's previously inspected cost kernel is scalar; no reference SIMD claim is made.

| Input | Before instructions | Current instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
|eight-literals|12712107.5|12705523.0|-0.051797%|-0.726746%|
|repeated-32769|83157723.0|83864708.5|+0.850174%|+5.442572%|
|33027-periodic-window|89033545.5|89739214.0|+0.792587%|-0.972604%|
|reymont-prefix-16384|138164827.5|139265354.0|+0.796532%|-1.649201%|
|reymont-prefix-32769|249227296.5|251240786.5|+0.807893%|-1.523886%|

Separate counter cohorts retain11 adverse instruction medians and8 adverse cycle medians. All20 allocation-request, requested-byte, reallocation and incremental live-heap-peak metrics remain equal. No streaming/copied-byte saving is claimed. First-version text/stack growth is53,248B/976B. Corrected growth is53088B/112B relative to R169; the entry stack is15728 to15840B. Correcting the masks removes864B of the first version's frame growth. Register pressure and layout have not been isolated as the sole cause of elapsed changes. Compilation, image restoration, diagnostic probes and the C helper stay outside evaluate_into.

Three exact-binary CPU-time profiles per variant cover eight literals, repeated32KB and natural32KB: 34382 rejected-version and34410 corrected-version complete calls, with cleanup zero. Each source-marker build is whole-Object identical to its own measured binary. Fresh source maps, current hot-PC windows, unknown native origins and outside-entry samples remain explicit. Outside-entry includes generated helpers and host/harness code; CPU-time samples are not a retired-instruction partition or an exact host/native split.

Corrected-source validation:3 focused ordered-reduction tests;626 compatible GPU tests with the same7 inherited exclusions; all209 selected CI names execute and pass in that suite;236 unchanged components/8 composition/1 real-capacity path;12 balanced original/decoded scalar controls under unchanged1.05 and allocation gates;132 workflow tests;15 architecture/catalog checks, runtime-only, formatting and warnings denied. Existing controls cover zero/short/max/remainder counts, nonzero/exhausted starts, U32 wrap, cross-lane/group and incoming ties, original/decoded and ordinary/optimized direct/restored execution, live aliases, finite boundaries, publication, error/retry and recipe refusals. No expected value or allowance increases.

Exact commands/features: unit-spec.json, unit/build.json, gpu-compatible-final-spec.json, qualification/*.json and checks/receipts.json. Production opt-level3, GPU tests opt-level1; aot,canonical-native,canonical-native-program,canonical-native-program-runtime,canonical-native-runtime,jit. Pinned nightly-2026-03-03/aarch64, mimalloc and frozen R145 Core/Engine/math/dependencies. The qualification shim is byte/hash-checked against the retained parent; fixture programs are unchanged. No dependency/math rebuild or new production unsafe.

Repaired setup faults: an orchestration expression failed parsing before execution; rustfmt corrected mask-array wrapping before the corrected build. The new qualification directory initially lacked its unchanged fixture shim and frozen-library symlink. The failed build receipt is saved. Scalar setup detected the same missing dependency, then its retry found the already-created baseline hardlink; it now verifies and reuses that hash. A replay validator initially looked for the shared C receipt in its own directory; it now reads the original parent receipt. An invalid duplicate-path patch request was corrected before any write. All required checks rerun without compiler changes, discarded observations or larger allowances. These are introduced harness/setup faults. No unresolved introduced functional failure remains.

Skipped: the same7 inherited GPU exclusions listed in gpu-compatible-final-spec.json, fresh full-dependency/all-feature/minimal rebuilds, an additional AOT harness, multi-megabyte inputs and the separate ten-workload suite. Ordinary hosted CI is cancelled by standing instruction, review automation preserved. PR973 remains draft pending green hosted CI and clean exact-head review; no green-hosted-CI claim is made.

Parent 86dc8106cfa571b56c5a0ae0485cef1f68c49ea9; baseline compiler fdb1c5c4e738ad3bb69233ef92448efcf075aa81; corrected measured compiler 3a3ae5dca4aa4eda77b928a9073110949295770d. Object 03e6b4e83977e9bfc17c15d93a7de8d88f5394356b8f03043b9f5ac6f5984e03; worker 9bc2c2be5c2406bff4fa3cc914b1d629c5ea2c14ce355b2f7e8e49e81dcc3d44. Source 331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9 and MBC a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7. Final docs/evidence fold into the same optimization commit with compiler-file hashes unchanged. Retained IR/work/ABI/MBC191/107/39/14; first-version IR190 stays experimental. Source patches/pins reproduce both variants.

Preserve complete evaluate_into, the original algorithm/tables/search/EOF/tie/block rules and source allocation sites:262144-entry U16 tree,1500513-word U32 cache,305257-word U64 path capacity, depth300/nice258, output13 clear/reuse and input aliases. Logical work is source accounting, not hardware instructions. Mech performs compression itself.

Each timing case was sealed immediately. The second cohort measures a changed compiler source after a demonstrated instruction-selection issue, not identical binaries rerun until passing. Public receipt-only replay reconstructs both variants and this report without compiling or compressing. Exclude binaries, bytecode, raw trace stores, copied assignments and private environment metadata. One optimization commit in PR973. Parity is not achieved; word-loop funding and vector cost-input shuffles remain candidates, not established savings.
