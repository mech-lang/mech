# R327 — Reuse selected readable bounds

**Retain R327 with the R272 runtime.** Across 81 cases the candidate paired elapsed change is -0.634%. The pooled and block gates pass 81/81 and 81/81. The candidate beats Roc in 46/81 primary cases (56.79%). The matched four-actor run records 3/81 faster than libdeflate and 0/81 faster than both in the candidate.

The 17 priority cases identified before the run change -0.763% by primary paired elapsed and -0.592% in the four-actor comparison. Their primary block-mean ratios change -0.820% and -0.748%. The largest adverse all-case movement is +1.790% on one-stored.

Retention uses the established combined gates and the repeated Reymont and 256 KiB text witnesses. The full priority-group results and individual-cohort misses are reported for review.

Candidate elapsed changes -0.672% and -0.595%. Crossings below parity include none. Crossings above parity include none.

Both individual cohorts pass the block-mean gate on all cases.

Measured parent ace28a0a938bdca0fcb2b91fbd5f8b48f5408034. Published parent 95ecc963343d64b6d57950ecfd3a3d26cd1c2f16. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R325 compiler closure and attached patch. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, and IR/work/ABI/bytecode 215/107/39/14. The pinned source, bytecode, allocator, tables, limits, buffers and full-call timing boundaries are identical.

## Reuse the selected readable offset

A selected unsigned offset < length comparison proves the same current offset plus one fits in U64. The source read then converts that one-based value back to the same zero-based offset. The compiler recognizes that exact modular identity and requires a dominating selected edge for the same current offset and length. It folds the repeated bounds check and uses the existing unit-increment proof for the overflow check. The zero default on the unselected edge stays in place.

The pass runs after selected byte forwarding. Its proof uses the current SSA offset and length, including at joins and after memory effects. The original loads keep their width, order and flags. Disabling this pass reproduces the retained Object and metadata byte for byte.

Four of 347 bodies change. The pass folds 26 comparisons including eight at the four directional reads. From the successful first guard through the byte load, each search read takes one instruction, down from seven. Each skip read takes two. The matcher shrinks from 12,784 to 12,272 bytes with a 624-byte frame. Whole-matcher static stack reads change from 541 to 543 and writes from 114 to 115.

## Matched results

| Input | R325 / Roc | R327 / Roc | R325 / libdeflate | R327 / libdeflate | Primary elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.181435× | 0.182187× | 2.301381× | 2.310929× | +0.263% |
| prefix-natural-4096 | 0.967744× | 0.958459× | 1.628820× | 1.613191× | -1.309% |
| prefix-random-16384 | 1.079242× | 1.060532× | 2.914248× | 2.863726× | -2.863% |
| prefix-natural-32769 | 1.300899× | 1.286170× | 1.181689× | 1.168310× | -1.407% |
| reymont-prefix-32769 | 1.299074× | 1.287031× | 1.184752× | 1.173769× | -1.370% |
| heldout-window-32768 | 1.296866× | 1.287287× | 1.178483× | 1.169778× | -1.097% |
| prefix-natural-32768 | 1.301017× | 1.291341× | 1.179694× | 1.170920× | -1.211% |
| prefix-natural-65537 | 1.306188× | 1.290472× | 1.097961× | 1.084751× | -1.085% |
| prefix-natural-262144 | 1.322366× | 1.308667× | 1.071909× | 1.060805× | -1.050% |
| prefix-natural-1048576 | 1.328210× | 1.314428× | 1.072080× | 1.060956× | -1.315% |
| prefix-periodic-1048576 | 0.707604× | 0.706768× | 1.197878× | 1.196463× | +0.026% |
| prefix-repeated-1048576 | 0.709147× | 0.711352× | 1.113753× | 1.117216× | +0.436% |

The separate four-actor cohort has 245,664 timed calls and exact raw stream agreement across all 81 cases. Each libdeflate call includes a fresh level-12 compressor. The 15-case pilot measured -0.494% by paired block-median ratios.

## Cases where Roc is the faster peer

Roc is faster than libdeflate in 17/81 matched cases. The nearest substantial inputs are natural text around 32 and 64 KiB. The frontier uses the same balanced blocks for all three ratios.

| Input | Mech / Roc | Mech / libdeflate | Elapsed cut to Roc |
|---|---:|---:|---:|
| prefix-natural-32769 | 1.286170× | 1.168310× | 22.25% |
| reymont-prefix-32769 | 1.287031× | 1.173769× | 22.30% |
| heldout-window-32768 | 1.287287× | 1.169778× | 22.32% |
| prefix-natural-33027 | 1.287883× | 1.172744× | 22.35% |
| heldout-window-32767 | 1.288712× | 1.170002× | 22.40% |
| prefix-natural-65537 | 1.290472× | 1.084751× | 22.51% |
| prefix-natural-32768 | 1.291341× | 1.170920× | 22.56% |
| prefix-natural-32767 | 1.302555× | 1.181348× | 23.23% |

## Independent counters and profiles

The six Mech payload and heap families agree across all 81 cases. CPU counter measurements and profile samples were collected in separate runs. The profile includes validation.

| Input | R325 instructions | R327 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 715,956.5 | 715,754.5 | -0.028% | -2.374% |
| prefix-natural-4096 | 9,213,934.5 | 9,121,345.0 | -1.005% | -0.560% |
| prefix-random-16384 | 10,447,578.5 | 10,199,169.0 | -2.378% | -0.750% |
| prefix-natural-32769 | 75,620,863.0 | 74,571,108.5 | -1.388% | -0.772% |
| reymont-prefix-32769 | 75,621,534.5 | 74,571,267.5 | -1.389% | -0.905% |
| heldout-window-32768 | 75,373,811.5 | 74,323,105.0 | -1.394% | -0.891% |
| prefix-natural-32768 | 75,372,922.5 | 74,322,179.0 | -1.394% | -1.001% |
| prefix-natural-65537 | 180,340,402.5 | 177,962,763.0 | -1.318% | -0.886% |
| prefix-natural-262144 | 755,092,199.5 | 744,867,527.0 | -1.354% | -0.876% |
| prefix-natural-1048576 | 2,699,594,514.5 | 2,658,021,583.5 | -1.540% | -0.847% |
| prefix-periodic-1048576 | 489,244,742.0 | 489,151,564.5 | -0.019% | +1.919% |
| prefix-repeated-1048576 | 497,077,606.5 | 496,981,224.0 | -0.019% | +0.625% |

| Profile | Running samples | Leading linked leaf | Samples |
|---|---:|---|---:|
| candidate-profiles/eight-literals | 5240 | mech_compact_closed_v2 | 1538 |
| candidate-profiles/prefix-natural-262144 | 5349 | mech_compact_numeric_15296_no_growth | 3103 |
| candidate-profiles/reymont-prefix-32769 | 4320 | mech_compact_numeric_15296_no_growth | 2147 |

On 256 KiB natural, the current profile assigns 1,908 of 5,349 running samples to the matcher and 3,103 to the parser. These are sampled CPU-time shares from the separate diagnostic cohort.

Both hot matcher directions still double the pending logical slot before a halfword store. The child lookup already scales its next slot to bytes, then the latch carries the logical slot. The pinned C worker computes the child address once and carries it forward. The next pilot will test carrying the scaled displacement with the current base held separately. The exact CLIF and linked excerpts accompany the profile.

## Qualification

The warnings-as-errors build and 59 CFG tests pass. The focused witness checks 96 before/after JIT pairs against an independent oracle. It covers successful and default paths, retained stores, alternate entries, a different length, a fresh load, signed and inclusive conditions, and a nonunit increment. Two earlier links failed with errno 28. Reclaiming verified build products allowed the same build to pass.

All 324 linked original and decoded calls agree on exact streams, independent inflate, held aliases, capacities and final cleanup. The Object, metadata, matcher CLIF and VCode hashes match between the two forms.

The replay checks all four balanced cohorts and the 81 matched cases from raw per-call samples. It regenerates the timing gates, hardware counters, matched peer results and this report.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r327-selected-readable-bound-55f8b7038a84.tar.gz). SHA-256 55f8b7038a845d9ce07df8407d1eccbf01371b0c3c1eb7af4b59ac990c56ea5d.
