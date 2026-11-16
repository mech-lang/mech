# R290 — Prove readable bounds at empty-run entry

**Retain R290 with the R272 runtime.** Candidate paired elapsed changes -0.308% across 81 fixed level-12 cases. The pooled and block gates pass 81/81 and 81/81. R288 beats Roc on 45/81 cases and R290 beats it on 45/81 (55.56%). The independent four-actor cohort gives R290 3/81 cases below libdeflate.

The two primary cohorts change elapsed by -0.400% and -0.216%. The largest paired slowdown is +1.474% on seven-stored-prefix. Cases crossing below 1× Roc are none. Cases crossing above are none.

Pooled gate misses are none. Block gate misses are none.

Published parent 74dcb741f5e0d5cfc6ddc5aed0b2f4d31e18a3ff. Measured parent de538c6e63a69bc16d03f5c03d4b4a37130a7385. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R288 source closure and the two-file overlay. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The level-12 benchmark source, runtime inputs, tables, capacities, limits, allocator and complete-call timer are pinned.

## Compiler rule and emitted code

The selected no-growth recurrence first performs its original append. The continuing empty run checks that remaining positions fit in the current cache cursor, that the cursor fits the same current cache view, and that the seeded node length stays positive without wrapping across those visits. The proof requires the actual unit decrements of count and cursor, the existing append length increment and a stable cache length. The current literal-table check and nonempty exit remain. A declined entry continues after the real append. A declined visit restarts the same original visit before any write.

Four of 347 bodies change. Each moves two checks from the continuing visit to the seed admission. The current view is reloaded through the existing packet path. The CLIF keeps the exact nonempty and literal-table exits and the original write. Each affected body grows from 1,744 to 1,808 bytes and keeps a 272-byte frame. The root, matcher and other bodies retain their instruction bytes.

| Linked path | R288 instructions / stack accesses | R290 instructions / stack accesses |
|---|---:|---:|
| From original append to admitted loop | 12 / 1 | 29 / 1 |
| Continuing empty visit | 30 / 0 | 24 / 0 |

The measured Roc worker has a 25-instruction continuing empty visit with a previous-node load and a capacity check. Libdeflate has a comparable 16-instruction visit with the previous cost in a register. The safe Rust witness slices the readable and writable ranges before the loop and retains the current literal-table check. The packet includes the exact linked excerpts and the Rust source.

The 27-case screen changes paired elapsed by -0.525%. Its pooled and block miss counts are 0 and 0. The 4 KiB natural case improves 0.59%. The 1 MiB natural case slows 0.37% and the 8 KiB random case slows 1.27%. The packet retains the complete screen and both full cohorts.

## Headline matched ratios

| Input | R288 / Roc | R290 / Roc | R288 / libdeflate | R290 / libdeflate | R290 / R288 elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.198503× | 0.198765× | 2.514570× | 2.517892× | +0.115% |
| prefix-random-8192 | 0.862860× | 0.863339× | 3.104736× | 3.106458× | -0.534% |
| prefix-natural-4096 | 1.063857× | 1.058589× | 1.821370× | 1.812350× | -0.810% |
| prefix-natural-32768 | 1.440659× | 1.449435× | 1.310253× | 1.318235× | +0.644% |
| prefix-natural-1048576 | 1.467280× | 1.472776× | 1.183473× | 1.187906× | +0.470% |
| prefix-random-1048576 | 1.782865× | 1.771598× | 2.520355× | 2.504427× | -0.659% |
| prefix-periodic-1048576 | 0.826206× | 0.820947× | 1.396533× | 1.387645× | -0.538% |
| prefix-repeated-1048576 | 0.817260× | 0.813896× | 1.286007× | 1.280713× | -0.520% |

The four-actor cohort contains 245,664 measured calls. The reference ratios use its same-case balanced blocks. The paired elapsed-change column and parity score use the two primary cohorts.

## Correctness and independent counters

- All 324 original and decoded linked compression calls preserve exact streams, independent inflation, capacities, retained aliases and final cleanup. The original and decoded Objects and metadata are identical.
- All 43 cleanup tests pass with warnings as errors. The focused emitted admission test executes 43,923 calls across original descriptor, reloaded descriptor and full-width immediate cases. An independent checked-add oracle tests both accepted and declined inputs.
- All 87 direct linked phase cases match the independent oracle across the retained and both candidate images. They cover an admitted run, zero trips, count greater than cursor, an out-of-view cursor, the full unsigned cursor, changed literal lengths, ties, checked error origin, failed publication and retry. The linked bodies match their source Objects.
- The logical payload count, requested bytes, copy and zeroing counters, peak private payload and outstanding private bytes agree for all 81 cases.

| Input | R288 instructions | R290 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 812,840.5 | 812,529.0 | -0.038% | -0.264% |
| prefix-random-8192 | 6,371,647.0 | 6,224,995.5 | -2.302% | -0.365% |
| prefix-natural-4096 | 10,683,562.0 | 10,825,963.0 | +1.333% | -0.416% |
| prefix-natural-32768 | 90,416,023.0 | 92,015,807.0 | +1.769% | -0.113% |
| prefix-natural-1048576 | 3,287,582,167.0 | 3,346,596,489.0 | +1.795% | +0.471% |
| prefix-random-1048576 | 728,412,152.0 | 710,599,819.5 | -2.445% | -0.402% |
| prefix-periodic-1048576 | 605,145,007.5 | 592,848,206.5 | -2.032% | -0.420% |
| prefix-repeated-1048576 | 612,874,067.0 | 600,579,241.5 | -2.006% | -0.358% |

The 1 MiB natural-text CPU worker reports a lifetime peak physical footprint of 54,543,296 bytes before and 54,543,296 bytes after. These process-wide samples include the harness.

Timing directions differ between cohorts on 9 cases. Both observations remain in the packet.

## Exact executable profiles

The profile replays sample PCs from each separately recorded linked executable. The seed and continuing visit spans are disjoint. Samples include validation and harness.

| Input | Running samples before / after | Parser before / after | Seed before / after | Continuing run before / after |
|---|---:|---:|---:|---:|
| eight-literals | 5338 / 5462 | 7 / 6 | 1 / 0 | 2 / 4 |
| prefix-natural-1048576 | 5365 / 5363 | 2681 / 2680 | 20 / 24 | 8 / 5 |
| prefix-natural-32768 | 5366 / 5358 | 2636 / 2640 | 27 / 32 | 33 / 23 |
| prefix-natural-4096 | 5562 / 5584 | 1579 / 1572 | 8 / 14 | 38 / 27 |
| prefix-periodic-1048576 | 5354 / 5358 | 709 / 675 | 0 / 1 | 562 / 524 |
| prefix-random-1048576 | 5355 / 5372 | 434 / 412 | 3 / 1 | 420 / 396 |
| prefix-repeated-1048576 | 5350 / 5369 | 659 / 642 | 1 / 6 | 507 / 497 |

The closest candidate case above parity is prefix-natural-4096 at 1.066012× Roc. Test the measured completion forwarding from R289 on the shorter checked empty route. Compare the two route bodies and require fresh paired timings before combining the optimizations.

The timing scope is the experimental compact entry. Ordinary managed integration remains open.

## All primary timings

| Input | Bytes | R288 / Roc | R290 / Roc | R290 / R288 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.061746× | 2.083637× | 1.010618× | pass/pass |
| one-stored | 1 | 2.000324× | 2.007865× | 1.003770× | pass/pass |
| seven-stored-prefix | 7 | 2.233627× | 2.266551× | 1.014740× | pass/pass |
| eight-literals | 8 | 0.194754× | 0.194978× | 1.001147× | pass/pass |
| eight-overlap | 8 | 0.185509× | 0.185571× | 1.000334× | pass/pass |
| 320-overlap | 320 | 0.214790× | 0.214586× | 0.999052× | pass/pass |
| eight-literals-prefix | 8 | 0.197190× | 0.197344× | 1.000781× | pass/pass |
| 128-biased-dynamic | 128 | 0.261171× | 0.260785× | 0.998523× | pass/pass |
| 32-high-entropy-stored | 32 | 0.264934× | 0.264084× | 0.996791× | pass/pass |
| 16384-distribution-change | 16384 | 1.550286× | 1.521915× | 0.981699× | pass/pass |
| 33027-periodic-window | 33027 | 0.730976× | 0.726304× | 0.993608× | pass/pass |
| random-64 | 64 | 0.277476× | 0.277694× | 1.000786× | pass/pass |
| random-256 | 256 | 0.333162× | 0.332706× | 0.998632× | pass/pass |
| random-1024 | 1024 | 0.399037× | 0.398839× | 0.999503× | pass/pass |
| repeated-1024 | 1024 | 0.262902× | 0.261924× | 0.996281× | pass/pass |
| random-32769 | 32769 | 1.445191× | 1.437872× | 0.994936× | pass/pass |
| repeated-32769 | 32769 | 0.717081× | 0.710215× | 0.990425× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.519311× | 0.517862× | 0.997210× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.389775× | 1.393609× | 1.002758× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.441063× | 1.449839× | 1.006091× | pass/pass |
| heldout-window-32767 | 32767 | 1.437844× | 1.446462× | 1.005993× | pass/pass |
| heldout-window-32768 | 32768 | 1.432071× | 1.435054× | 1.002084× | pass/pass |
| heldout-slides-65537 | 65537 | 1.472088× | 1.477750× | 1.003846× | pass/pass |
| heldout-random-65537 | 65537 | 1.599661× | 1.588791× | 0.993205× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.444773× | 1.438265× | 0.995495× | pass/pass |
| prefix-natural-1024 | 1024 | 0.520245× | 0.518655× | 0.996942× | pass/pass |
| prefix-natural-4096 | 4096 | 1.074719× | 1.066012× | 0.991899× | pass/pass |
| prefix-natural-8192 | 8192 | 1.284239× | 1.282243× | 0.998446× | pass/pass |
| prefix-natural-16384 | 16384 | 1.395516× | 1.398169× | 1.001901× | pass/pass |
| prefix-natural-32767 | 32767 | 1.447379× | 1.452555× | 1.003576× | pass/pass |
| prefix-natural-32768 | 32768 | 1.439509× | 1.448784× | 1.006443× | pass/pass |
| prefix-natural-32769 | 32769 | 1.437986× | 1.447479× | 1.006602× | pass/pass |
| prefix-natural-33027 | 33027 | 1.444941× | 1.449299× | 1.003016× | pass/pass |
| prefix-natural-65535 | 65535 | 1.443225× | 1.449617× | 1.004429× | pass/pass |
| prefix-natural-65536 | 65536 | 1.441860× | 1.448856× | 1.004852× | pass/pass |
| prefix-natural-65537 | 65537 | 1.437736× | 1.446016× | 1.005759× | pass/pass |
| prefix-natural-131072 | 131072 | 1.448706× | 1.453240× | 1.003129× | pass/pass |
| prefix-natural-262144 | 262144 | 1.452005× | 1.460967× | 1.006172× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.468055× | 1.474953× | 1.004699× | pass/pass |
| prefix-random-1024 | 1024 | 0.400780× | 0.400640× | 0.999651× | pass/pass |
| prefix-random-4096 | 4096 | 0.686193× | 0.680792× | 0.992128× | pass/pass |
| prefix-random-8192 | 8192 | 0.866876× | 0.862246× | 0.994658× | pass/pass |
| prefix-random-16384 | 16384 | 1.203162× | 1.197442× | 0.995246× | pass/pass |
| prefix-random-32767 | 32767 | 1.448695× | 1.440388× | 0.994266× | pass/pass |
| prefix-random-32768 | 32768 | 1.448808× | 1.441428× | 0.994906× | pass/pass |
| prefix-random-32769 | 32769 | 1.446391× | 1.439359× | 0.995138× | pass/pass |
| prefix-random-33027 | 33027 | 1.457789× | 1.447013× | 0.992608× | pass/pass |
| prefix-random-65535 | 65535 | 1.625522× | 1.617184× | 0.994871× | pass/pass |
| prefix-random-65536 | 65536 | 1.624883× | 1.616774× | 0.995009× | pass/pass |
| prefix-random-65537 | 65537 | 1.619923× | 1.610963× | 0.994469× | pass/pass |
| prefix-random-131072 | 131072 | 1.719405× | 1.709220× | 0.994076× | pass/pass |
| prefix-random-262144 | 262144 | 1.762800× | 1.754036× | 0.995028× | pass/pass |
| prefix-random-1048576 | 1048576 | 1.781316× | 1.769577× | 0.993410× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.328953× | 0.328204× | 0.997723× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.464547× | 0.457184× | 0.984150× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.539435× | 0.533855× | 0.989655× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.638246× | 0.636551× | 0.997343× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.712725× | 0.708350× | 0.993863× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.717250× | 0.711496× | 0.991978× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.720027× | 0.716170× | 0.994643× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.729090× | 0.724115× | 0.993177× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.776397× | 0.770728× | 0.992698× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.776510× | 0.769063× | 0.990410× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.778243× | 0.771113× | 0.990838× | pass/pass |
| prefix-periodic-131072 | 131072 | 0.805831× | 0.800617× | 0.993529× | pass/pass |
| prefix-periodic-262144 | 262144 | 0.818232× | 0.813640× | 0.994388× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 0.825287× | 0.820850× | 0.994624× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.263574× | 0.262907× | 0.997470× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.408431× | 0.404315× | 0.989924× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.502667× | 0.498053× | 0.990821× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.599045× | 0.594472× | 0.992365× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.687246× | 0.682338× | 0.992859× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.688599× | 0.684328× | 0.993798× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.708910× | 0.706090× | 0.996023× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.730047× | 0.721882× | 0.988816× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.769655× | 0.763262× | 0.991694× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.771184× | 0.766276× | 0.993636× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.772525× | 0.766070× | 0.991644× | pass/pass |
| prefix-repeated-131072 | 131072 | 0.805828× | 0.799061× | 0.991602× | pass/pass |
| prefix-repeated-262144 | 262144 | 0.817595× | 0.814138× | 0.995771× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 0.817975× | 0.813718× | 0.994796× | pass/pass |

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r290-empty-window-ae112b5e355f.tar.gz). SHA-256 ae112b5e355fccf8aa2ce21423773215f97e203e802d82072ce47daa022a7bb5.
