# R157: current-traversal projection facts

The numeric lowering path now reuses the maintained packed traversal fact analysis to omit an aggregate payload guard only when the same current tag was proved on the selected path. The source tag selection remains. Unknown or overwritten values, unmatched joins, cold entry, unsupported mutation and analysis caps keep the checked emitter; facts restart at each source latch. Image reconstruction derives the proof independently. The shared frame move recipe preserves sequential Pack/Project and parallel Copy, including overlapping word homes. No compressor source change or additional host-to-native coverage is claimed.

Published base: ede060a777109403306b411446687b41a44f7ffe (#984). Reviewed comparison compiler: 58690b58dfc4ccdf44b074c15d3c140a28603866. New compiler: 78d6a87f8cb0c61c434ea73d6e8bb17e4d2cb679, IR/work/ABI/MBC 180/107/39/14. R156 (637fe885bb919d7afc4fa18fc7d74eb5f9ae8207) is the intermediate control, not the final timing baseline. The exact test-only parent 549c2265870964d9fc8e3d17bcc93dd5b8759937 passes behavior controls and fails the new guard-elision assertion at zero.

The full compressor omits 36 static guards in four source clones. Identity-preserving diagnostic source mapping proves that the current primary Object retains the initial case selection at site15535 and removes three ARM64 instructions at each payload projection15537/15538/15539: nine actual instructions at those three profiled sites. This is static evidence, not an executed-visit or per-function instruction count. The source plan, complete numeric recipes and source prices remain equal. The earlier R155 study attributes 23.35% of natural32KB CPU-time weights to this path-cost loop; the pinned reference uses an 11-instruction scalar candidate loop there, without SIMD.

The frame-order control restores the original emission ordering. R156 and R157 generated instruction sections are byte-identical (9,060,104 bytes; SHA256 99d1f5fcb61e1a4bc3c7b2db8aee2ca6b4a238b136f556eaf580a9a171d1925b). Whole linked executable text differs because it also contains host/runtime code. The control therefore does not explain the adverse periodic/repeated instruction counts, and no extra native gain is attributed to it. Its four-case development cohort stays separate.

The direct R154-to-R157 final cohort has 3,968 primary observations (4,048 with warmups), 14 faster and six slower cases. The geometric mean after/before is 0.9821099871936066 (-1.789001%). The worst case is +3.609041%, inside the unchanged 1.05 per-case gate. All 20 exact-byte, independent-inflate, logical-work, input-alias, output-prefix/capacity/reuse and zero-final-managed-storage controls pass. Algorithms, tables, depth300, nice258, EOF limits, source allocation sites, real scratch capacities, allocator and evaluate_into timing stay pinned.

| Case | Before ms | After ms | Before / reference | After / reference | Change |
|---|---:|---:|---:|---:|---:|
|eight-literals|1.337896|1.331813|4.865075x|4.842955x|-0.454669%|
|16384-distribution-change|6.144501|5.966480|7.768016x|7.542958x|-2.897241%|
|33027-periodic-window|8.726166|8.639687|6.986522x|6.917283x|-0.991037%|
|random-32769|6.700271|6.730084|6.417884x|6.446440x|+0.444945%|
|repeated-32769|8.293667|8.212854|6.249937x|6.189038x|-0.974388%|
|reymont-prefix-32769|33.539166|29.256583|7.719026x|6.733391x|-12.768903%|
|reymont-prefix-16384|18.240146|16.006584|8.579560x|7.528967x|-12.245308%|
|random-1024|1.235000|1.238500|4.130435x|4.142140x|+0.283401%|
|repeated-1024|1.527271|1.513125|4.743078x|4.699146x|-0.926227%|
|reymont-prefix-1024|1.901396|1.825458|5.796939x|5.565421x|-3.993802%|
|320-overlap|1.332917|1.330625|4.533730x|4.525935x|-0.171916%|
|random-256|1.303104|1.300437|4.372832x|4.363883x|-0.204665%|
|128-biased-dynamic|1.130042|1.139354|3.965058x|3.997733x|+0.824085%|
|random-64|1.409520|1.362542|4.794287x|4.634497x|-3.332942%|
|32-high-entropy-stored|1.305167|1.352270|4.686415x|4.855549x|+3.609041%|
|eight-overlap|1.545479|1.580729|5.256731x|5.376629x|+2.280846%|
|eight-literals-prefix|1.364625|1.367125|4.805018x|4.813820x|+0.183201%|
|seven-stored-prefix|0.121291|0.120416|below resolution|below resolution|-0.721403%|
|one-stored|0.122834|0.121813|below resolution|below resolution|-0.831610%|
|empty|0.121417|0.119958|below resolution|below resolution|-1.201644%|

Reference multiples use this cohort's measured reference denominator, never a historical denominator. Eight bytes improves 1.3378955 to1.3318125ms, or4.865075x to4.842955x against the fresh0.275ms reference; its higher multiple than earlier reports does not represent a matched Mech slowdown.

The separate CPU and heap cohorts are not pooled with primary elapsed or multiplied by sampling weights. All 20 allocation requests, requested bytes, reallocations and incremental peak live usable heap are exactly equal. RSS and lifetime physical peaks include restoration, allocator residency and harness state; they are not incremental compression live memory.

| Separate CPU cohort | Before instructions | After instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
|empty|1060342.0|1070860.5|+0.991991%|-0.425468%|
|one-stored|1113092.0|1123596.5|+0.943723%|-0.202969%|
|seven-stored-prefix|1112953.0|1123473.0|+0.945233%|+0.048162%|
|eight-literals|12700683.5|12697344.5|-0.026290%|-0.646223%|
|eight-overlap|11893014.5|11889310.5|-0.031144%|-2.152284%|
|320-overlap|12595809.0|12601459.0|+0.044856%|+0.525643%|
|eight-literals-prefix|12702116.5|12699596.5|-0.019839%|+22.345841%|
|128-biased-dynamic|10678287.0|10676805.0|-0.013879%|+18.193847%|
|32-high-entropy-stored|11288355.5|11285680.5|-0.023697%|-7.201100%|
|16384-distribution-change|77371637.0|76960685.0|-0.531140%|+0.756405%|
|33027-periodic-window|152449317.5|155366767.5|+1.913718%|-0.883948%|
|random-64|11465078.5|11463365.0|-0.014945%|+0.299565%|
|random-256|12110659.5|12113680.5|+0.024945%|+0.391835%|
|random-1024|12553399.5|12571076.0|+0.140810%|-1.905206%|
|repeated-1024|16008812.0|16070112.0|+0.382914%|-1.911401%|
|random-32769|90196013.0|90851513.5|+0.726751%|+1.559198%|
|repeated-32769|146206434.0|149114010.5|+1.988679%|-1.775335%|
|reymont-prefix-1024|19000050.5|18706951.5|-1.542622%|-4.241676%|
|reymont-prefix-16384|190558611.5|180260826.0|-5.403999%|-12.840672%|
|reymont-prefix-32769|348436144.5|328349762.0|-5.764724%|-12.958267%|

Natural32KB improves12.77% elapsed and5.76% instructions; periodic/repeated instructions increase1.91%/1.99%, despite their lower primary elapsed. Prefix-eight and biased128 have adverse cycle cohorts of22.35%/18.19% despite almost unchanged instructions. Their cause is unestablished; these observations are retained. No unmeasured clock/cache or register-allocation explanation is claimed.

All12 balanced scalar original/decoded controls pass unchanged1.05 gates, equal pure-scalar requests/bytes and zero loop allocation slope. After/before elapsed ratios range0.990291 to0.994899. Current executable CPU-time samples (not instruction partitions):

| Case | Generated code exclusive | Host callbacks and cleanup | Outside/unrecognized |
|---|---:|---:|---:|
|eight-literals|13.73%|77.30%|8.97%|
|reymont-prefix-32769|87.34%|11.60%|1.05%|
|repeated-32769|85.15%|13.13%|1.72%|
|33027-periodic-window|80.20%|17.02%|2.78%|

Generated text grows9,045,788 to9,059,856 bytes (+14,068); entry stack falls15,984 to15,952 bytes. Final primary restoration takes37.559099417 /37.502983666 seconds, outside compression timing: a descriptive pair, not a matched startup improvement claim. Initial full Object emission takes133.175533 seconds while qualification overlaps, so no matched compile-time claim is made.

Passing local checks:589 GPU tests with the same seven inherited exclusions; eight focused tests;168 native CI cases inventoried and actually executed; unchanged236 components, eight composition tests, one real-capacity path;132 workflow Python tests;15 architecture/catalog checks; runtime-only metadata, format and warnings as errors. GPU opt3 and unit/component opt1 use nightly-2026-03-03 aarch64 with frozen R145 Core/Engine/math dependencies. The exact commands, features and outcomes are retained in unit/build.json, unit/test.json, gpu-compatible.json, checks/receipts.json, qualification/*.json and scalar-balanced/plan.json. Some copied descriptive test_revision/manifest-directory fields still name R154; actual commands run in the R157 worktree with the newly built R157 test binary. Raw receipts are preserved; qualification-provenance.json clarifies this metadata discrepancy.

No functional failure remains. R156's initial test/harness mistakes and strict identity rejections are retained as corrected introduced failures. R157's first profile attempt was denied permission to invoke ps before any worker or measurement started; the identical diagnostic succeeded with permission. Seven inherited GPU exclusions and unrun fresh full-math/all-feature/minimal graphs remain explicit. Actual restored linked compression runs pass; no additional standalone linked-suite run is claimed. Hosted CI is intentionally cancelled and no green claim is made. The PR remains draft for external review.

Decision: retain this bounded proof-sharing change with the original frame order and the adverse observations above. Historical R156 candidate receipts remain immutable. Next: current array/version/geometry, induction and readable-window facts for the same bounded read-only loop, then separately totality/exact settlement and ordered reduction/unrolling or SIMD. Its funding is already prepaid; do not add a second loan. A separately pinned source experiment follows compiler work. These parts of the brief remain unfinished; parity is not achieved.

Numerical replay: python3 checkpoint.py reconstructs results.json and REPORT.md from the extracted immutable receipts without compiling, compressing or writing a worktree. --write-doc is an explicit optional destination. External archive hashes and source pins are in benchmarks/mech-deflate/experiments.json; no native binaries or bytecode are uploaded.
