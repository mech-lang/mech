# R314 — Prove scalar reads within checked windows

**Retain R314 with the R272 runtime.** Across 81 cases the primary paired elapsed change is -0.880%. The pooled and block gates pass 81/81 and 81/81. Mech beats Roc in 46/81 primary cases (56.79%). The matched four-actor run records 3/81 faster than libdeflate and 0/81 faster than both.

The two primary cohorts change -0.861% and -0.900%. The greatest adverse paired movement is +2.897% on empty. Crossings below parity include none. Crossings above parity include none.

Measured parent 45dcf7459c5123092ef679061ea8fdf6d4fca3a4. Published parent 45dcf7459c5123092ef679061ea8fdf6d4fca3a4. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R313 closure and attached patch. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, and IR/work/ABI/bytecode 215/107/39/14. The pinned source, bytecode, allocator, tables, limits, buffers and full-call timing boundaries are identical.

## Prove the two initial scalar reads

The selected original checks prove that a full extent fits in the current input with a nonwrapping endpoint. The per-visit offset is strictly shorter than that extent. The earlier candidate base is either zero or a nonwrapping subtraction from the current position on every incoming route. The compiler follows identity arguments through the joins, then proves that both scalar byte offsets are below the same captured length. Their one-based increment is consequently nonwrapping. The selected memory reads keep their order, addresses and types.

A route lacking the dominating check, nonwrapping endpoint, strict bound, or matching length retains the original zero default and failure behavior. The later directional reads keep their existing checks because the final match length can come from a different route.

The compiler replaces 8 comparisons in 1 routine. 1 of 347 bodies change. The vector matcher body changes from 12,952 to 12,924 bytes and its stack frame from 640 to 624 bytes. The successful initial byte pair takes eight linked ARM64 instructions from the first address calculation through the second zero extension. The same span takes nineteen in R313.

The sampled stack-load share inside the matcher changes from 28.88% to 30.26%. These are running-sample shares from the separate profiling runs.

## Matched results

| Input | R313 / Roc | R314 / Roc | R313 / libdeflate | R314 / libdeflate | Primary elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.194407× | 0.189065× | 2.698778× | 2.624625× | +0.467% |
| prefix-natural-4096 | 0.997631× | 0.996125× | 1.702697× | 1.700125× | +0.222% |
| prefix-random-16384 | 1.095081× | 1.088869× | 2.943468× | 2.926768× | -0.136% |
| prefix-natural-65537 | 1.353817× | 1.346338× | 1.135731× | 1.129457× | -0.218% |
| reymont-prefix-32769 | 1.346953× | 1.346506× | 1.228727× | 1.228318× | +0.020% |
| prefix-natural-33027 | 1.350092× | 1.349437× | 1.231186× | 1.230589× | -0.020% |
| prefix-natural-32768 | 1.349327× | 1.352768× | 1.227778× | 1.230909× | +0.044% |
| prefix-natural-262144 | 1.380414× | 1.374114× | 1.121081× | 1.115964× | -0.261% |
| prefix-natural-1048576 | 1.385693× | 1.389411× | 1.118462× | 1.121463× | -0.089% |
| prefix-periodic-1048576 | 0.729676× | 0.708285× | 1.235195× | 1.198983× | -3.068% |
| prefix-repeated-1048576 | 0.729166× | 0.710335× | 1.151823× | 1.122077× | -2.566% |

The separate four-actor cohort has 245,664 timed calls and exact raw stream agreement across all 81 cases. Each libdeflate call includes a fresh level-12 compressor. The 15-case pilot measured -0.357% by paired block-median ratios.

## Cases where Roc is the faster peer

Roc is faster than libdeflate in 17/81 matched cases. The nearest substantial inputs are natural text around 32 and 64 KiB. The frontier uses the same balanced blocks for all three ratios.

| Input | Mech / Roc | Mech / libdeflate | Elapsed cut to Roc |
|---|---:|---:|---:|
| prefix-natural-65537 | 1.346338× | 1.129457× | 25.72% |
| reymont-prefix-32769 | 1.346506× | 1.228318× | 25.73% |
| prefix-natural-33027 | 1.349437× | 1.230589× | 25.90% |
| prefix-natural-32769 | 1.349628× | 1.230202× | 25.91% |
| prefix-natural-32767 | 1.350500× | 1.227539× | 25.95% |
| heldout-window-32767 | 1.352323× | 1.231723× | 26.05% |
| prefix-natural-32768 | 1.352768× | 1.230909× | 26.08% |
| heldout-window-32768 | 1.353873× | 1.230625× | 26.14% |

## Independent counters and profiles

The six Mech payload and heap families agree across all 81 cases. CPU counter measurements and profile samples were collected in separate runs. The profile includes validation.

| Input | R313 instructions | R314 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 716,044.5 | 716,209.0 | +0.023% | -2.349% |
| prefix-natural-4096 | 9,420,950.5 | 9,384,251.5 | -0.390% | -1.155% |
| prefix-random-16384 | 10,570,016.5 | 10,657,243.5 | +0.825% | -0.358% |
| prefix-natural-65537 | 188,866,279.5 | 187,040,343.0 | -0.967% | -0.666% |
| reymont-prefix-32769 | 78,836,327.0 | 78,148,160.0 | -0.873% | +0.087% |
| prefix-natural-33027 | 79,524,449.0 | 78,824,501.5 | -0.880% | -0.375% |
| prefix-natural-32768 | 78,587,824.0 | 77,898,330.5 | -0.877% | -0.013% |
| prefix-natural-262144 | 794,316,743.0 | 785,867,355.0 | -1.064% | -0.404% |
| prefix-natural-1048576 | 2,862,405,597.5 | 2,827,465,153.5 | -1.221% | -0.229% |
| prefix-periodic-1048576 | 503,846,522.5 | 490,274,010.5 | -2.694% | -1.530% |
| prefix-repeated-1048576 | 511,617,418.0 | 498,100,675.5 | -2.642% | -1.357% |

| Profile | Running samples | Leading linked leaf | Samples |
|---|---:|---|---:|
| candidate-profiles-screen/eight-literals | 5249 | mech_compact_closed_v2 | 1584 |
| candidate-profiles-screen/prefix-natural-262144 | 5355 | mech_compact_numeric_15296_no_growth | 2955 |
| candidate-profiles-screen/reymont-prefix-32769 | 4560 | mech_compact_numeric_15296_no_growth | 2261 |

## Qualification

The compiler and focused Rust tests use warnings as errors. Fifty-three CFG tests pass. The new control executes 80 before/after JIT pairs across the proved saturated candidate, alternate entry, wrapping full endpoint, inclusive offset, and different-length cases. The 324 linked original and decoded calls agree on exact bytes, independent inflate, held aliases, capacities, and final managed cleanup. Both linked forms have identical Object and metadata hashes.

The next selected-mismatch pilot has a static witness through four pure record and stop decisions. Its continuation needs ten current words already present on the incoming edge. Both directional reads remain outside that proof. Separate assembly and elapsed evidence are required for the new edge.

The replay checks all four balanced cohorts and the 81 matched cases from raw per-call samples. It regenerates the timing gates, hardware counters, dual-peer frontier and this report.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r314-scalar-byte-spans-4249086829d1.tar.gz). SHA-256 4249086829d1fb4f59ca1d3a1d6016055fd4cc4c3950054867a5562b9a3eebd9.
