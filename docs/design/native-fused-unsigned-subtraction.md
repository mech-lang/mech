# R323 — Fuse unsigned subtraction and selection

**Keep R320 with the R272 runtime.** The 256 KiB natural case moves from 1.076893× to 1.078831× libdeflate. The candidate changes elapsed +0.180% in the matched comparison and +0.084% in the separate timing screen. The paired geometric mean changes +0.160% across 15 cases. The pooled and block gates pass 15/15 and 15/15.

Measured parent ecd961c4f707cb61f23dd4cb5cc2be99c82b92ea. Published parent 549e7052a6fe3c1370c207041fec88bed81623bb. PR base 92f75db84e2523693648084bea16cd52ad60777a. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, IR/work/ABI/bytecode 215/107/39/14. The source, bytecode, capacities, allocator, search limits, tables and complete-call boundary are pinned.

## Native selection

The Cranelift AArch64 selector recognizes the I64 selection of zero on unsigned less-than and the subtraction of those same operands otherwise. It emits SUBS and CSEL with the hardware zero register. The reference executable uses that same pair. The C executable uses signed offsets into its input window. The R322 and R323 matcher CLIF hashes match.

Only the matcher changes among 347 bodies. From R320 through R322 and R323 the body is 12,784, 12,720 and 12,712 bytes. The stack frame stays at 624 bytes. Static stack reads are 541, 553 and 552 respectively. The linked selection takes two instructions in each direction.

## Matched results

| Input | R320 / Roc | R323 / Roc | R320 / libdeflate | R323 / libdeflate | Primary elapsed change |
|---|---:|---:|---:|---:|---:|
| empty | 2.079487× | 2.117661× | 0.016270× | 0.016569× | +0.000% |
| one-stored | 1.981104× | 2.052165× | 0.017424× | 0.018049× | +0.000% |
| seven-stored-prefix | 2.186690× | 2.265426× | 0.018088× | 0.018739× | +0.000% |
| eight-literals | 0.183679× | 0.182636× | 2.324789× | 2.311582× | -0.495% |
| reymont-prefix-32769 | 1.316846× | 1.315170× | 1.196370× | 1.194848× | -0.144% |
| heldout-window-32768 | 1.319629× | 1.316549× | 1.198149× | 1.195354× | +0.136% |
| prefix-natural-32767 | 1.319035× | 1.315911× | 1.198277× | 1.195439× | -0.023% |
| prefix-natural-32768 | 1.315596× | 1.313113× | 1.198870× | 1.196607× | -0.103% |
| prefix-natural-65537 | 1.321800× | 1.322060× | 1.112196× | 1.112414× | +0.041% |
| prefix-natural-131072 | 1.325665× | 1.326177× | 1.091015× | 1.091436× | +0.545% |
| prefix-natural-262144 | 1.335215× | 1.337618× | 1.076893× | 1.078831× | +0.084% |
| prefix-natural-1048576 | 1.350000× | 1.348106× | 1.087417× | 1.085891× | +0.218% |
| prefix-random-16384 | 1.093582× | 1.092230× | 2.940169× | 2.936533× | +0.324% |
| prefix-periodic-1048576 | 0.714811× | 0.718743× | 1.214029× | 1.220708× | +0.591% |
| prefix-repeated-1048576 | 0.711639× | 0.720416× | 1.115889× | 1.129652× | +1.241% |

The comparison with R320 contains 154,656 timed calls. A separate comparison with R322 contains 154,656 timed calls. Both use balanced level-12 runtime input, matching validation and fresh C compressor allocation.

| Input | R322 / Roc | R323 / Roc | R322 / libdeflate | R323 / libdeflate | Elapsed change |
|---|---:|---:|---:|---:|---:|
| empty | 2.128490× | 2.101504× | 0.016735× | 0.016523× | -1.268% |
| one-stored | 2.009541× | 1.968313× | 0.018085× | 0.017714× | -2.052% |
| seven-stored-prefix | 2.308433× | 2.235573× | 0.019091× | 0.018489× | -3.156% |
| eight-literals | 0.182507× | 0.182034× | 2.315681× | 2.309684× | -0.259% |
| reymont-prefix-32769 | 1.319296× | 1.315819× | 1.201026× | 1.197860× | -0.264% |
| heldout-window-32768 | 1.315562× | 1.314622× | 1.193519× | 1.192665× | -0.071% |
| prefix-natural-32767 | 1.314313× | 1.313325× | 1.197910× | 1.197009× | -0.075% |
| prefix-natural-32768 | 1.315003× | 1.311733× | 1.194702× | 1.191731× | -0.249% |
| prefix-natural-65537 | 1.331246× | 1.327175× | 1.118688× | 1.115267× | -0.306% |
| prefix-natural-131072 | 1.334756× | 1.326948× | 1.099272× | 1.092842× | -0.585% |
| prefix-natural-262144 | 1.339104× | 1.338393× | 1.080414× | 1.079840× | -0.053% |
| prefix-natural-1048576 | 1.340782× | 1.345245× | 1.082782× | 1.086386× | +0.333% |
| prefix-random-16384 | 1.086337× | 1.081349× | 2.915336× | 2.901948× | -0.459% |
| prefix-periodic-1048576 | 0.698584× | 0.718962× | 1.186652× | 1.221267× | +2.917% |
| prefix-repeated-1048576 | 0.702272× | 0.721141× | 1.103422× | 1.133068× | +2.687% |

## Independent counters and qualification

On 256 KiB natural the separate CPU cohort reports 759,494,043.0 to 755,467,655.0 retired instructions (-0.530%) and +0.328% cycles. The six payload counters agree across all 15 memory cases.

The candidate profile records 1,922 of 5,344 running samples in the matcher. The separate R320 profile records 1,915 of 5,359. Both runs include validation.

The warnings-as-errors build and 57 CFG tests pass. The 1,200 before/after JIT pairs cover exclusive arms, unchanged live arguments, reached stores, unsigned extremes and alternate entries. The 500 selector calls cover matching and changed operands, signed comparison and a nonzero selection floor. The original image passes 162 linked calls with exact streams, independent inflation, capacities, held aliases and final managed cleanup.

The comparison with R322 records a small improvement on 256 KiB natural and a slowdown on 1 MiB natural. R320 remains retained after the two comparisons with it. The candidate stopped at the 15-case timing screen with original-image qualification. Full timing and decoded-image qualification remain acceptance steps for any adoption.

[Verified patch, linked machine code and pilot observations](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r323-fused-saturation-aaeb45bc963a.tar.gz). SHA-256 aaeb45bc963a1d7c2459627537ce62461187bff132de0602ff11099e85176ae8.
