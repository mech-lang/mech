# Fixed 32-byte wide comparison pilot

The candidate replaces the generic wide-loop dispatch at two certified 32-byte comparisons with two 16-byte comparisons and an unchanged scalar mismatch path. The source and fixture inventory are fixed. Timed workers use the compact diagnostic entry.

Both natural priority cases improved in the early cohort.

## Five-case matched cohort

The four actors retained 6,528 calls across 24 rotated orders per case.

| Input | Wide-mask/Roc | Fixed-32/Roc | Fixed-32/libdeflate | Fixed-32/wide-mask |
| --- | ---: | ---: | ---: | ---: |
| eight-literals | 0.179703× | 0.179249× | 2.300246× | 0.997478× |
| reymont-prefix-32769 | 1.266465× | 1.263044× | 1.147055× | 0.997299× |
| prefix-natural-262144 | 1.303781× | 1.296645× | 1.046677× | 0.994527× |
| prefix-periodic-1048576 | 0.786700× | 0.779592× | 1.325499× | 0.990964× |
| prefix-repeated-1048576 | 0.774627× | 0.766858× | 1.207029× | 0.989970× |

## Exact linked matcher

| Actor | Code bytes | Stack bytes | Vector loads | Fixed-32 headers |
| --- | ---: | ---: | ---: | ---: |
| mech-wide-mask | 11340 | 624 | 60 | None |
| mech-fixed-extent-32 | 11100 | 624 | 48 | 1372, 2713 |

## Independent hardware counters

The two workers use the same probe around complete compression, including heap construction, transfer and cleanup. The ratios are paired medians from 24 blocks.

| Input | Wide-mask instructions | Fixed-32 instructions | Instructions ratio | Cycles ratio |
| --- | ---: | ---: | ---: | ---: |
| eight-literals | 640,390.0 | 641,437.0 | 1.001644× | 1.018689× |
| reymont-prefix-32769 | 70,116,292.0 | 69,803,332.0 | 0.995524× | 1.000454× |
| prefix-natural-262144 | 707,085,412.5 | 703,450,786.0 | 0.994872× | 0.998506× |
| prefix-periodic-1048576 | 460,550,993.5 | 457,350,986.0 | 0.993055× | 0.972978× |
| prefix-repeated-1048576 | 468,782,183.5 | 465,543,990.5 | 0.992948× | 0.976219× |

## Exact executable profiles

These are running timer PCs from separate captures, including validation. Baseline and candidate spans cover different instruction ranges.
The candidate route mapping remains ambiguous. Exact matcher PC counts are preserved.

| Input | Actor | Running timer PCs | Matcher PCs | Audited fixed-span PCs |
| --- | --- | ---: | ---: | ---: |
| reymont-prefix-32769 | mech-wide-mask | 4578 | 1743 | 79 |
| reymont-prefix-32769 | mech-fixed-extent-32 | 5312 | 2016 | Unmapped |
| prefix-natural-262144 | mech-wide-mask | 5636 | 2109 | 82 |
| prefix-natural-262144 | mech-fixed-extent-32 | 5249 | 1963 | Unmapped |
| prefix-periodic-1048576 | mech-wide-mask | 5478 | 3040 | 246 |
| prefix-periodic-1048576 | mech-fixed-extent-32 | 5302 | 2853 | Unmapped |
| prefix-repeated-1048576 | mech-wide-mask | 5365 | 2790 | 233 |
| prefix-repeated-1048576 | mech-fixed-extent-32 | 5343 | 2734 | Unmapped |

## Qualification

Both original workers passed two complete checks on each of the 81 fixtures. The checks cover exact output, independent inflation, capacity, input and output aliases and final cleanup. The focused Rust suite passed its recorded controls. The archive retains every timed row and the exact hashes of source, Objects, descriptors and workers.

This diagnostic compares the fixed-width patch with the wide-mask candidate. R355 remains the retained baseline.

[Raw observations and offline replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/fixed-extent-32-diagnostics-d7a533c443a6.tar.gz).
