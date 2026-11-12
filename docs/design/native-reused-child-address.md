# R324 — Reuse the address carried into tree descent

**Keep R320 with the R272 runtime.** The 256 KiB natural case moves from 1.074343× to 1.086044× libdeflate. The candidate changes elapsed +1.089% in the matched comparison and +1.092% in the separate timing screen. The paired geometric mean changes +0.258% across 15 cases. The pooled and block gates pass 15/15 and 15/15.

Measured parent ecd961c4f707cb61f23dd4cb5cc2be99c82b92ea. Published parent 118de8daf63318e5dd914fe37581cea264c38f96. PR base 92f75db84e2523693648084bea16cd52ad60777a. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, IR/work/ABI/bytecode 215/107/39/14. The original source, bytecode, capacities, allocator, search limits, tables and complete-call boundary are pinned.

## Carry and reuse a typed address

R316 carried addresses through the loop but still used independently folded addresses for the four child lookups. R324 tests the original typed-address transformation on R320 and adds a halfword addressing rule. During backward selection an already-lowered use can require the complete address in a register. A zero-displacement access to that same shifted address can use its register directly. Standard folding remains for single-use addresses and nonzero displacements. The isolated backend build with address carrying disabled reproduces the retained Object and metadata exactly.

The compiler changes twelve store operands and passes each previously computed child address through its existing backedge slot. The original memory access widths, flags and order match. The four child loads now use the register containing the same address that is carried forward. The search stores avoid the immediate stack reload of the tree base. The following address construction still reloads that base. The C executable keeps its two pending child pointers and loads through the newly computed pointer.

Only the matcher changes among 347 bodies. Its code is 12,784 to 12,720 bytes and its frame 624 to 624 bytes. Static stack reads move 541 to 536, and stack writes 114 to 126.

## Matched results

| Input | R320 / Roc | R324 / Roc | R320 / libdeflate | R324 / libdeflate | Primary elapsed change |
|---|---:|---:|---:|---:|---:|
| empty | 2.096996× | 2.146918× | 0.016508× | 0.016901× | +0.000% |
| one-stored | 1.999700× | 2.057055× | 0.017690× | 0.018197× | +0.000% |
| seven-stored-prefix | 2.222481× | 2.290039× | 0.018483× | 0.019045× | +0.000% |
| eight-literals | 0.181917× | 0.181392× | 2.315682× | 2.309002× | -0.632% |
| reymont-prefix-32769 | 1.319182× | 1.328843× | 1.197334× | 1.206103× | +0.809% |
| heldout-window-32768 | 1.317190× | 1.325928× | 1.197356× | 1.205298× | +0.760% |
| prefix-natural-32767 | 1.315423× | 1.324978× | 1.195551× | 1.204236× | +0.733% |
| prefix-natural-32768 | 1.312107× | 1.325713× | 1.193336× | 1.205711× | +0.524% |
| prefix-natural-65537 | 1.316318× | 1.325605× | 1.113490× | 1.121346× | +0.607% |
| prefix-natural-131072 | 1.324563× | 1.340624× | 1.090628× | 1.103853× | +0.596% |
| prefix-natural-262144 | 1.331616× | 1.346119× | 1.074343× | 1.086044× | +1.092% |
| prefix-natural-1048576 | 1.337303× | 1.351997× | 1.079199× | 1.091057× | +0.868% |
| prefix-random-16384 | 1.077799× | 1.073234× | 2.919001× | 2.906638× | -1.201% |
| prefix-periodic-1048576 | 0.708541× | 0.708105× | 1.202392× | 1.201653× | -0.157% |
| prefix-repeated-1048576 | 0.710992× | 0.710775× | 1.117332× | 1.116990× | -0.096% |

The comparison contains 154,656 timed calls. It uses balanced level-12 runtime input, matching validation and fresh C compressor allocation.

## Independent counters and qualification

On 256 KiB natural the separate CPU cohort reports 759,424,081.0 to 759,122,282.0 retired instructions (-0.040%) and +0.608% cycles. The six payload counters agree across all 15 memory cases.

The candidate profile records 1,944 of 5,364 running samples in the matcher. The separate R320 profile records 1,915 of 5,359. Both runs include validation.

Stack-load PCs account for 25.80% to 33.28% of the separate running matcher samples. The static stack-load count falls while more running samples land on stack loads. Retired instructions and cycles come from a separate whole-call cohort.

The warnings-as-errors build and 57 CFG tests pass. The 180 before/after JIT pairs cover the typed-address proof, numerical and alternate base uses, shared expressions, missing address seeds, changed bases, wrapping arithmetic and reached and rejected loop visits. The 320 AArch64 selector calls cover shared and single-use addresses, commuted inputs, two shift amounts and nonzero displacement. The original image passes 162 linked calls with exact streams, independent inflation, capacities, held aliases and final managed cleanup.

Both timing comparisons slow the 256 KiB natural target, 1 MiB natural and Reymont. Keep the measured R320 compiler.

The candidate stopped at the 15-case timing screen with original-image qualification. Full timing and decoded-image qualification remain acceptance steps for any adoption.

[Verified patch, linked machine code and pilot observations](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r324-reused-child-address-64f33a967557.tar.gz). SHA-256 64f33a967557eab17b4adc7cda6c66e04e79e0757754b8db59bd56bb3af6f4b0.
