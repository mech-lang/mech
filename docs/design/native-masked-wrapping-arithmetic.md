# R280 — Narrow masked wrapping arithmetic

**Retain R280 with the R272 runtime.** Candidate paired elapsed changes -1.900% across 81 fixed level-12 cases. The pooled and block gates pass 81/81 and 81/81. R279 beats Roc on 45/81 cases and R280 beats it on 45/81 (55.56%). The independent four-actor cohort gives R280 3/81 cases below libdeflate.

The two primary cohorts change elapsed by -1.892% and -1.909%. The largest slowdown is +1.589% on seven-stored-prefix. Cases crossing below 1× Roc are none. Cases crossing above are none.

Pooled gate misses are none. Block gate misses are none.

Measured and published parent 344cc850f2e59b482abd0fb97090dcdc6054260b. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R279 closure and this three-file overlay. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, IR/work/ABI/bytecode 215/107/39/14.

## Compiler rule and emitted code

A wrapping add, subtract or multiply preserves its low 32 bits when performed on the low 32 bits of the operands. The pass counts all SSA uses, including arguments passed to successor blocks. It uses I32 arithmetic and zero-extends the result when every existing consumer masks to 32 bits. A full-width use keeps the original producer. The replacement keeps the existing program checks, alias and ownership paths, failure origins and all source arithmetic results. The rule operates on typed IR and requires no benchmark source identities.

The experimental pass runs in the existing no-growth body. Seven producers qualify in each of four bodies. The other bodies, including the root and vector matcher, have identical body hashes and frame sizes. Each changed body shrinks from 1,760 to 1,744 bytes and keeps its 272-byte frame. Exact linked ARM64 inspection of the primary path routine finds five to eleven unshifted 32-bit adds, zero to one 32-bit subtract, and eighteen to seven explicit low-word masks. These are static instructions. The exact pinned C cost routine also uses 32-bit arithmetic.

The initial 27-case screen changes paired elapsed by -2.221%. The screen records 0 pooled and 0 block misses. It keeps the adverse eight-literal and large natural-text observations.

## Headline matched ratios

| Input | R279 / Roc | R280 / Roc | R279 / libdeflate | R280 / libdeflate | R280 / R279 elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.196939× | 0.197073× | 2.512061× | 2.513758× | +0.201% |
| prefix-random-8192 | 0.961595× | 0.935451× | 3.463437× | 3.369271× | -2.891% |
| prefix-natural-4096 | 1.098251× | 1.109993× | 1.864200× | 1.884130× | +0.615% |
| prefix-natural-1048576 | 1.520798× | 1.527103× | 1.226933× | 1.232019× | +0.145% |
| prefix-random-1048576 | 1.975834× | 1.916228× | 2.790917× | 2.706722× | -2.525% |
| prefix-periodic-1048576 | 0.968052× | 0.933103× | 1.640215× | 1.581000× | -3.634% |
| prefix-repeated-1048576 | 0.949175× | 0.917297× | 1.490858× | 1.440788× | -3.243% |

The fresh four-actor cohort contains 245,664 measured calls. The same-case Mech and peer ratios use its balanced blocks. The final elapsed-change column and the parity score use the two primary cohorts. The library cohort also records an adverse empty-case ratio of 1.0758× R279.

## Correctness and independent counters

- 324 complete original and decoded linked calls check all 81 exact streams, capacities, independent inflation, retained input/output aliases, and final cleanup. The original and decoded Object and metadata match byte-for-byte.
- 36 direct linked controls compare the retained phase, original candidate, and decoded candidate. They check empty-to-match and vector transitions, changing cost tables, checked assertion failure, retry, default-producing reads and wrapped costs near 2³². The linked test copies preserve the exact production routine instructions.
- The JIT checks 768 concrete add, subtract and multiply calls across wrapping values and mixed wide consumers.
- The logical payload count, requested bytes, copy and zeroing counters, peak private payload and outstanding private bytes agree for all 81 cases.

| Input | R279 instructions | R280 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 812,048.0 | 811,933.0 | -0.014% | -0.755% |
| prefix-random-8192 | 6,965,778.0 | 6,941,127.0 | -0.354% | -3.391% |
| prefix-natural-4096 | 11,235,818.5 | 11,231,457.5 | -0.039% | +0.073% |
| prefix-natural-1048576 | 3,489,601,620.0 | 3,489,465,850.0 | -0.004% | +0.201% |
| prefix-random-1048576 | 817,544,436.5 | 814,558,649.0 | -0.365% | -2.708% |
| prefix-periodic-1048576 | 667,039,020.5 | 664,958,047.0 | -0.312% | -3.523% |
| prefix-repeated-1048576 | 674,604,769.5 | 672,523,316.0 | -0.309% | -3.279% |

The 1 MiB natural-text CPU worker reports a lifetime peak physical footprint of 54,526,912 bytes before and 54,543,360 bytes after. These process-wide samples include the harness.

## Exact candidate profile

These are CPU-time samples from the exact linked candidate in a separate cohort. They include harness and validation work.

| Input | Running samples | Path parser samples | Vector matcher samples | Matcher stack-transfer PCs |
|---|---:|---:|---:|---:|
| eight-literals | 5322 | 2 | 625 | 17 |
| prefix-natural-1048576 | 5364 | 2591 | 2396 | 1591 |
| prefix-natural-32768 | 5371 | 2591 | 2168 | 1420 |
| prefix-natural-4096 | 5586 | 1527 | 1778 | 1011 |
| prefix-periodic-1048576 | 5386 | 643 | 3914 | 2074 |
| prefix-random-1048576 | 5360 | 394 | 3559 | 2725 |
| prefix-repeated-1048576 | 5359 | 603 | 3621 | 1906 |

The matcher disassembly is equal across the two linked executables. The random-input profile includes loads of the same input length from one stack slot around consecutive four-byte window checks. The natural-text profile reaches several checks against the same tree length. Sample PCs locate the instructions for proof and allocation analysis. They do not measure the latency of those individual instructions.

The closest candidate case above parity is prefix-natural-4096 at 1.108073× Roc. Isolate the four matched library prefetch hints at fixed instruction addresses. Use the measured effect and current matcher samples to decide whether to emit a bounded hint.

## All primary timings

| Input | Bytes | R279 / Roc | R280 / Roc | R280 / R279 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.118788× | 2.088075× | 0.985504× | pass/pass |
| one-stored | 1 | 1.973157× | 1.978791× | 1.002855× | pass/pass |
| seven-stored-prefix | 7 | 2.240370× | 2.275971× | 1.015891× | pass/pass |
| eight-literals | 8 | 0.196961× | 0.197356× | 1.002007× | pass/pass |
| eight-overlap | 8 | 0.187206× | 0.187325× | 1.000638× | pass/pass |
| 320-overlap | 320 | 0.219493× | 0.218047× | 0.993416× | pass/pass |
| eight-literals-prefix | 8 | 0.198161× | 0.197704× | 0.997692× | pass/pass |
| 128-biased-dynamic | 128 | 0.262524× | 0.263573× | 1.003994× | pass/pass |
| 32-high-entropy-stored | 32 | 0.266956× | 0.267666× | 1.002660× | pass/pass |
| 16384-distribution-change | 16384 | 1.634330× | 1.604573× | 0.981792× | pass/pass |
| 33027-periodic-window | 33027 | 0.841755× | 0.816309× | 0.969771× | pass/pass |
| random-64 | 64 | 0.280825× | 0.281409× | 1.002078× | pass/pass |
| random-256 | 256 | 0.342259× | 0.340740× | 0.995564× | pass/pass |
| random-1024 | 1024 | 0.422919× | 0.416735× | 0.985378× | pass/pass |
| repeated-1024 | 1024 | 0.282206× | 0.275259× | 0.975383× | pass/pass |
| random-32769 | 32769 | 1.603266× | 1.551368× | 0.967630× | pass/pass |
| repeated-32769 | 32769 | 0.817460× | 0.791485× | 0.968225× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.562151× | 0.551740× | 0.981481× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.443125× | 1.447714× | 1.003180× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.488657× | 1.491772× | 1.002092× | pass/pass |
| heldout-window-32767 | 32767 | 1.491149× | 1.493247× | 1.001407× | pass/pass |
| heldout-window-32768 | 32768 | 1.490841× | 1.495043× | 1.002818× | pass/pass |
| heldout-slides-65537 | 65537 | 1.533298× | 1.534731× | 1.000935× | pass/pass |
| heldout-random-65537 | 65537 | 1.786970× | 1.726320× | 0.966060× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.549980× | 1.512979× | 0.976128× | pass/pass |
| prefix-natural-1024 | 1024 | 0.534701× | 0.534958× | 1.000480× | pass/pass |
| prefix-natural-4096 | 4096 | 1.101304× | 1.108073× | 1.006146× | pass/pass |
| prefix-natural-8192 | 8192 | 1.321690× | 1.325503× | 1.002885× | pass/pass |
| prefix-natural-16384 | 16384 | 1.440792× | 1.445485× | 1.003257× | pass/pass |
| prefix-natural-32767 | 32767 | 1.487996× | 1.492685× | 1.003151× | pass/pass |
| prefix-natural-32768 | 32768 | 1.485661× | 1.490713× | 1.003401× | pass/pass |
| prefix-natural-32769 | 32769 | 1.490805× | 1.494000× | 1.002144× | pass/pass |
| prefix-natural-33027 | 33027 | 1.491190× | 1.494322× | 1.002100× | pass/pass |
| prefix-natural-65535 | 65535 | 1.495748× | 1.497313× | 1.001046× | pass/pass |
| prefix-natural-65536 | 65536 | 1.493563× | 1.497046× | 1.002332× | pass/pass |
| prefix-natural-65537 | 65537 | 1.489873× | 1.493864× | 1.002679× | pass/pass |
| prefix-natural-131072 | 131072 | 1.490738× | 1.484186× | 0.995605× | pass/pass |
| prefix-natural-262144 | 262144 | 1.503114× | 1.501875× | 0.999176× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.522053× | 1.524253× | 1.001445× | pass/pass |
| prefix-random-1024 | 1024 | 0.424425× | 0.416886× | 0.982237× | pass/pass |
| prefix-random-4096 | 4096 | 0.743198× | 0.723992× | 0.974159× | pass/pass |
| prefix-random-8192 | 8192 | 0.968964× | 0.940952× | 0.971090× | pass/pass |
| prefix-random-16384 | 16384 | 1.335045× | 1.290567× | 0.966685× | pass/pass |
| prefix-random-32767 | 32767 | 1.605297× | 1.554920× | 0.968618× | pass/pass |
| prefix-random-32768 | 32768 | 1.612155× | 1.561702× | 0.968705× | pass/pass |
| prefix-random-32769 | 32769 | 1.611740× | 1.556700× | 0.965850× | pass/pass |
| prefix-random-33027 | 33027 | 1.616371× | 1.566706× | 0.969273× | pass/pass |
| prefix-random-65535 | 65535 | 1.810072× | 1.753947× | 0.968993× | pass/pass |
| prefix-random-65536 | 65536 | 1.805649× | 1.751725× | 0.970136× | pass/pass |
| prefix-random-65537 | 65537 | 1.793369× | 1.739245× | 0.969820× | pass/pass |
| prefix-random-131072 | 131072 | 1.905196× | 1.846171× | 0.969019× | pass/pass |
| prefix-random-262144 | 262144 | 1.952591× | 1.897022× | 0.971541× | pass/pass |
| prefix-random-1048576 | 1048576 | 1.977034× | 1.927106× | 0.974746× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.342819× | 0.336703× | 0.982160× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.512996× | 0.495096× | 0.965107× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.608746× | 0.585463× | 0.961753× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.729783× | 0.708368× | 0.970655× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.825994× | 0.799365× | 0.967762× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.832652× | 0.804433× | 0.966109× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.832764× | 0.802522× | 0.963685× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.855652× | 0.825525× | 0.964791× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.900576× | 0.870918× | 0.967067× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.900775× | 0.870326× | 0.966198× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.904938× | 0.873996× | 0.965807× | pass/pass |
| prefix-periodic-131072 | 131072 | 0.943836× | 0.908531× | 0.962594× | pass/pass |
| prefix-periodic-262144 | 262144 | 0.959483× | 0.924634× | 0.963679× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 0.969720× | 0.934485× | 0.963665× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.280113× | 0.273509× | 0.976421× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.453310× | 0.437020× | 0.964065× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.567097× | 0.546315× | 0.963353× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.691319× | 0.671523× | 0.971365× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.795441× | 0.766332× | 0.963406× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.794403× | 0.764663× | 0.962562× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.814781× | 0.787348× | 0.966330× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.834190× | 0.808396× | 0.969079× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.887779× | 0.858470× | 0.966986× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.886580× | 0.856455× | 0.966021× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.886290× | 0.858445× | 0.968582× | pass/pass |
| prefix-repeated-131072 | 131072 | 0.926769× | 0.895253× | 0.965994× | pass/pass |
| prefix-repeated-262144 | 262144 | 0.945185× | 0.914667× | 0.967712× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 0.949153× | 0.918369× | 0.967567× | pass/pass |

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r280-low-word-arithmetic-bdb9996eb6b8.tar.gz). SHA-256 bdb9996eb6b82e3e4cfce42576e2234913000889954b7018761fecfc51249d0a.

R236 measured broader low-word narrowing across the root and 33 numeric bodies. Its 25-case pooled geometric improvement was 0.179%, with roughly 2.5–2.7% regressions on several random and repetitive cases. R280 limits the experiment to four current no-growth path bodies and measures it against the newer compiler and 81-case suite. [Earlier measurements](native-low-word-arithmetic.md).
