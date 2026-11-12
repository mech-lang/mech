# R341 — Test a direct stack address

**Retain R333 with the R272 runtime.** The fixed-layout diagnostic stops at the 16-case screen. Natural 256 KiB changes +0.111% in the matched comparison and +0.166% in the primary screen. Reymont changes +0.091% and +0.457% in those comparisons.

Measured compiler ebc837def6d5c878cb3ecdeadd0e3f078a3f12e4. Published parent 987d7507e08b3fc5a01380a0523d4b5214ed0c95. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the exact retained R333 Object and an archived four-word substitution. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The benchmark source, bytecode, allocator, limits, tables, capacities and complete-call boundary are pinned.

## Replace the address reload at the same instruction address

The linked frame computes the table base as sp + 0x60. A dominating store publishes that address to sp + 0x870 before any lookup. The setup loop fills the two banks within [sp + 0x60, sp + 0x860). The spill lives beyond the initialized table. The proof walks the linked branches, verifies the only direct accesses to this spill and checks fixed stack geometry through the load. The live value agrees with the one-instruction address calculation.

The diagnostic changes four 32-bit instructions from ldr x11, [sp, #0x870] to add x11, sp, #0x60. It changes 12 Object bytes. All four linked workers keep the same text outside those four words and the same symbol addresses. The metadata, register assignments, 1,520-byte phase sizes, 2,272-byte frames and cost-table preparation agree. The current mapped route has 14 instructions and goes from two loads to one. The default route keeps the original lookup.

The pinned reference materializes a static slot-map base with adr. The C binary keeps its cost and slot-map bases in registers through the reduction. The separate Rust fixture forms a fixed-frame table address directly and keeps it in a register. The experiment tests an address dependency on the retained program.

## Matched measurements

| Input | R333 / Roc | R341 / Roc | R333 / libdeflate | R341 / libdeflate | Matched change | Primary change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 2.115441× | 2.136408× | 0.016654× | 0.016819× | +0.991% | +0.000% |
| one-stored | 1.995740× | 1.999806× | 0.017746× | 0.017782× | +0.204% | +0.000% |
| seven-stored-prefix | 2.235233× | 2.219960× | 0.018632× | 0.018504× | -0.683% | +0.000% |
| eight-literals | 0.181731× | 0.181872× | 2.312210× | 2.314010× | +0.078% | +0.178% |
| reymont-prefix-32769 | 1.257781× | 1.258926× | 1.147641× | 1.148685× | +0.091% | +0.457% |
| heldout-window-32768 | 1.257139× | 1.259484× | 1.144441× | 1.146576× | +0.187% | +0.574% |
| prefix-natural-32767 | 1.258050× | 1.258556× | 1.148010× | 1.148472× | +0.040% | +0.106% |
| prefix-natural-32768 | 1.257475× | 1.257322× | 1.147206× | 1.147066× | -0.012% | +0.391% |
| prefix-natural-65537 | 1.268570× | 1.270692× | 1.068308× | 1.070095× | +0.167% | +0.192% |
| prefix-natural-131072 | 1.260775× | 1.265163× | 1.041027× | 1.044651× | +0.348% | +0.204% |
| prefix-natural-262144 | 1.279854× | 1.281277× | 1.034609× | 1.035759× | +0.111% | +0.166% |
| prefix-natural-1048576 | 1.281286× | 1.284059× | 1.035759× | 1.038001× | +0.216% | +0.186% |
| prefix-random-16384 | 1.069252× | 1.062283× | 2.884289× | 2.865491× | -0.652% | -0.395% |
| prefix-random-1048576 | 1.592662× | 1.589546× | 2.252573× | 2.248165× | -0.196% | -0.078% |
| prefix-periodic-1048576 | 0.706778× | 0.705133× | 1.207407× | 1.204597× | -0.233% | +0.008% |
| prefix-repeated-1048576 | 0.710554× | 0.710493× | 1.116797× | 1.116702× | -0.009% | -0.197% |

154,752 balanced complete calls. The screen gates pass 16/16 pooled and 16/16 block. The primary paired geometric mean changes +0.112%. The highest matched change is +0.991% on empty.

| Input | R333 instructions | R341 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 715,833.5 | 715,922.0 | +0.012% | -0.975% |
| reymont-prefix-32769 | 70,038,504.5 | 70,038,402.5 | -0.000% | +0.159% |
| prefix-natural-262144 | 693,693,164.0 | 693,741,745.0 | +0.007% | +0.493% |
| prefix-natural-1048576 | 2,488,052,165.5 | 2,488,120,609.5 | +0.003% | +0.469% |
| prefix-random-16384 | 10,196,097.0 | 10,199,218.5 | +0.031% | +0.205% |
| prefix-random-1048576 | 576,162,098.0 | 576,156,726.0 | -0.001% | +0.392% |
| prefix-periodic-1048576 | 488,910,145.0 | 488,916,913.0 | +0.001% | +0.012% |
| prefix-repeated-1048576 | 496,680,537.5 | 496,681,957.0 | +0.000% | +0.060% |

The six heap distributions match on 16/16 cases. The separately sampled profiles use the exact candidate and include validation.

| Candidate profile input | Running samples | Composed phase linked PCs |
|---|---:|---:|
| eight-literals | 5,218 | 0 |
| prefix-natural-262144 | 5,346 | 3,047 |
| reymont-prefix-32769 | 5,121 | 2,617 |

## Qualification

Warnings-as-errors worker builds pass. The linked oracle checks all 65,536 offsets on each of two current cost snapshots and both image forms for 262,144 results. It includes both defaults, changed costs at the same address, the source rejection origin and retry. The 324 full checks preserve exact streams, independent inflation, capacities, held aliases and cleanup. The benchmark uses the compact diagnostic entry. The change is a fixed-layout diagnostic of generated code.

The retained all-81 scoreboard remains R333. The archive preserves every pilot result.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r341-stack-address-16e1853060cd.tar.gz). SHA-256 16e1853060cde039af57af5a39f020765a993301bd3b4899aabf9a0fb4a85a8b.
