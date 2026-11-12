# R355 — Weight actual loops in register allocation

This experiment was measured on the pinned compression compiler before the #990 integration. The result is recorded on top of integration commit `c99d62036a4a5f8b5ac38e44f01a1a5703cf0d3f`. The merged compiler has separate qualification pending.

**Retain R355 with the R272 runtime.** The 81-case primary geometric mean changes -4.066%. The fixed 17-case priority group changes -1.812% in the four-actor comparison.

In the matched run, the candidate wins 47/81 against Roc (58.02%), 8/81 against libdeflate and 0/81 against both.

## Full qualification

The two Mech executables and their complete-call boundaries match the prior campaign. Their launches use one byte-identical retained metadata file after verification of the complete input and output layouts.

368,496 complete calls across two fresh primary cohorts and 245,664 calls in a separate four-actor comparison. The comparisons fix level 12, runtime inputs, scratch construction and cleanup inside the call, warmup placement, and output ownership. Independent inflation and validation are outside the timing interval. Each libdeflate call constructs a fresh compressor using the pinned allocator.

Pooled gates pass 81/81. Block-median gates pass 81/81. All six heap distributions match on 81/81 cases. The highest primary movement is -0.330% on eight-literals. The highest matched movement is -0.184% on eight-literals-prefix.

| Input | R350 / Roc | Candidate / Roc | R350 / libdeflate | Candidate / libdeflate | Primary A | Primary B | Matched change |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| reymont-prefix-32769 | 1.207532× | 1.187702× | 1.101282× | 1.083197× | -1.748% | -1.958% | -1.642% |
| prefix-natural-262144 | 1.218120× | 1.197968× | 0.988171× | 0.971824× | -1.288% | -1.794% | -1.654% |
| prefix-natural-131072 | 1.216692× | 1.198636× | 1.001063× | 0.986207× | -1.390% | -1.994% | -1.484% |
| prefix-natural-1048576 | 1.233933× | 1.211662× | 0.995631× | 0.977661× | -1.896% | -1.852% | -1.805% |
| eight-literals | 0.183427× | 0.181837× | 2.333083× | 2.312853× | +0.219% | -0.876% | -0.867% |
| prefix-random-16384 | 1.019527× | 0.973704× | 2.755735× | 2.631877× | -4.406% | -4.340% | -4.495% |
| prefix-natural-65536 | 1.209767× | 1.191474× | 1.018069× | 1.002675× | -1.681% | -1.850% | -1.512% |
| prefix-repeated-1048576 | 0.692360× | 0.647542× | 1.090039× | 1.019478× | -6.594% | -6.610% | -6.473% |
| prefix-repeated-262144 | 0.695735× | 0.650503× | 1.131380× | 1.057826× | -6.468% | -6.475% | -6.501% |
| heldout-window-32767 | 1.208369× | 1.186594× | 1.102194× | 1.082331× | -1.668% | -1.874% | -1.802% |
| prefix-natural-32769 | 1.208501× | 1.186840× | 1.100603× | 1.080876× | -2.210% | -1.779% | -1.792% |
| prefix-natural-65535 | 1.210422× | 1.190118× | 1.016891× | 0.999834× | -1.803% | -1.956% | -1.677% |
| prefix-natural-65537 | 1.214363× | 1.191679× | 1.018055× | 0.999037× | -1.672% | -1.904% | -1.868% |
| eight-literals-prefix | 0.182728× | 0.182391× | 2.342924× | 2.338611× | -0.770% | -1.139% | -0.184% |

Matched crossings below Roc parity include prefix-random-16384.

Matched crossings below libdeflate parity include prefix-natural-65535, prefix-natural-65537, prefix-natural-131072.

## Empty input with shared metadata

| Empty call cohort | R350 block median | R355 block median | Change |
| --- | ---: | ---: | ---: |
| primary-a | 70.919 ns | 69.949 ns | -1.367% |
| primary-b | 72.428 ns | 69.754 ns | -3.692% |
| combined | 71.757 ns | 69.797 ns | -2.731% |

Both executables use the shared R344 startup descriptor. Their root Object body and numeric dispatch agree. The machine audit records every changed numeric body.


## Independent counter cohort

| Input | R350 instructions | R355 instructions | Instruction change | Cycle change |
| --- | ---: | ---: | ---: | ---: |
| reymont-prefix-32769 | 64,669,248 | 62,552,915 | -3.273% | -1.325% |
| prefix-natural-262144 | 633,702,361 | 611,267,480 | -3.540% | -1.353% |
| prefix-natural-1048576 | 2,290,715,163 | 2,187,596,550 | -4.502% | -1.510% |
| eight-literals | 716,289 | 716,072 | -0.030% | -1.747% |
| prefix-random-1048576 | 557,562,682 | 521,064,789 | -6.546% | -3.346% |

## Exact candidate profile after timing

Running timer samples include validation. They are a separate cohort from both elapsed time and hardware counters.

| Input | Running PCs | Matcher | Parser | Composed parser | Other |
| --- | ---: | ---: | ---: | ---: | ---: |
| prefix-natural-1048576 | 5352 | 2251 | 2747 | 2747 | 354 |
| prefix-natural-262144 | 5541 | 2081 | 3086 | 3086 | 374 |
| prefix-random-1048576 | 5354 | 3299 | 491 | 0 | 1564 |
| prefix-random-16384 | 5240 | 2271 | 544 | 0 | 2425 |

## Screen, machine code, and correctness evidence

**Advance to full timing.**

Measured compiler head `51bbe5d5f7a0a4cf40624287b83c90dc24cdbcf3`. Publication parent `c64f420292420901459d9bc9cd2d2d22bd74ac22`. PR base `92f75db84e2523693648084bea16cd52ad60777a`. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The timed workers use the existing compact diagnostic entry and the shared startup descriptor. Source, bytecode, allocator, output capacities and complete-call timing boundaries are pinned.

## Register allocation and linked code

Regalloc2 approximates nesting by the order of blocks and backward edges. The candidate finds dominating latches in reverse postorder and walks predecessors to build the natural loop forest. An inner loop is collapsed to its header during an enclosing traversal. Multiple latches share one loop and unreachable predecessors contribute no depth. The prior estimate remains in use for CFGs with a non-dominating reverse-postorder backedge. The timed patch climbs the dominator chain for each retreating edge. A deep graph with many latches can make those queries quadratic. An interval-based version is prepared for separate equivalence testing.

The exact retained matcher graph has 3,300 blocks and 38 natural loops. Its output publication block is assigned depth four by the layout estimate despite belonging to no loop. Depths differ for 953 blocks. All 209 output-pointer memory operands are outside natural loops. The search latch and depth comparison remain at depth two. Matcher CLIF is identical. In the selected search latch, two spills, a reload and four moves disappear. The linked conditional branch now jumps directly to the search header. Depth and tree base stay in registers at the inspected exit and child sites. Across the search layout range, static stack operations drop from 62 to 9. Across the skip layout range they drop from 128 to 34, with one new spill at the skip latch. These are static instruction counts. The vector matcher frame falls from 624 to 576 bytes. The change affects 18 of 351 bodies, including four parser variants that grow by 64 code bytes. The root Object body is byte-identical and all 4,600 calls and all vector selections agree.

## Paired screen

| Input | R350 / Roc | R355 / Roc | R350 / libdeflate | R355 / libdeflate | Matched change | Primary change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 2.076720× | 2.028440× | 0.016507× | 0.016123× | -2.325% | +0.000% |
| one-stored | 2.002783× | 1.964875× | 0.017838× | 0.017500× | -1.893% | +0.000% |
| seven-stored-prefix | 2.274213× | 2.208412× | 0.018817× | 0.018273× | -2.893% | +0.000% |
| eight-literals | 0.182322× | 0.181887× | 2.317620× | 2.312085× | -0.239% | +0.039% |
| reymont-prefix-32769 | 1.205062× | 1.187644× | 1.097378× | 1.081517× | -1.445% | -1.232% |
| heldout-window-32768 | 1.206871× | 1.190034× | 1.097636× | 1.082324× | -1.395% | -1.664% |
| prefix-natural-32767 | 1.207073× | 1.186829× | 1.100228× | 1.081775× | -1.677% | -1.648% |
| prefix-natural-32768 | 1.204987× | 1.186960× | 1.098788× | 1.082349× | -1.496% | -1.918% |
| prefix-natural-65537 | 1.216671× | 1.197304× | 1.021537× | 1.005276× | -1.592% | -1.877% |
| prefix-natural-131072 | 1.222185× | 1.199229× | 1.003298× | 0.984454× | -1.878% | -1.386% |
| prefix-natural-262144 | 1.221208× | 1.200610× | 0.985615× | 0.968991× | -1.687% | -1.784% |
| prefix-natural-1048576 | 1.235229× | 1.210675× | 0.997397× | 0.977571× | -1.988% | -1.793% |
| prefix-random-16384 | 1.022648× | 0.976774× | 2.763436× | 2.639475× | -4.486% | -4.479% |
| prefix-random-1048576 | 1.541371× | 1.480754× | 2.170342× | 2.084989× | -3.933% | -3.761% |
| prefix-periodic-1048576 | 0.688377× | 0.637450× | 1.168318× | 1.081885× | -7.398% | -7.459% |
| prefix-repeated-1048576 | 0.692232× | 0.647406× | 1.086179× | 1.015842× | -6.476% | -6.648% |

56,592 primary calls and 154,752 balanced four-actor calls. The 1.05 gates pass 16/16 pooled and 16/16 block. The primary paired geometric mean changes -2.251%. The matched priority mean changes -1.844%. All six heap distributions match on 16/16 cases.

## Separate hardware counters and profile

| Input | R350 instructions | R355 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 716,469.5 | 716,243.0 | -0.032% | -2.945% |
| reymont-prefix-32769 | 64,659,072.5 | 62,543,126.0 | -3.272% | -1.339% |
| prefix-natural-131072 | 311,988,985.0 | 301,311,445.0 | -3.422% | -1.403% |
| prefix-natural-262144 | 634,002,078.0 | 611,574,583.0 | -3.537% | -1.237% |
| prefix-natural-1048576 | 2,290,306,878.5 | 2,187,144,075.5 | -4.504% | -1.399% |
| prefix-random-16384 | 9,848,805.5 | 9,348,371.5 | -5.081% | -4.114% |
| prefix-random-1048576 | 557,567,061.0 | 521,075,027.0 | -6.545% | -3.495% |

| Input | Running timer PCs | Vector matcher PCs | Parser PCs |
|---|---:|---:|---:|
| eight-literals | 5257 | 680 | 0 |
| prefix-natural-1048576 | 5362 | 2288 | 2731 |
| prefix-natural-262144 | 5359 | 2032 | 2974 |
| prefix-random-1048576 | 5360 | 3325 | 0 |
| prefix-random-16384 | 5244 | 2267 | 0 |
| reymont-prefix-32769 | 4463 | 1684 | 2255 |

Profiles include validation. Hardware counters and sampling use separate cohorts.

## Qualification

Warnings-as-errors builds pass. The allocator test binary passes 15 tests, including five new CFGs for a terminal block interleaved in layout, shuffled nesting, multiple latches, a self-loop with an unreachable predecessor, and an irreducible cycle. Both image forms agree byte for byte and pass 324 complete stream checks with exact bytes, independent inflation, capacities, aliases and cleanup. The exact retained compiler re-emits its original Object and metadata.

[Verified register allocator patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r355-natural-loop-register-allocation-0cc27f7aba63.tar.gz). SHA-256 0cc27f7aba630c05733bcf8829ef7ad2986a4a9e0d7d67406d97871e5cf1eadb.
