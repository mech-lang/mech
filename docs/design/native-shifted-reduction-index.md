# R326 — Share the forward index with the selected reduction item

**Keep R325 with the R272 runtime.** The 256 KiB natural case moves from 1.067580× to 1.075381× libdeflate. The candidate changes elapsed +0.731% in the matched comparison and +0.952% in the separate timing screen. The paired geometric mean changes +0.423% across 15 cases. The pooled and block gates pass 15/15 and 15/15.

Measured parent ace28a0a938bdca0fcb2b91fbd5f8b48f5408034. Published parent ace28a0a938bdca0fcb2b91fbd5f8b48f5408034. PR base 92f75db84e2523693648084bea16cd52ad60777a. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, IR/work/ABI/bytecode 215/107/39/14. The original source, bytecode, capacities, allocator, search limits, tables and complete-call boundary are pinned.

## Carry the shifted forward index

The retained continuing loop uses the logical index for the forward price load and adds the same forward-window offset again to pack the selected item. The candidate uses the same shifted index for both. It requires a proved forward window, a nonzero offset that fits the admitted stop bound, and the exact offset on each selected-item use of the induction word. A bare induction use in the item or cost keeps the original loop.

The short route seeds the continuation from the first visit's shifted index. The vector route shifts the first remainder index. Both compare against the same shifted stop. The exhausted exit restores the original stop. The first candidate and vector winner calculation keep their original index semantics. Disabling the experiment reproduces the retained Object and metadata byte for byte.

Four of 347 bodies change. The recurring scalar route has 16 to 15 instructions with the same two loads and no stack accesses. The selected parser code is 1,552 to 1,568 bytes, both with a 256-byte frame. Static reads in the whole parser change from 22 to 20 and writes from 21 to 20.

## Matched results

| Input | R325 / Roc | R326 / Roc | R325 / libdeflate | R326 / libdeflate | Primary elapsed change |
|---|---:|---:|---:|---:|---:|
| empty | 2.106675× | 2.129342× | 0.016674× | 0.016853× | +0.000% |
| one-stored | 2.005113× | 2.057598× | 0.017912× | 0.018380× | +0.000% |
| seven-stored-prefix | 2.321599× | 2.257653× | 0.021932× | 0.021328× | +0.000% |
| eight-literals | 0.195854× | 0.193027× | 2.513101× | 2.476833× | +0.337% |
| reymont-prefix-32769 | 1.324923× | 1.360039× | 1.222269× | 1.254664× | +1.254% |
| heldout-window-32768 | 1.325878× | 1.370348× | 1.212400× | 1.253063× | +0.163% |
| prefix-natural-32767 | 1.313172× | 1.317654× | 1.216693× | 1.220846× | +0.603% |
| prefix-natural-32768 | 1.310083× | 1.314704× | 1.191573× | 1.195776× | +0.497% |
| prefix-natural-65537 | 1.312614× | 1.318318× | 1.104301× | 1.109100× | +0.493% |
| prefix-natural-131072 | 1.314817× | 1.328343× | 1.078880× | 1.089979× | +1.445% |
| prefix-natural-262144 | 1.321792× | 1.331450× | 1.067580× | 1.075381× | +0.952% |
| prefix-natural-1048576 | 1.330653× | 1.337619× | 1.073797× | 1.079418× | +0.577% |
| prefix-random-16384 | 1.110408× | 1.108824× | 2.925143× | 2.920971× | -0.552% |
| prefix-periodic-1048576 | 0.714407× | 0.717187× | 1.212390× | 1.217106× | +0.224% |
| prefix-repeated-1048576 | 0.713551× | 0.715286× | 1.119808× | 1.122531× | +0.367% |

The comparison contains 154,656 timed calls. It uses balanced level-12 runtime input, matching validation and fresh C compressor allocation.

## Independent counters and qualification

On 256 KiB natural the separate CPU cohort reports 755,479,556.5 to 748,362,027.5 retired instructions (-0.942%) and +0.286% cycles. The six payload counters agree across all 15 memory cases.

The candidate profile records 3,093 of 5,364 running samples in the parser. The separate R325 profile records 3,035 of 5,349. Both profiles include validation. The sampled PCs and exact linked assembly accompany the archived timing and counter cohorts.

The warnings-as-errors build and 12 ordered-reduction tests pass. They cover the original and shifted selected item against the scalar oracle with both image forms, ties, wrapping, short starts, vector tails, rejected windows and the unshifted terminal stop. The original image passes 162 linked calls with exact streams, independent inflation, capacities, held aliases and final managed cleanup.

Both timing comparisons slow Reymont, 256 KiB natural and 1 MiB natural. The separate CPU cohort retires fewer instructions and takes more cycles on all three. Keep R325.

The candidate stopped at the 15-case timing screen with original-image qualification. Full timing and decoded-image qualification remain acceptance steps for any adoption.

[Verified patch, linked machine code and pilot observations](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r326-shifted-reduction-index-1b8dc5f1b3cc.tar.gz). SHA-256 1b8dc5f1b3ccfac7d4a8526cc054d905f8c3f97fc0f887740efa975934d3b0f5.
