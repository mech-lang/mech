# R169: current affine windows and native loop entries

Retain the compiler-only change with its measured tradeoff. The same complete level-12 program now admits current immutable affine read windows and merges separate activation entries before native loops. This lets existing invariant-argument cleanup use each activation's current values without transporting them through every backedge. The compressor source, bytecode, canonical plan, complete numeric recipes and allocator remain unchanged.

Repeated32KB improves28.995002% (5.415459x to3.845247x Roc); periodic33KB improves27.739703% (6.123566x to4.424907x). Natural32KB worsens1.780137% and eight literals0.853361%. In the confirmation all20 unchanged1.05 gates pass, with6 faster/14 slower medians and2.690497% geometric improvement. This is a matching-heavy improvement, not a universal speedup.

The initial full20 cohort failed the32-byte high-entropy gate: +5.679977%. Preserve that failure. A single predeclared full20 confirmation used the identical binaries, counts, balanced blocks and1.05 gates; it passes all20. Initial geometric improvement is2.256358%, with3 faster/17 slower. The within-paired-block median ratio1.004488 is diagnostic only, not a replacement gate. Timing variation is observed; its cause is not established. No code changed between cohorts and no iterative rerun-until-pass was used.

| Input | Before ms | After ms | Before / Roc | After / Roc | Change |
|---|---:|---:|---:|---:|---:|
|reymont-prefix-32769|19.000166|19.338396|4.371874x|4.449700x|+1.780137%|
|eight-literals|1.342750|1.354209|4.727993x|4.768340x|+0.853361%|
|33027-periodic-window|7.648333|5.526708|6.123566x|4.424907x|-27.739703%|
|random-32769|6.347770|6.399542|6.057033x|6.106433x|+0.815586%|
|repeated-32769|7.197146|5.110333|5.415459x|3.845247x|-28.995002%|
|16384-distribution-change|5.567167|5.609104|7.082910x|7.136265x|+0.753292%|
|reymont-prefix-16384|10.785541|10.885584|5.070776x|5.117811x|+0.927557%|
|random-1024|1.233250|1.250708|4.083609x|4.141419x|+1.415650%|
|repeated-1024|1.458438|1.422646|4.600751x|4.487842x|-2.454132%|
|reymont-prefix-1024|1.633000|1.655625|4.993884x|5.063073x|+1.385487%|
|320-overlap|1.310208|1.311250|4.396673x|4.400168x|+0.079491%|
|random-256|1.306208|1.317104|4.442884x|4.479946x|+0.834170%|
|128-biased-dynamic|1.124438|1.148083|3.877372x|3.958907x|+2.102828%|
|random-64|1.252292|1.261333|4.363385x|4.394889x|+0.721996%|
|32-high-entropy-stored|1.222521|1.245729|4.289546x|4.370981x|+1.898455%|
|eight-overlap|1.297001|1.317021|4.441783x|4.510344x|+1.543561%|
|eight-literals-prefix|1.362021|1.376604|4.847050x|4.898947x|+1.070688%|
|seven-stored-prefix|0.120688|0.119834|below resolution|below resolution|-0.707198%|
|one-stored|0.126292|0.126188|below resolution|below resolution|-0.082745%|
|empty|0.116750|0.116042|below resolution|below resolution|-0.606424%|

Each full matrix executes3968 primary/4048 total complete calls. The rejected window-only screen and current preheader screen each execute512 primary/532 total calls. Every observation verifies exact reference bytes, independent inflate, unchanged logical work, input aliases, prefix, output capacity and final managed cleanup zero. Cohorts are not pooled or multiplied. Three tiny stored-only reference medians are zero; no denominator is invented. Largest input33027B; no multi-megabyte result is claimed.

The first R169 window-only variant is rejected: repeated+69.105391% and periodic+66.148754% elapsed, with roughly35%/34% more instructions. Disassembly showed activation values copied around the loop. The minimal generic witness has two distinct outside seeds and16 invariant fields. The ordinary one-atom analysis correctly declines those separate seeds. A general preheader merges them once before the loop, after which the existing dominance-scoped solver reduces18 formals to2 genuinely changing fields. Checked scalar widths, float bit patterns, current arguments on fresh visits, zero trips, effects, failure, unchanged prior publication and retry remain covered.

The preheader pass snapshots natural-loop headers, redirects ordinary outside edges through one typed merge block and recomputes dominance once. Backedges keep their destination. Entry headers, empty parameters and edge-only exceptional arguments conservatively decline. It moves no source instruction, effect, charge or failure. No benchmark identity, speculative load, runtime permission cache, additional egraph or analysis cap is introduced.

Affine-window derivation reuses current carrier coordinates, the selected-edge recurrence proof and the existing resolver. It proves the guarded stride and unchanged captured fields; current initialized array lengths, limits, addition overflow and bases are admitted freshly for each reached activation. The existing checked fallback handles refusal. Facts remain tied to the current array/activation and are lost at unsupported joins or mutations. Successful admission does not erase finite work charges or error precedence. The maintained artifact path independently derives its proof; no bytecode permissions are serialized. IR189/work107/ABI39/MBC14.

In the actual full program32 preheaders enable16,298 redundant phi-parameter removals, compared with12,377 in the window-only variant. The selected word header reduces78 to5 parameters. These are static compiler counts, not dynamic copies. The source-mapping build's entire Object is identical to the measured Object and independently identifies header837/site2733/version2733. The complete machine path falls from70 instructions/19 stack accesses to25/3; both have2 input loads. The rejected window-only path is123/101/2. The pinned reference's identified scalar pattern has10 static instructions. These are explicit steady matching-word paths, not whole-program retired counts or a claim of equivalent per-instruction cost.

| Input | Before instructions | After instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
|eight-literals|12539924.5|12713347.5|+1.382967%|+3.467704%|
|repeated-32769|129787383.0|83163568.5|-35.923226%|-27.935680%|
|33027-periodic-window|135868470.5|89031746.0|-34.472107%|-27.672303%|
|reymont-prefix-16384|137355407.0|138236819.0|+0.641702%|+1.656174%|
|reymont-prefix-32769|247729806.0|249253997.5|+0.615264%|+2.193614%|

Separate counter cohorts retain all adverse measurements: 15 instruction medians and 13 cycle medians rise. Allocation requests, requested bytes, reallocations and incremental live-heap peaks are equal across all20 cases. Generated text changes8926476 to8920996B (-5480B); entry stack15792 to15728B (-64B). Requested capacity is not initialized copy volume; no streaming/copy-volume claim is made. Restored-worker setup is37.258842s before/37.371932s after, outside evaluate_into, not a startup cohort. Build receipts retain the frozen-dependency library/producer/Object/link observations separately.

Six fresh exact-binary CPU-time profiles execute73481 complete calls with cleanup zero. Source-attributed word extension is21.445390% of repeated running-timer samples; natural32KB is dominated by other native scopes, with word extension0.743292%. Unattributed native origins and outside-entry samples remain explicit. Outside entry includes generated helpers as well as host/harness code. Sample shares are not a retired-instruction partition or an exact native-versus-host split. The next implementation must use the actual current hot instructions; remaining funding/paid-flag work and bounded wider comparison remain candidates, not measured savings.

Validation:6 focused local-phi JIT tests and5 current captured-window controls;626 compatible GPU tests with the same7 inherited exclusions; all209 selected CI names actually execute and pass in that suite;236 unchanged components/8 composition/1 real-capacity path;12 balanced original/decoded scalar controls with unchanged1.05 and allocation gates;132 workflow tests;15 architecture/catalog checks, runtime-only configuration, formatting and warnings denied. New modules remain below350 lines. No Core/Engine/math or unsafe-boundary relaxation is included. Test commands and exact features are in unit-spec.json, gpu-compatible-final-spec.json, qualification/*.json and checks/receipts.json. Production opt-level3, GPU tests opt-level1; features aot, canonical-native, canonical-native-program, canonical-native-program-runtime, canonical-native-runtime, jit. Pinned nightly-2026-03-03/aarch64, mimalloc and frozen R145 dependencies.

Development command failures: a mismatched Python bracket in the baseline disassembly script and a unit-build command launched outside the checkout were corrected and rerun. A packet script used an obsolete GPU-receipt filename; its partial unsealed staging directory was removed, the filename corrected and packaging rerun. An exploratory source-map query iterated tag keys instead of values and was corrected using the descriptor. Raw observations and compiler files are unchanged by these repairs. The rejected window-only performance experiment and initial32B gate failure remain distinct from functional correctness. No unresolved introduced functional failure remains. Inherited exclusions are listed exactly in gpu-compatible-final-spec.json. Skipped: fresh dependency/all-feature/minimal rebuilds, additional standalone AOT harness, full multi-megabyte inputs and the separate ten-workload suite. Ordinary hosted CI is cancelled by standing instruction; review automation is preserved. PR973 stays draft pending green hosted CI and clean exact-head review.

Optimization parent 4ab701de9b4ca994a1ae151259d5a697132f1210; baseline effective compiler f1c4b67e774f55255e2c72ea4422e27d36b6be3c; measured current compiler fdb1c5c4e738ad3bb69233ef92448efcf075aa81. Object a07bdb9bd1da56f113c2bf81b9b6bbe17d0e5679d15314174ec891ba4c4a99d0; worker 08c57458f4983a262bf49366a212aeb0f22d4bbe2d9477bfd8db571a5b4a4437. Source 331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9 and MBC a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7. Documentation/evidence are folded into the same optimization commit; final compiler file hashes must match the measured source. Failed window-only source5493bda433cdadf9fa5a242397b96d76825afea3 is preserved by its patch and pins, not used as the retained baseline.

The complete evaluate_into boundary remains fixed: real262144-entry U16 tree,1500513-word U32 cache,305257-word U64 path capacity, depth300/nice258, original EOF/search/tie/block rules, source allocation sites, output capacity13 with clear/reuse, input aliases and allocator. Logical work is source accounting, not hardware instructions. The separate R165 histogram source gain is excluded. Match-extension unroll/wider comparison remains open. Parity is not achieved.

Per-case timings and scalar cases were sealed immediately without per-case git commits. Public replay uses only hash-verified receipts, source/diagnostic logs and explicit historical versions; no native binaries, bytecode or raw trace stores are uploaded. Numerical reconstruction does not run a compiler or compressor. Both failed and successful cohorts are preserved. Source/log evidence follows the standing upload authorization.
