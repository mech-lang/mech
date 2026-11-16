# R289 — Forward completed ordered reductions

**Retain R288 with the R272 runtime.** Candidate paired elapsed changes +0.149% across 81 fixed level-12 cases. The pooled and block gates pass 81/81 and 81/81. R288 beats Roc on 45/81 cases and R289 beats it on 45/81 (55.56%). The independent four-actor cohort gives R289 3/81 cases below libdeflate.

The two primary cohorts change elapsed by +0.316% and -0.018%. The largest paired slowdown is +1.756% on prefix-periodic-1048576. Cases crossing below 1× Roc are none. Cases crossing above are none.

Pooled gate misses are none. Block gate misses are none.

Published and measured parent de538c6e63a69bc16d03f5c03d4b4a37130a7385. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R288 source closure and the six-file overlay. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The level-12 benchmark source, runtime inputs, tables, capacities, limits, allocator and complete-call timer are pinned.

## Compiler rule and emitted code

The existing ordered-reduction proof emits a terminal tag after establishing the exact cost, selected item and final induction. The emitter records its completion block. Only those blocks in the capacity-proved no-growth callee can use the existing acyclic SSA forwarding proof. It follows current typed values through the two selected tag checks and stops at the original parent cursor update. An alternate entry continues through the original checks and uses its current offset cost. The rule preserves source funding, strict minimum ties, the checked path, reached errors, effects and original publication.

Four of 347 bodies change. Each has one eligible edge, two selected conditions and zero repair parameters. In the inspected CLIF, the completed block loses the offset-cost parameter and jumps directly to the parent with minimum, item, final induction and current cursor. The fallback still reads its offset cost. The before VCode saves the offset cost at stack offset 152 on two continuing entries and reloads it on completion. The after VCode has none of those three transfers. The four code bodies shrink from 1,744 to 1,712 bytes and their frames shrink from 272 to 256 bytes. The root and matcher bodies are identical.

| Selected path through parent handoff | R288 instructions / stack transfers | R289 instructions / stack transfers |
|---|---:|---:|
| One candidate | 11 / 0 | 7 / 0 |
| Last scalar tail candidate | 12 / 1 | 7 / 0 |
| Vector prefix with no tail | 13 / 0 | 9 / 0 |

The exact peer excerpts show an 11-instruction scalar Roc candidate visit with one cost comparison and two selections. The libdeflate loop uses a strict branch to commit an improvement. The safe Rust witness returns the current cost, item and cursor after walking two slices. These excerpts cover different loop boundaries from the Mech handoff spans.

The continuing empty parser visit uses the same 30 opcodes and zero stack accesses in both images. Its register assignment and linked offsets differ. It still computes the cache ordinal twice and checks the preceding node after forwarding the value. The matched-route shortening alone leaves the observed repetitive-input cycle increase unexplained.

The 27-case screen changes paired elapsed by +0.382%. Its pooled and block miss counts are 0 and 0. The 4 KiB natural case improves 1.10%, while 1 MiB natural, periodic and repeated slow by 0.57%, 1.63% and 1.59% respectively. The packet retains the complete screen and both full cohorts.

## Headline matched ratios

| Input | R288 / Roc | R289 / Roc | R288 / libdeflate | R289 / libdeflate | R289 / R288 elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.196876× | 0.196718× | 2.522400× | 2.520377× | -0.069% |
| prefix-random-8192 | 0.860138× | 0.870280× | 3.091741× | 3.128194× | -0.716% |
| prefix-natural-4096 | 1.065574× | 1.055204× | 1.819938× | 1.802227× | -1.268% |
| prefix-natural-32768 | 1.442815× | 1.443408× | 1.313717× | 1.314257× | +0.146% |
| prefix-natural-1048576 | 1.468797× | 1.475574× | 1.184393× | 1.189857× | +0.461% |
| prefix-random-1048576 | 1.778947× | 1.782779× | 2.519743× | 2.525171× | +0.228% |
| prefix-periodic-1048576 | 0.825465× | 0.839404× | 1.398178× | 1.421788× | +1.756% |
| prefix-repeated-1048576 | 0.817552× | 0.831064× | 1.287013× | 1.308284× | +1.579% |

The four-actor cohort contains 245,664 measured calls. The reference ratios use its same-case balanced blocks. The paired elapsed-change column and parity score use the two primary cohorts.

## Correctness and independent counters

- All 324 original and decoded linked compression calls preserve exact streams, independent inflation, capacities, retained aliases and final cleanup. The original and decoded Objects and metadata are identical.
- All 44 cleanup tests pass with warnings as errors. The focused selected-edge test makes 3,840 compiled before/after calls across six variants, including inactive and invalid tags, checked failure order, zero visits, fresh loop visits, calls and memory effects.
- All 57 direct linked phase cases match the independent oracle across the baseline and both candidate forms. They check incoming ties, strict wins, successive inputs, scalar and vector completions, wrapped costs, invalid reached lengths, unchanged failed publication and retry. The linked phase bytes match their source Objects.
- The logical payload count, requested bytes, copy and zeroing counters, peak private payload and outstanding private bytes agree for all 81 cases.

| Input | R288 instructions | R289 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 812,599.0 | 812,575.0 | -0.003% | -1.073% |
| prefix-random-8192 | 6,372,360.0 | 6,372,137.5 | -0.003% | -0.116% |
| prefix-natural-4096 | 10,683,466.5 | 10,420,762.0 | -2.459% | -1.492% |
| prefix-natural-32768 | 90,416,915.0 | 87,730,547.0 | -2.971% | -0.251% |
| prefix-natural-1048576 | 3,287,540,359.0 | 3,188,503,335.0 | -3.012% | +0.471% |
| prefix-random-1048576 | 728,441,217.5 | 728,378,529.0 | -0.009% | +0.200% |
| prefix-periodic-1048576 | 605,143,101.5 | 604,986,540.5 | -0.026% | +1.696% |
| prefix-repeated-1048576 | 612,878,117.0 | 612,722,933.5 | -0.025% | +1.572% |

The 1 MiB natural-text CPU worker reports a lifetime peak physical footprint of 54,576,064 bytes before and 54,576,064 bytes after. These process-wide samples include the harness.

Timing directions differ between cohorts on 18 cases. Both observations remain in the packet.

The initial assembly audit expected the same vector branch direction in both images. The corrected audit follows the actual taken and fallthrough routes. The primary and screen observations stay pinned.

## Exact executable profiles

The profile replays sample PCs from each separately recorded linked executable. The completion column counts the shared tail once per sample and excludes the deciding compare and branch. Samples include validation and harness.

| Input | Running samples before / after | Parser before / after | Handoff before / after | Handoff stack before / after | Empty loop before / after |
|---|---:|---:|---:|---:|---:|
| eight-literals | 5338 / 5345 | 7 / 7 | 0 / 0 | 0 / 0 | 2 / 1 |
| prefix-natural-1048576 | 5365 / 5366 | 2681 / 2686 | 546 / 500 | 280 / 0 | 8 / 10 |
| prefix-natural-32768 | 5366 / 5369 | 2636 / 2646 | 434 / 393 | 223 / 0 | 33 / 34 |
| prefix-natural-4096 | 5562 / 5604 | 1579 / 1536 | 107 / 94 | 66 / 0 | 38 / 37 |
| prefix-periodic-1048576 | 5354 / 5369 | 709 / 807 | 0 / 0 | 0 / 0 | 562 / 559 |
| prefix-random-1048576 | 5355 / 5360 | 434 / 438 | 0 / 0 | 0 / 0 | 420 / 423 |
| prefix-repeated-1048576 | 5350 / 5364 | 659 / 744 | 1 / 0 | 1 / 0 | 507 / 507 |

The closest candidate case above parity is prefix-natural-4096 at 1.061907× Roc. Try one current-window admission before the existing continuing empty run. Bound its cache cursor with the remaining visit count and current cache length, and use the preceding append to prove that the previous node exists. Retain the current literal-table check and the original fallback. Measure this on the retained compiler before reconsidering the handoff.

The timing scope is the experimental compact entry. Ordinary managed integration remains open.

## All primary timings

| Input | Bytes | R288 / Roc | R289 / Roc | R289 / R288 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.206943× | 2.104388× | 0.953531× | pass/pass |
| one-stored | 1 | 2.042173× | 2.033900× | 0.995949× | pass/pass |
| seven-stored-prefix | 7 | 2.213999× | 2.188166× | 0.988332× | pass/pass |
| eight-literals | 8 | 0.197155× | 0.197018× | 0.999305× | pass/pass |
| eight-overlap | 8 | 0.186132× | 0.185746× | 0.997924× | pass/pass |
| 320-overlap | 320 | 0.214795× | 0.214895× | 1.000465× | pass/pass |
| eight-literals-prefix | 8 | 0.197396× | 0.196844× | 0.997203× | pass/pass |
| 128-biased-dynamic | 128 | 0.262926× | 0.262691× | 0.999105× | pass/pass |
| 32-high-entropy-stored | 32 | 0.267408× | 0.266640× | 0.997129× | pass/pass |
| 16384-distribution-change | 16384 | 1.539787× | 1.523028× | 0.989116× | pass/pass |
| 33027-periodic-window | 33027 | 0.727882× | 0.735536× | 1.010515× | pass/pass |
| random-64 | 64 | 0.282307× | 0.282339× | 1.000114× | pass/pass |
| random-256 | 256 | 0.336785× | 0.336655× | 0.999613× | pass/pass |
| random-1024 | 1024 | 0.404368× | 0.404631× | 1.000648× | pass/pass |
| repeated-1024 | 1024 | 0.267596× | 0.267970× | 1.001399× | pass/pass |
| random-32769 | 32769 | 1.448508× | 1.443452× | 0.996510× | pass/pass |
| repeated-32769 | 32769 | 0.714719× | 0.717993× | 1.004582× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.525189× | 0.522831× | 0.995511× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.394518× | 1.392741× | 0.998726× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.444110× | 1.443519× | 0.999591× | pass/pass |
| heldout-window-32767 | 32767 | 1.441556× | 1.444020× | 1.001709× | pass/pass |
| heldout-window-32768 | 32768 | 1.440597× | 1.442748× | 1.001493× | pass/pass |
| heldout-slides-65537 | 65537 | 1.472664× | 1.475798× | 1.002128× | pass/pass |
| heldout-random-65537 | 65537 | 1.600803× | 1.599098× | 0.998935× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.442211× | 1.447321× | 1.003543× | pass/pass |
| prefix-natural-1024 | 1024 | 0.520645× | 0.518811× | 0.996477× | pass/pass |
| prefix-natural-4096 | 4096 | 1.075540× | 1.061907× | 0.987325× | pass/pass |
| prefix-natural-8192 | 8192 | 1.281618× | 1.272148× | 0.992611× | pass/pass |
| prefix-natural-16384 | 16384 | 1.399052× | 1.395028× | 0.997124× | pass/pass |
| prefix-natural-32767 | 32767 | 1.438245× | 1.441211× | 1.002062× | pass/pass |
| prefix-natural-32768 | 32768 | 1.443965× | 1.446069× | 1.001457× | pass/pass |
| prefix-natural-32769 | 32769 | 1.444041× | 1.449340× | 1.003669× | pass/pass |
| prefix-natural-33027 | 33027 | 1.444384× | 1.445405× | 1.000707× | pass/pass |
| prefix-natural-65535 | 65535 | 1.447789× | 1.451148× | 1.002320× | pass/pass |
| prefix-natural-65536 | 65536 | 1.446962× | 1.450841× | 1.002681× | pass/pass |
| prefix-natural-65537 | 65537 | 1.442360× | 1.444368× | 1.001392× | pass/pass |
| prefix-natural-131072 | 131072 | 1.452407× | 1.457778× | 1.003697× | pass/pass |
| prefix-natural-262144 | 262144 | 1.452392× | 1.459997× | 1.005236× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.468783× | 1.475559× | 1.004613× | pass/pass |
| prefix-random-1024 | 1024 | 0.400712× | 0.399690× | 0.997451× | pass/pass |
| prefix-random-4096 | 4096 | 0.681199× | 0.678797× | 0.996474× | pass/pass |
| prefix-random-8192 | 8192 | 0.875416× | 0.869149× | 0.992841× | pass/pass |
| prefix-random-16384 | 16384 | 1.198558× | 1.193398× | 0.995695× | pass/pass |
| prefix-random-32767 | 32767 | 1.449557× | 1.446709× | 0.998035× | pass/pass |
| prefix-random-32768 | 32768 | 1.448846× | 1.445825× | 0.997915× | pass/pass |
| prefix-random-32769 | 32769 | 1.443422× | 1.436792× | 0.995407× | pass/pass |
| prefix-random-33027 | 33027 | 1.451140× | 1.447586× | 0.997551× | pass/pass |
| prefix-random-65535 | 65535 | 1.626003× | 1.624168× | 0.998872× | pass/pass |
| prefix-random-65536 | 65536 | 1.623064× | 1.621437× | 0.998998× | pass/pass |
| prefix-random-65537 | 65537 | 1.623827× | 1.621085× | 0.998312× | pass/pass |
| prefix-random-131072 | 131072 | 1.715846× | 1.722978× | 1.004157× | pass/pass |
| prefix-random-262144 | 262144 | 1.762621× | 1.765423× | 1.001590× | pass/pass |
| prefix-random-1048576 | 1048576 | 1.784742× | 1.788820× | 1.002285× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.329046× | 0.328842× | 0.999380× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.461022× | 0.455389× | 0.987780× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.535213× | 0.536075× | 1.001612× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.633628× | 0.636384× | 1.004350× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.712029× | 0.719686× | 1.010753× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.716075× | 0.720370× | 1.005997× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.719644× | 0.728355× | 1.012104× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.729906× | 0.735583× | 1.007778× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.770807× | 0.781136× | 1.013401× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.772850× | 0.780142× | 1.009435× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.771276× | 0.781624× | 1.013417× | pass/pass |
| prefix-periodic-131072 | 131072 | 0.805771× | 0.813195× | 1.009213× | pass/pass |
| prefix-periodic-262144 | 262144 | 0.818476× | 0.827327× | 1.010813× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 0.824989× | 0.839476× | 1.017560× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.264009× | 0.264761× | 1.002848× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.413383× | 0.411642× | 0.995786× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.500385× | 0.501642× | 1.002513× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.603295× | 0.608165× | 1.008072× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.685865× | 0.692649× | 1.009890× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.689814× | 0.696924× | 1.010306× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.712575× | 0.717434× | 1.006819× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.730846× | 0.734140× | 1.004507× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.766697× | 0.772041× | 1.006971× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.770031× | 0.777113× | 1.009197× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.768562× | 0.774799× | 1.008115× | pass/pass |
| prefix-repeated-131072 | 131072 | 0.803929× | 0.814177× | 1.012747× | pass/pass |
| prefix-repeated-262144 | 262144 | 0.815758× | 0.829077× | 1.016327× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 0.818014× | 0.830930× | 1.015790× | pass/pass |

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r289-reduction-handoff-359f5531d865.tar.gz). SHA-256 359f5531d865fafb3afeb0e17897275f7887b4ea724667018df0a6c6b46d8860.
