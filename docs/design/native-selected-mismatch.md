# R315 — Forward selected mismatch paths through pure decisions

**Retain R315 with the R272 runtime.** Across 81 cases the primary paired elapsed change is -0.568%. The pooled and block gates pass 81/81 and 81/81. Mech beats Roc in 45/81 primary cases (55.56%). The matched four-actor run records 3/81 faster than libdeflate and 0/81 faster than both.

The two primary cohorts change -0.581% and -0.556%. The greatest adverse paired movement is +5.157% on seven-stored-prefix. Crossings below parity include none. Crossings above parity include none.

The individual block-mean gates missed on one-stored in primary-a (+8.482%), seven-stored-prefix in primary-a (+10.807%). Both pass the combined block-mean gate.

Measured parent fd47444b440f6422cdefe957f2b2cd6a251ab44c. Published parent fd47444b440f6422cdefe957f2b2cd6a251ab44c. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R314 closure and attached patch. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, and IR/work/ABI/bytecode 215/107/39/14. The pinned source, bytecode, allocator, tables, limits, buffers and full-call timing boundaries are identical.

## Carry a selected mismatch into the direction checks

The selected false edge already establishes that the compared bytes differ. Its chain of ordinary single-predecessor jumps carries that fact into the current visit. The compiler can bypass scalar dispatch whose branch outcomes are fixed by that fact. It repairs the destination parameters with the current incoming values. A route with an intervening effect, an alternate entry to the source chain, a shared continuation that cannot be repaired, or a live value without a valid incoming definition retains its original path.

The matcher witness resolves four record and stop decisions on the first mismatch. The direct continuation carries ten current words to the two original directional reads. The alternate matching path and the original read, write, and failure order are preserved.

The compiler forwards 3 edges in 1 routine. 1 of 347 bodies change. The vector matcher body changes from 12,924 to 12,776 bytes and its stack frame from 624 to 640 bytes.

The sampled stack-load share inside the matcher changes from 30.26% to 24.64%. These are running-sample shares from separate profiling runs.

## Matched results

| Input | R314 / Roc | R315 / Roc | R314 / libdeflate | R315 / libdeflate | Primary elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.181540× | 0.180131× | 2.323341× | 2.305305× | -0.489% |
| prefix-natural-4096 | 0.981286× | 0.974793× | 1.678325× | 1.667220× | -1.774% |
| prefix-random-16384 | 1.094439× | 1.074619× | 2.969679× | 2.915899× | -0.932% |
| heldout-window-32767 | 1.350152× | 1.333156× | 1.227907× | 1.212450× | -1.133% |
| prefix-natural-33027 | 1.350752× | 1.333240× | 1.233692× | 1.217698× | -0.801% |
| prefix-natural-32768 | 1.349818× | 1.333927× | 1.230225× | 1.215742× | -1.187% |
| prefix-natural-65537 | 1.353221× | 1.342407× | 1.139669× | 1.130562× | -1.120% |
| prefix-natural-262144 | 1.372258× | 1.354530× | 1.113989× | 1.099598× | -1.332% |
| prefix-natural-1048576 | 1.386745× | 1.368633× | 1.120419× | 1.105786× | -0.695% |
| prefix-periodic-1048576 | 0.708865× | 0.708797× | 1.202042× | 1.201926× | +0.027% |
| prefix-repeated-1048576 | 0.710593× | 0.711106× | 1.117946× | 1.118753× | +0.064% |

The separate four-actor cohort has 245,664 timed calls and exact raw stream agreement across all 81 cases. Each libdeflate call includes a fresh level-12 compressor. The 15-case pilot measured -0.694% by paired block-median ratios.

## Cases where Roc is the faster peer

Roc is faster than libdeflate in 17/81 matched cases. The nearest substantial inputs are natural text around 32 and 64 KiB. The frontier uses the same balanced blocks for all three ratios.

| Input | Mech / Roc | Mech / libdeflate | Elapsed cut to Roc |
|---|---:|---:|---:|
| heldout-window-32767 | 1.333156× | 1.212450× | 24.99% |
| prefix-natural-33027 | 1.333240× | 1.217698× | 24.99% |
| prefix-natural-32768 | 1.333927× | 1.215742× | 25.03% |
| prefix-natural-32767 | 1.334376× | 1.212894× | 25.06% |
| reymont-prefix-32769 | 1.334761× | 1.214831× | 25.08% |
| heldout-window-32768 | 1.334876× | 1.214766× | 25.09% |
| prefix-natural-32769 | 1.337416× | 1.216909× | 25.23% |
| prefix-natural-65537 | 1.342407× | 1.130562× | 25.51% |

## Independent counters and profiles

The six Mech payload and heap families agree across all 81 cases. CPU counter measurements and profile samples were collected in separate runs. The profile includes validation.

| Input | R314 instructions | R315 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 716,048.0 | 716,114.0 | +0.009% | +4.578% |
| prefix-natural-4096 | 9,384,425.0 | 9,272,561.0 | -1.192% | -0.941% |
| prefix-random-16384 | 10,654,917.5 | 10,496,502.5 | -1.487% | -0.408% |
| heldout-window-32767 | 77,903,592.5 | 76,086,154.0 | -2.333% | -1.288% |
| prefix-natural-33027 | 78,825,091.0 | 76,982,511.0 | -2.338% | -1.166% |
| prefix-natural-32768 | 77,906,691.0 | 76,087,961.0 | -2.334% | -0.806% |
| prefix-natural-65537 | 187,043,446.5 | 182,368,062.0 | -2.500% | -1.369% |
| prefix-natural-262144 | 785,912,839.0 | 764,532,545.0 | -2.720% | -1.238% |
| prefix-natural-1048576 | 2,827,510,355.0 | 2,739,427,301.5 | -3.115% | -1.572% |
| prefix-periodic-1048576 | 490,277,771.0 | 488,178,022.0 | -0.428% | -0.003% |
| prefix-repeated-1048576 | 498,099,972.5 | 496,002,126.0 | -0.421% | +0.037% |

| Profile | Running samples | Leading linked leaf | Samples |
|---|---:|---|---:|
| candidate-profiles-screen/eight-literals | 5278 | mech_compact_closed_v2 | 1614 |
| candidate-profiles-screen/prefix-natural-262144 | 5372 | mech_compact_numeric_15296_no_growth | 3053 |
| candidate-profiles-screen/reymont-prefix-32769 | 4448 | mech_compact_numeric_15296_no_growth | 2220 |

## Qualification

The compiler and focused Rust tests use warnings as errors. Fifty-four CFG tests pass. The new control executes 96 before/after JIT pairs across ordinary mismatch and equality, a load that must be kept, and an alternate entry. The 324 linked original and decoded calls agree on exact bytes, independent inflate, held aliases, capacities, and final managed cleanup. Both linked forms have identical Object and metadata hashes.

The next pilot carries a typed child address in the same slot as the original child index. An invariant table base, identical wrapping scale and complete use proof are required. The separate pilot will measure the resulting address calculations and reloads.

The replay checks all four balanced cohorts and the 81 matched cases from raw per-call samples. It regenerates the timing gates, hardware counters, dual-peer frontier and this report.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r315-selected-mismatch-7b46d63fffeb.tar.gz). SHA-256 7b46d63fffebcff62e0c84b97d11da07f002c1a6fb6569553f260f89f1d41a68.
