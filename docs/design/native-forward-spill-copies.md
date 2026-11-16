# R292 — Retain spill copy facts across taken branches

**Retain R292 with the R272 runtime.** Candidate paired elapsed changes -0.765% across 81 fixed level-12 cases. The pooled and block gates pass 81/81 and 81/81. R291 beats Roc on 45/81 cases and R292 beats it on 45/81 (55.56%). The separate four-actor cohort gives R292 3/81 cases below libdeflate.

The two primary cohorts change elapsed by -0.632% and -0.897%. The largest paired slowdown is +0.917% on seven-stored-prefix. Cases crossing below 1× Roc are none. Cases crossing above are none.

Pooled gate misses are none. Block gate misses are none.

Published and measured parent 67ede1cf4acb32c588ba3a90ab78ec70f3ddb05a. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R291 source closure and two modified files in regalloc2 0.15.2. Apple ARM64, Cranelift 0.131.3 and IR/work/ABI/bytecode 215/107/39/14. The level-12 source, inputs, tables, capacities, allocator runtime and complete-call timer are pinned.

## Allocator rule and generated code

After resolving current edits and scanning ordinary definitions and clobbers through the end of a block, the allocator saves its physical copy state for a later successor when that successor has exactly one actual predecessor edge. The successor takes this state at its layout entry. The intervening layout has its own state. An entry, join or backward edge starts with empty facts. The existing resolver still executes its ordered parallel-copy edits and invalidates writes to either allocation and dedicated scratch clobbers. The rule is independent of the compressor and applies after placement.

The two original witnesses are x3 at stack offset 568 and x12 at offset 288. Each predecessor loads the value, compares it and branches across the other arm. In the candidate the selected successor uses the same register immediately. The VCode and linked assembly verify both eliminated loads. The actual reference binaries illustrate the same register-lifetime objective. Roc keeps its window delta in x12 from subtraction through the comparison and encoding. Libdeflate keeps a signed tree link in w15 from the load through the comparison and subsequent indexed input read. These excerpts cover the corresponding operations in each implementation.

| Body or static transfer count | R291 | R292 |
|---|---:|---:|
| Entry code bytes | 1,043,368 | 997,660 |
| General matcher code bytes | 51,856 | 51,024 |
| Vector matcher code bytes | 27,124 | 26,520 |
| Vector matcher stack load instructions | 1116 | 987 |
| Vector matcher stack store instructions | 225 | 204 |

31 of 347 bodies change. All retain their frame sizes. The vector matcher frame is 880 bytes and the entry frame is 57,072 bytes. The four no-growth parser bodies retain their code bytes.

The 27-case screen changes paired elapsed by -0.737%. It records 0 pooled and 0 block misses. The complete screen and both full cohorts are archived.

## Headline matched ratios

| Input | R291 / Roc | R292 / Roc | R291 / libdeflate | R292 / libdeflate | R292 / R291 elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.197142× | 0.194734× | 2.528726× | 2.497838× | -0.142% |
| prefix-random-8192 | 0.861287× | 0.847291× | 3.109778× | 3.059246× | -1.092% |
| prefix-natural-4096 | 1.063860× | 1.055261× | 1.817828× | 1.803135× | -0.562% |
| prefix-natural-32768 | 1.439279× | 1.444705× | 1.308548× | 1.313480× | +0.128% |
| prefix-natural-1048576 | 1.473780× | 1.479314× | 1.188070× | 1.192531× | +0.365% |
| prefix-random-1048576 | 1.775667× | 1.765666× | 2.510527× | 2.496386× | -0.383% |
| prefix-periodic-1048576 | 0.824648× | 0.812344× | 1.396811× | 1.375970× | -1.432% |
| prefix-repeated-1048576 | 0.815422× | 0.804040× | 1.281453× | 1.263566× | -1.433% |

The four-actor cohort contains 245,664 measured calls. The reference ratios use its same-case balanced blocks. The paired elapsed-change column and parity score use the two primary cohorts.

## Qualification and independent counters

- Eight allocator tests per build run the real resolver and check its output. The positive control removes the selected reload across an alternate spill rewrite. Joins, backward edges, reached definitions, clobbers, and intervening edits retain the necessary moves.
- The optional checker completes all 346 emitted child bodies with the exact candidate allocator. The checked vector matcher VCode matches the measured one byte for byte. The checker stops before the large entry. That entry is covered by the linked compression checks.
- 324 complete original and decoded linked calls preserve all exact streams, independent inflation, capacities, aliases and final cleanup. The original and decoded Objects and metadata are identical. The 87 direct parser controls and 44 cleanup tests pass, along with 43,923 focused window and 3,840 continuation calls. The builds use warnings as errors.
- The logical payload count, requested bytes, copy and zeroing counters, peak private payload and outstanding private bytes agree for all 81 cases.

The first allocator build exposed the edition-2018 array iterator type. The corrected explicit value iterator passes formatting and the warnings-as-errors build. The first checker wrapper expected the wrong Rust panic status at its deliberate diagnostic stop. The repeated diagnostic records status 101 and the same completed child checks. Two first-pass focus filters selected zero tests on the unmodified test fixture. The retained window and continuation test overlay selects both tests and executes the expected counts. Its first build reused the older scratch test output path. The verified test executable was copied to the current directory and the older scratch test executable was rebuilt byte for byte. All these receipts are in the packet.

| Input | R291 instructions | R292 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 812,544.5 | 778,422.0 | -4.199% | -5.003% |
| prefix-random-8192 | 6,226,611.5 | 6,163,123.0 | -1.020% | -1.211% |
| prefix-natural-4096 | 10,417,608.0 | 10,338,060.5 | -0.764% | +0.701% |
| prefix-natural-32768 | 87,788,502.5 | 87,424,687.0 | -0.414% | +0.344% |
| prefix-natural-1048576 | 3,189,708,497.0 | 3,182,053,713.0 | -0.240% | +0.368% |
| prefix-random-1048576 | 710,542,372.5 | 706,998,512.5 | -0.499% | -0.568% |
| prefix-periodic-1048576 | 592,616,268.0 | 581,649,762.0 | -1.851% | -1.440% |
| prefix-repeated-1048576 | 600,354,319.5 | 589,077,653.0 | -1.878% | -1.415% |

The natural 1 MiB process lifetime peak physical footprint moves from 54,559,680 to 54,559,744 bytes. This includes the harness.

Timing directions differ between cohorts on 11 cases. The packet retains both observations.

## Exact executable profiles

The CPU-time profiles include harness and validation samples. The counts are locations observed during sampling, independently of the retired-instruction and cycle cohorts.

| Input | Running samples before / after | Vector matcher before / after | Matcher stack loads before / after | Matcher stack stores before / after |
|---|---:|---:|---:|---:|
| eight-literals | 5473 / 5450 | 640 / 643 | 12 / 17 | 2 / 2 |
| prefix-natural-1048576 | 5351 / 5511 | 2304 / 2370 | 1038 / 1059 | 186 / 140 |
| prefix-natural-32768 | 5371 / 5339 | 2080 / 2086 | 881 / 835 | 169 / 180 |
| prefix-natural-4096 | 5659 / 5521 | 1681 / 1636 | 507 / 526 | 186 / 158 |
| prefix-periodic-1048576 | 5354 / 5510 | 3691 / 3785 | 1911 / 1556 | 289 / 225 |
| prefix-random-1048576 | 5360 / 5386 | 3438 / 3426 | 1392 / 1301 | 646 / 635 |
| prefix-repeated-1048576 | 5363 / 5371 | 3383 / 3439 | 1777 / 1453 | 252 / 222 |

On 1 MiB random input, the first witnessed reload had 15 samples. Its preceding load had 445 and the corresponding candidate load has 472. Stack offset 568 accounts for 686 samples across all candidate loads of that offset.

The existing vector caller already checks the read-only U8 view. The vector matcher still evaluates 25 row and column predicate pairs on that same descriptor. 17 pairs directly guard packed reads. The inspected linked geometry branch takes nine instructions and two stack loads. On 1 MiB random input this span has 21 samples. It supplies a source-derived candidate for the next compiler round.

The closest candidate case above parity is prefix-natural-4096 at 1.070295× Roc. Consume the caller’s current flat-view proof in source-guarded packed reads. Preserve the source readability check and the general rectangular body. Verify both original and decoded artifacts with a matrix refusal control before timing.

The timing scope is the experimental compact entry. Ordinary managed integration remains open.

## All primary timings

| Input | Bytes | R291 / Roc | R292 / Roc | R292 / R291 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.109223× | 2.124955× | 1.007459× | pass/pass |
| one-stored | 1 | 1.975604× | 1.975328× | 0.999860× | pass/pass |
| seven-stored-prefix | 7 | 2.217075× | 2.237410× | 1.009172× | pass/pass |
| eight-literals | 8 | 0.196499× | 0.196221× | 0.998584× | pass/pass |
| eight-overlap | 8 | 0.186811× | 0.186163× | 0.996530× | pass/pass |
| 320-overlap | 320 | 0.216058× | 0.215874× | 0.999146× | pass/pass |
| eight-literals-prefix | 8 | 0.197237× | 0.196378× | 0.995645× | pass/pass |
| 128-biased-dynamic | 128 | 0.261014× | 0.259748× | 0.995147× | pass/pass |
| 32-high-entropy-stored | 32 | 0.263732× | 0.262275× | 0.994476× | pass/pass |
| 16384-distribution-change | 16384 | 1.528286× | 1.504526× | 0.984453× | pass/pass |
| 33027-periodic-window | 33027 | 0.729198× | 0.717700× | 0.984232× | pass/pass |
| random-64 | 64 | 0.275318× | 0.274792× | 0.998091× | pass/pass |
| random-256 | 256 | 0.330905× | 0.329450× | 0.995602× | pass/pass |
| random-1024 | 1024 | 0.400491× | 0.397766× | 0.993195× | pass/pass |
| repeated-1024 | 1024 | 0.263766× | 0.261844× | 0.992713× | pass/pass |
| random-32769 | 32769 | 1.442204× | 1.430808× | 0.992099× | pass/pass |
| repeated-32769 | 32769 | 0.712963× | 0.701883× | 0.984459× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.515157× | 0.512684× | 0.995199× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.388343× | 1.384479× | 0.997217× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.439329× | 1.439471× | 1.000099× | pass/pass |
| heldout-window-32767 | 32767 | 1.441538× | 1.439172× | 0.998359× | pass/pass |
| heldout-window-32768 | 32768 | 1.435187× | 1.436476× | 1.000898× | pass/pass |
| heldout-slides-65537 | 65537 | 1.472832× | 1.471231× | 0.998913× | pass/pass |
| heldout-random-65537 | 65537 | 1.590542× | 1.583873× | 0.995807× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.433049× | 1.424358× | 0.993936× | pass/pass |
| prefix-natural-1024 | 1024 | 0.518247× | 0.513266× | 0.990389× | pass/pass |
| prefix-natural-4096 | 4096 | 1.076347× | 1.070295× | 0.994377× | pass/pass |
| prefix-natural-8192 | 8192 | 1.275470× | 1.270074× | 0.995769× | pass/pass |
| prefix-natural-16384 | 16384 | 1.390212× | 1.385210× | 0.996402× | pass/pass |
| prefix-natural-32767 | 32767 | 1.435682× | 1.438543× | 1.001993× | pass/pass |
| prefix-natural-32768 | 32768 | 1.435960× | 1.437793× | 1.001276× | pass/pass |
| prefix-natural-32769 | 32769 | 1.435642× | 1.436907× | 1.000881× | pass/pass |
| prefix-natural-33027 | 33027 | 1.439434× | 1.443491× | 1.002818× | pass/pass |
| prefix-natural-65535 | 65535 | 1.441336× | 1.445697× | 1.003026× | pass/pass |
| prefix-natural-65536 | 65536 | 1.446660× | 1.447778× | 1.000772× | pass/pass |
| prefix-natural-65537 | 65537 | 1.439365× | 1.439918× | 1.000385× | pass/pass |
| prefix-natural-131072 | 131072 | 1.445872× | 1.451115× | 1.003626× | pass/pass |
| prefix-natural-262144 | 262144 | 1.446874× | 1.457208× | 1.007143× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.464247× | 1.469591× | 1.003650× | pass/pass |
| prefix-random-1024 | 1024 | 0.407243× | 0.403511× | 0.990836× | pass/pass |
| prefix-random-4096 | 4096 | 0.686706× | 0.675906× | 0.984272× | pass/pass |
| prefix-random-8192 | 8192 | 0.910722× | 0.900776× | 0.989078× | pass/pass |
| prefix-random-16384 | 16384 | 1.201789× | 1.192286× | 0.992093× | pass/pass |
| prefix-random-32767 | 32767 | 1.444895× | 1.435061× | 0.993195× | pass/pass |
| prefix-random-32768 | 32768 | 1.445650× | 1.437453× | 0.994330× | pass/pass |
| prefix-random-32769 | 32769 | 1.437695× | 1.426909× | 0.992497× | pass/pass |
| prefix-random-33027 | 33027 | 1.447919× | 1.437382× | 0.992723× | pass/pass |
| prefix-random-65535 | 65535 | 1.620704× | 1.610011× | 0.993402× | pass/pass |
| prefix-random-65536 | 65536 | 1.615912× | 1.603609× | 0.992386× | pass/pass |
| prefix-random-65537 | 65537 | 1.619409× | 1.606177× | 0.991829× | pass/pass |
| prefix-random-131072 | 131072 | 1.712950× | 1.705231× | 0.995494× | pass/pass |
| prefix-random-262144 | 262144 | 1.753559× | 1.749540× | 0.997708× | pass/pass |
| prefix-random-1048576 | 1048576 | 1.776203× | 1.769396× | 0.996168× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.323364× | 0.321752× | 0.995015× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.460858× | 0.451411× | 0.979502× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.543249× | 0.532335× | 0.979909× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.629570× | 0.620077× | 0.984923× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.711154× | 0.699906× | 0.984183× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.715069× | 0.701710× | 0.981318× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.717079× | 0.707286× | 0.986343× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.727074× | 0.715611× | 0.984233× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.769064× | 0.758525× | 0.986297× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.772677× | 0.760535× | 0.984286× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.773348× | 0.760680× | 0.983620× | pass/pass |
| prefix-periodic-131072 | 131072 | 0.802116× | 0.791524× | 0.986795× | pass/pass |
| prefix-periodic-262144 | 262144 | 0.814883× | 0.803976× | 0.986616× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 0.822884× | 0.811100× | 0.985680× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.262073× | 0.260105× | 0.992488× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.411271× | 0.401642× | 0.976588× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.497083× | 0.490609× | 0.986976× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.599768× | 0.593267× | 0.989162× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.685135× | 0.675352× | 0.985720× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.685594× | 0.678578× | 0.989767× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.709575× | 0.697338× | 0.982755× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.727972× | 0.716029× | 0.983594× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.764014× | 0.752905× | 0.985460× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.768682× | 0.754457× | 0.981495× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.765876× | 0.754224× | 0.984787× | pass/pass |
| prefix-repeated-131072 | 131072 | 0.800937× | 0.788282× | 0.984200× | pass/pass |
| prefix-repeated-262144 | 262144 | 0.814689× | 0.802830× | 0.985444× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 0.815716× | 0.804028× | 0.985671× | pass/pass |

[Verified allocator patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r292-forward-copies-2e0a52544b0b.tar.gz). SHA-256 2e0a52544b0b656e6f7e223ea30eb485f0ab8ac7f33ae047eb1242f0c797f72e.
