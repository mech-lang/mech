# R168: remove invariant arguments within native loops

Retain the general compiler change. Native nested loops need not carry a changing outer value through their own backedges when that value stays unchanged during one child activation. The pass replaces those local phi arguments with their current dominating SSA values. It removes10,180 redundant phi parameters across the actual compressor, including73 of78 at the identified word-loop header. The remaining five fields are preserved. This is not a dynamic copy count.

The authoritative final-source cohort improves geometric elapsed by 2.639531%:19 medians improve and one worsens. All20 unchanged1.05 gates pass. No compressor source or allocator change is included, and the separate R165 histogram source gain is excluded.

| Input | Before ms | After ms | Before / Roc | After / Roc | Change |
|---|---:|---:|---:|---:|---:|
|reymont-prefix-32769|19.960625|18.991459|4.574977x|4.352844x|-4.855392%|
|eight-literals|1.359708|1.339479|4.721208x|4.650969x|-1.487746%|
|33027-periodic-window|8.054417|7.632271|6.443533x|6.105817x|-5.241168%|
|random-32769|6.638583|6.336042|6.304448x|6.017133x|-4.557328%|
|repeated-32769|7.657167|7.214270|5.748624x|5.416119x|-5.784078%|
|16384-distribution-change|5.747458|5.538958|7.275263x|7.011340x|-3.627682%|
|reymont-prefix-16384|11.210584|10.734562|5.254551x|5.031433x|-4.246175%|
|random-1024|1.261875|1.226083|4.070565x|3.955108x|-2.836374%|
|repeated-1024|1.495542|1.455167|4.630159x|4.505161x|-2.699658%|
|reymont-prefix-1024|1.658313|1.620563|5.040464x|4.925722x|-2.276411%|
|320-overlap|1.323708|1.307125|4.383139x|4.328227x|-1.252807%|
|random-256|1.309271|1.294896|4.393527x|4.345289x|-1.097939%|
|128-biased-dynamic|1.148916|1.118313|3.927920x|3.823291x|-2.663727%|
|random-64|1.299375|1.316812|4.465206x|4.525129x|+1.341991%|
|32-high-entropy-stored|1.328375|1.243333|4.644670x|4.347320x|-6.401955%|
|eight-overlap|1.314833|1.290354|4.613451x|4.527558x|-1.861795%|
|eight-literals-prefix|1.359000|1.333354|4.735192x|4.645834x|-1.887086%|
|seven-stored-prefix|0.119958|0.119625|below resolution|below resolution|-0.278013%|
|one-stored|0.119875|0.119688|below resolution|below resolution|-0.156413%|
|empty|0.116958|0.116375|below resolution|below resolution|-0.498470%|

The final matrix executes3968 primary/4048 total complete calls, with exact reference streams, independent inflate, unchanged logical work, aliases, prefixes, real capacities and zero final managed bytes/nodes. Random64 is the contrary elapsed case (+1.341991%). Three tiny stored-only reference medians remain zero; no denominator is invented. Largest input33027B; no multi-megabyte timing is claimed.

Earlier cohorts remain separate. The pre-layout20-case cohort has3968 primary/4048 total calls and2.107831% geometric improvement; its eight-byte case worsens0.317450%. The initial five-case screen has512 primary/532 total calls. Required Rust module relocation leaves the entire compressor Object byte-identical but changes linked Rust host text, so the final-source20-case cohort was rerun. These cohorts are not pooled or multiplied.

The pass runs after existing constant-branch/unreachable/global-phi cleanup and before the single ordinary backend data pass. Natural loop headers are processed inside-out. Phi-only dependencies within each dominated scope are solved with a monotone Bottom/One/Many worklist. Dominating current outside SSA values are distinct opaque atoms, including changing outer phis. Only one proved outside atom permits replacement. Local instructions, multiple seeds, edge-only results, type mismatches and non-dominating/irreducible inputs conservatively retain their parameters. No new analysis cap, runtime cache, speculative execution, code cloning or extra egraph is introduced. Instructions, CFG edges, effects, charges and failure order are unchanged. The IR identity advances to187; work107/ABI39/MBC14 remain.

The mapping build is whole-Object identical to the final primary Object. Current block45672 has5 arguments before and after backend optimization. Its tag0x198a independently identifies current word-loop header837/site2733; no old PC map is reused. R167's74 post-backend candidates are not73 actual removals: one comparison-balance input remains opaque to this earlier pass. Eighteen actual current hot-PC disassembly windows are archived; funding, access-helper and arithmetic guards still survive in word extension. No claim that the loop is fully simplified or that every eliminated formal was a physical memory transfer.

Generated text changes8903912 to8926476B (+22564B); machine entry stack16480 to15792B (-688B). One recorded restored-worker setup is36.838531s before/37.548808s after, outside evaluate_into; these are observations, not a startup performance cohort. Final library build26.523s, producer build1.667s, compressor Object generation151.677s and worker link5.484s are recorded outside primary timing. Frozen R145 Core/Engine/math/dependencies are reused; no full dependency rebuild.

| Input | Before instructions | After instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
|eight-literals|12673711.5|12538352.0|-1.068034%|-1.372556%|
|repeated-32769|141124794.5|129786085.0|-8.034527%|-5.762029%|
|33027-periodic-window|147349819.5|135869291.0|-7.791342%|-5.792743%|
|reymont-prefix-16384|142584666.5|137395120.0|-3.639625%|-4.326288%|
|reymont-prefix-32769|259835029.5|247776345.0|-4.640900%|-5.637984%|

All20 retired-instruction medians fall; 7 cycle medians rise. Every allocation-request, requested-byte, reallocation and incremental live-heap peak measurement remains equal across all20. CPU and heap runs are separate diagnostic cohorts, not primary elapsed. Requested capacity is not initialized copy volume; this patch makes no input-streaming or total-copy claim.

Five fresh exact-binary CPU-time profiles execute43740 complete calls with final cleanup zero. Source-attributed word extension accounts for20.396652% of repeated and19.478166% of periodic running-timer samples. Exact main-entry samples are80.585881% repeated,76.952346% periodic and80.805345% natural32KB. Outside-entry samples include other generated helpers as well as host/harness code. Unattributed native origins remain explicit; compact cost reduction inherits initializer context and is under-attributed by the path label. These samples are not a retired-instruction partition.

Validation with warnings denied:4 focused JIT tests;619 compatible GPU tests with the same7 inherited exclusions; all202 CI-selected names executed and passed in that final suite;236 unchanged components/8 composition/1 real-capacity path;12 balanced original/decoded scalar controls under unchanged1.05 and allocation gates;132 workflow tests; all15 architecture/catalog checks, runtime-only configuration, formatting and unsafe-boundary audit. New Rust modules stay below350 lines. Test features: aot, canonical-native, canonical-native-program, canonical-native-program-runtime, canonical-native-runtime, jit; production opt-level3 and GPU tests opt-level1. Component/root tests use canonical-native-program. Pinned nightly-2026-03-03/aarch64 with the pinned mimalloc object and frozen dependency libraries.

Focused controls cover current outer values over repeated child visits, zero trips, changed arguments, wrapping seeds, genuine inner updates, multiple outside seeds, an irreducible body entry, I8/I16/I32/I64/F32/F64 bit patterns, effects, early error, unchanged prior publication and retry. Existing original/decoded and finite-resource tests run in the compatible GPU suite. The new optimizer does not remove source operations or change admission.

Development failures are repaired and retained in receipts: test fixture path E0583; module-file/directory layout conflict; and missing exact test-only JIT unsafe registration. The owned-module, five-word ABI and initialized three-word output contract is registered without relaxing the audit. There is no new production unsafe. The final GPU spec's copied test_revision label names the R164 exclusion baseline; its command runs the newly built final-layout test binary. The build receipt records sourcecae6 plus a working-patch digest; effective final compiler file hashes are checked against f1c4. A separate validation-provenance receipt clarifies these labels without rewriting raw receipts.

Skipped: the same7 inherited GPU exclusions named in gpu-compatible-final-spec.json; fresh dependency/all-feature/minimal rebuilds; an additional standalone AOT harness; full multi-megabyte inputs and the separate ten-workload suite. Hosted ordinary CI is cancelled by standing instruction. #973 remains draft pending green hosted CI and clean exact-head review. No unresolved introduced functional failure remains. Parity is not achieved.

Baseline compiler9017ab40d19047f39bf1832c5c85d05f00accb5d (published8364514bd5), optimization parent4967229cd93b8acd7bd6f26fffa5d79f10e576d2, final measured effective compilerf1c4b67e774f55255e2c72ea4422e27d36b6be3c. Objectaef815ef0fb31fc21071d39ede6b154c9e9e50595833594cb5d5eafb079f5500; worker2b331a51c77c606151210cece63c599cca80da4cd2fe572a654eb211766b2963. Source331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9 and MBCa2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7. Full evaluate_into timing, real262144-entry U16 tree/1500513-word U32 cache/305257-word U64 path capacity, depth300/nice258, original EOF/pass/search/tie/block rules, source allocation sites, output capacity13 with clear/reuse, input aliases and allocator remain pinned. Work values are source accounting, not machine instructions.

The brief's selected path-cost proof/scalar/funding/four-lane experiment and20-case qualification are complete, as is the separately pinned R165 histogram-reuse source experiment. Match-extension bounded unroll/wider comparison is still open; R166/R167 prototypes are rejected. This R168 follow-up removes a demonstrated state-transport obstruction before another bounded loop transformation. No further compiler implementation is started in this round.

Per-case observations were sealed immediately without per-case git commits. Invariant compiler IR was accidentally repeated in unpublished packets; transparent compaction preserves every measured payload hash, each original manifest/receipt and superseded archive digests, and publishes the unchanged IR once in a mapping packet. No timing row is altered. Public numerical replay uses receipts only, without native binaries, bytecode or raw trace stores. Source/log evidence follows the standing upload authorization.
