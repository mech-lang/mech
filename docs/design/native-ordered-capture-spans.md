# R301 — Reuse checked spans for ordered captures

**Retain R301 with the R272 runtime.** Candidate paired elapsed changes -0.475% across 81 fixed level-12 cases. The pooled and block gates pass 81/81 and 81/81. R297 beats Roc on 45/81 cases and R301 beats it on 45/81 (55.56%). The separate four-actor cohort gives R301 3/81 cases below libdeflate.

The two primary cohorts change elapsed by -0.435% and -0.514%. The largest paired slowdown is +1.876% on empty. Cases crossing below 1× Roc are none. Cases crossing above are none.

Pooled gate misses are none. Block gate misses are none.

Measured parent c2aa0ebc8984e608791e0fca42fcc0ea8f44d7b9. Published parent dca3d786e1a17f8a2307985e5be517ed9144933c. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R297 closure and four changed compiler files. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The level-12 source, inputs, tables, capacities, allocator runtime and complete-call timer are pinned.

## Current proof and generated code

The current match-extension entry already establishes a nonwrapping end within its current view. It also orders the two current immutable bases. The emitter follows dominating selected edges and a true conjunction to reuse those facts. The two capture windows must share the exact extent and length SSA values. Both bases must be no later than the checked base. Different extents, different lengths, missing order or nonwrap evidence, and alternate entries keep the original checks.

One of 347 bodies changes. The vector matcher shrinks from 13,828 to 13,696 bytes and from 784 to 768 frame bytes. Four native capture gates each drop four redundant bounds. The two fixed-width prefix checks are unchanged from R297. The induction and width checks at the selected gate still choose the original native or checked route.

| Linked recurring capture gate | R297 | R301 |
|---|---:|---:|
| Instructions | 28 | 12 |
| Comparisons | 7 | 3 |
| Stack loads | 3 | 1 |

The 27-case screen changes paired elapsed -0.984%. Its pooled and block misses are 0 and 0. The exact linked peer and safe Rust excerpts are included as readable-window controls.

## Headline matched ratios

| Input | R297 / Roc | R301 / Roc | R297 / libdeflate | R301 / libdeflate | R301 / R297 elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.195561× | 0.196638× | 2.469847× | 2.483453× | +0.116% |
| prefix-random-8192 | 0.810624× | 0.820570× | 2.892809× | 2.928306× | -0.176% |
| prefix-natural-4096 | 0.996537× | 1.001627× | 1.704040× | 1.712745× | -0.232% |
| prefix-natural-32768 | 1.367864× | 1.367951× | 1.244985× | 1.245064× | -0.045% |
| prefix-natural-1048576 | 1.405964× | 1.403915× | 1.134504× | 1.132851× | -0.065% |
| prefix-random-1048576 | 1.713533× | 1.713116× | 2.426087× | 2.425496× | +0.117% |
| prefix-periodic-1048576 | 0.790321× | 0.779962× | 1.339501× | 1.321945× | -1.399% |
| prefix-repeated-1048576 | 0.786521× | 0.777683× | 1.235952× | 1.222064× | -1.227% |

The four-actor cohort contains 245,664 measured calls. The reference ratios use its same-case balanced blocks. The paired elapsed-change column and parity score use the two primary cohorts.

## Qualification and independent counters

- All 324 complete original and decoded calls produce the expected byte stream and independently inflate. Capacities and held aliases agree. The two forms have identical Objects and metadata. Both end with zero live private payload and owners.
- Two compiled gate tests check 3,443 inputs, including 2,688 with a selective flat-view fact. Controls cover current row and column views, different-view geometry, rectangular refusal and offset/length overflow. Six composed tests cover nested captured offsets, shape changes, aliases, handoffs and recovery.
- The 87 direct parser controls and 48 cleanup tests pass with warnings as errors. The existing packed-window test rejects missing, stale, wrapping and different-view proofs. The 43,923 window and 3,840 continuation calls pass.
- The logical payload count, requested bytes, copy and zeroing counters, peak private payload and outstanding private bytes agree for all 81 cases.

The new compiled witness checks 229,376 inputs across seven variants, with two proved base orders, absent proof components, different geometry and an alternate entry. Both the wrapped arithmetic and the reached source failure are compared with an independent checked oracle.

| Input | R297 instructions | R301 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 773,119.0 | 772,940.5 | -0.023% | -4.887% |
| prefix-random-8192 | 5,828,047.0 | 5,843,100.0 | +0.258% | -0.695% |
| prefix-natural-4096 | 9,634,392.0 | 9,663,939.5 | +0.307% | +0.265% |
| prefix-natural-32768 | 80,171,821.0 | 80,615,994.0 | +0.554% | -0.143% |
| prefix-natural-1048576 | 2,911,666,583.0 | 2,932,916,375.5 | +0.730% | -0.169% |
| prefix-random-1048576 | 654,866,994.5 | 657,605,597.5 | +0.418% | +0.064% |
| prefix-periodic-1048576 | 561,511,906.5 | 552,104,987.5 | -1.675% | -1.379% |
| prefix-repeated-1048576 | 569,319,150.0 | 559,946,725.5 | -1.646% | -1.225% |

The natural 1 MiB process lifetime peak physical footprint moves from 54,592,448 to 54,576,064 bytes. This includes the harness.

Timing directions differ between cohorts on 31 cases. The packet retains both observations.

## Exact executable profiles

The CPU-time profiles include harness and validation samples. The counts are locations observed during sampling, independently of the retired-instruction and cycle cohorts.

| Input | Running samples before / after | Vector matcher before / after | Selected gate samples before / after | Matcher stack loads before / after |
|---|---:|---:|---:|---:|
| eight-literals | 5551 / 5340 | 675 / 729 | 0 / 0 | 16 / 18 |
| prefix-natural-1048576 | 5363 / 5356 | 2319 / 2309 | 0 / 0 | 674 / 707 |
| prefix-natural-32768 | 5379 / 5366 | 2097 / 2061 | 0 / 0 | 627 / 643 |
| prefix-natural-4096 | 5589 / 5580 | 1646 / 1683 | 0 / 0 | 494 / 481 |
| prefix-periodic-1048576 | 5366 / 5370 | 3636 / 3608 | 358 / 151 | 1694 / 1758 |
| prefix-random-1048576 | 5389 / 5384 | 3385 / 3375 | 0 / 0 | 1529 / 1547 |
| prefix-repeated-1048576 | 5377 / 5377 | 3352 / 3340 | 323 / 126 | 1548 / 1630 |

The selected gate is one of the four rewritten gates. Its offsets come from each exact linked worker. These are sampled locations and do not count dynamic instructions.

The closest candidate case above parity is prefix-natural-4096 at 1.007199× Roc. Check whether the existing shifted equality proof can cover the earlier four-word comparison in the skip path. The R275 proof starts after those comparisons in the long route. Require the same current immutable view, a fully covered prefix and the original entry checks before any bypass. Measure the linked prefix and proof state separately from changes to tree addresses.

The timing scope is the experimental compact entry. Ordinary managed integration remains open.

## All primary timings

| Input | Bytes | R297 / Roc | R301 / Roc | R301 / R297 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.128004× | 2.167915× | 1.018756× | pass/pass |
| one-stored | 1 | 2.049980× | 2.051169× | 1.000580× | pass/pass |
| seven-stored-prefix | 7 | 2.278096× | 2.231936× | 0.979738× | pass/pass |
| eight-literals | 8 | 0.192771× | 0.192994× | 1.001158× | pass/pass |
| eight-overlap | 8 | 0.182434× | 0.182217× | 0.998811× | pass/pass |
| 320-overlap | 320 | 0.209544× | 0.209359× | 0.999115× | pass/pass |
| eight-literals-prefix | 8 | 0.192573× | 0.192828× | 1.001327× | pass/pass |
| 128-biased-dynamic | 128 | 0.256582× | 0.257066× | 1.001887× | pass/pass |
| 32-high-entropy-stored | 32 | 0.263082× | 0.262672× | 0.998440× | pass/pass |
| 16384-distribution-change | 16384 | 1.450845× | 1.448439× | 0.998341× | pass/pass |
| 33027-periodic-window | 33027 | 0.701949× | 0.691693× | 0.985390× | pass/pass |
| random-64 | 64 | 0.277147× | 0.277547× | 1.001441× | pass/pass |
| random-256 | 256 | 0.330257× | 0.330509× | 1.000763× | pass/pass |
| random-1024 | 1024 | 0.389344× | 0.389069× | 0.999295× | pass/pass |
| repeated-1024 | 1024 | 0.256968× | 0.255416× | 0.993959× | pass/pass |
| random-32769 | 32769 | 1.372681× | 1.372389× | 0.999787× | pass/pass |
| repeated-32769 | 32769 | 0.686945× | 0.674506× | 0.981892× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.497529× | 0.496865× | 0.998666× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.325217× | 1.326803× | 1.001197× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.372221× | 1.371960× | 0.999810× | pass/pass |
| heldout-window-32767 | 32767 | 1.369724× | 1.369960× | 1.000173× | pass/pass |
| heldout-window-32768 | 32768 | 1.373189× | 1.370167× | 0.997799× | pass/pass |
| heldout-slides-65537 | 65537 | 1.406932× | 1.404793× | 0.998480× | pass/pass |
| heldout-random-65537 | 65537 | 1.525595× | 1.525816× | 1.000145× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.375691× | 1.372479× | 0.997665× | pass/pass |
| prefix-natural-1024 | 1024 | 0.496663× | 0.496199× | 0.999065× | pass/pass |
| prefix-natural-4096 | 4096 | 1.009537× | 1.007199× | 0.997683× | pass/pass |
| prefix-natural-8192 | 8192 | 1.216041× | 1.219631× | 1.002952× | pass/pass |
| prefix-natural-16384 | 16384 | 1.321834× | 1.324245× | 1.001825× | pass/pass |
| prefix-natural-32767 | 32767 | 1.369607× | 1.370000× | 1.000287× | pass/pass |
| prefix-natural-32768 | 32768 | 1.371538× | 1.370925× | 0.999553× | pass/pass |
| prefix-natural-32769 | 32769 | 1.372388× | 1.372584× | 1.000143× | pass/pass |
| prefix-natural-33027 | 33027 | 1.372881× | 1.374525× | 1.001198× | pass/pass |
| prefix-natural-65535 | 65535 | 1.382911× | 1.382307× | 0.999564× | pass/pass |
| prefix-natural-65536 | 65536 | 1.383347× | 1.382501× | 0.999388× | pass/pass |
| prefix-natural-65537 | 65537 | 1.373947× | 1.373743× | 0.999852× | pass/pass |
| prefix-natural-131072 | 131072 | 1.391280× | 1.393075× | 1.001290× | pass/pass |
| prefix-natural-262144 | 262144 | 1.390583× | 1.390491× | 0.999934× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.407705× | 1.406790× | 0.999349× | pass/pass |
| prefix-random-1024 | 1024 | 0.394639× | 0.393515× | 0.997154× | pass/pass |
| prefix-random-4096 | 4096 | 0.662059× | 0.661626× | 0.999346× | pass/pass |
| prefix-random-8192 | 8192 | 0.828335× | 0.826878× | 0.998241× | pass/pass |
| prefix-random-16384 | 16384 | 1.140652× | 1.138001× | 0.997676× | pass/pass |
| prefix-random-32767 | 32767 | 1.377570× | 1.377931× | 1.000262× | pass/pass |
| prefix-random-32768 | 32768 | 1.377851× | 1.376436× | 0.998973× | pass/pass |
| prefix-random-32769 | 32769 | 1.374619× | 1.374839× | 1.000160× | pass/pass |
| prefix-random-33027 | 33027 | 1.382284× | 1.379281× | 0.997828× | pass/pass |
| prefix-random-65535 | 65535 | 1.558976× | 1.557287× | 0.998916× | pass/pass |
| prefix-random-65536 | 65536 | 1.554036× | 1.554707× | 1.000432× | pass/pass |
| prefix-random-65537 | 65537 | 1.554065× | 1.554060× | 0.999997× | pass/pass |
| prefix-random-131072 | 131072 | 1.647539× | 1.648982× | 1.000876× | pass/pass |
| prefix-random-262144 | 262144 | 1.692346× | 1.695747× | 1.002010× | pass/pass |
| prefix-random-1048576 | 1048576 | 1.709167× | 1.711158× | 1.001165× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.320171× | 0.319352× | 0.997442× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.442036× | 0.439391× | 0.994016× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.516520× | 0.511950× | 0.991152× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.609645× | 0.600330× | 0.984721× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.683363× | 0.674536× | 0.987082× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.685626× | 0.676669× | 0.986936× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.690965× | 0.681927× | 0.986920× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.701659× | 0.689175× | 0.982208× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.737079× | 0.726956× | 0.986266× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.743838× | 0.731808× | 0.983827× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.741069× | 0.731573× | 0.987187× | pass/pass |
| prefix-periodic-131072 | 131072 | 0.773557× | 0.761786× | 0.984784× | pass/pass |
| prefix-periodic-262144 | 262144 | 0.785249× | 0.773359× | 0.984857× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 0.790547× | 0.779488× | 0.986012× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.256721× | 0.256529× | 0.999252× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.391515× | 0.387699× | 0.990253× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.487049× | 0.484552× | 0.994873× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.579930× | 0.572593× | 0.987348× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.661239× | 0.653685× | 0.988576× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.662205× | 0.657096× | 0.992285× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.689320× | 0.678321× | 0.984043× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.703556× | 0.694597× | 0.987267× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.738062× | 0.727382× | 0.985530× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.740025× | 0.729196× | 0.985366× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.740755× | 0.730939× | 0.986748× | pass/pass |
| prefix-repeated-131072 | 131072 | 0.773593× | 0.763421× | 0.986851× | pass/pass |
| prefix-repeated-262144 | 262144 | 0.786315× | 0.777089× | 0.988267× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 0.786089× | 0.776443× | 0.987729× | pass/pass |

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r301-ordered-capture-spans-97a258fe1a09.tar.gz). SHA-256 97a258fe1a0912372d0011526ca6c59d3a8a8662ed94ef67007ced74af024bda.
