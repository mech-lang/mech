# R156 candidate checkpoint

The existing packed traversal analysis now also supplies current-site aggregate projection facts to numeric lowering. The exact full compressor elides 36 projection guards in four source clones, including the three profiled payload projections. All canonical plans, numeric recipes, source prices, source and bytecode pins are unchanged. IR/work/ABI/MBC are 179/107/39/14. This is a candidate, not a retained or reviewed result.

The corrected small test needs a normal recursive array-carrying parent to enter numeric lowering. The exact test-only parent at 549c2265870964d9fc8e3d17bcc93dd5b8759937 passes both behavior controls and fails the guard-elision expectation at zero. Candidate focused tests cover original/decoded programs, direct/restored images, current arguments and visits, failures and retries, unchanged publication/aliases and finite boundaries. Existing malformed-tag, join, overwrite and analysis-cap controls moved into the shared analysis unchanged.

The predeclared final 20-case cohort retains 3,968 primary observations. Seven cases improve and 13 worsen. The geometric mean ratio is 0.9906666199069278; the largest adverse ratio is 1.0173052791959278. All fixed 1.05 case gates pass. The development screen is retained separately and never pooled.

| Case | Before ms | After ms | Before / reference | After / reference | Change |
|---|---:|---:|---:|---:|---:|
|eight-literals|1.343354|1.359376|4.584826x|4.639507x|+1.192649%|
|16384-distribution-change|6.201229|6.039708|7.790489x|7.587573x|-2.604661%|
|33027-periodic-window|8.795250|8.665875|7.047476x|6.943810x|-1.470964%|
|random-32769|6.688625|6.795166|6.364059x|6.465430x|+1.592868%|
|repeated-32769|8.310500|8.260208|6.248496x|6.210683x|-0.605162%|
|reymont-prefix-32769|33.544854|29.272500|7.704376x|6.723128x|-12.736243%|
|reymont-prefix-16384|18.289375|16.058000|8.614873x|7.563825x|-12.200390%|
|random-1024|1.234354|1.252916|4.114513x|4.176388x|+1.503823%|
|repeated-1024|1.528604|1.536458|4.688969x|4.713063x|+0.513835%|
|reymont-prefix-1024|1.889125|1.849812|5.639179x|5.521828x|-2.080990%|
|320-overlap|1.329354|1.347729|4.401834x|4.462679x|+1.382250%|
|random-256|1.302480|1.317917|4.370737x|4.422540x|+1.185239%|
|128-biased-dynamic|1.132833|1.152438|3.988850x|4.057879x|+1.730528%|
|random-64|1.248125|1.266729|4.364075x|4.429122x|+1.490515%|
|32-high-entropy-stored|1.227875|1.239063|4.308333x|4.347589x|+0.911168%|
|eight-overlap|1.295313|1.314354|4.466595x|4.532255x|+1.470031%|
|eight-literals-prefix|1.336688|1.357458|4.577699x|4.648829x|+1.553841%|
|seven-stored-prefix|0.119875|0.120083|below resolution|below resolution|+0.173514%|
|one-stored|0.121188|0.121021|below resolution|below resolution|-0.137803%|
|empty|0.116500|0.116792|below resolution|below resolution|+0.250644%|

All 20 cases match exact pinned bytes, independent inflation, aliases, output prefix/capacity/reuse, logical work and zero final managed storage. All 20 allocation counts, requested bytes, reallocations and incremental usable peaks are equal in the separate heap cohort. All 12 scalar gates pass, with unchanged pure-scalar counts/bytes and zero loop allocation slope. Scalar elapsed ratios range from 0.9877808388369853 to 0.9952042157170248. Counter, heap, sample and primary elapsed cohorts stay separate.

| Separate CPU cohort | Instruction change | Cycle change |
|---|---:|---:|
|eight-literals|-0.736090%|-1.021458%|
|33027-periodic-window|+1.833529%|-1.045730%|
|repeated-32769|+1.938983%|-1.064407%|
|reymont-prefix-16384|-5.561029%|-12.800835%|
|reymont-prefix-32769|-5.762956%|-12.770367%|

Actual-executable profiles cover eight bytes, natural 32,769 bytes, repeated 32,769 bytes and periodic 33,027 bytes. These are CPU-time weights, not instruction partitions. Generated-code shares are 12.42%, 87.81%, 86.31% and 81.49%, respectively. Preserve the small-case contrary elapsed/cycle cohorts and the adverse periodic/repeated instruction cohorts.

Generated text grows 9,045,788 to 9,059,856 bytes; entry stack decreases 15,984 to 15,952 bytes. Initial full Object emission takes 133.901 seconds while unit qualification overlaps, so there is no matched compile-time claim. Final primary image restoration is 37.639610458 / 37.343839916 seconds, outside compression timing and only a descriptive pair.

Passing local qualification: 588 GPU tests with the same seven inherited exclusions; seven focused tests; 167 native CI tests inventoried and actually executed; unchanged 236 components, eight composition tests and one real-capacity path; 132 workflow tests; 15 architecture checks; runtime-only metadata, formatting and warnings as errors. GPU opt3 and unit/component opt1 use nightly-2026-03-03 aarch64 and frozen R145 dependencies. No fresh full-math/all-feature/minimal graph or hosted-CI claim is made.

Introduced harness failures are preserved: the copied template first named an old identity symbol, then embedded an old descriptor. Both were corrected; strict identity rejection occurred before any compression measurement. Initial test-only tuple storage and missing-parent selection mistakes were also corrected and retained. No functional test failure remains. No public PR or retention decision is claimed at this checkpoint.

Next bounded control: the shared frame recipe changed Pack from interleaved read/write emission to parallel reads, whereas only Copy required parallel reads. That incidental emission-order change may affect register allocation; causation for the wider instruction changes is not established. Restore the original emission order while making the analysis observe the same recipe, then repeat the same witnesses. Do not expand to windows, funding, SIMD or a source variant before this control. The source algorithm, allocations, capacities, limits and timing boundary stay pinned.
