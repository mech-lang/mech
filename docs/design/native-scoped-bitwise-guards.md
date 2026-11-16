# R297 — Fold exact flags in selected reductions

**Retain R297 with the R272 runtime.** Candidate paired elapsed changes -0.635% across 81 fixed level-12 cases. The pooled and block gates pass 81/81 and 81/81. R296 beats Roc on 45/81 cases and R297 beats it on 45/81 (55.56%). The separate four-actor cohort gives R297 3/81 cases below libdeflate.

The two primary cohorts change elapsed by -0.588% and -0.681%. The largest paired slowdown is +0.167% on random-1024. Cases crossing below 1× Roc are none. Cases crossing above are none.

Pooled gate misses are none. Block gate misses are none.

Published and measured parent 69d48c4d6b5353d04fa2f2c35f96dd30f97e49cc. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R296 closure and three changed compiler files. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The level-12 source, inputs, tables, capacities, allocator runtime and complete-call timer are pinned.

## Current proof and generated code

The range analysis resolves exact scalar AND and OR operands. The additional pass runs only for a capacity-proved native phase with a completed ordered reduction. It uses the existing cleanup controls and rewrites a branch only when its predicate has one exact value. The original dynamic count guard and checked routes remain.

Four of 347 bodies change. Each moves from 1,664 to 1,536 code bytes and from 256 to 240 frame bytes. These four bodies match the corresponding broad candidate bodies byte for byte. The remaining 343 bodies match the control, including the root and matcher.

| Linked reduction gate | R296 | R297 |
|---|---:|---:|
| Constant flag materialization, extension and branch | 3 | 0 |
| Dynamic checked match-count boundary | present | present |

The before image emits MOV, UXTB and CBNZ for the already true flag. The revised image still checks the current count against the same 255 boundary before subtracting two. The remaining dynamic scalar and vector choices use that result.

| 27-case screen | Paired elapsed change | Pooled misses | Block misses |
|---|---:|---:|---:|
| Broad first screen | +0.447% | 0 | 0 |
| Broad in four-actor screen | +0.353% | 0 | 0 |
| Scoped in four-actor screen | -0.310% | 0 | 0 |

The broad compiler changed 14 bodies and increased paired time in both screens. It remains a measured control. The first broad test overlay omitted an inherited test. The corrected build passes all 48 cleanup tests. The scoped build passes all 47 cleanup tests, including the inherited test and the compiled exact-flag witness.

## Headline matched ratios

| Input | R296 / Roc | R297 / Roc | R296 / libdeflate | R297 / libdeflate | R297 / R296 elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.193690× | 0.194091× | 2.477664× | 2.482799× | -0.268% |
| prefix-random-8192 | 0.817598× | 0.821070× | 2.941261× | 2.953750× | -0.781% |
| prefix-natural-4096 | 1.013778× | 1.004644× | 1.727450× | 1.711885× | -1.251% |
| prefix-natural-32768 | 1.381658× | 1.368415× | 1.260559× | 1.248477× | -1.230% |
| prefix-natural-1048576 | 1.421741× | 1.406241× | 1.146626× | 1.134125× | -2.250% |
| prefix-random-1048576 | 1.707312× | 1.710297× | 2.418515× | 2.422743× | -0.095% |
| prefix-periodic-1048576 | 0.790692× | 0.790694× | 1.339846× | 1.339850× | +0.087% |
| prefix-repeated-1048576 | 0.786961× | 0.786132× | 1.235358× | 1.234056× | +0.008% |

The four-actor cohort contains 245,664 measured calls. The reference ratios use its same-case balanced blocks. The paired elapsed-change column and parity score use the two primary cohorts.

## Qualification and independent counters

- All 324 complete original and decoded calls produce the expected byte stream and independently inflate. Capacities and held aliases agree. The two forms have identical Objects and metadata. Both end with zero live private payload and owners.
- Two compiled gate tests check 3,443 inputs, including 2,688 with a selective flat-view fact. Controls cover current row and column views, different-view geometry, rectangular refusal and offset/length overflow. Six composed tests cover nested captured offsets, shape changes, aliases, handoffs and recovery.
- The 87 direct parser controls and 47 cleanup tests pass with warnings as errors. The existing packed-window test rejects missing, stale, wrapping and different-view proofs. The 43,923 window and 3,840 continuation calls pass.
- The logical payload count, requested bytes, copy and zeroing counters, peak private payload and outstanding private bytes agree for all 81 cases.

The exact-flag witness executes 20 compiled calls. It covers both fixed flag outcomes, an independent dynamic condition, checked alternate control and fresh visits. The inherited shared-limit witness also passes 5,184 calls.

| Input | R296 instructions | R297 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 774,520.0 | 775,474.5 | +0.123% | +2.473% |
| prefix-random-8192 | 5,826,245.5 | 5,825,361.5 | -0.015% | -0.583% |
| prefix-natural-4096 | 9,693,900.0 | 9,634,788.5 | -0.610% | -2.151% |
| prefix-natural-32768 | 80,798,616.5 | 80,175,169.5 | -0.772% | -0.890% |
| prefix-natural-1048576 | 2,935,691,529.5 | 2,911,634,261.0 | -0.819% | -0.814% |
| prefix-random-1048576 | 654,869,241.0 | 654,857,165.5 | -0.002% | -0.133% |
| prefix-periodic-1048576 | 561,543,489.0 | 561,509,587.0 | -0.006% | +0.116% |
| prefix-repeated-1048576 | 569,350,369.0 | 569,314,077.0 | -0.006% | -0.024% |

The natural 1 MiB process lifetime peak physical footprint moves from 54,608,896 to 54,576,064 bytes. This includes the harness.

Timing directions differ between cohorts on 16 cases. The packet retains both observations.

## Exact executable profiles

The CPU-time profiles include harness and validation samples. The counts are locations observed during sampling, independently of the retired-instruction and cycle cohorts.

| Input | Running samples before / after | Four selected parser bodies before / after | Old constant gate samples |
|---|---:|---:|---:|
| eight-literals | 5483 / 5551 | 5 / 9 | 0 |
| prefix-natural-1048576 | 5368 / 5363 | 2752 / 2707 | 92 |
| prefix-natural-32768 | 5368 / 5379 | 2749 / 2674 | 103 |
| prefix-natural-4096 | 5605 / 5589 | 2696 / 2629 | 62 |
| prefix-periodic-1048576 | 5368 / 5366 | 742 / 738 | 0 |
| prefix-random-1048576 | 5365 / 5389 | 417 / 438 | 0 |
| prefix-repeated-1048576 | 5402 / 5377 | 674 / 678 | 1 |

The gate samples above refer to the exact R296 15296 body. The R297 gate is absent. The packet also includes the pinned Roc and libdeflate reduction excerpts and the independent safe Rust control.

The closest candidate case above parity is prefix-natural-4096 at 1.013120× Roc. Test one shared readable bound for both windows of an admitted immutable equality scan. In the exact R296 matcher, the recurring two-window check receives 397 samples on periodic 1 MiB and 362 on repeated 1 MiB. The prototype will use the later base against the same current length and limit, with the original separate unsigned start check.

The timing scope is the experimental compact entry. Ordinary managed integration remains open.

## All primary timings

| Input | Bytes | R296 / Roc | R297 / Roc | R297 / R296 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.106463× | 2.071923× | 0.983603× | pass/pass |
| one-stored | 1 | 2.024767× | 1.982573× | 0.979161× | pass/pass |
| seven-stored-prefix | 7 | 2.238813× | 2.197580× | 0.981583× | pass/pass |
| eight-literals | 8 | 0.193374× | 0.192855× | 0.997317× | pass/pass |
| eight-overlap | 8 | 0.182374× | 0.182046× | 0.998199× | pass/pass |
| 320-overlap | 320 | 0.209888× | 0.209693× | 0.999069× | pass/pass |
| eight-literals-prefix | 8 | 0.193605× | 0.193500× | 0.999455× | pass/pass |
| 128-biased-dynamic | 128 | 0.256052× | 0.255933× | 0.999535× | pass/pass |
| 32-high-entropy-stored | 32 | 0.263548× | 0.263301× | 0.999064× | pass/pass |
| 16384-distribution-change | 16384 | 1.448280× | 1.442479× | 0.995995× | pass/pass |
| 33027-periodic-window | 33027 | 0.702539× | 0.697707× | 0.993123× | pass/pass |
| random-64 | 64 | 0.276274× | 0.275131× | 0.995864× | pass/pass |
| random-256 | 256 | 0.329257× | 0.328678× | 0.998241× | pass/pass |
| random-1024 | 1024 | 0.388217× | 0.388864× | 1.001666× | pass/pass |
| repeated-1024 | 1024 | 0.258303× | 0.257845× | 0.998227× | pass/pass |
| random-32769 | 32769 | 1.379102× | 1.373344× | 0.995825× | pass/pass |
| repeated-32769 | 32769 | 0.688792× | 0.685679× | 0.995481× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.500213× | 0.497359× | 0.994295× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.337865× | 1.319279× | 0.986108× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.387047× | 1.370414× | 0.988009× | pass/pass |
| heldout-window-32767 | 32767 | 1.389474× | 1.371070× | 0.986755× | pass/pass |
| heldout-window-32768 | 32768 | 1.386404× | 1.369443× | 0.987766× | pass/pass |
| heldout-slides-65537 | 65537 | 1.423523× | 1.406638× | 0.988138× | pass/pass |
| heldout-random-65537 | 65537 | 1.530771× | 1.527428× | 0.997816× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.386462× | 1.374850× | 0.991625× | pass/pass |
| prefix-natural-1024 | 1024 | 0.501165× | 0.498211× | 0.994105× | pass/pass |
| prefix-natural-4096 | 4096 | 1.025958× | 1.013120× | 0.987487× | pass/pass |
| prefix-natural-8192 | 8192 | 1.230181× | 1.210509× | 0.984009× | pass/pass |
| prefix-natural-16384 | 16384 | 1.343011× | 1.323262× | 0.985295× | pass/pass |
| prefix-natural-32767 | 32767 | 1.385652× | 1.368362× | 0.987522× | pass/pass |
| prefix-natural-32768 | 32768 | 1.383716× | 1.366696× | 0.987700× | pass/pass |
| prefix-natural-32769 | 32769 | 1.385462× | 1.368455× | 0.987725× | pass/pass |
| prefix-natural-33027 | 33027 | 1.390383× | 1.369880× | 0.985253× | pass/pass |
| prefix-natural-65535 | 65535 | 1.400099× | 1.381584× | 0.986776× | pass/pass |
| prefix-natural-65536 | 65536 | 1.398486× | 1.378041× | 0.985381× | pass/pass |
| prefix-natural-65537 | 65537 | 1.394243× | 1.388391× | 0.995803× | pass/pass |
| prefix-natural-131072 | 131072 | 1.406931× | 1.386034× | 0.985147× | pass/pass |
| prefix-natural-262144 | 262144 | 1.407210× | 1.389079× | 0.987116× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.433075× | 1.400838× | 0.977505× | pass/pass |
| prefix-random-1024 | 1024 | 0.391111× | 0.390723× | 0.999009× | pass/pass |
| prefix-random-4096 | 4096 | 0.654696× | 0.645938× | 0.986622× | pass/pass |
| prefix-random-8192 | 8192 | 0.824457× | 0.818019× | 0.992191× | pass/pass |
| prefix-random-16384 | 16384 | 1.138559× | 1.129296× | 0.991865× | pass/pass |
| prefix-random-32767 | 32767 | 1.383250× | 1.377915× | 0.996143× | pass/pass |
| prefix-random-32768 | 32768 | 1.382633× | 1.379705× | 0.997882× | pass/pass |
| prefix-random-32769 | 32769 | 1.375675× | 1.373917× | 0.998722× | pass/pass |
| prefix-random-33027 | 33027 | 1.386877× | 1.383514× | 0.997575× | pass/pass |
| prefix-random-65535 | 65535 | 1.549974× | 1.550655× | 1.000439× | pass/pass |
| prefix-random-65536 | 65536 | 1.557178× | 1.555351× | 0.998827× | pass/pass |
| prefix-random-65537 | 65537 | 1.554349× | 1.553936× | 0.999734× | pass/pass |
| prefix-random-131072 | 131072 | 1.648397× | 1.646341× | 0.998753× | pass/pass |
| prefix-random-262144 | 262144 | 1.686659× | 1.684526× | 0.998735× | pass/pass |
| prefix-random-1048576 | 1048576 | 1.714481× | 1.712851× | 0.999049× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.320269× | 0.320030× | 0.999255× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.448578× | 0.440998× | 0.983103× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.522499× | 0.516635× | 0.988777× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.618708× | 0.613274× | 0.991217× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.683759× | 0.681063× | 0.996057× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.688038× | 0.686588× | 0.997892× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.690945× | 0.688232× | 0.996072× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.700163× | 0.699039× | 0.998395× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.740535× | 0.738407× | 0.997126× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.742269× | 0.740390× | 0.997469× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.744417× | 0.742660× | 0.997640× | pass/pass |
| prefix-periodic-131072 | 131072 | 0.773778× | 0.771507× | 0.997065× | pass/pass |
| prefix-periodic-262144 | 262144 | 0.783688× | 0.784643× | 1.001218× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 0.790630× | 0.791317× | 1.000868× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.255903× | 0.255618× | 0.998888× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.401607× | 0.393779× | 0.980507× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.485631× | 0.482626× | 0.993813× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.581945× | 0.578154× | 0.993485× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.661830× | 0.660624× | 0.998178× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.660823× | 0.657174× | 0.994477× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.684665× | 0.683914× | 0.998904× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.703902× | 0.700627× | 0.995347× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.737835× | 0.737650× | 0.999749× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.741693× | 0.737187× | 0.993925× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.740838× | 0.738078× | 0.996274× | pass/pass |
| prefix-repeated-131072 | 131072 | 0.774633× | 0.773292× | 0.998269× | pass/pass |
| prefix-repeated-262144 | 262144 | 0.787074× | 0.786196× | 0.998885× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 0.787255× | 0.787318× | 1.000080× | pass/pass |

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r297-scoped-bitwise-guards-9f03311546d6.tar.gz). SHA-256 9f03311546d6b3c6cf5450b5cfce886f6641667e9efed66467d425f25b1495aa.
