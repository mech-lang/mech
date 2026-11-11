# R309 — Reuse repeated unsigned bounds checks

**Retain R309 with the R272 runtime.** R309 changes paired elapsed -0.525% across 81 fixed level-12 cases. The pooled and block timing gates pass 81/81 and 81/81. The primary cohort moves from 45/81 to 46/81 faster than Roc (56.79%). The matched peer cohort has 3/81 cases faster than libdeflate.

The two primary cohort changes are -0.391% and -0.659%. The highest paired ratio is +1.459% on empty. Crossings below parity include prefix-natural-4096. Crossings above include none.

Pooled gate misses include none. Block gate misses include none.

Measured and published parent bfc0b19277de46fda48a8a9b42c85d396c78ba55. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R308 closure and the attached patch. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, and IR/work/ABI/bytecode 215/107/39/14. The source program, bytecode, tables, limits, allocator, capacities and full-call timing boundary are identical.

## Reusing a comparison within one visit

The selected comparison dominates a later unsigned comparison with the same current SSA operands. Normalizing modular I64 additions allows the same cursor to appear as a one-based conversion followed by subtraction. The proof follows the original false route through effect-free instructions to the second test. The existing terminal rejection receives only values available at the first branch and values defined on the rejection route itself. The compiler redirects the first false edge to that rejection and makes the second branch unconditional on the remaining routes. The original status and origin stores remain in order. Any intervening memory operation or checked failure on the skipped route, a different operand, or an alternate predecessor outside the first test leaves the original code in place.

The initial experiment was limited to nested private phases and emitted the exact retained Object. The literal tally runs through the general numeric phase. The final experiment invokes the same proof there and selects no source names or input sizes.

## Linked machine evidence

44 static repeated checks are removed across 32 routines. 32 of 347 existing bodies change and 169 of 228 shared addresses move. The main tally body goes from 704 to 640 bytes. Its frame remains 192 bytes. The root body is byte-identical in the Object.

The literal update used two bounds comparisons around its read and write. The linked path now has a single comparison and conditional branch, with the original rejection and the load/add/store behind the guard. The linked Roc tally uses a bitfield-derived address and load/add/store. The separately compiled safe Rust witness admits a 256-cell slice at entry and lowers the literal field to UBFX in the loop. The matched libdeflate binary likewise has no local bounds branch on its frequency update. CLIF, VCode and all three linked instruction excerpts are in the packet.

The 28-case pilot passes both timing gates. In the pilot the 16 KiB random input moves from 1.113143× to 1.084006× Roc. The paired median changes -2.215% on that case and -0.589% across the pilot.

## Matched results

| Input | R308 / Roc | R309 / Roc | R308 / libdeflate | R309 / libdeflate | Primary elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.182902× | 0.182504× | 2.336349× | 2.331272× | -0.208% |
| prefix-natural-4096 | 1.000205× | 0.996521× | 1.709546× | 1.703248× | -0.728% |
| prefix-random-16384 | 1.114838× | 1.100570× | 3.033896× | 2.995069× | -1.792% |
| prefix-natural-32768 | 1.366986× | 1.365716× | 1.245824× | 1.244667× | -0.433% |
| prefix-natural-262144 | 1.389048× | 1.387369× | 1.126092× | 1.124730× | -0.197% |
| prefix-natural-1048576 | 1.403890× | 1.402181× | 1.131306× | 1.129929× | -0.541% |
| prefix-random-1048576 | 1.694342× | 1.674439× | 2.397382× | 2.369221× | -0.892% |
| prefix-periodic-1048576 | 0.740463× | 0.740256× | 1.253161× | 1.252810× | +0.029% |
| prefix-repeated-1048576 | 0.739780× | 0.739540× | 1.162214× | 1.161836× | +0.061% |

The four-actor cohort contains 245,664 measured calls and agrees on 81/81 raw streams. Libdeflate includes a fresh compressor in each call. The paired elapsed change and parity score use the primary cohorts.

## Independent instruction and heap counts

The six measured Mech payload and heap families agree across all 81 cases.

| Input | R308 instructions | R309 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 719,960.5 | 717,239.5 | -0.378% | +0.482% |
| prefix-natural-4096 | 9,535,490.0 | 9,513,337.0 | -0.232% | +0.096% |
| prefix-random-16384 | 10,928,791.5 | 10,733,170.0 | -1.790% | -0.944% |
| prefix-natural-32768 | 79,876,099.5 | 79,723,320.5 | -0.191% | -0.376% |
| prefix-natural-262144 | 806,837,753.5 | 805,568,460.5 | -0.157% | -0.334% |
| prefix-natural-1048576 | 2,912,174,582.5 | 2,908,150,611.0 | -0.138% | -0.236% |
| prefix-random-1048576 | 641,112,553.5 | 629,117,266.5 | -1.871% | -0.822% |
| prefix-periodic-1048576 | 517,505,839.0 | 517,445,840.0 | -0.012% | -0.149% |
| prefix-repeated-1048576 | 525,220,925.5 | 525,166,154.5 | -0.010% | +0.046% |

## Exact-image profiles

Running samples include validation activity and are separate from the elapsed and counter runs. For 16 KiB random, the retained profile attributed 2,144 of 4,768 running samples to the matcher and 1,031 to the tally. The peer profiles and linked PC maps are in the packet.

The closest input above libdeflate parity is 256 KiB natural. R308 measures 1.126092× and R309 measures 1.124730× libdeflate in the same balanced cohort. The respective R309 and libdeflate medians are 62.853 and 55.953 ms. Matching that peer ratio requires about 11.09% lower elapsed time.

The exact target profile assigns 2,147 of 5,607 running samples to the Mech matcher and 3,112 to the path parser. Libdeflate assigns 1,341 of 5,356 to its enclosing match-collection routine and 3,898 to its minimum-cost parser. The machine excerpts show libdeflate carrying child addresses in registers and using CMP/CCMP for the two stop conditions. Mech reloads the table pointer and emits two CSETs, ORR, UXT(B), and Boolean selection branches. These independent sample shares guide the next matcher experiment.

The profiler returned an internal export error, and another export attempt encountered a full volume. The original traces were recovered after reclaiming verified old build products. All profiles completed and the original timing and counter observations stayed pinned.

| Input | Running samples | Matcher | Main tally | Root |
|---|---:|---:|---:|---:|
| eight-literals | 5260 | 695 | 0 | 1622 |
| prefix-natural-1048576 | 5472 | 2370 | 53 | 82 |
| prefix-natural-262144 | 5607 | 2147 | 91 | 82 |
| prefix-natural-4096 | 5275 | 1630 | 116 | 386 |
| prefix-periodic-1048576 | 5352 | 3529 | 12 | 96 |
| prefix-random-1048576 | 5357 | 3369 | 993 | 21 |
| prefix-random-16384 | 5244 | 2391 | 1135 | 266 |
| prefix-repeated-1048576 | 5346 | 3239 | 9 | 175 |

## Qualification

The compiler and focused Rust test build used warnings as errors. All 51 CFG tests passed, including out-of-range visits, differing indices and lengths, intervening loads and stores, a path bypassing the first comparison, a cyclic false path, and unavailable error operands. The 36 linked controls agree on statuses, error origins, result, prior publications, aliases, heap counters and final cleanup. The original and decoded images are identical and pass 324 complete calls with exact bytes and independent inflation.

The patch is qualified in the compact native compiler experiment. The measured entry enforces source bounds and arithmetic. The ordinary managed admission and recovery interface is outside the timing scope.

The closest R309 primary case above parity is prefix-random-16384 at 1.112643× Roc.

## All primary cases

| Input | Bytes | R308 / Roc | R309 / Roc | R309 / R308 | Pooled / block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.083817× | 2.114210× | 1.014585× | pass/pass |
| one-stored | 1 | 2.000391× | 1.991542× | 0.995576× | pass/pass |
| seven-stored-prefix | 7 | 2.265913× | 2.245196× | 0.990857× | pass/pass |
| eight-literals | 8 | 0.188392× | 0.187999× | 0.997919× | pass/pass |
| eight-overlap | 8 | 0.175335× | 0.174119× | 0.993065× | pass/pass |
| 320-overlap | 320 | 0.202672× | 0.201651× | 0.994961× | pass/pass |
| eight-literals-prefix | 8 | 0.182954× | 0.181913× | 0.994308× | pass/pass |
| 128-biased-dynamic | 128 | 0.243636× | 0.243178× | 0.998117× | pass/pass |
| 32-high-entropy-stored | 32 | 0.250707× | 0.249030× | 0.993312× | pass/pass |
| 16384-distribution-change | 16384 | 1.422599× | 1.407498× | 0.989385× | pass/pass |
| 33027-periodic-window | 33027 | 0.659732× | 0.655927× | 0.994232× | pass/pass |
| random-64 | 64 | 0.264414× | 0.263203× | 0.995423× | pass/pass |
| random-256 | 256 | 0.316052× | 0.315106× | 0.997006× | pass/pass |
| random-1024 | 1024 | 0.374578× | 0.373152× | 0.996195× | pass/pass |
| repeated-1024 | 1024 | 0.243383× | 0.241993× | 0.994288× | pass/pass |
| random-32769 | 32769 | 1.357756× | 1.341508× | 0.988033× | pass/pass |
| repeated-32769 | 32769 | 0.654350× | 0.651038× | 0.994938× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.482297× | 0.480542× | 0.996361× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.316682× | 1.311117× | 0.995773× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.365771× | 1.363101× | 0.998046× | pass/pass |
| heldout-window-32767 | 32767 | 1.368204× | 1.365103× | 0.997733× | pass/pass |
| heldout-window-32768 | 32768 | 1.364390× | 1.361197× | 0.997660× | pass/pass |
| heldout-slides-65537 | 65537 | 1.402163× | 1.398666× | 0.997506× | pass/pass |
| heldout-random-65537 | 65537 | 1.509030× | 1.488758× | 0.986566× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.361782× | 1.350978× | 0.992067× | pass/pass |
| prefix-natural-1024 | 1024 | 0.485187× | 0.483190× | 0.995886× | pass/pass |
| prefix-natural-4096 | 4096 | 1.005912× | 0.998585× | 0.992717× | pass/pass |
| prefix-natural-8192 | 8192 | 1.213853× | 1.207740× | 0.994964× | pass/pass |
| prefix-natural-16384 | 16384 | 1.324067× | 1.319401× | 0.996476× | pass/pass |
| prefix-natural-32767 | 32767 | 1.372202× | 1.364881× | 0.994665× | pass/pass |
| prefix-natural-32768 | 32768 | 1.367978× | 1.362052× | 0.995668× | pass/pass |
| prefix-natural-32769 | 32769 | 1.364718× | 1.361329× | 0.997517× | pass/pass |
| prefix-natural-33027 | 33027 | 1.372714× | 1.365646× | 0.994851× | pass/pass |
| prefix-natural-65535 | 65535 | 1.374321× | 1.370985× | 0.997572× | pass/pass |
| prefix-natural-65536 | 65536 | 1.371615× | 1.373513× | 1.001384× | pass/pass |
| prefix-natural-65537 | 65537 | 1.366012× | 1.364228× | 0.998693× | pass/pass |
| prefix-natural-131072 | 131072 | 1.383083× | 1.381038× | 0.998521× | pass/pass |
| prefix-natural-262144 | 262144 | 1.389050× | 1.386313× | 0.998029× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.407137× | 1.399529× | 0.994593× | pass/pass |
| prefix-random-1024 | 1024 | 0.379115× | 0.377920× | 0.996846× | pass/pass |
| prefix-random-4096 | 4096 | 0.661234× | 0.647324× | 0.978964× | pass/pass |
| prefix-random-8192 | 8192 | 0.818681× | 0.809367× | 0.988624× | pass/pass |
| prefix-random-16384 | 16384 | 1.132947× | 1.112643× | 0.982079× | pass/pass |
| prefix-random-32767 | 32767 | 1.366227× | 1.348450× | 0.986988× | pass/pass |
| prefix-random-32768 | 32768 | 1.374415× | 1.355008× | 0.985879× | pass/pass |
| prefix-random-32769 | 32769 | 1.353959× | 1.337603× | 0.987920× | pass/pass |
| prefix-random-33027 | 33027 | 1.362100× | 1.341860× | 0.985140× | pass/pass |
| prefix-random-65535 | 65535 | 1.537553× | 1.519550× | 0.988291× | pass/pass |
| prefix-random-65536 | 65536 | 1.542464× | 1.525729× | 0.989151× | pass/pass |
| prefix-random-65537 | 65537 | 1.535280× | 1.514598× | 0.986529× | pass/pass |
| prefix-random-131072 | 131072 | 1.627724× | 1.611039× | 0.989750× | pass/pass |
| prefix-random-262144 | 262144 | 1.671033× | 1.654465× | 0.990085× | pass/pass |
| prefix-random-1048576 | 1048576 | 1.694943× | 1.679826× | 0.991081× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.308076× | 0.306406× | 0.994580× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.431176× | 0.424540× | 0.984609× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.493884× | 0.491425× | 0.995022× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.581921× | 0.579682× | 0.996153× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.642690× | 0.642633× | 0.999911× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.655361× | 0.650690× | 0.992872× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.655101× | 0.654009× | 0.998333× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.658816× | 0.657541× | 0.998064× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.696995× | 0.694820× | 0.996880× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.698456× | 0.694541× | 0.994395× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.697163× | 0.695817× | 0.998070× | pass/pass |
| prefix-periodic-131072 | 131072 | 0.726486× | 0.724887× | 0.997799× | pass/pass |
| prefix-periodic-262144 | 262144 | 0.734659× | 0.735122× | 1.000630× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 0.740010× | 0.740222× | 1.000286× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.244129× | 0.242775× | 0.994455× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.397039× | 0.391200× | 0.985292× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.460094× | 0.457754× | 0.994914× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.549348× | 0.549877× | 1.000963× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.635027× | 0.633454× | 0.997522× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.626565× | 0.627043× | 1.000763× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.648158× | 0.647125× | 0.998407× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.664733× | 0.662915× | 0.997265× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.695924× | 0.695540× | 0.999448× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.698216× | 0.696690× | 0.997815× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.697597× | 0.696414× | 0.998304× | pass/pass |
| prefix-repeated-131072 | 131072 | 0.729098× | 0.728710× | 0.999469× | pass/pass |
| prefix-repeated-262144 | 262144 | 0.740082× | 0.740469× | 1.000523× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 0.739298× | 0.739747× | 1.000607× | pass/pass |

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r309-repeated-bounds-39d04a53adb0.tar.gz). SHA-256 39d04a53adb00165b0c4a0d95992d9d2db584297d52ead7621fbb65b331f57bd.
