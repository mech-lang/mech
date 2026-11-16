# R291 — Combine checked empty runs and reduction handoffs

**Retain R291 with the R272 runtime.** Candidate paired elapsed changes -0.262% across 81 fixed level-12 cases. The pooled and block gates pass 81/81 and 81/81. R290 beats Roc on 45/81 cases and R291 beats it on 45/81 (55.56%). The independent four-actor cohort gives R291 3/81 cases below libdeflate.

The two primary cohorts change elapsed by -0.452% and -0.071%. The largest paired slowdown is +1.967% on prefix-repeated-65537. Cases crossing below 1× Roc are none. Cases crossing above are none.

Pooled gate misses are none. Block gate misses are none.

Published parent 5f04a33851bb4038310d29c63712a184368a5d90. Measured parent 5f04a33851bb4038310d29c63712a184368a5d90. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R290 source closure and the six-file overlay. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The level-12 benchmark source, runtime inputs, tables, capacities, limits, allocator and complete-call timer are pinned.

## Compiler rule and emitted code

This tests the R289 completion proof under the R290 empty-loop entry proof. The latter still requires the actual unit count and cursor decrements, positive append length, current cache view and live literal-table guard. The former forwards the minimum, selected item and final induction after their original terminal tag is established. It bypasses two selected tag checks before the same parent cursor update. Alternate entries retain those checks and their current offset cost. The existing admission, source charges and checked failures apply.

Four of 347 bodies change. Each admits one completed edge with two selected conditions and zero repair parameters. All four shrink from 1,808 to 1,760 code bytes and from 272 to 256 frame bytes. One offset-cost reload disappears on the scalar tail. The root and matcher instruction bytes match the control.

| Linked path | R290 instructions / stack accesses | R291 instructions / stack accesses |
|---|---:|---:|
| From original append to admitted loop | 29 / 1 | 28 / 0 |
| Continuing empty visit | 24 / 0 | 24 / 0 |
| One candidate to parent | 13 / 0 | 7 / 0 |
| Scalar tail to parent | 14 / 1 | 7 / 0 |
| Vector prefix with no tail to parent | 15 / 0 | 9 / 0 |

Roc and libdeflate use 25 and 16 linked instructions respectively for their corresponding empty visits. The ordered candidate excerpts contain 11 and 14 instructions, respectively. Those excerpts have different boundaries from the Mech completion spans above. The packet also includes independently compiled safe Rust controls for both routes.

The 27-case screen changes paired elapsed by -0.281%. Its pooled and block miss counts are 0 and 0. Natural 4 KiB and 1 MiB change by -1.042% and -0.756%. The packet preserves the screen and both full cohorts.

## Headline matched ratios

| Input | R290 / Roc | R291 / Roc | R290 / libdeflate | R291 / libdeflate | R291 / R290 elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.198204× | 0.198646× | 2.523229× | 2.528860× | -0.540% |
| prefix-random-8192 | 0.879097× | 0.864606× | 3.161676× | 3.109561× | +0.336% |
| prefix-natural-4096 | 1.070267× | 1.055364× | 1.822284× | 1.796910× | -0.574% |
| prefix-natural-32768 | 1.447756× | 1.439962× | 1.320429× | 1.313321× | -0.797% |
| prefix-natural-1048576 | 1.473845× | 1.472239× | 1.190040× | 1.188742× | -0.531% |
| prefix-random-1048576 | 1.774716× | 1.779286× | 2.505780× | 2.512232× | +0.182% |
| prefix-periodic-1048576 | 0.820582× | 0.822567× | 1.390151× | 1.393515× | +0.139% |
| prefix-repeated-1048576 | 0.814741× | 0.817178× | 1.279751× | 1.283579× | +0.157% |

The four-actor cohort contains 245,664 measured calls. The reference ratios use its same-case balanced blocks. The paired elapsed-change column and parity score use the two primary cohorts.

## Correctness and independent counters

- All 324 original and decoded linked compression calls preserve exact streams, independent inflation, capacities, retained aliases and final cleanup. The original and decoded Objects and metadata are identical.
- All 44 cleanup tests pass with warnings as errors. The focused emitted admission test executes 43,923 calls across original descriptor, reloaded descriptor and full-width immediate cases. An independent checked-add oracle tests both accepted and declined inputs. The completed edge test adds 3,840 compiled before/after calls across six variants, including inactive tags, effect barriers, first reached errors and fresh visits.
- All 87 direct linked phase cases match the independent oracle across the retained and both candidate images. They cover an admitted run, zero trips, count greater than cursor, an out-of-view cursor, the full unsigned cursor, changed literal lengths, ties, checked error origin, failed publication and retry. The linked bodies match their source Objects.
- The logical payload count, requested bytes, copy and zeroing counters, peak private payload and outstanding private bytes agree for all 81 cases.

| Input | R290 instructions | R291 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 814,134.5 | 813,921.0 | -0.026% | -2.597% |
| prefix-random-8192 | 6,224,175.5 | 6,226,048.5 | +0.030% | -0.271% |
| prefix-natural-4096 | 10,823,941.0 | 10,419,702.5 | -3.735% | -0.653% |
| prefix-natural-32768 | 92,014,808.0 | 87,786,493.0 | -4.595% | +0.002% |
| prefix-natural-1048576 | 3,346,495,272.0 | 3,189,582,280.5 | -4.689% | +0.124% |
| prefix-random-1048576 | 710,632,729.0 | 710,542,843.5 | -0.013% | -0.210% |
| prefix-periodic-1048576 | 592,845,626.5 | 592,615,533.5 | -0.039% | +0.120% |
| prefix-repeated-1048576 | 600,576,896.5 | 600,354,488.0 | -0.037% | +0.221% |

The 1 MiB natural-text CPU worker reports a lifetime peak physical footprint of 54,592,448 bytes before and 54,576,064 bytes after. These process-wide samples include the harness.

Timing directions differ between cohorts on 31 cases. Both observations remain in the packet.

The initial assembly audit used an outdated CLIF value name. The corrected assertion checks the same literal ordinal in the unchanged candidate image.

## Exact executable profiles

The profile replays sample PCs from each separately recorded linked executable. The seed and continuing visit and the combined completion spans occupy disjoint PCs. Samples include validation and harness.

| Input | Running samples before / after | Parser before / after | Seed before / after | Continuing run before / after | Completion before / after |
|---|---:|---:|---:|---:|---:|
| eight-literals | 5462 / 5473 | 6 / 5 | 0 / 0 | 4 / 2 | 0 / 0 |
| prefix-natural-1048576 | 5363 / 5351 | 2680 / 2653 | 24 / 6 | 5 / 11 | 591 / 552 |
| prefix-natural-32768 | 5358 / 5371 | 2640 / 2666 | 32 / 9 | 23 / 20 | 495 / 443 |
| prefix-natural-4096 | 5584 / 5659 | 1572 / 1595 | 14 / 11 | 27 / 26 | 159 / 162 |
| prefix-periodic-1048576 | 5358 / 5354 | 675 / 717 | 1 / 2 | 524 / 548 | 0 / 1 |
| prefix-random-1048576 | 5372 / 5360 | 412 / 419 | 1 / 2 | 396 / 400 | 0 / 0 |
| prefix-repeated-1048576 | 5369 / 5363 | 642 / 660 | 6 / 1 | 497 / 501 | 2 / 1 |

The 1 MiB random profile has 3,438 vector matcher samples of 5,360 total. A taken edge preserves x3 after the first load from spill offset 568. The linked target loads the same register and slot again. The two PCs have 445 and 15 samples, respectively. The second PC and the actual CFG edge give a witness for the next pilot.

## Earlier experiments from the review brief

- R275 retained equality reuse in the two proved immutable skip scans. At 1 MiB periodic and repeated elapsed improved 8.17% and 7.63%. [Equality-reuse report](native-shifted-immutable-equality.md).
- R274 retained the selected item. R276 compared a dedicated singleton entry with one peeled visit and retained the latter. [Short-reduction report](native-short-reduction-first-visit.md).
- R277 tested the full byte map. It changed 1 MiB natural-text elapsed by -0.56%. The retained benchmark uses its original source. [Dense-map report](native-dense-offset-slot-experiment.md).
- R278 retained a coherent empty append run. R279 and R290 refined its descriptor lifetimes and entry bounds. [Empty-window report](native-empty-window.md).
- R272 retained private zero pages. R273 measured read-only scalar reductions and R282 measured guarded prefetch. A vector widening reduction and write-before-read proof still need their own evidence. [Scalar reduction](native-readonly-scalar-reductions.md) and [prefetch screen](native-private-write-prefetch.md).

The closest candidate case above parity is prefix-natural-4096 at 1.068077× Roc. Test spill-copy reuse across a nonadjacent unique predecessor. Count the verified reloads removed from the linked matcher and measure the whole-call effect.

The timing scope is the experimental compact entry. Ordinary managed integration remains open.

## All primary timings

| Input | Bytes | R290 / Roc | R291 / Roc | R291 / R290 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.167304× | 2.094550× | 0.966431× | pass/pass |
| one-stored | 1 | 2.035224× | 1.995594× | 0.980528× | pass/pass |
| seven-stored-prefix | 7 | 2.254802× | 2.228971× | 0.988544× | pass/pass |
| eight-literals | 8 | 0.200418× | 0.199334× | 0.994595× | pass/pass |
| eight-overlap | 8 | 0.188248× | 0.188885× | 1.003383× | pass/pass |
| 320-overlap | 320 | 0.218810× | 0.220005× | 1.005461× | pass/pass |
| eight-literals-prefix | 8 | 0.199530× | 0.200225× | 1.003483× | pass/pass |
| 128-biased-dynamic | 128 | 0.265141× | 0.266237× | 1.004132× | pass/pass |
| 32-high-entropy-stored | 32 | 0.265732× | 0.265544× | 0.999294× | pass/pass |
| 16384-distribution-change | 16384 | 1.527168× | 1.525276× | 0.998761× | pass/pass |
| 33027-periodic-window | 33027 | 0.726258× | 0.726848× | 1.000813× | pass/pass |
| random-64 | 64 | 0.280149× | 0.281676× | 1.005453× | pass/pass |
| random-256 | 256 | 0.337434× | 0.337671× | 1.000702× | pass/pass |
| random-1024 | 1024 | 0.412883× | 0.415026× | 1.005190× | pass/pass |
| repeated-1024 | 1024 | 0.264912× | 0.265282× | 1.001398× | pass/pass |
| random-32769 | 32769 | 1.443038× | 1.444558× | 1.001054× | pass/pass |
| repeated-32769 | 32769 | 0.712267× | 0.712621× | 1.000497× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.518199× | 0.520027× | 1.003528× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.398797× | 1.389601× | 0.993426× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.451284× | 1.437443× | 0.990462× | pass/pass |
| heldout-window-32767 | 32767 | 1.449302× | 1.438053× | 0.992238× | pass/pass |
| heldout-window-32768 | 32768 | 1.449259× | 1.435655× | 0.990613× | pass/pass |
| heldout-slides-65537 | 65537 | 1.482656× | 1.470920× | 0.992085× | pass/pass |
| heldout-random-65537 | 65537 | 1.588143× | 1.585785× | 0.998515× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.440458× | 1.434145× | 0.995618× | pass/pass |
| prefix-natural-1024 | 1024 | 0.518911× | 0.519367× | 1.000880× | pass/pass |
| prefix-natural-4096 | 4096 | 1.074246× | 1.068077× | 0.994257× | pass/pass |
| prefix-natural-8192 | 8192 | 1.294042× | 1.280955× | 0.989886× | pass/pass |
| prefix-natural-16384 | 16384 | 1.398326× | 1.388659× | 0.993087× | pass/pass |
| prefix-natural-32767 | 32767 | 1.451421× | 1.439997× | 0.992129× | pass/pass |
| prefix-natural-32768 | 32768 | 1.451145× | 1.439586× | 0.992035× | pass/pass |
| prefix-natural-32769 | 32769 | 1.449891× | 1.437736× | 0.991616× | pass/pass |
| prefix-natural-33027 | 33027 | 1.450358× | 1.437127× | 0.990877× | pass/pass |
| prefix-natural-65535 | 65535 | 1.454504× | 1.442686× | 0.991875× | pass/pass |
| prefix-natural-65536 | 65536 | 1.454160× | 1.441588× | 0.991354× | pass/pass |
| prefix-natural-65537 | 65537 | 1.448980× | 1.435717× | 0.990846× | pass/pass |
| prefix-natural-131072 | 131072 | 1.458876× | 1.446908× | 0.991796× | pass/pass |
| prefix-natural-262144 | 262144 | 1.461364× | 1.447346× | 0.990408× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.474605× | 1.466777× | 0.994692× | pass/pass |
| prefix-random-1024 | 1024 | 0.401857× | 0.403142× | 1.003199× | pass/pass |
| prefix-random-4096 | 4096 | 0.675110× | 0.671445× | 0.994572× | pass/pass |
| prefix-random-8192 | 8192 | 0.864099× | 0.867006× | 1.003364× | pass/pass |
| prefix-random-16384 | 16384 | 1.194136× | 1.193905× | 0.999807× | pass/pass |
| prefix-random-32767 | 32767 | 1.450016× | 1.445060× | 0.996582× | pass/pass |
| prefix-random-32768 | 32768 | 1.444536× | 1.443232× | 0.999098× | pass/pass |
| prefix-random-32769 | 32769 | 1.446940× | 1.445840× | 0.999240× | pass/pass |
| prefix-random-33027 | 33027 | 1.451219× | 1.445885× | 0.996325× | pass/pass |
| prefix-random-65535 | 65535 | 1.621945× | 1.620762× | 0.999271× | pass/pass |
| prefix-random-65536 | 65536 | 1.622112× | 1.621617× | 0.999695× | pass/pass |
| prefix-random-65537 | 65537 | 1.613712× | 1.612605× | 0.999314× | pass/pass |
| prefix-random-131072 | 131072 | 1.710582× | 1.706949× | 0.997877× | pass/pass |
| prefix-random-262144 | 262144 | 1.759353× | 1.759252× | 0.999943× | pass/pass |
| prefix-random-1048576 | 1048576 | 1.772741× | 1.775968× | 1.001820× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.329227× | 0.330832× | 1.004877× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.461306× | 0.458363× | 0.993620× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.530297× | 0.532251× | 1.003685× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.635485× | 0.632665× | 0.995563× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.710543× | 0.710030× | 0.999278× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.711756× | 0.711050× | 0.999008× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.717481× | 0.716494× | 0.998625× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.726270× | 0.723588× | 0.996308× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.771730× | 0.771130× | 0.999223× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.772775× | 0.771339× | 0.998142× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.772987× | 0.773039× | 1.000068× | pass/pass |
| prefix-periodic-131072 | 131072 | 0.799241× | 0.799254× | 1.000016× | pass/pass |
| prefix-periodic-262144 | 262144 | 0.812120× | 0.812477× | 1.000439× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 0.820829× | 0.821968× | 1.001388× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.263415× | 0.264198× | 1.002971× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.409654× | 0.408927× | 0.998227× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.496457× | 0.497049× | 1.001192× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.598875× | 0.600207× | 1.002223× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.683896× | 0.684169× | 1.000399× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.686052× | 0.685098× | 0.998609× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.709925× | 0.707799× | 0.997007× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.786189× | 0.763127× | 0.970666× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.799351× | 0.802009× | 1.003325× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.792689× | 0.793411× | 1.000910× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.769111× | 0.784239× | 1.019670× | pass/pass |
| prefix-repeated-131072 | 131072 | 0.801528× | 0.801228× | 0.999625× | pass/pass |
| prefix-repeated-262144 | 262144 | 0.813476× | 0.814541× | 1.001310× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 0.813906× | 0.815187× | 1.001573× | pass/pass |

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r291-empty-and-handoff-385cd6a1d97b.tar.gz). SHA-256 385cd6a1d97b05b1b6dd6322e84537bdb208096df0ff50a97ed685a8455fb482.
