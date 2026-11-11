# R296 — Share fixed-width readable window bounds

**Retain R296 with the R272 runtime.** Candidate paired elapsed changes -1.015% across 81 fixed level-12 cases. The pooled and block gates pass 81/81 and 81/81. R295 beats Roc on 45/81 cases and R296 beats it on 45/81 (55.56%). The separate four-actor cohort gives R296 3/81 cases below libdeflate.

The two primary cohorts change elapsed by -1.131% and -0.900%. The largest paired slowdown is +1.713% on empty. Cases crossing below 1× Roc are none. Cases crossing above are none.

Pooled gate misses are none. Block gate misses are none.

Published and measured parent ae6bd4815b98455ed5820a2f9b25ef968054d38f. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R295 closure and three changed compiler files. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The level-12 source, inputs, tables, capacities, allocator runtime and complete-call timer are pinned.

## Current proof and generated code

The compiler recognizes a positive width, an unchanged length and a checked start comparison. It follows both rejection edges through effect-free forwarding and requires the first common block and all its current arguments to agree. It subtracts width minus one from the length with saturation once per qualifying pair in the straight-line activation prologue. The new unsigned start test has exactly the same readable and rejected inputs. Distinct values, effects, late length definitions and alternate predecessors retain the existing control.

One of 347 bodies changes. The vector matcher shrinks from 14,184 to 13,828 body bytes and keeps its 784-byte frame. Five width-four and twelve width-eight checks become comparisons against two shared SSA limits. The 29 packed loads, source checks and general fallback remain available.

| Two linked successful width-four reads | R295 | R296 |
|---|---:|---:|
| Instructions | 23 | 15 |
| Stack loads | 6 | 4 |

Both current reads compare against the same register-held limit. The pinned Roc matcher forms its saturating length-minus-three once and reloads it for a single comparison. The linked libdeflate matcher checks its current readable span before entering its direct prefix loads. A separate safe Rust control uses a subtraction and width check for each read. The packet includes those exact machine excerpts and witness source.

The 27-case screen changes paired elapsed by -1.444%. Its pooled and block misses are 0 and 0. The packet preserves all observations.

## Headline matched ratios

| Input | R295 / Roc | R296 / Roc | R295 / libdeflate | R296 / libdeflate | R296 / R295 elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.197683× | 0.198009× | 2.473054× | 2.477136× | +0.075% |
| prefix-random-8192 | 0.822424× | 0.824442× | 2.954963× | 2.962213× | -0.554% |
| prefix-natural-4096 | 1.006309× | 1.003424× | 1.746063× | 1.741056× | -0.367% |
| prefix-natural-32768 | 1.388167× | 1.383901× | 1.263827× | 1.259943× | -0.314% |
| prefix-natural-1048576 | 1.425235× | 1.424104× | 1.149017× | 1.148105× | +0.091% |
| prefix-random-1048576 | 1.709757× | 1.712933× | 2.420899× | 2.425396× | -0.115% |
| prefix-periodic-1048576 | 0.810910× | 0.790040× | 1.375087× | 1.339696× | -2.680% |
| prefix-repeated-1048576 | 0.804576× | 0.786582× | 1.265708× | 1.237401× | -2.263% |

The four-actor cohort contains 245,664 measured calls. The reference ratios use its same-case balanced blocks. The paired elapsed-change column and parity score use the two primary cohorts.

## Qualification and independent counters

- All 324 complete original and decoded calls produce the expected byte stream and independently inflate. Capacities and held aliases agree. The two forms have identical Objects and metadata. Both end with zero live private payload and owners.
- Two compiled gate tests check 3,443 inputs, including 2,688 with a selective flat-view fact. Controls cover current row and column views, different-view geometry, rectangular refusal and offset/length overflow. Six composed tests cover nested captured offsets, shape changes, aliases, handoffs and recovery.
- The 87 direct parser controls and 46 cleanup tests pass with warnings as errors. The existing packed-window test rejects missing, stale, wrapping and different-view proofs. The 43,923 window and 3,840 continuation calls pass.
- The logical payload count, requested bytes, copy and zeroing counters, peak private payload and outstanding private bytes agree for all 81 cases.

The shared-limit test makes 5,184 compiled calls across four widths and four control-flow variants. It covers an empty or short view, full unsigned values, independently supplied lengths, different rejection values and an effect on a formerly selected path. The initial build required the length in the first block and emitted the retained Object byte for byte. The revised proof admits the second setup block through its single unconditional predecessor. The initial build and audit receipts are included.

| Input | R295 instructions | R296 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 772,928.0 | 773,226.0 | +0.039% | +0.622% |
| prefix-random-8192 | 5,845,954.5 | 5,826,937.0 | -0.325% | -1.368% |
| prefix-natural-4096 | 9,748,086.0 | 9,691,410.5 | -0.581% | -1.050% |
| prefix-natural-32768 | 81,294,495.0 | 80,797,880.5 | -0.611% | -0.304% |
| prefix-natural-1048576 | 2,952,653,317.0 | 2,935,714,160.5 | -0.574% | +0.007% |
| prefix-random-1048576 | 661,097,952.0 | 654,868,654.0 | -0.942% | -0.665% |
| prefix-periodic-1048576 | 574,115,090.5 | 561,546,960.5 | -2.189% | -3.778% |
| prefix-repeated-1048576 | 581,874,625.0 | 569,353,274.0 | -2.152% | -2.344% |

The natural 1 MiB process lifetime peak physical footprint moves from 54,559,744 to 54,510,528 bytes. This includes the harness.

Timing directions differ between cohorts on 12 cases. The packet retains both observations.

## Exact executable profiles

The CPU-time profiles include harness and validation samples. The counts are locations observed during sampling, independently of the retired-instruction and cycle cohorts.

| Input | Running samples before / after | Matcher before / after | Two width-four paths before / after | Matcher stack loads before / after |
|---|---:|---:|---:|---:|
| eight-literals | 5338 / 5483 | 675 / 669 | 0 / 0 | 12 / 18 |
| prefix-natural-1048576 | 5391 / 5368 | 2309 / 2282 | 29 / 17 | 647 / 712 |
| prefix-natural-32768 | 5380 / 5368 | 2074 / 2053 | 35 / 21 | 584 / 618 |
| prefix-natural-4096 | 5638 / 5605 | 1682 / 1640 | 32 / 34 | 461 / 485 |
| prefix-periodic-1048576 | 5387 / 5368 | 3694 / 3647 | 0 / 0 | 1970 / 1757 |
| prefix-random-1048576 | 5360 / 5365 | 3366 / 3370 | 541 / 510 | 1782 / 1482 |
| prefix-repeated-1048576 | 5366 / 5402 | 3401 / 3366 | 0 / 2 | 1851 / 1558 |

The current capacity-proved parser has the same Object body as R295. The emitted reduction admission still materializes a constant true and branches on it. The next experiment applies exact bitwise range reasoning to that gate before register allocation.

The closest candidate case above parity is prefix-natural-4096 at 1.025017× Roc. Preserve exact scalar bitwise guard values before register allocation. The current capacity-proved parser reaches a reduction admission that combines known true flags with AND and OR. The final binary still materializes one and branches on it in each admitted segment. Measure early removal with the original dynamic admission and error paths.

The timing scope is the experimental compact entry. Ordinary managed integration remains open.

## All primary timings

| Input | Bytes | R295 / Roc | R296 / Roc | R296 / R295 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.092387× | 2.128220× | 1.017125× | pass/pass |
| one-stored | 1 | 1.996955× | 2.027149× | 1.015120× | pass/pass |
| seven-stored-prefix | 7 | 2.229213× | 2.234221× | 1.002247× | pass/pass |
| eight-literals | 8 | 0.191964× | 0.192108× | 1.000751× | pass/pass |
| eight-overlap | 8 | 0.182984× | 0.182612× | 0.997967× | pass/pass |
| 320-overlap | 320 | 0.211588× | 0.210874× | 0.996625× | pass/pass |
| eight-literals-prefix | 8 | 0.193229× | 0.191655× | 0.991852× | pass/pass |
| 128-biased-dynamic | 128 | 0.252745× | 0.253175× | 1.001699× | pass/pass |
| 32-high-entropy-stored | 32 | 0.259244× | 0.258740× | 0.998053× | pass/pass |
| 16384-distribution-change | 16384 | 1.452397× | 1.446420× | 0.995885× | pass/pass |
| 33027-periodic-window | 33027 | 0.721470× | 0.697585× | 0.966895× | pass/pass |
| random-64 | 64 | 0.271635× | 0.271307× | 0.998791× | pass/pass |
| random-256 | 256 | 0.324767× | 0.324642× | 0.999614× | pass/pass |
| random-1024 | 1024 | 0.386247× | 0.386113× | 0.999654× | pass/pass |
| repeated-1024 | 1024 | 0.259264× | 0.256542× | 0.989502× | pass/pass |
| random-32769 | 32769 | 1.376759× | 1.377758× | 1.000725× | pass/pass |
| repeated-32769 | 32769 | 0.710809× | 0.693000× | 0.974946× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.500049× | 0.497605× | 0.995113× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.334750× | 1.329622× | 0.996159× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.390876× | 1.384562× | 0.995461× | pass/pass |
| heldout-window-32767 | 32767 | 1.389686× | 1.384257× | 0.996094× | pass/pass |
| heldout-window-32768 | 32768 | 1.388796× | 1.384308× | 0.996769× | pass/pass |
| heldout-slides-65537 | 65537 | 1.424473× | 1.422231× | 0.998426× | pass/pass |
| heldout-random-65537 | 65537 | 1.525784× | 1.528692× | 1.001906× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.388107× | 1.384119× | 0.997127× | pass/pass |
| prefix-natural-1024 | 1024 | 0.500611× | 0.497049× | 0.992884× | pass/pass |
| prefix-natural-4096 | 4096 | 1.028793× | 1.025017× | 0.996330× | pass/pass |
| prefix-natural-8192 | 8192 | 1.234548× | 1.226483× | 0.993467× | pass/pass |
| prefix-natural-16384 | 16384 | 1.346054× | 1.340921× | 0.996187× | pass/pass |
| prefix-natural-32767 | 32767 | 1.387699× | 1.381693× | 0.995672× | pass/pass |
| prefix-natural-32768 | 32768 | 1.391011× | 1.386637× | 0.996855× | pass/pass |
| prefix-natural-32769 | 32769 | 1.389996× | 1.385243× | 0.996580× | pass/pass |
| prefix-natural-33027 | 33027 | 1.388200× | 1.382486× | 0.995884× | pass/pass |
| prefix-natural-65535 | 65535 | 1.405519× | 1.397112× | 0.994018× | pass/pass |
| prefix-natural-65536 | 65536 | 1.402094× | 1.395430× | 0.995247× | pass/pass |
| prefix-natural-65537 | 65537 | 1.396861× | 1.393915× | 0.997891× | pass/pass |
| prefix-natural-131072 | 131072 | 1.404613× | 1.404336× | 0.999803× | pass/pass |
| prefix-natural-262144 | 262144 | 1.409105× | 1.409977× | 1.000619× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.421324× | 1.422624× | 1.000915× | pass/pass |
| prefix-random-1024 | 1024 | 0.390439× | 0.391076× | 1.001631× | pass/pass |
| prefix-random-4096 | 4096 | 0.652506× | 0.652288× | 0.999666× | pass/pass |
| prefix-random-8192 | 8192 | 0.822490× | 0.817932× | 0.994458× | pass/pass |
| prefix-random-16384 | 16384 | 1.139435× | 1.133228× | 0.994552× | pass/pass |
| prefix-random-32767 | 32767 | 1.378654× | 1.377326× | 0.999037× | pass/pass |
| prefix-random-32768 | 32768 | 1.382485× | 1.380630× | 0.998658× | pass/pass |
| prefix-random-32769 | 32769 | 1.378811× | 1.376265× | 0.998154× | pass/pass |
| prefix-random-33027 | 33027 | 1.382026× | 1.380138× | 0.998634× | pass/pass |
| prefix-random-65535 | 65535 | 1.555445× | 1.556417× | 1.000625× | pass/pass |
| prefix-random-65536 | 65536 | 1.556000× | 1.552692× | 0.997874× | pass/pass |
| prefix-random-65537 | 65537 | 1.555056× | 1.553727× | 0.999145× | pass/pass |
| prefix-random-131072 | 131072 | 1.650404× | 1.648093× | 0.998600× | pass/pass |
| prefix-random-262144 | 262144 | 1.690193× | 1.690625× | 1.000255× | pass/pass |
| prefix-random-1048576 | 1048576 | 1.713739× | 1.711775× | 0.998854× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.320064× | 0.317600× | 0.992302× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.452417× | 0.440364× | 0.973359× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.528327× | 0.517660× | 0.979810× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.629609× | 0.609123× | 0.967461× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.697862× | 0.681555× | 0.976633× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.698758× | 0.681483× | 0.975277× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.708400× | 0.689301× | 0.973040× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.714908× | 0.696103× | 0.973696× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.759757× | 0.738845× | 0.972475× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.761375× | 0.741233× | 0.973545× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.760791× | 0.740282× | 0.973042× | pass/pass |
| prefix-periodic-131072 | 131072 | 0.791630× | 0.769775× | 0.972392× | pass/pass |
| prefix-periodic-262144 | 262144 | 0.803124× | 0.782358× | 0.974144× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 0.811505× | 0.789754× | 0.973197× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.258798× | 0.256887× | 0.992619× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.400403× | 0.392166× | 0.979428× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.493566× | 0.483218× | 0.979033× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.594788× | 0.581547× | 0.977738× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.675563× | 0.658555× | 0.974823× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.679845× | 0.660345× | 0.971318× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.700974× | 0.682869× | 0.974172× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.721954× | 0.701253× | 0.971327× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.755667× | 0.736001× | 0.973976× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.756621× | 0.737754× | 0.975065× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.756564× | 0.738305× | 0.975866× | pass/pass |
| prefix-repeated-131072 | 131072 | 0.793164× | 0.774807× | 0.976856× | pass/pass |
| prefix-repeated-262144 | 262144 | 0.805626× | 0.786796× | 0.976627× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 0.805614× | 0.787382× | 0.977369× | pass/pass |

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r296-shared-window-limits-9948e46eb542.tar.gz). SHA-256 9948e46eb542279f6c8ef9077fcf6ce2ff79889d99d643858145318332c11582.
