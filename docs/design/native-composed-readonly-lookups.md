# R333 — Compose read-only cost lookups

**Retain R333 with the R272 runtime.** R333 changes paired elapsed -0.551% across 81 cases. The pooled and block timing gates pass 81/81 and 81/81. The candidate wins 46/81 primary cases against Roc (56.79%), 3/81 against libdeflate and 0/81 against both in the four-actor run.

The 17 established priority cases change -1.413% in the primary paired result and -1.915% in the matched comparison. Their primary block-mean ratios change -2.067% and -1.529%. The largest paired adverse movement is +5.668% on one-stored. Its pooled medians are 83 and 83 ns. Both established gates pass.

Reymont's block means change -4.706% and -2.155%, with -1.873% in the matched comparison. Natural 256 KiB changes -2.453% and -2.590%, with -1.274% matched.

Crossings below Roc parity include none. Crossings above include none.
Crossings below libdeflate parity include none. Crossings above include none.

Both individual cohorts pass every block-mean gate.

Measured compiler ba8f02f61b3735a2160cc6177e76c6f7cc59cf7b. Published parent 67d7562bc4efda7b3ec8cacb0c346c4e15f4e2e5. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R327 closure and the archived nine-file compiler delta. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, and IR/work/ABI/bytecode 215/107/39/14. The benchmark source, bytecode, allocator, search limits, input and output capacities, original tables and complete-call timing boundary are pinned.

## Compose the current captures

The compiler identifies two defaulting reads in an effect-free CFG region. The first is a flat immutable U8 view with exact length and payload bounds derived from full typed descriptors. The second reads a separate flat U32 view. The wrapped selector has two exact results over the known packed input range. Here those values are 0 and 7. The slot payload maximum of 15 and bank displacement 14 imply a 30-entry floor for the current cost view.

A fixed 2 KiB stack table is filled from both actual captures at each selected phase call. The successful read uses the bounded index and one bank bit. The original zero default of the first read uses the current cost at the bank displacement. The caller checks the current cost length, an estimate of available match records from the current cache cursor and countdown, and the existing private no-growth contract. Sparse calls and short views take the existing phase. The estimate affects routing only. New cost contents at the same address rebuild the table. The table belongs to the phase stack. All four linked call sites contain the current cost floor and record threshold.

In the linked ARM64 binary, the successful lookup from the first range comparison to the first use of the resulting cost falls from 12 instructions and three stack reloads to six instructions and one stack reload. The scalar table-fill loop executes 13 instructions per cell for 256 cells and writes two U32 banks. The retained peer excerpts show a compact scalar reduction in Roc and a dense offset-class lookup followed by a cost read in libdeflate.

Four local variants add 6,080 code bytes. The inspected phase is 1,520 bytes with a 2,272-byte frame, compared with 1,552 and 256 bytes for the retained phase. The root moves from 979,776 to 1,000,052 code bytes and from 57,440 to 57,472 stack bytes. All 346 other existing bodies are byte-identical. Disabling the compiler flag restores the R327 Object and metadata byte for byte.

## Matched results

| Input | R327 / Roc | R333 / Roc | R327 / libdeflate | R333 / libdeflate | Primary elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.184753× | 0.185797× | 2.352217× | 2.365505× | +0.089% |
| prefix-natural-4096 | 0.989985× | 0.974822× | 1.656656× | 1.631282× | -2.282% |
| prefix-random-16384 | 1.065231× | 1.061372× | 2.872542× | 2.862136× | -0.464% |
| prefix-natural-131072 | 1.272104× | 1.243290× | 1.070558× | 1.046309× | -2.625% |
| prefix-natural-32769 | 1.284679× | 1.257145× | 1.170855× | 1.145761× | -2.166% |
| heldout-window-32768 | 1.288238× | 1.259260× | 1.173586× | 1.147187× | -1.985% |
| reymont-prefix-32769 | 1.283504× | 1.259470× | 1.170657× | 1.148736× | -2.873% |
| prefix-natural-65537 | 1.288771× | 1.261873× | 1.087115× | 1.064426× | -2.164% |
| prefix-natural-262144 | 1.319093× | 1.302291× | 1.069925× | 1.056297× | -2.425% |
| prefix-natural-1048576 | 1.308070× | 1.280082× | 1.058228× | 1.035586× | -2.150% |
| prefix-random-1048576 | 1.588463× | 1.597372× | 2.239233× | 2.251792× | -0.065% |
| prefix-periodic-1048576 | 0.710743× | 0.715957× | 1.199195× | 1.207993× | +0.105% |
| prefix-repeated-1048576 | 0.717065× | 0.714685× | 1.133171× | 1.129410× | -0.112% |

The two primary cohorts contain 368,496 timed calls. The four-actor run contains 245,664 timed calls at fixed level 12 with runtime input, matched warmup and validation placement, and a fresh libdeflate compressor in every complete call. The independent inflation and the expected stream bytes agree in all 81 cases.

The 16-case screen changes -0.960% by paired elapsed. All 16 passed both screen timing gates. Current matched frontier figures are prefix-natural-1048576 at 1.035586× libdeflate and prefix-natural-131072 at 1.243290× Roc within the faster-Roc group.

## Separate counter and profile cohorts

Natural 256 KiB retires 744,941,068.0 to 693,513,477.5 instructions (-6.904%). Cycles change -2.469%. The six heap families have identical per-case distributions on 81/81 cases. The additional table is stack storage.

| Input | R327 instructions | R333 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 717,122.5 | 717,539.0 | +0.058% | +4.544% |
| prefix-natural-4096 | 9,154,272.5 | 8,737,319.0 | -4.555% | -1.705% |
| prefix-random-16384 | 10,197,412.5 | 10,198,347.5 | +0.009% | -0.717% |
| prefix-natural-131072 | 366,568,266.5 | 341,460,868.5 | -6.849% | -1.849% |
| prefix-natural-32769 | 74,601,473.0 | 70,101,284.0 | -6.032% | -1.439% |
| heldout-window-32768 | 74,356,752.5 | 69,823,320.0 | -6.097% | -1.620% |
| reymont-prefix-32769 | 74,597,120.5 | 70,070,577.5 | -6.068% | -1.612% |
| prefix-natural-65537 | 177,970,982.0 | 165,974,310.5 | -6.741% | -1.769% |
| prefix-natural-262144 | 744,941,068.0 | 693,513,477.5 | -6.904% | -2.469% |
| prefix-natural-1048576 | 2,658,217,311.5 | 2,488,956,126.0 | -6.367% | -1.949% |
| prefix-random-1048576 | 576,171,934.0 | 576,167,855.5 | -0.001% | -0.460% |
| prefix-periodic-1048576 | 489,163,519.0 | 488,914,372.0 | -0.051% | -1.885% |
| prefix-repeated-1048576 | 496,984,230.0 | 496,679,069.0 | -0.061% | -0.757% |

| Profile input | Running samples | Composed phase linked PCs |
|---|---:|---:|
| eight-literals | 5,255 | 0 |
| prefix-natural-262144 | 5,359 | 3,056 |
| reymont-prefix-32769 | 4,538 | 2,326 |

The profiles include validation and were collected separately from elapsed and hardware counters. The leading sampled address on both natural inputs is in the continuing reduction completion, after the new lookup path.

## Qualification

The GPU compiler and producer build with warnings as errors. Original and decoded Objects, metadata, and inspected parser CLIF and VCode agree byte for byte. The 324 full linked checks cover the exact stream, independent inflate, original capacity, held input and output aliases and cleanup on both forms.

The focused linked native witness checks all 65,536 packed offsets for two current cost snapshots on both artifact forms. Each snapshot uses the same cost address with changed contents, including wrapping cost arithmetic. Both native paths agree with an independent parser oracle and return the same source failure and origin on a rejected match length, followed by a successful call. It enters the phase directly using a valid packet. The linked root-call excerpts separately verify selection.

A separate 240-call linked suite covers short cost views, changed literal contents, rebound views, held aliases, failure and retry. Those smaller fixtures decline composition and exercise the existing route. Whole-call ownership and cleanup controls run through the compact diagnostic entry. Ordinary managed-entry qualification remains separate.

R328 measured the scaled-displacement matcher pilot, R329 measured the immutable-payload proof alone, and R332 measured classification at production with native parser admission restored. R333 combines the two dependent reads at the consumer and leaves the match-record representation unchanged.

The next bounded profile target is reduction completion after the cost lookup. At the leading linked PC, the phase moves the selected U32 cost and the exhausted child cursor to the outer loop. The reference loop retains an advancing index and decrements a remaining count. Compare the whole transition and the loop-carried state before changing either instruction sequence.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r333-native-composed-cost-c9a1a0928434.tar.gz). SHA-256 c9a1a0928434cc0c2030bffdf3f22e26d426bbb544e3c7695827fd67a9f045bf.
