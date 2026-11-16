# R159: fresh U32 comparison facts

The maintained numeric complementary-case rule now recognizes U32 comparisons alongside U64 comparisons. The shared scalar emitter returns an icmp normalized to0/1; no operand-range assumption is needed. A fact applies only to the immediate current producer/predecessor chain in the same selected iteration, excluding all existing loop, cold, funding and unsupported-region frontiers. Arbitrary raw Bool/tag homes keep their defaults. The comparison itself, source visits/prices and checked arithmetic remain. No benchmark source, algorithm, allocation, buffer-capacity, allocator or timing-boundary change is included.

Source SHA256 331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9; bytecode SHA256 a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7; allocator SHA256 38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874. The real262144-entry U16 tree,1500513-word U32 cache,305257-entry U64 path capacity, depth300, nice258 and original EOF limits stay pinned.

Published base #985: e666915d8e6b2a97ca760998244a8d638ce0e149. Research base:7aa8e9c89f6763c0fd95958d333556c1be328dc4. Before compiler:78d6a87f8cb0c61c434ea73d6e8bb17e4d2cb679. Production:09853c8157a7cc27448791543f976348a0158868. Test-only parent:e41ade09dc3c9477c12045432b1a11514006b873. CI inventory change:8eb6e578fe. IR/work/ABI/MBC are181/107/39/14; older native images remain rejected. Source plan, complete numeric recipes and source prices equal the before image.

The exact test-only parent passes three existing behavior/projection/finite-budget controls and fails the new original/decoded U32 emitted-proof expectation at zero. The candidate passes all four. Existing controls cover true/false/ties, current visits and arguments, zero trips, direct/restored execution, failures/retries, unchanged output/aliases and cleanup. The new test also refuses a missing comparison producer and verifies that actual Object metadata uses the current proofs. Existing raw-Bool, invalid-tag and frontier controls remain.

The full compressor has51 additional static case proofs (34 U32Lt,17 U32Ne), eight in the four path-cost clones. Dynamic eligible/executed guard counts were not instrumented. Source-location diagnostics regenerate an Object byte-identical to the actual primary Object,05a0473a1ca38db3837bcb988d24af3fefed2370851108bd3ea8123ad905a6a1. At the profiled clone's sites15599/15608, each12-byte guard sequence disappears: six actual ARM64 instructions, including two comparisons. The two necessary source cost comparisons remain. This is not the reference's final one-comparison/two-select loop; further simplification remains.

The predeclared final20-case cohort retains3,968 primary observations (4,048 including warmups). Eighteen medians improve and two worsen; geometric after/before is0.9886524021185078 (-1.134760%). Worst+2.706879%; every unchanged1.05 case gate passes. The separate four-case development screen retains all adverse observations and is never pooled with the final cohort.

| Case | Before ms | After ms | Before / reference | After / reference | Change |
|---|---:|---:|---:|---:|---:|
|eight-literals|1.344354|1.343792|4.733641x|4.731662x|-0.041804%|
|16384-distribution-change|6.061479|5.964646|7.653383x|7.531118x|-1.597531%|
|33027-periodic-window|8.695438|8.930813|6.945238x|7.133237x|+2.706879%|
|random-32769|6.764042|6.720438|6.429697x|6.388249x|-0.644644%|
|repeated-32769|8.263167|8.475354|6.217582x|6.377242x|+2.567871%|
|reymont-prefix-32769|29.317042|28.176875|6.734905x|6.472978x|-3.889091%|
|reymont-prefix-16384|16.088313|15.427979|7.570971x|7.260225x|-4.104433%|
|random-1024|1.253355|1.232792|4.122877x|4.055235x|-1.640637%|
|repeated-1024|1.531291|1.528438|4.668572x|4.659872x|-0.186346%|
|reymont-prefix-1024|1.885084|1.811834|5.729739x|5.507093x|-3.885795%|
|320-overlap|1.363146|1.362000|4.574314x|4.570470x|-0.084034%|
|random-256|1.326208|1.324083|4.495620x|4.488417x|-0.160231%|
|128-biased-dynamic|1.223625|1.157083|4.233997x|4.003747x|-5.438104%|
|random-64|1.266000|1.252458|4.411150x|4.363965x|-1.069668%|
|32-high-entropy-stored|1.238604|1.228979|4.300708x|4.267288x|-0.777085%|
|eight-overlap|1.312813|1.305584|4.511385x|4.486541x|-0.550688%|
|eight-literals-prefix|1.406458|1.367833|4.833191x|4.700459x|-2.746259%|
|seven-stored-prefix|0.121125|0.120646|below resolution|below resolution|-0.395872%|
|one-stored|0.121000|0.120625|below resolution|below resolution|-0.309917%|
|empty|0.117187|0.117146|below resolution|below resolution|-0.034987%|

Each pair uses that cohort's fresh reference denominator; do not compound ratios from separate rounds. All20 exact raw-byte, independent-inflate, logical-work, input-alias, output-prefix/capacity/reuse and zero-final-managed-storage checks pass. All20 heap requests, requested bytes, reallocations and incremental peak live usable bytes are exactly equal. This is an instruction/control change, not an allocation improvement. RSS/lifetime peaks are retained separately and include image/harness residency.

| Separate CPU cohort | Before instructions | After instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
|empty|1072164.5|1072235.0|+0.006575%|-0.066938%|
|one-stored|1124899.0|1124984.0|+0.007556%|-1.610958%|
|seven-stored-prefix|1124715.5|1124840.5|+0.011114%|+0.182297%|
|eight-literals|12698692.0|12700648.0|+0.015403%|-0.113916%|
|eight-overlap|11890648.5|11892666.5|+0.016971%|-0.574366%|
|320-overlap|12602829.5|12604770.5|+0.015401%|+3.047949%|
|eight-literals-prefix|12700207.5|12702281.5|+0.016330%|+0.885401%|
|128-biased-dynamic|10678035.5|10679591.0|+0.014567%|+2.004896%|
|32-high-entropy-stored|11286898.5|11289869.5|+0.026323%|+10.294734%|
|16384-distribution-change|76960585.0|76726119.0|-0.304657%|+1.637165%|
|33027-periodic-window|155368014.0|155402444.5|+0.022161%|+0.460644%|
|random-64|11464657.5|11468129.5|+0.030284%|+0.225114%|
|random-256|12114947.0|12119245.5|+0.035481%|-0.392605%|
|random-1024|12572384.0|12575899.5|+0.027962%|+1.182514%|
|repeated-1024|16071495.0|16074128.0|+0.016383%|-0.699998%|
|random-32769|90852911.5|90531909.5|-0.353321%|-1.362480%|
|repeated-32769|149115253.0|149141369.0|+0.017514%|+0.059886%|
|reymont-prefix-1024|18708211.5|18571988.0|-0.728148%|-1.515654%|
|reymont-prefix-16384|180258221.0|177371049.5|-1.601686%|-3.637035%|
|reymont-prefix-32769|328555634.5|323622867.0|-1.501349%|-3.266922%|

Natural32KB removes4,932,767.5 instructions (-1.501349%) in the separate counter cohort; cycles fall3.266922%. Periodic/repeated instructions are slightly adverse (+0.022161%/+0.017514%) and cycles+0.460644%/+0.059886%, versus primary elapsed+2.706879%/+2.567871%. Biased128's5.44% primary gain has almost unchanged instruction count. Do not attribute every elapsed difference to fewer retired instructions; the code-layout/cache explanation remains unproved. Primary, counter, heap and sampling cohorts stay separate.

All12 balanced scalar gates pass, with unchanged pure-scalar allocations/bytes and zero loop allocation slope. Elapsed ratios range0.997286 to1.001541, including two adverse controls. Current actual-executable CPU-time contexts (not instruction partitions):

| Input | Generated code exclusive | Host callbacks/cleanup | Outside/unrecognized |
|---|---:|---:|---:|
|eight-literals|12.78%|77.29%|9.94%|
|reymont-prefix-32769|87.70%|11.17%|1.13%|
|repeated-32769|85.63%|12.38%|2.00%|
|33027-periodic-window|84.28%|14.06%|1.66%|

Generated text grows9,059,856 to9,110,288 bytes (+50,432); entry stack falls15,952 to15,872 bytes (-80). Final image restoration is37.868903167/37.560953167 seconds, outside compression timing: descriptive observations, not a matched startup gain. Main Object emission is133.676 seconds while qualification overlaps; no matched compile-time claim is made. Code growth and the two large-case regressions are explicit retention tradeoffs.

Passing local receipts:590 GPU tests with the same seven inherited exclusions;10 focused;169 CI-selected cases inventoried and actually executed;236 unchanged components, eight composition and one real-capacity path;132 workflow tests;15 architecture/catalog checks; runtime-only, formatting and warnings as errors. GPU opt3, unit/root qualification opt1 and nightly-2026-03-03 aarch64 use frozen R145 Core/Engine/math dependencies. The selected CI cases execute in that retained aot/jit/canonical-native feature superset, not a freshly rebuilt minimal feature graph. Exact commands/features are in unit-spec.json, library-spec.json, checks/receipts.json and qualification/*.json.

Corrected introduced harness failures are preserved: the first test reporting iterator borrowed temporary metadata; spec copying assumed an environment field; the first library build lacked an output parent. None affected a timing. Descriptive scope text in the immutable four-case and final20 receipts was copied from R157/IR180; image/worker hashes and actual execution identify R159/IR181. measurement-scope-erratum.json records the correction without rewriting observations or repacking historical archives. No functional failure remains. Seven inherited GPU exclusions and unrun fresh full-math/all-feature/minimal graphs remain explicit. Actual restored linked compression runs pass; no additional standalone linked-suite claim is made. Hosted CI stays cancelled under instruction; no green claim is made and the PR stays draft.

Decision: retain the bounded comparison-family extension under unchanged gates, including its contrary measurements. The R158 investigation still motivates current scalar array windows and masked narrowing facts using the maintained unsigned transfer rules. Preserve get-or-zero selection, current view/version/geometry, overflow/error order and fallback; then prove whole-loop totality and exact settlement using the already prepaid loan. Ordered reduction/unrolling/SIMD and a separate pinned source variant remain unfinished. Parity is not achieved.

Numerical replay: checkpoint.py INPUT OUTPUT reconstructs results.json and REPORT.md from hash-verified receipts, without compiling, compressing or writing a worktree. --write-doc is optional and explicit. Experiment-index assets exclude native binaries, bytecode and raw trace containers.
