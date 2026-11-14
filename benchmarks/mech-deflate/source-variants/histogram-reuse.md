# R165: reuse the all-literal histogram for initial costs

This is a separately pinned source experiment on the unchanged R164 compiler. The original compressor remains the default. Natural32KB improves **4.577225× → 4.445344× Roc**, repeated32KB **5.729042× → 5.460320×**, periodic33KB **6.451039× → 6.147931×**, and eight bytes **4.719510× → 4.690838×** in this matched cohort. The geometric20-case gain is **1.878198%**. This is source work elimination, not another compiler gain.

## Change and fidelity

Block planning computes a288-entry U32 histogram for its all-literal candidate, then scans the same input range again to choose initial costs. The variant passes the existing first256 literal frequencies to a helper derived from the pinned pricing body. Its EOF entry and remaining entries are outside the unchanged selector loop. The original standalone default-cost function remains byte-for-byte intact, including operation/error order. Huffman construction preserves the returned histogram. One block-planning call changes; the second histogram construction and byte scan are omitted.

Prior experiments were independent cost-table construction (R09) and zero-trip match/skip guards (R118, rechecked R126). Neither is combined here. The default source remains **331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9**, MBC **a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7**. Variant source **fd9ebdb22c0ea1cf40ab2c1428dd4c2a7f61b39fb6c27507a6f316e7a6f1483f**, MBC **b9a598a3580ca6e368bae8a03f499367f60de256963a8de80fda03bee0165b9d**. Source revision **c0ea51478d3dcfa66a361fe0c979695b04a1c3c3**, parent **8364514bd585a0285537068d24922e2a11eaabc6**; compiler **9017ab40d19047f39bf1832c5c85d05f00accb5d**, published compiler8364514bd585a0285537068d24922e2a11eaabc6. Final docs/evidence and additional slice controls are folded into the same source-experiment commit. The timed transform and all compiler/dependency bytes remain unchanged.

IR/work/ABI/MBC **184/107/39/14**. The exact same compiled GPU library and frozen R145 Core/Engine/math/dependencies are used on both sides; there is no compiler build or semantic change. Preserve search depth300/nice258, pass limits, tie rules, block decisions, tables, original EOF bounds, real262144-entry U16 tree/1500513-word U32 cache/305257-word U64 paths, input aliases, output reservation13 for eight bytes, output clear/reuse, pinned mimalloc and complete evaluate_into timing. The source-level allocation difference is the intentional histogram removal. Mech performs all compression; independent inflate and comparison stay outside timing.

## Complete-call timings

The predeclared balanced cohort has **3968 primary/4048 total calls**, eight blocks, no trimming, **17 faster/3 slower** medians, and all unchanged1.05 gates pass. All20 streams equal the pinned raw bytes, independently inflate, retain prefix/capacity/backing/input aliases and end with zero managed bytes/nodes. Work is constant within each source/case and is reported separately; source work is not a hardware instruction count. The three stored-only cases below8 bytes do not enter the changed block planner: their small regressions and changed artifact-level work charges are retained without assigning an unproved cause. Zero reference medians remain below resolution, with no invented denominator.

| Case | Pinned ms | Variant ms | Pinned / Roc | Variant / Roc | Change |
|---|---:|---:|---:|---:|---:|
|reymont-prefix-32769|19.997896|19.421708|4.577225×|4.445344×|-2.881238%|
|eight-literals|1.354499|1.346271|4.719510×|4.690838×|-0.607531%|
|33027-periodic-window|8.070249|7.691062|6.451039×|6.147931×|-4.698585%|
|random-32769|6.653291|6.372125|6.330439×|6.062916×|-4.225976%|
|repeated-32769|7.631083|7.273146|5.729042×|5.460320×|-4.690520%|
|16384-distribution-change|5.784729|5.579562|7.276389×|7.018318×|-3.546692%|
|reymont-prefix-16384|11.226271|10.939667|5.280466×|5.145657×|-2.552976%|
|random-1024|1.258521|1.226021|4.112812×|4.006605×|-2.582358%|
|repeated-1024|1.489625|1.461270|4.626165×|4.538107×|-1.903466%|
|reymont-prefix-1024|1.667229|1.634417|5.083015×|4.982979×|-1.968056%|
|320-overlap|1.354938|1.326979|4.516458×|4.423263×|-2.063453%|
|random-256|1.324812|1.306729|4.460648×|4.399761×|-1.364986%|
|128-biased-dynamic|1.154229|1.128167|4.085768×|3.993512×|-2.258000%|
|random-64|1.275854|1.258292|4.492444×|4.430604×|-1.376529%|
|32-high-entropy-stored|1.241625|1.230813|4.326220×|4.288545×|-0.870835%|
|eight-overlap|1.321333|1.305417|4.603949×|4.548491×|-1.204579%|
|eight-literals-prefix|1.356771|1.340833|4.646476×|4.591894×|-1.174701%|
|seven-stored-prefix|0.119771|0.120563|below resolution|below resolution|+0.660844%|
|one-stored|0.119625|0.120750|below resolution|below resolution|+0.940439%|
|empty|0.115917|0.117166|below resolution|below resolution|+1.077931%|

## Instructions and memory

These are independent complete-call counter cohorts: eight CPU/two heap observations per side above1024B, sixteen CPU/four heap otherwise. Fifteen instruction medians decrease and five increase; nineteen cycle medians decrease and eight-overlap increases. Repeated and periodic instructions rise about0.8% despite their faster elapsed medians. Preserve those contrary observations. Samples do not partition retired instructions; independent cohorts are not multiplied into elapsed claims.

| Case | Before instructions | Variant instructions | Instruction change | Cycle change | Requests delta | Requested B delta | Incremental peak usable B delta |
|---|---:|---:|---:|---:|---:|---:|---:|
|empty|1060397.0|1060504.0|+0.010091%|-0.473796%|+0|-6920|-16384|
|one-stored|1113130.0|1113184.0|+0.004851%|-0.833351%|+0|-6920|-16384|
|seven-stored-prefix|1112941.5|1113105.0|+0.014691%|-0.793611%|+0|-6920|-16384|
|eight-literals|12672897.0|12619316.5|-0.422796%|-2.684119%|-10|-9264|-17832|
|eight-overlap|11866022.0|11812236.0|-0.453277%|+1.687513%|-10|-9264|-17832|
|320-overlap|12430729.5|12371169.0|-0.479139%|-1.426268%|-10|-9264|-17832|
|eight-literals-prefix|12672773.5|12619402.0|-0.421151%|-0.638662%|-10|-9264|-17832|
|128-biased-dynamic|10643602.5|10579439.5|-0.602832%|-1.593927%|-10|-9264|-17832|
|32-high-entropy-stored|11253602.0|11198451.5|-0.490070%|-0.163806%|-10|-9264|-17832|
|16384-distribution-change|73632569.0|72049788.5|-2.149566%|-2.931823%|-20|-11608|-17832|
|33027-periodic-window|147337907.5|148505418.5|+0.792404%|-4.777974%|-10|-9264|-17832|
|random-64|11422338.5|11364903.5|-0.502830%|-2.891703%|-10|-9264|-17832|
|random-256|12026145.0|11951865.5|-0.617650%|-0.369041%|-10|-9264|-17832|
|random-1024|12417736.5|12282190.0|-1.091556%|-3.934384%|-10|-9264|-17832|
|repeated-1024|15579640.0|15541875.5|-0.242396%|-1.417628%|-10|-9264|-17832|
|random-32769|86887901.5|84097401.0|-3.211610%|-3.907131%|-10|-9264|-17832|
|repeated-32769|141120572.0|142281814.5|+0.822873%|-4.620321%|-10|-9264|-17832|
|reymont-prefix-1024|17053101.0|16922378.0|-0.766564%|-1.779357%|-10|-9264|-17832|
|reymont-prefix-16384|142555810.5|141401427.5|-0.809776%|-2.677611%|-20|-11608|-17832|
|reymont-prefix-32769|260336580.5|257508313.5|-1.086389%|-4.445978%|-20|-11608|-17832|

Requests fall10 or20 on cases entering block planning and remain equal on the three short stored-only paths. Requested bytes fall6920–11608B and incremental live heap peaks fall16384–17832B. Reallocations and their relocation-copy upper bounds remain unchanged. Totals include artifact-level setup differences; they are not an isolated per-histogram allocation-family census. Input is still retained in memory; no input-streaming or total copied-byte claim is made.

Actual Object **d772a48fffde5163a138ac984ea2c894f33e5255cf045d28a273faa459ddbd09** retains four real vector minimum bodies and the R164 compact scalar/SIMD lowering. Generated text **8,903,912→8,941,100B (+37,188)**; entry stack **16,480→16,160B (-320)**. These code/stack tradeoffs are retained alongside elapsed and heap gains. Bounded disassembly, exact commands and actual hashes are in actual-machine. Static instruction sites are not executed visits or a per-function instruction ledger. Restore times are outside the primary timer and retained in session stderr; no repeated startup gain is claimed.

## Fresh executable profile

Exclusive CPU-time contexts on the actual variant, with concurrent stdout draining, exact output/work checks and complete cleanup. These are complete-call samples, not native-operation coverage or instruction shares.

| Case | Generated exclusive | Host callbacks/cleanup | Outside |
|---|---:|---:|---:|
|128-biased-dynamic|13.379%|72.411%|14.210%|
|eight-literals|11.180%|79.287%|9.534%|
|reymont-prefix-32769|82.502%|16.101%|1.397%|
|repeated-32769|83.433%|14.571%|1.996%|
|33027-periodic-window|80.293%|17.577%|2.130%|

## Validation and decision

- Seven focused tests pass, including all eight pinned default-cost fixtures and shifted input slices, Mech-computed first histograms, original/decoded execution, EOF exclusion, live input aliases, overflow rejection with unchanged previous output, cleanup and successful retry. Drift rejection is intentionally caught; its panic text under nocapture is an expected test observation.
- 236 unchanged component,8 composition and1 real-capacity path tests pass without changed expected values or larger allowances. The original pinned assembly hash test passes.
- The new three source-variant tests are included in the existing canonical_artifact_program target run by full CI. They are actually executed in the focused receipt. GPU's198-test compiler inventory is unchanged.
- Warnings denied and formatting pass. The byte-identical compiler retains R164's615 compatible GPU tests (same7 exclusions),27 focused compiler tests,198 CI-selected tests,132 workflow/15 architecture/runtime-only checks and12 scalar controls; these are reused receipts, not fresh R165 executions.
- Actual decoded/restored linked compression passes all20 cases. Canonical source compilation and revision14 encode/decode roundtrip are checked before native emission.

The initial producer harness passed &String where TextSnapshot required &str; corrected to as_str before any performance call. No unexpected compiler gap or benchmark resource denial was found. Earlier factoring of the helper passed its focused checks but was revised before timing to preserve the original public function byte-for-byte. The copied evidence sealer also expected a fresh GPU receipt for this source-only round; corrected to use explicit reused compiler receipts, preserving the initial closure and appending final closure. All snapshots and adverse observations remain archived. No unresolved failure remains.

The same seven inherited GPU exclusions, fresh dependency/all-feature/minimal builds, an extra standalone AOT suite, full multi-MB timing and the separate ten-workload suite remain skipped. Ordinary hosted CI is cancelled by standing instruction. PR973 stays draft pending green hosted CI and clean exact-head review. One source optimization commit, no additional PR.

The brief's selected compiler experiment and its separate source experiment are now implemented and measured. Match-extension unroll/wider comparison remains an unimplemented alternative. Keep the pinned source baseline for future compiler attribution; this variant is separately labeled. Next compiler work should inspect and simplify a currently hot native recurrence, beginning with the remaining match-extension candidate, while retaining the complete20-case controls. Parity is not achieved.

Evidence excludes native binaries, bytecode, raw trace stores, private environment metadata and copied assignments. Run checkpoint.py INPUT OUTPUT to reconstruct results.json and REPORT.md without compiling or compressing.

Decision: **Retain as a separately labeled histogram-reuse source variant. All twenty unchanged case gates pass; geometric elapsed improves 1.878198 percent, with natural32 2.881238 percent, repeated32 4.690520 percent and periodic33 4.698585 percent. Preserve three stored-only regressions, five adverse instruction medians and one adverse cycle median. Keep the pinned source default and compiler bytes unchanged.**
