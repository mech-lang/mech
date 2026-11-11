# R312 — Defer exit-only phase packet loads

**Retain R312 with the R272 runtime.** Across 81 cases the primary paired elapsed change is -0.330%. The pooled and block gates pass 81/81 and 81/81. Mech beats Roc in 46/81 primary cases (56.79%). The matched four-actor run records 3/81 faster than libdeflate and 0/81 faster than both.

The two primary cohorts change -0.339% and -0.321%. The greatest adverse paired movement is +0.994% on random-64. Crossings below parity include none. Crossings above parity include none.

Measured parent 6120aba1263cab886cd1d66c3c3c8d610c590172. Published parent cac3b50fff3fdeb5cbe30b44d970341945e5793b. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R309 closure and attached patch. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, and IR/work/ABI/bytecode 215/107/39/14. The pinned source, bytecode, allocator, tables, limits, buffers and full-call timing boundaries are identical.

## Shorter lifetimes for phase output words

The private numeric call receives disjoint invocation-local input and output stack slots. An aligned I64 input word with a single direct copy to an in-range output word can be loaded immediately before that store on a return-only unconditional route. The input packet is immutable throughout the synchronous phase. The existing output store, status and failure origins retain their order. A dynamic address, escaping input pointer, multiple uses, wrong width or extent, or cyclic or conditional continuation leaves the original load in place.

The compiler moves 102 static loads in 43 routines. 43 of 347 existing bodies change. The vector matcher body goes from 13,840 to 13,112 bytes and its stack frame goes from 752 to 656 bytes. The linked assembly and original and decoded CLIF and VCode are included.

## Matched results

| Input | R309 / Roc | R312 / Roc | R309 / libdeflate | R312 / libdeflate | Primary elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.182481× | 0.181236× | 2.321540× | 2.305701× | -0.111% |
| prefix-natural-4096 | 0.985796× | 0.983635× | 1.683414× | 1.679723× | +0.117% |
| prefix-random-16384 | 1.090115× | 1.081763× | 2.965464× | 2.942743× | -0.489% |
| prefix-natural-32767 | 1.358774× | 1.351416× | 1.239151× | 1.232441× | -0.526% |
| reymont-prefix-32769 | 1.359593× | 1.351458× | 1.243343× | 1.235904× | -0.312% |
| heldout-window-32768 | 1.363360× | 1.355723× | 1.242120× | 1.235162× | -0.400% |
| prefix-natural-32768 | 1.364082× | 1.355973× | 1.241793× | 1.234411× | -0.525% |
| prefix-natural-262144 | 1.390089× | 1.385044× | 1.125828× | 1.121742× | -0.042% |
| prefix-natural-1048576 | 1.403100× | 1.400530× | 1.131102× | 1.129031× | -0.307% |
| prefix-periodic-1048576 | 0.739356× | 0.738884× | 1.254795× | 1.253994× | -0.056% |
| prefix-repeated-1048576 | 0.739931× | 0.739725× | 1.163443× | 1.163118× | +0.081% |

The separate four-actor cohort has 245,664 timed calls and exact raw stream agreement across all 81 cases. Each libdeflate call includes a fresh level-12 compressor. The 10-case pilot measured -0.286% by paired block-median ratios.

## Cases where Roc is the faster peer

Roc is faster than libdeflate in 17/81 matched cases. The nearest substantial inputs are natural text around 32 KiB. The frontier uses the same balanced blocks for all three ratios.

| Input | Mech / Roc | Mech / libdeflate | Elapsed cut to Roc |
|---|---:|---:|---:|
| prefix-natural-32767 | 1.351416× | 1.232441× | 26.00% |
| reymont-prefix-32769 | 1.351458× | 1.235904× | 26.01% |
| heldout-window-32768 | 1.355723× | 1.235162× | 26.24% |
| prefix-natural-32768 | 1.355973× | 1.234411× | 26.25% |
| prefix-natural-32769 | 1.356143× | 1.234673× | 26.26% |
| heldout-window-32767 | 1.359242× | 1.235675× | 26.43% |
| prefix-natural-33027 | 1.360973× | 1.238387× | 26.52% |
| prefix-natural-65537 | 1.366528× | 1.148920× | 26.82% |

## Independent counters and profiles

The six Mech payload and heap families agree across all 81 cases. CPU counter measurements and profile samples were collected in separate runs. The profile includes validation.

| Input | R309 instructions | R312 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 717,374.5 | 716,100.0 | -0.178% | +1.895% |
| prefix-natural-4096 | 9,513,366.0 | 9,489,189.5 | -0.254% | +0.036% |
| prefix-random-16384 | 10,731,521.0 | 10,670,672.5 | -0.567% | -0.538% |
| prefix-natural-32767 | 79,719,956.0 | 79,474,327.5 | -0.308% | -0.735% |
| reymont-prefix-32769 | 79,968,611.0 | 79,722,968.0 | -0.307% | -0.717% |
| heldout-window-32768 | 79,720,155.5 | 79,474,575.0 | -0.308% | -0.661% |
| prefix-natural-32768 | 79,724,328.5 | 79,474,927.0 | -0.313% | -0.706% |
| prefix-natural-262144 | 805,555,675.0 | 803,391,569.5 | -0.269% | -0.246% |
| prefix-natural-1048576 | 2,908,221,519.0 | 2,899,485,197.0 | -0.300% | -0.407% |
| prefix-periodic-1048576 | 517,448,409.0 | 515,350,251.0 | -0.405% | -0.069% |
| prefix-repeated-1048576 | 525,165,983.5 | 523,077,482.0 | -0.398% | -0.079% |

| Profile | Running samples | Leading linked leaf | Samples |
|---|---:|---|---:|
| priority-profiles/libdeflate | 5341 | deflate_find_min_cost_path | 3676 |
| priority-profiles/mech-candidate | 5363 | mech_compact_numeric_15296_no_growth | 2652 |
| priority-profiles/mech-r309 | 4953 | mech_compact_numeric_15296_no_growth | 2448 |
| priority-profiles/roc | 5048 | 0x104624f8d | 702 |
| candidate-profiles/heldout-window-32768 | 5345 | mech_compact_numeric_15296_no_growth | 2636 |
| candidate-profiles/prefix-natural-1048576 | 5307 | mech_compact_numeric_15296_no_growth | 2635 |
| candidate-profiles/prefix-natural-32768 | 5316 | mech_compact_numeric_15296_no_growth | 2647 |
| candidate-profiles/prefix-natural-4096 | 5300 | mech_compact_numeric_837_vector | 1583 |
| candidate-profiles/prefix-periodic-1048576 | 5350 | mech_compact_numeric_837_vector | 3523 |
| candidate-profiles/prefix-random-16384 | 5262 | mech_compact_numeric_837_vector | 2400 |
| candidate-profiles/prefix-repeated-1048576 | 5347 | mech_compact_numeric_837_vector | 3225 |
| candidate-profiles-screen/eight-literals | 5237 | mech_compact_closed_v2 | 1641 |
| candidate-profiles-screen/prefix-natural-262144 | 5571 | mech_compact_numeric_15296_no_growth | 3119 |

## Qualification

Both production builds and the focused Rust tests use warnings as errors. Two packet tests and 51 existing CFG tests pass. The packet controls exercise a selected successful copy, rejection order, two uses, escape, invalid width or extent, and nonterminal routes. The 324 linked original and decoded calls agree on exact output, independent inflate, aliases, capacities and zero final private payload and owner records. Both image forms have identical Object and metadata hashes.

The replay checks all four balanced cohorts and the 81 matched cases from raw per-call samples. It regenerates the timing gates, hardware counters, dual-peer frontier and this report.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r312-exit-packet-loads-d154750225b4.tar.gz). SHA-256 d154750225b40930f2e3941204e143234dbdd7cbedd75a11321a784ac5bfcbde.
