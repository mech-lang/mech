# R306 — Guard bounded indexed updates inside the native phase

**Retain R304 with the R272 runtime.** Paired elapsed changes -0.675% across the fixed 81 level-12 cases. The pooled and block timing gates pass 81/81 and 79/81. R304 beats Roc on 45/81 cases, and R306 beats it on 45/81 (55.56%). The four-way cohort has 3/81 below libdeflate.

Primary cohort changes are -0.715% and -0.634%. The highest paired elapsed ratio is +7.575% on empty. Crossings below 1× Roc include none. Crossings above include none.

Pooled gate misses include none. Block gate misses include empty, one-stored.

| Missed case | Paired elapsed change | Block change A | Block change B | Retired instructions before / after | Cycles before / after |
|---|---:|---:|---:|---:|---:|
| empty | +7.575% | +6.478% | +9.690% | 6,268 / 6,268 | 2,927.0 / 2,832.0 |
| one-stored | +7.003% | +6.116% | +8.011% | 6,384 / 6,384 | 2,925.5 / 2,879.0 |

Both elapsed cohorts miss the same two block gates. The separate hardware cohort records identical instruction counts and fewer cycles for those cases. These observations leave the elapsed regressions unattributed. The gate results govern retention.

Measured parent 9283b56817d59042eb73420d0e05a3bc9ec2216f. Published parent f4836ed924b4e4f20d621296083293089ab84e4f. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R304 closure and eight changed compiler files. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The benchmark source, level, tables, allocator, capacities and complete-call timing contract are pinned.

## Bound proof and machine placement

The caller admits the same current owners, capacities and geometry. Inside a complete fixed-view numeric phase the compiler proves a finite positive index range for a changing indexed update. The actual packet length is checked before the first source operation. A passing view can use that length in the existing traversal proof, including through exact frame moves. The original indexed address calculation applies on both routes. A shorter view calls the original checked body with the same packet and publishes through the same caller.

Six of 347 existing bodies change, from 704 to 692 bytes each. The literal update goes from 13 recurring instructions to six. The single new entry branch precedes the loop. Six checked alternatives are byte-identical to their original Object bodies. The linker retains three checked bodies. The original 57,104-byte root frame and Object machine instructions are identical.

The first placement put checked bodies before the root and moved it by 2,304 bytes. Deferring their definitions to after the root keeps all 227 existing compact function addresses. Both placements emit the same phase instructions and symbolic relocations. The final linked root changes 234 words at the same symbolic data-address relocations. The raw instruction and relocation records are included.

| Timing screen | Paired elapsed change | Block gate misses |
|---|---:|---|
| First local guard | -0.277% | empty, one-stored |
| Same first placement in the four-way screen | -0.391% | one-stored |
| Deferred placement in the four-way screen | -0.401% | none |

## Matched ratios

| Input | R304 / Roc | R306 / Roc | R304 / libdeflate | R306 / libdeflate | Paired elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.194884× | 0.195143× | 2.476163× | 2.479454× | +0.018% |
| prefix-random-8192 | 0.802185× | 0.797779× | 2.889884× | 2.874010× | -2.420% |
| prefix-natural-4096 | 0.994263× | 0.990673× | 1.709139× | 1.702966× | -1.288% |
| prefix-natural-32768 | 1.367403× | 1.365188× | 1.246020× | 1.244001× | -0.587% |
| prefix-natural-1048576 | 1.404100× | 1.403708× | 1.133464× | 1.133148× | -0.018% |
| prefix-random-1048576 | 1.690447× | 1.647253× | 2.396628× | 2.335389× | -2.683% |
| prefix-periodic-1048576 | 0.742053× | 0.741241× | 1.256932× | 1.255556× | +0.073% |
| prefix-repeated-1048576 | 0.740299× | 0.739935× | 1.167137× | 1.166563× | +0.082% |

The four-way cohort contains 245,664 measured calls. Reference ratios use its balanced blocks. The elapsed-change column and parity score use the two primary cohorts.

## Qualification and independent hardware counters

- All 324 original and decoded calls match the expected output and independently inflate. Both artifacts agree byte-for-byte. Capacities and held aliases pass, and final private memory and owners are zero.
- The 36 linked controls confirm the exact floor, shorter fallback, zero trips, count wrapping, late bounds rejection, lexical precedence, stable prior publication and retry. The 2 phase-bound and 49 cleanup tests pass with warnings as errors.
- The six tracked heap families agree on all 81 cases.

| Input | R304 instructions | R306 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 772,988.0 | 772,755.5 | -0.030% | +3.112% |
| prefix-random-8192 | 5,721,729.0 | 5,609,239.5 | -1.966% | -3.024% |
| prefix-natural-4096 | 9,606,556.0 | 9,586,656.5 | -0.207% | -0.365% |
| prefix-natural-32768 | 80,075,192.5 | 79,983,318.5 | -0.115% | -0.329% |
| prefix-natural-1048576 | 2,912,654,497.0 | 2,911,112,877.0 | -0.053% | +0.026% |
| prefix-random-1048576 | 641,386,570.0 | 620,421,321.0 | -3.269% | -2.405% |
| prefix-periodic-1048576 | 517,899,943.5 | 517,899,112.0 | -0.000% | -0.076% |
| prefix-repeated-1048576 | 525,866,638.5 | 525,872,558.5 | +0.001% | -0.004% |

Timing directions differ between the two cohorts on 21 cases. All observations remain in the archive.

## Exact executable profiles

Running CPU samples include the worker and validation. Hardware instructions and cycles come from the separate counter cohort.

| Input | Running samples before / after | Literal tally before / after | Matcher before / after |
|---|---:|---:|---:|
| eight-literals | 5461 / 5288 | 1 / 5 | 646 / 623 |
| prefix-natural-1048576 | 5491 / 5357 | 82 / 85 | 2375 / 2304 |
| prefix-natural-32768 | 5422 / 5359 | 133 / 139 | 2081 / 2043 |
| prefix-natural-4096 | 5566 / 5531 | 102 / 107 | 1673 / 1650 |
| prefix-periodic-1048576 | 5386 / 5489 | 16 / 14 | 3477 / 3572 |
| prefix-random-1048576 | 5412 / 5497 | 1042 / 951 | 3399 / 3531 |
| prefix-repeated-1048576 | 5485 / 5362 | 9 / 15 | 3298 / 3230 |

The closest candidate case above parity is prefix-natural-4096 at 1.004575× Roc. For random input, inspect whether the existing read guard and later write guard can share one current-view predicate inside the original checked body. The false edge must preserve the original write rejection and the source operations that precede it. This would avoid adding a second callee or a phase-entry check. A separate parity-frontier experiment can address the short natural-text reduction path.

The timing scope is the experimental compact entry. Ordinary managed integration remains open.

## All primary timings

| Input | Bytes | R304 / Roc | R306 / Roc | R306 / R304 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.099631× | 2.258685× | 1.075754× | pass/miss |
| one-stored | 1 | 1.999695× | 2.139738× | 1.070032× | pass/miss |
| seven-stored-prefix | 7 | 2.223784× | 2.278891× | 1.024781× | pass/pass |
| eight-literals | 8 | 0.194736× | 0.194772× | 1.000184× | pass/pass |
| eight-overlap | 8 | 0.183388× | 0.183272× | 0.999369× | pass/pass |
| 320-overlap | 320 | 0.209700× | 0.209688× | 0.999942× | pass/pass |
| eight-literals-prefix | 8 | 0.193800× | 0.194024× | 1.001155× | pass/pass |
| 128-biased-dynamic | 128 | 0.255173× | 0.254657× | 0.997976× | pass/pass |
| 32-high-entropy-stored | 32 | 0.265319× | 0.264132× | 0.995526× | pass/pass |
| 16384-distribution-change | 16384 | 1.454073× | 1.416004× | 0.973819× | pass/pass |
| 33027-periodic-window | 33027 | 0.665067× | 0.659666× | 0.991879× | pass/pass |
| random-64 | 64 | 0.279844× | 0.278829× | 0.996374× | pass/pass |
| random-256 | 256 | 0.331169× | 0.328726× | 0.992625× | pass/pass |
| random-1024 | 1024 | 0.388254× | 0.386303× | 0.994974× | pass/pass |
| repeated-1024 | 1024 | 0.253285× | 0.252871× | 0.998365× | pass/pass |
| random-32769 | 32769 | 1.410881× | 1.378496× | 0.977046× | pass/pass |
| repeated-32769 | 32769 | 0.671544× | 0.661867× | 0.985590× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.488095× | 0.487192× | 0.998150× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.317717× | 1.317994× | 1.000210× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.370482× | 1.365334× | 0.996243× | pass/pass |
| heldout-window-32767 | 32767 | 1.372478× | 1.369657× | 0.997945× | pass/pass |
| heldout-window-32768 | 32768 | 1.371722× | 1.367380× | 0.996834× | pass/pass |
| heldout-slides-65537 | 65537 | 1.450726× | 1.379241× | 0.950724× | pass/pass |
| heldout-random-65537 | 65537 | 1.516040× | 1.469462× | 0.969276× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.364910× | 1.342981× | 0.983934× | pass/pass |
| prefix-natural-1024 | 1024 | 0.494730× | 0.492861× | 0.996222× | pass/pass |
| prefix-natural-4096 | 4096 | 1.017680× | 1.004575× | 0.987123× | pass/pass |
| prefix-natural-8192 | 8192 | 1.207926× | 1.199725× | 0.993211× | pass/pass |
| prefix-natural-16384 | 16384 | 1.319766× | 1.318272× | 0.998868× | pass/pass |
| prefix-natural-32767 | 32767 | 1.369259× | 1.365621× | 0.997343× | pass/pass |
| prefix-natural-32768 | 32768 | 1.379132× | 1.371036× | 0.994129× | pass/pass |
| prefix-natural-32769 | 32769 | 1.372079× | 1.366875× | 0.996207× | pass/pass |
| prefix-natural-33027 | 33027 | 1.371301× | 1.367870× | 0.997498× | pass/pass |
| prefix-natural-65535 | 65535 | 1.378481× | 1.375524× | 0.997855× | pass/pass |
| prefix-natural-65536 | 65536 | 1.379162× | 1.378626× | 0.999612× | pass/pass |
| prefix-natural-65537 | 65537 | 1.371864× | 1.371425× | 0.999680× | pass/pass |
| prefix-natural-131072 | 131072 | 1.385037× | 1.382097× | 0.997877× | pass/pass |
| prefix-natural-262144 | 262144 | 1.389027× | 1.385756× | 0.997645× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.405096× | 1.404844× | 0.999820× | pass/pass |
| prefix-random-1024 | 1024 | 0.387585× | 0.384701× | 0.992561× | pass/pass |
| prefix-random-4096 | 4096 | 0.644560× | 0.633796× | 0.983301× | pass/pass |
| prefix-random-8192 | 8192 | 0.815072× | 0.795350× | 0.975803× | pass/pass |
| prefix-random-16384 | 16384 | 1.123498× | 1.086237× | 0.966834× | pass/pass |
| prefix-random-32767 | 32767 | 1.380267× | 1.344989× | 0.974441× | pass/pass |
| prefix-random-32768 | 32768 | 1.366391× | 1.320726× | 0.966580× | pass/pass |
| prefix-random-32769 | 32769 | 1.356959× | 1.316868× | 0.970455× | pass/pass |
| prefix-random-33027 | 33027 | 1.373213× | 1.327310× | 0.966573× | pass/pass |
| prefix-random-65535 | 65535 | 1.541128× | 1.494865× | 0.969981× | pass/pass |
| prefix-random-65536 | 65536 | 1.542264× | 1.495621× | 0.969757× | pass/pass |
| prefix-random-65537 | 65537 | 1.536620× | 1.493323× | 0.971823× | pass/pass |
| prefix-random-131072 | 131072 | 1.627229× | 1.582122× | 0.972280× | pass/pass |
| prefix-random-262144 | 262144 | 1.672619× | 1.630786× | 0.974989× | pass/pass |
| prefix-random-1048576 | 1048576 | 1.696460× | 1.650948× | 0.973172× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.315512× | 0.315493× | 0.999939× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.430601× | 0.424822× | 0.986579× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.500810× | 0.497932× | 0.994253× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.584469× | 0.579665× | 0.991780× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.645659× | 0.645527× | 0.999796× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.652925× | 0.650542× | 0.996350× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.655736× | 0.652997× | 0.995823× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.664373× | 0.658775× | 0.991573× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.697574× | 0.695849× | 0.997528× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.699968× | 0.698559× | 0.997987× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.699016× | 0.700355× | 1.001916× | pass/pass |
| prefix-periodic-131072 | 131072 | 0.727713× | 0.725246× | 0.996610× | pass/pass |
| prefix-periodic-262144 | 262144 | 0.735465× | 0.735800× | 1.000455× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 0.740766× | 0.741307× | 1.000730× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.253811× | 0.252978× | 0.996719× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.386638× | 0.383045× | 0.990707× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.461235× | 0.461025× | 0.999546× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.554501× | 0.553758× | 0.998660× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.624838× | 0.625416× | 1.000925× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.628474× | 0.627100× | 0.997814× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.649911× | 0.647318× | 0.996010× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.669298× | 0.663959× | 0.992022× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.697180× | 0.697069× | 0.999840× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.699142× | 0.696870× | 0.996751× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.700466× | 0.698956× | 0.997845× | pass/pass |
| prefix-repeated-131072 | 131072 | 0.731786× | 0.729990× | 0.997546× | pass/pass |
| prefix-repeated-262144 | 262144 | 0.742039× | 0.741001× | 0.998601× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 0.741197× | 0.741804× | 1.000818× | pass/pass |

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r306-local-tally-bounds-e3dbe45fafb2.tar.gz). SHA-256 e3dbe45fafb2471264e9e3d619f482a88ed6d7d1e90a90ca861961649b97c929.
