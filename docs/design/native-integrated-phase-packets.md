# Maintained dense numeric phase integration (R215)

**The qualified dense phase interface and canonical-home facts are now the maintained normal compiler implementation, with compiled-image IR215.** All 678 maintained tests, 52 real linked controls and all 20 full compression cases pass. All unchanged 1.05 gates pass against R199 and the frozen R214 candidate. This completes the ordinary integration checkpoint, not the whole brief or parity.

## Revisions and design

Previous published head `7cba443d824c451cc2ad3102ae2dda1d0341ecbd`. Separate inherited inventory repair `4d20781d403156b81b2fa7656d54e860523db01f` is the compiler-change base. Frozen old production `7748204bd12e57a20369ce7970f31f8d15c3901f` and retained whole-native diagnostic `efd9069b589512630891810191e7de35d57325d8` remain comparison artifacts. IR/work/ABI/MBC: 215/107/39/14. Same nightly 2026-03-03, Cranelift 0.131.3, production frontend and pinned mimalloc 2.1.7. No LLVM change.

Eight maintained Rust files integrate the R214 candidate: dense input/result transport for one existing complete nested append/direct-array computation class, exact successful live-word query and the SAME canonical-home SSA flag provider. The real compressor operation is U64 near-optimal path append/scan at source line 724 and lines 726–732. Four full-compressor definitions use 80 input/33 result words; the real 305257-capacity small witness uses 26/14. No source/site/function names select the class.

Current canonical ownership/admission/funding, first-change/growth, source checks, cold synchronization and exact errors remain. Results are copied only after successful status/completion. The original inline route remains when the ORIGINAL source boundary envelope cannot fund the generated template. No source prices or resource allowances increase. Packet loads retain their own default flags; canonical disjoint/nontrapping facts describe canonical spills/reloads only. Disabled optimization controls remain tested.

The actual ARM64 Mach-O text section is byte-identical to frozen R214: 8,106,656 B, SHA256 `67b984df53f9a5b8e86d8b7cdc40e62472c79cd407ba1bf474edc19a93592561`. Full Objects/metadata differ in identity/data/relocations as expected. Each affected function retains 242 load eliminations, 1163 post-cleanup instructions and 3888/3920 code bytes. This is integration of a qualified representation, not another newly native source operation.

## Image and CI inventory checks

IR215 changes the Object association/cache identity while public ABI39, work107, descriptor envelope1 and MBC14 stay. Current/preceding metadata can restore under the unsafe build-owned metadata contract; restoration grants no executable authority. A probe linked against the actual unchanged saved IR214 Object verifies current LinkedProgramArtifact binding rejects it. Actual new linked Objects bind and execute successfully. The initial probe asserted rejection at the wrong metadata-only layer; that failure and correction are retained, with no production decoder change.

A clean-parent build verifies the inherited CI guard mismatch: expected251, actually260. All 790 tracked GPU source blobs match the clean parent exactly. The separate repair changes only251→260. This integration adds one standalone identity regression and changes the exact guard260→261. Actual before/after test lists differ ONLY by that test. Filters, transitive Cargo features, warnings-as-errors and allowances remain. The package feature closure includes aot→jit; an invalid manually reduced harness profile is recorded and corrected without changing manifest gating.

## Complete all-size timing

11,488 complete Mech calls / 11,328 retained observations, all 24 balanced permutations of four variants per case, plus 5120 fresh reference compressions. Geometric elapsed vs R199 is -0.469%; vs frozen R214 is +0.496%. Preserve this mixed timing alongside identical native text. Prior R214’s two-cohort gain is not substituted for this fresh integrated result.

| Input | R199 / Roc | Frozen R214 / Roc | R211 diagnostic / Roc | Integrated normal / Roc | Integrated vs R199 |
|---|---:|---:|---:|---:|---:|
| reymont-prefix-32769 | 3.459408× | 3.484045× | 2.750593× | 3.486334× | +0.778% |
| 33027-periodic-window | 3.798533× | 3.820200× | 2.616567× | 3.813050× | +0.382% |
| repeated-32769 | 3.287242× | 3.304862× | 2.429315× | 3.303768× | +0.503% |
| random-32769 | 5.279897× | 5.262915× | 4.523539× | 5.270833× | -0.172% |
| 16384-distribution-change | 6.978429× | 6.987305× | 4.244363× | 6.968631× | -0.140% |
| eight-literals | 4.675264× | 4.559294× | 0.837484× | 4.621795× | -1.144% |
| reymont-prefix-16384 | 4.296049× | 4.309761× | 2.798523× | 4.314994× | +0.441% |
| empty | unresolved | unresolved | unresolved | unresolved | -0.159% |
| one-stored | unresolved | unresolved | unresolved | unresolved | -0.307% |
| seven-stored-prefix | unresolved | unresolved | unresolved | unresolved | -0.222% |
| eight-overlap | 4.589789× | 4.460827× | 0.826512× | 4.536090× | -1.170% |
| 320-overlap | 4.273620× | 4.198647× | 0.902314× | 4.236761× | -0.862% |
| eight-literals-prefix | 4.679949× | 4.556040× | 0.838415× | 4.636977× | -0.918% |
| 128-biased-dynamic | 3.933695× | 3.837355× | 0.765876× | 3.897559× | -0.919% |
| 32-high-entropy-stored | 4.242451× | 4.184671× | 0.897720× | 4.213415× | -0.684% |
| random-64 | 4.327091× | 4.242815× | 0.960584× | 4.252035× | -1.735% |
| random-256 | 4.265158× | 4.210478× | 1.150194× | 4.218161× | -1.102% |
| random-1024 | 4.035203× | 3.950219× | 1.269389× | 3.982122× | -1.315% |
| repeated-1024 | 4.203416× | 4.148163× | 1.059590× | 4.193582× | -0.234% |
| reymont-prefix-1024 | 4.679454× | 4.640157× | 1.416542× | 4.662861× | -0.355% |

16 lower / 4 higher medians vs R199; all 20 original gates pass. Tiny stored-only reference medians round to zero, leaving their multiples unresolved. R211 omits ordinary guarantees and uses a different surrounding representation; its column is not a contract-cost floor.

Timing covers evaluate_into, including real allocations/copies, computation, managed cleanup and publication. Seed/clear/prefix, input imports/rebind, output slots, byte verification/inflate and drop remain excluded. Source SHA `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; strict MBC14 SHA `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. Level12 algorithm/tables, depth300/nice258/EOF rules, source allocation sites, real 262144-U16 tree / 1500513-U32 cache / 305257-U64 path capacity and eight-byte output capacity13 remain unchanged.

## Independent counters and memory

4160 separate CPU/heap Mech calls; 24 retained per case/variant/cohort. Fresh reference CPU/heap measurements use the same 40 pinned executables, 1280 windows / 10240 compressions. Counter overhead remains unadjusted. These are whole-call counters, independent from primary timing and sampled stacks.

| Input | Instructions vs R199 | Instructions vs R214 | Integrated instructions / Roc | Cycles vs R199 |
|---|---:|---:|---:|---:|---:|
| reymont-prefix-32769 | +1.805% | +0.040% | 4.454612× | +1.048% |
| 33027-periodic-window | +1.075% | +0.005% | 3.803243× | +0.970% |
| repeated-32769 | +1.164% | +0.008% | 3.536307× | +0.786% |
| random-32769 | +1.481% | +0.003% | 6.131602× | +0.230% |
| 16384-distribution-change | +1.431% | +0.007% | 7.003597× | +0.424% |
| eight-literals | +0.060% | +0.027% | 9.116848× | +0.404% |
| reymont-prefix-16384 | +1.653% | +0.011% | 4.868324× | +1.363% |

All 20 instruction medians remain higher than R199. Cycles: 11 lower / 9 higher. The contrary measurements stay visible. All 20 allocation-request, requested-byte, realloc and incremental usable-peak measurements match R199. No allocation reduction is claimed.

Generated code metadata: 8,152,436 → 8,106,416 B; reported native stack: 16,480 → 20,560 B (+4080 B). Static metadata differs from actual process RSS/stack peak; heap counters are not initialized-copy volume. Physical final-output backing reuse remains unestablished.

## Fresh exact-executable profiles

Six wall profiles execute 8,118 correct complete calls; six exact-PC profiles execute 19,460. All seven generated symbols are inventoried from the exact timed Object; imported helpers are excluded. Worker sampling includes surrounding preparation/validation/decompression and is separate from evaluate_into timing.

| Input | Generated exclusive | Host below generated | Other worker/system |
|---|---:|---:|---:|
| 16384-distribution-change | 52.683% | 43.515% | 3.801% |
| 33027-periodic-window | 64.242% | 31.384% | 4.375% |
| eight-literals | 10.082% | 77.339% | 12.579% |
| random-32769 | 72.981% | 24.401% | 2.618% |
| repeated-32769 | 69.688% | 25.670% | 4.643% |
| reymont-prefix-32769 | 78.271% | 19.468% | 2.262% |

These describe maintained ordinary execution, not the experimental whole-native root. The host bucket includes real buffer/algorithm work and is not wholly removable overhead. Sample shares are not per-function retired-instruction counts.

## Validation and failure disposition

- 678 maintained tests pass / zero failures / same seven inherited exclusions, six focused checks separately pass. Exact GPU package features: canonical-native-program, canonical-native-program-runtime, canonical-native, canonical-native-runtime, aot and jit, matching the Cargo transitive closure. Frozen optimized dependencies, warnings as errors and unit opt1 remain.
- Original/decoded, inline/outlined, JIT/restored, disjoint-state disabled and nontrapping-home disabled cases preserve current arguments, finite control/library/comparison/memory/node/transport denials, first change/growth/short-capacity fallback, exact error order/prefix, aliases, unchanged prior output/backing, cleanup and retry.
- 52 real linked before/after original/decoded controls pass with matching compiler/runtime versions. Work and error strings agree exactly; assertion precedes failing arithmetic, zero trips and fresh inputs are exercised, prior publication survives failure and every failure is followed by success. Component Objects/metadata/executable images agree byte-for-byte between original/decoded forms.
- 40 all-input linked smoke calls and every timing/counter/profile call verify exact raw output and independent decompression. Final managed bytes/nodes are zero. Largest input33027 B. No multi-MB run.
- All 15 architecture/catalog checks and 132 workflow tests pass, with source checks repeated after the final CI count update. Actual CI selection lists exactly261 nonzero tests; its new test runs in the focused and complete suites. No filters/features/allowances/warning policy were weakened.
- The initial wrong-layer image diagnostic and omitted Object linker argument are resolved harness failures, retained with sources/commands. The invalid feature-subset build was another harness failure; Cargo’s real aot→jit closure already existed. No production workaround or semantics change was made for these diagnostic errors.
- Strict decoded MBC14 drives complete compression. A second full-MBC image compilation was not repeated; original/decoded component and maintained-suite tests are explicit. Full dependency/all-feature rebuild, separate ten-workload suite and completed remote CI remain skipped.
- Existing draft PR#973 only, one integration commit for this measured round plus the separately reviewable inherited CI repair. Ordinary CI cancelled by request; review automation retained. No green-CI or clean exact-head review claim.

## Brief disposition and next step

The maintained ordinary integration checkpoint is complete after root owner-transfer work (R211 diagnostic), the measured/unretained boundary experiment (R212), ordinary qualification (R213) and combined memory-home facts (R214). The full brief remains unfinished: stable owner-record pooling and payload allocation-site census are still separate unimplemented follow-ups. R211 retains the other whole-native improvements; no claim that all experimental authority omissions are restored.

Next finish the bounded allocation follow-up with a real source-site census and stable owner-record storage that preserves final-output payload layout, actual aliases, cleanup and retry. For further ordinary transport speed, eliminate a duplicated canonical→packet→canonical step using existing current-entry/exit and ownership facts. Do not treat smaller logical interfaces as proof of less physical copying, or reuse canonical alias tags for distinct packet addresses. Rejected R205/R206 variants are not reproposed unchanged.

Replay heap-summary.py then analyze.py on the evidence to reproduce the numbers without executing a compressor. Native reproduction requires the pinned toolchain/dependencies and recorded commands. Public evidence omits native binaries/Objects, bytecode, executable descriptors and raw traces.

## Public evidence

[Exact maintained source patch, all20 timing/counter results,678 tests,52 linked checks, clean-parent CI reproduction and fresh profiles](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r215-integrated-phase-83d6b42a65a1.tar.gz) (3311227 bytes; 711 files). SHA256 `83d6b42a65a174fe7a2b4934b2db21670167ff9ba1ad9add2119ba20de56fd12`. Server digests, downloaded file hashes and numerical replay match.
