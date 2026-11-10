# R253: reuse current scalar predicate facts

**Retain R253.** All 81 balanced elapsed medians and independent instruction/cycle medians improve. Raw observation medians improve on 79 cases and tie on two near-resolution cases; none regress. Both fixed confirmation cohorts pass every unchanged 1.05 gate, and eight bytes stays below the 1x ceiling. Random 1 MiB improves 3.221790x -> 3.083160x Roc; natural text 2.240265x -> 2.174189x. Parity on larger inputs remains incomplete.

Git parent `d78da2862824993d16cf4958243feb36737d04be`; PR base `92f75db84e2523693648084bea16cd52ad60777a`. The patch affects nine paths in the archived R252 closure, including moving a shared proof helper, and reconstructs 822 source files. It does not apply directly to the maintained tree. Ordinary R215 remains `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`, IR/work/ABI/MBC 215/107/39/14. Source, bytecode, algorithm, tables, depth 300, nice length 258, near-EOF limits, real buffers, allocation sites, allocator and timing boundaries remain fixed.

## The proof and what it removes

A comparison produces a normalized 0/1 word. Once the false edge of `word == 1` is selected, that same current word must be zero. The former selected-range pass handled the full machine-word endpoints but missed this current interval endpoint. It retained an impossible invalid-Boolean arm and its large owner-cleanup paths. The R252 acyclic route experiment could forward some pure control, but stopped at effects and introduced extra transport parameters.

With `MECH_CURRENT_PREDICATES=1`, the compact root carries current immutable integer SSA facts down the unique selected edge’s dominator subtree. It intersects the actual current interval, inverts supported comparisons/extensions and rolls facts back before another path. Loads, stores and calls remain in order. A fresh load has a distinct SSA identity and inherits no fact about old memory. Shared incoming edges, unsupported interpretations and contradictions lose the fact. Raw values 2/MAX retain their invalid-input behavior; source Boolean type alone supplies no authority.

The pass reuses the R252 scalar condition helper, moved into shared word-range analysis, and the existing structural cleanup. It does not add result-transport parameters or infer alias, ownership or resource permission. No new native operation/region class is selected. The archived R252 forwarding flag remains disabled in these measurements.

Exactly 1,957 branches fold; 167,770 selected edges admit current facts. Root code shrinks **5,623,612 -> 940,264 bytes** (-83.2801%). The machine frame falls **55,136 -> 50,560 bytes** (-4,576 B). Static call sites fall **50,834 -> 4,215**; these are not executed callback counts. All 136 original routines, seven selected vector bodies and their metadata remain byte-identical.

The complete linked worker shrinks **6,640,168 -> 1,884,504 bytes**. The matched reference worker is 758,128 bytes; all include their harness/support. Code-size changes are separate from elapsed results. The final source-location image reproduces the entire measured Object/metadata. Disabling the new flag restores the complete R250 Object/metadata.

## Attribution control and conventional compiler comparison

A separately built compiler skips the new proof and runs the identical additional structural cleanup. It restores the entire R250 Object and, when linked with the same output basename, the entire measured R250 worker byte for byte. The gains therefore require the new proof, rather than merely repeating cleanup. No new timings are assigned to a changed control executable. The one-file ablation patch and complete source manifest are retained.

Independent C and Rust assembly witnesses use the same normalized-versus-raw scalar construct. Both produce three instructions for the normalized form and retain the explicit 987 fallback in the six-instruction raw form. This is a generic scalar proof, with no compression-specific selection. These witnesses are static compiler comparisons, not C/Rust compression timings. The pinned reference compressor and its existing collector/matcher inspection remain unchanged.

## Complete-call measurements

All 81 configurations use two primary cohorts, each with 24 balanced blocks and all six control/candidate/reference orders repeated four times. Warmup is 200 ms per actor/case. Level 12, runtime input and live borrowed output ownership match. Full compression, scratch construction, output transfer and private cleanup are timed. Preparation, validation, logging and prior-output drop are outside. CPU and heap are separate 24-block cohorts. No builds, tests, diagnostics or profiles overlap these cohorts.

Ratios use pooled observation medians against the same fresh reference. Elapsed changes use balanced block means. Instructions and cycles come from the independent counter cohort; do not multiply or conflate them.

| Case | Roc ms | R250 ms | R253 ms | R250 / Roc | R253 / Roc | Elapsed change | Instructions | Cycles |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| eight-literals | 0.289333 | 0.143646 | 0.124583 | 0.496470x | 0.430586x | -12.4937% | -4.3235% | -18.0527% |
| 33027-periodic-window | 1.269791 | 1.948063 | 1.808333 | 1.534159x | 1.424118x | -7.1727% | -4.1944% | -6.8302% |
| random-32769 | 1.067667 | 2.849479 | 2.688584 | 2.668885x | 2.518187x | -5.6465% | -4.9941% | -6.3336% |
| repeated-32769 | 1.350479 | 1.967896 | 1.839583 | 1.457184x | 1.362171x | -6.5203% | -4.1784% | -6.5586% |
| reymont-prefix-32769 | 4.374917 | 9.715479 | 9.320709 | 2.220723x | 2.130488x | -4.0633% | -1.9323% | -3.0009% |
| heldout-random-65537 | 1.941000 | 5.722979 | 5.429021 | 2.948469x | 2.797023x | -5.1365% | -5.0350% | -5.1381% |
| prefix-natural-1048576 | 161.127271 | 360.967792 | 350.321187 | 2.240265x | 2.174189x | -2.9495% | -1.5313% | -1.2893% |
| prefix-random-1048576 | 28.773438 | 92.701979 | 88.713125 | 3.221790x | 3.083160x | -4.3029% | -5.0483% | -4.3744% |
| prefix-periodic-1048576 | 31.405437 | 55.786834 | 53.104396 | 1.776343x | 1.690930x | -4.8084% | -4.1889% | -4.8915% |
| prefix-repeated-1048576 | 36.034188 | 58.863147 | 57.516730 | 1.633536x | 1.596171x | -2.2874% | -4.0864% | -5.0374% |

All 81 descriptive geometric elapsed change is -7.7411%; original large 11 is -6.1079%. No pooled elapsed/instruction/cycle row regresses; two raw observation medians tie while their balanced estimates improve. Every individual cohort observation remains available, including any contrary individual sample or near-resolution empty/one/seven-byte result.

Eight’s 12.4937% balanced elapsed reduction is distinct from its 4.3235% instruction and 18.0527% cycle reductions in a separate cohort. Natural 1 MiB improves 2.9495% elapsed, 1.5313% instructions and 1.2893% cycles. No complete per-function retired-instruction ledger or single-cause decomposition is claimed.

## Exactly 32 KiB

| Family | R250 / Roc | R253 / Roc | R253 ms | Balanced change |
|---|---:|---:|---:|---:|
| natural | 2.229170x | 2.131487x | 9.301854 | -4.3820% |
| random | 2.707886x | 2.538700x | 2.669708 | -6.2479% |
| periodic | 1.532381x | 1.428118x | 1.779791 | -6.8040% |
| repeated | 1.463455x | 1.369818x | 1.833187 | -6.3984% |

## Absolute counters and memory

| Case | Roc instructions | R250 instructions | R253 instructions |
|---|---:|---:|---:|
| eight-literals | 1,361,838.0 | 1,225,093.0 | 1,172,126.0 |
| heldout-random-65537 | 21,044,168.0 | 93,182,896.5 | 88,491,141.0 |
| prefix-natural-1048576 | 1,670,733,390.5 | 4,628,183,262.0 | 4,557,310,610.0 |
| prefix-random-1048576 | 320,162,750.5 | 1,514,401,096.0 | 1,437,949,254.0 |

All 81 payload requests, requested capacity, initialized copies, zeroing, private peaks and listed whole-heap medians remain exactly equal between Mech variants. Heap figures below are R253, also equal to R250. Requested bytes are not copied bytes; incremental live usable peak is the fixed heap probe’s metric, not OS RSS. Process/physical/lifetime figures include code, parsed metadata, harness and allocator retention.

| Case | Mech / Roc requests | Mech / Roc requested bytes | Mech / Roc incremental live usable peak | Mech initialized payload copied |
|---|---:|---:|---:|---:|
| eight-literals | 230 / 96 | 9,030,708 / 9,021,944 | 9,192,488 / 9,257,360 | 2,188 |
| prefix-natural-1048576 | 1,556 / 695 | 26,677,303 / 10,390,299 | 10,823,992 / 10,371,440 | 40,676 |
| prefix-random-1048576 | 914 / 373 | 26,493,368 / 10,213,500 | 10,824,168 / 10,371,440 | 15,316 |

## Fresh exact-image profile

| Case | Running samples | Generated root | Numeric phases | Other image/harness/library |
|---|---:|---:|---:|---:|
| eight-literals | 5475 | 1079 | 712 | 3684 |
| prefix-natural-1048576 | 5361 | 864 | 4444 | 53 |
| prefix-periodic-1048576 | 5502 | 2251 | 3166 | 85 |
| prefix-random-1048576 | 5552 | 3383 | 2131 | 38 |

These CPU-time samples include harness/validation and are not retired-instruction partitions or removable-time budgets. Current hot-window disassembly still shows root carrier movement on random input; natural text remains dominated by existing numeric path phases. Old image addresses are never applied to the new executable.

## Qualification, failures and coverage

712 maintained tests pass with the same seven inherited exclusions. Five new focused tests contain 3,942 before/after native calls and are included in that inventory. They cover normalized/raw invalid flags, repeated arguments, fresh visits/captured predicates, float bit patterns/NaNs, parallel payload ordering/shared joins, assertion-versus-overflow order, effects/calls and new loads after mutation. Existing R252 forwarding tests also remain passing.

3,500 independent source calls across six programs/seven execution variants and original/decoded forms pass, including 700 zero trips. Current values, matrix geometry, input/output aliases, prior publication/backing, failure output, cleanup and retry agree. All seven small original/decoded image pairs are exact. The 162 immediate complete calls and every primary/counter/profile call retain exact compressed bytes, independent inflate, aliases, capacity and zero final private payload/owners. Another 162 observed complete calls reproduce all 81 R250 native-phase counts exactly. Instrumentation is absent from timing workers.

Builds retain warnings-as-errors, opt3 and aot/jit/canonical-native/canonical-native-runtime/canonical-native-program/canonical-native-program-runtime. The test root uses opt1 with pinned nightly-2026-03-03 on Apple ARM64. Existing R250 extended matrix/buffer/boundary qualification is inherited, not relabeled fresh. Full candidate/disabled decoded compression images are not emitted; decoded execution is covered by the seven source pairs and maintained suite.

One extra setup assertion initially compared a differently named ablation executable with the baseline and failed byte identity. Rebuilding with the same basename restores the entire baseline byte for byte. Both measured actors already used the same basename. The failed file/hash and corrected comparison are preserved. The strict patch check also detected that BSD patch left the deleted helper as an empty file; the explicit -E reconstruction recipe removes it and verifies the exact file set. No introduced semantic failure or allowance increase remains. Previous environmental allocator/checkout recovery is inherited from R252, with the same verified source pins.

The seven maintained exclusions, ordinary extra-contract omissions and enclosing mutable-child funding/native-selection refusal remain inherited. Ordinary IR215 and its exact managed resource/admission/accounting/publication/recovery path are unchanged. The compact executable still omits that additional managed contract; its tested private cleanup/alias/error behavior is not a claim of restoring it. No completed hosted CI run is claimed; exact-head CI is cancelled under the standing instruction and review workflows are preserved.

## Every measured configuration

| Case | Bytes | R250 / Roc | R253 / Roc | Balanced change | Cohort A | Cohort B | Instructions |
|---|---:|---:|---:|---:|---:|---:|---:|
| empty | 0 | 1.976190x | 1.976190x | -13.0590% | -13.2418% | -13.1360% | -0.9192% |
| one-stored | 1 | 2.000000x | 1.976190x | -23.8669% | -23.3284% | -24.1405% | -1.2862% |
| seven-stored-prefix | 7 | 1.976190x | 1.976190x | -18.8236% | -18.0407% | -19.5812% | -1.2761% |
| eight-literals | 8 | 0.496470x | 0.430586x | -12.4937% | -11.2105% | -14.5125% | -4.3235% |
| eight-overlap | 8 | 0.490952x | 0.428488x | -13.0673% | -12.4876% | -14.0070% | -4.2322% |
| 320-overlap | 320 | 0.544897x | 0.476647x | -12.6865% | -12.1805% | -12.9323% | -4.1314% |
| eight-literals-prefix | 8 | 0.487295x | 0.430005x | -11.0230% | -10.6152% | -12.4605% | -4.3234% |
| 128-biased-dynamic | 128 | 0.498919x | 0.436179x | -12.5528% | -12.3767% | -12.7740% | -4.5889% |
| 32-high-entropy-stored | 32 | 0.523856x | 0.450353x | -14.7901% | -14.0485% | -15.0826% | -5.1060% |
| 16384-distribution-change | 16384 | 2.688227x | 2.323779x | -13.5572% | -13.5995% | -13.7935% | -4.9444% |
| 33027-periodic-window | 33027 | 1.534159x | 1.424118x | -7.1727% | -6.8507% | -7.4330% | -4.1944% |
| random-64 | 64 | 0.560038x | 0.478032x | -14.9124% | -15.1274% | -14.4720% | -5.4364% |
| random-256 | 256 | 0.654488x | 0.554668x | -14.9916% | -15.7018% | -13.7753% | -6.1342% |
| random-1024 | 1024 | 0.737837x | 0.656141x | -11.1018% | -10.8402% | -11.2505% | -5.3625% |
| repeated-1024 | 1024 | 0.634346x | 0.569490x | -10.6173% | -9.0893% | -12.5141% | -3.8866% |
| random-32769 | 32769 | 2.668885x | 2.518187x | -5.6465% | -5.4433% | -6.3682% | -4.9941% |
| repeated-32769 | 32769 | 1.457184x | 1.362171x | -6.5203% | -6.1587% | -6.9611% | -4.1784% |
| reymont-prefix-1024 | 1024 | 0.915964x | 0.807463x | -12.0746% | -11.6116% | -12.6932% | -4.2161% |
| reymont-prefix-16384 | 16384 | 2.178044x | 2.060267x | -5.4075% | -5.2727% | -5.5604% | -2.1837% |
| reymont-prefix-32769 | 32769 | 2.220723x | 2.130488x | -4.0633% | -4.0709% | -4.0779% | -1.9323% |
| heldout-window-32767 | 32767 | 2.227375x | 2.132613x | -4.2544% | -4.1009% | -4.3551% | -1.9363% |
| heldout-window-32768 | 32768 | 2.227672x | 2.135918x | -4.1188% | -4.1047% | -4.1646% | -1.9372% |
| heldout-slides-65537 | 65537 | 2.293668x | 2.205655x | -3.8372% | -3.7299% | -3.9128% | -1.8154% |
| heldout-random-65537 | 65537 | 2.948469x | 2.797023x | -5.1365% | -5.0488% | -5.2730% | -5.0350% |
| heldout-mixed-131073 | 131073 | 2.442930x | 2.270648x | -7.0523% | -7.0113% | -7.0726% | -3.3384% |
| prefix-natural-1024 | 1024 | 0.915214x | 0.804925x | -11.9433% | -11.7769% | -12.4198% | -4.2162% |
| prefix-natural-4096 | 4096 | 1.720220x | 1.569873x | -8.7400% | -9.3731% | -9.3728% | -2.5199% |
| prefix-natural-8192 | 8192 | 2.003041x | 1.856638x | -7.3090% | -6.9783% | -8.0350% | -2.2761% |
| prefix-natural-16384 | 16384 | 2.190738x | 2.056135x | -6.1442% | -5.9833% | -6.3576% | -2.1836% |
| prefix-natural-32767 | 32767 | 2.231778x | 2.134866x | -4.3424% | -4.4237% | -4.3424% | -1.9358% |
| prefix-natural-32768 | 32768 | 2.229170x | 2.131487x | -4.3820% | -4.3876% | -4.3898% | -1.9351% |
| prefix-natural-32769 | 32769 | 2.228753x | 2.131146x | -4.3795% | -4.4338% | -4.3833% | -1.9359% |
| prefix-natural-33027 | 33027 | 2.225623x | 2.130753x | -4.2626% | -4.0520% | -4.3652% | -1.9312% |
| prefix-natural-65535 | 65535 | 2.201721x | 2.123837x | -3.5374% | -3.3930% | -3.6938% | -1.5582% |
| prefix-natural-65536 | 65536 | 2.201227x | 2.124178x | -3.5003% | -3.2589% | -3.6611% | -1.5578% |
| prefix-natural-65537 | 65537 | 2.243909x | 2.166866x | -3.4334% | -3.3983% | -3.6840% | -1.5511% |
| prefix-natural-131072 | 131072 | 2.201440x | 2.128889x | -3.2956% | -3.2629% | -3.3057% | -1.4612% |
| prefix-natural-262144 | 262144 | 2.205956x | 2.140922x | -2.9481% | -2.9593% | -2.9356% | -1.4108% |
| prefix-natural-1048576 | 1048576 | 2.240265x | 2.174189x | -2.9495% | -3.0410% | -2.9185% | -1.5313% |
| prefix-random-1024 | 1024 | 0.737984x | 0.658209x | -10.5867% | -10.3591% | -10.8738% | -5.3646% |
| prefix-random-4096 | 4096 | 1.358550x | 1.210273x | -10.9144% | -11.0372% | -11.0183% | -5.3357% |
| prefix-random-8192 | 8192 | 1.726901x | 1.564482x | -9.4052% | -9.3142% | -10.3098% | -5.2385% |
| prefix-random-16384 | 16384 | 2.281403x | 2.112230x | -7.4153% | -7.4514% | -7.2170% | -5.0863% |
| prefix-random-32767 | 32767 | 2.692352x | 2.529481x | -6.0494% | -6.1271% | -5.8649% | -5.0174% |
| prefix-random-32768 | 32768 | 2.707886x | 2.538700x | -6.2479% | -6.3236% | -6.1271% | -5.0173% |
| prefix-random-32769 | 32769 | 2.684537x | 2.523804x | -5.9874% | -5.8474% | -6.1857% | -4.9917% |
| prefix-random-33027 | 33027 | 2.702402x | 2.529977x | -6.3804% | -6.1063% | -6.4612% | -4.9910% |
| prefix-random-65535 | 65535 | 2.957766x | 2.797302x | -5.4252% | -5.4460% | -5.3971% | -4.9543% |
| prefix-random-65536 | 65536 | 2.962113x | 2.799693x | -5.4833% | -5.7138% | -5.2784% | -4.9509% |
| prefix-random-65537 | 65537 | 2.948459x | 2.792447x | -5.2913% | -5.6722% | -5.0734% | -4.9375% |
| prefix-random-131072 | 131072 | 3.104938x | 2.924923x | -5.7977% | -5.9476% | -5.7116% | -5.0019% |
| prefix-random-262144 | 262144 | 3.143963x | 3.000630x | -4.5590% | -4.7528% | -4.4654% | -4.9437% |
| prefix-random-1048576 | 1048576 | 3.221790x | 3.083160x | -4.3029% | -4.3541% | -4.2008% | -5.0483% |
| prefix-periodic-1024 | 1024 | 0.657210x | 0.584372x | -11.0898% | -10.5869% | -11.5580% | -4.1396% |
| prefix-periodic-4096 | 4096 | 1.018763x | 0.896756x | -11.9760% | -10.3617% | -13.0181% | -4.0072% |
| prefix-periodic-8192 | 8192 | 1.176310x | 1.077666x | -8.3859% | -7.8895% | -8.8818% | -3.9579% |
| prefix-periodic-16384 | 16384 | 1.370205x | 1.260670x | -7.9941% | -7.2407% | -8.4119% | -4.2239% |
| prefix-periodic-32767 | 32767 | 1.536621x | 1.430592x | -6.9001% | -6.7505% | -7.1297% | -4.2206% |
| prefix-periodic-32768 | 32768 | 1.532381x | 1.428118x | -6.8040% | -6.5463% | -7.1653% | -4.2206% |
| prefix-periodic-32769 | 32769 | 1.533775x | 1.428978x | -6.8326% | -6.5569% | -7.0253% | -4.1942% |
| prefix-periodic-33027 | 33027 | 1.531628x | 1.430057x | -6.6316% | -6.8143% | -7.1009% | -4.1944% |
| prefix-periodic-65535 | 65535 | 1.655486x | 1.555904x | -6.0153% | -5.9703% | -5.7259% | -4.2054% |
| prefix-periodic-65536 | 65536 | 1.652601x | 1.552138x | -6.0791% | -5.9523% | -6.0542% | -4.2054% |
| prefix-periodic-65537 | 65537 | 1.655574x | 1.553409x | -6.1710% | -6.4010% | -6.0995% | -4.1920% |
| prefix-periodic-131072 | 131072 | 1.715754x | 1.623045x | -5.4034% | -5.3951% | -5.5159% | -4.1946% |
| prefix-periodic-262144 | 262144 | 1.749652x | 1.662526x | -4.9796% | -4.7375% | -5.1149% | -4.1927% |
| prefix-periodic-1048576 | 1048576 | 1.776343x | 1.690930x | -4.8084% | -4.7982% | -4.7414% | -4.1889% |
| prefix-repeated-1024 | 1024 | 0.630400x | 0.566308x | -9.9666% | -10.4391% | -9.5142% | -3.8869% |
| prefix-repeated-4096 | 4096 | 0.970059x | 0.858050x | -11.5467% | -10.0844% | -11.9227% | -3.9190% |
| prefix-repeated-8192 | 8192 | 1.143431x | 1.048841x | -8.2725% | -8.4889% | -8.0991% | -3.9092% |
| prefix-repeated-16384 | 16384 | 1.305312x | 1.207345x | -7.5053% | -6.6093% | -7.8942% | -4.1950% |
| prefix-repeated-32767 | 32767 | 1.456847x | 1.373327x | -5.7329% | -5.5634% | -6.4948% | -4.2053% |
| prefix-repeated-32768 | 32768 | 1.463455x | 1.369818x | -6.3984% | -6.2775% | -6.4940% | -4.2049% |
| prefix-repeated-32769 | 32769 | 1.466358x | 1.369173x | -6.6277% | -6.1576% | -7.2389% | -4.1784% |
| prefix-repeated-33027 | 33027 | 1.464600x | 1.372711x | -6.2740% | -6.1189% | -6.3752% | -4.1786% |
| prefix-repeated-65535 | 65535 | 1.569091x | 1.482657x | -5.5086% | -5.5937% | -5.5530% | -4.1974% |
| prefix-repeated-65536 | 65536 | 1.571538x | 1.484218x | -5.5563% | -5.9049% | -5.8958% | -4.1974% |
| prefix-repeated-65537 | 65537 | 1.581430x | 1.486302x | -6.0153% | -5.6800% | -5.8266% | -4.1839% |
| prefix-repeated-131072 | 131072 | 1.631247x | 1.545218x | -5.2738% | -5.3820% | -5.3104% | -4.1949% |
| prefix-repeated-262144 | 262144 | 1.659798x | 1.573378x | -5.2066% | -5.3509% | -5.0226% | -4.1916% |
| prefix-repeated-1048576 | 1048576 | 1.633536x | 1.596171x | -2.2874% | -5.0253% | -4.5901% | -4.0864% |

## Remaining brief

This retains a general predicate-proof improvement, not the broad archived R252 transport variant. Empty outer OptimalPathMatches attribution/forwarding, an enclosing multi-position recurrence with exact cold handoff, and the single R247 placement control remain outstanding. The funding refusal is not relaxed. Continue from the new profile; do not repeat completed phase-only isolation. Any combination with archived forwarding must have its own measurements and physical transport evidence.

The packet contains exact source/patches/manifests, all 81 raw aggregate cohorts/portable inputs and expected bytes, current-image aggregate PCs and narrow disassembly, C/Rust scalar analogues, build/test receipts and failure dispositions. It excludes native executables/Objects, bytecode, complete compressor descriptors/IR/maps, raw traces/stacks and secrets. Offline replay reconstructs five reports without executing a compiler or compressor.

## Public evidence

[Evidence archive](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r253-current-predicates-d1a351896db9.tar.gz), 18,669,710 bytes, SHA-256 `d1a351896db95a70115194d1583a7b0a3f7271535fe5997ccc2d9a3881b950d7`. The downloaded packet verifies all 2,098 hashes, five byte-identical offline reports and reconstruction of both the 822-file compiler and its separate structural-only ablation. Native executables/Objects, bytecode, complete compressor descriptors/IR/maps and raw traces are excluded.
