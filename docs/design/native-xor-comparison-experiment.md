# R167: comparison experiment and nested phi diagnosis

No compiler change is retained. The universal integer XOR-zero comparison experiment preserves a natural-text gain in its receipts, but repeated and periodic inputs regress in every matched block. Its all-case elapsed improvement is only 0.095938%. The retained R164 compiler and separate R165 source variant remain unchanged; this round uses the original pinned source throughout.

| Input | Before ms | Candidate ms | Before / Roc | Candidate / Roc | Change |
|---|---:|---:|---:|---:|---:|
|reymont-prefix-32769|19.960083|19.651625|4.585363x|4.514501x|-1.545377%|
|eight-literals|1.364291|1.353958|4.753629x|4.717625x|-0.757389%|
|33027-periodic-window|8.208105|8.415624|6.529916x|6.695008x|+2.528233%|
|random-32769|6.652271|6.635459|6.335496x|6.319484x|-0.252726%|
|repeated-32769|7.644709|7.848125|5.743583x|5.896412x|+2.660880%|
|16384-distribution-change|5.779021|5.707812|7.296744x|7.206834x|-1.232190%|
|reymont-prefix-16384|11.253042|11.039605|5.284359x|5.184130x|-1.896709%|
|random-1024|1.253063|1.248354|4.094977x|4.079590x|-0.375759%|
|repeated-1024|1.484958|1.499396|4.569103x|4.613526x|+0.972249%|
|reymont-prefix-1024|1.672083|1.661083|5.082319x|5.048884x|-0.657862%|
|320-overlap|1.321417|1.321896|4.464247x|4.465865x|+0.036249%|
|random-256|1.313583|1.319250|4.400613x|4.419598x|+0.431415%|
|128-biased-dynamic|1.145042|1.142375|3.989693x|3.980401x|-0.232917%|
|random-64|1.267854|1.267708|4.327147x|4.326650x|-0.011476%|
|32-high-entropy-stored|1.240875|1.244792|4.308594x|4.322193x|+0.315624%|
|eight-overlap|1.312208|1.311750|4.620451x|4.618836x|-0.034941%|
|eight-literals-prefix|1.360625|1.356146|4.691810x|4.676364x|-0.329224%|
|seven-stored-prefix|0.120417|0.119812|below resolution|below resolution|-0.502006%|
|one-stored|0.120417|0.119958|below resolution|below resolution|-0.380760%|
|empty|0.116958|0.116333|below resolution|below resolution|-0.534378%|

The final matrix has3968 primary/4048 total complete calls:14 faster/6 slower medians, all unchanged1.05 gates pass. The separate five-case development screen has512 primary/532 total calls and is not pooled. Every stream agrees with the reference, independently inflates, preserves source work, aliases, prefixes and real capacities, and closes with zero final managed bytes/nodes. No multi-megabyte timing was attempted.

The first rewrite ran before existing phi/alias cleanup and produced byte-identical text; it was stopped before timing. Moving the generic integer Equal/NotEqual rewrite after cleanup produces real direct comparisons and leaves XOR to mismatch localization. There are18 explicitly verified compare/conditional-branch/XOR/rbit sequences in the actual candidate Object. These are static sites, not visits. The reference pattern is retained binary evidence from R166; all20 reference elapsed cohorts here are fresh. Canonical source plans, complete numeric recipes and prices remain exactly equal.

Generated text grows51148B (8903912 to8955060); entry stack changes-144B (16480 to16336). The source-only histogram experiment is excluded. Source331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9 and MBCa2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7, the262144-entry U16 tree,1500513-word U32 cache,305257-word U64 paths, depth300/nice258, EOF rules, source allocation sites, output reservation/clear/reuse, input aliases, pinned allocator and evaluate_into remain fixed.

| Separate CPU cohort | Instructions before | Candidate | Change | Cycles change |
|---|---:|---:|---:|---:|
|repeated-32769|141120672.0|139756833.5|-0.966434%|+0.151868%|
|33027-periodic-window|147338042.0|145970065.5|-0.928461%|+0.243649%|
|reymont-prefix-32769|259889330.0|259919176.5|+0.011484%|-2.513643%|
|reymont-prefix-16384|142534984.0|142733001.0|+0.138925%|-1.221086%|
|eight-literals|12672867.5|12677550.0|+0.036949%|+3.325769%|

Across all20 cases,13 instruction medians and14 cycle medians rise. All20 allocation requests, requested bytes, reallocations and incremental live-heap peaks are equal. These are separate counter and heap cohorts; their elapsed fields are not primary timings. No per-function instruction partition, total-copy volume or input-streaming claim is made.

A fresh current-binary profile executes4462 complete repeated-input calls. It has5578 running timer PCs, of which4580 (82.108283%) lie in the exact current main entry extent. Outside samples include other generated helpers and host/harness code. The hottest actual windows still contain visit/balance work, access-helper conditions and stack accesses. No old source-address map is applied to this new Object, and CPU samples do not partition retired instructions.

The separate retained-binary diagnostic is byte-identical to the original R164 Object. It maps the word-loop's78 header arguments:26 visit counters,20 canonical source scalars,13 outer collector scalar fields,3 loan words,3 nested scalar words,1 first-visit flag and12 unresolved homes. An independent CFG reconstruction reaches all280448 blocks/405068 edges and derives74 local phi chains with one dominating outside phi value. The other four carry the selected first-visit flag, changing compute balance, advancing length and this child's visit counter. This is an IR optimization candidate, not a claim that74 machine copies are saved or that it exactly explains R166's222 stack accesses.

The standalone minimal reproducer keeps16 parent fields unchanged within a child while its outer phi changes between child activations. The existing global constant-phi pass and final backend retain all18 child parameters. Six actual JIT executions cover zero outer/inner visits, changed arguments and wrapping values with exact answers. The next bounded experiment is generic dominance-scoped phi simplification, retaining genuinely changing state, all source arithmetic and funding. Multiple outside values, changed inner updates, wrong dominance/alternate entries and current activation changes need explicit acceptance controls before the implementation expands.

Focused prototype validation:3 GPU unit tests pass with warnings denied; all20 restored complete-stream cases pass. The minimal phi reproducer builds with warnings denied and passes6 executions. No new full GPU, scalar latency, component, architecture or hosted CI qualification is claimed for the rejected prototype. The maintained R164/R165 receipts remain historical; the restored CI inventory remains198. Ordinary hosted CI is cancelled by standing instruction, and #973 stays draft pending green CI and exact-head review.

Introduced development failures: an old descriptor path in the new harness is rejected by strict image identity before any compressor call, then corrected and relinked. The current timed worker is pinned separately; the original artifact-comparison worker checksum is explicitly retained as the rejected harness checksum. Early diagnostic parsers first included a comment value and then omitted typed brif edges; both were corrected before any complete-graph conclusion. These are repaired diagnostic/harness errors. The candidate's measured repetitive-input regressions remain preserved as the reason for rejection. No introduced production failure remains after restoration.

Basead32f9a1ff04ba1934ad2bc95217e369ea167f40; local candidateca4c0d37b8682b41365b6527f6ac13e5a967bdbb; current Object5b9f7221f703a8cbfd235924370aa5d2e4cc6448c205cbeee288d7437f9605c3; timed worker7d68901c7ad13b07831d1b5906c6a7e70bb25108ec572bf47962f672a710fba2. The complete candidate patch and hashes are archived, so the local trial need not remain in published history. Candidate IR186/work107/ABI39/MBC14 is not retained; production remains184/107/39/14 with frozen R145 Core/Engine/math/dependencies. Both production sources and CI edits are restored byte-for-byte to the base. Parity is not achieved.

Replay round167 with the maintained experiment.py replay-round command and experiments.json index. Replay reads committed/public receipts without compiling or compressing.
