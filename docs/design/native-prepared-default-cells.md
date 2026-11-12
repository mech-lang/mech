# R342 — Prepare both default costs

**Retain R333 with the R272 runtime.** The compiler experiment stops at the 16-case screen. Natural 256 KiB changes +1.173% in the matched comparison and +1.239% in the primary screen. Reymont changes +0.756% and +1.080% in those comparisons.

Measured compiler ebc837def6d5c878cb3ecdeadd0e3f078a3f12e4. Published parent 2606499b4d263bc48394a35888b6628da8a828b3. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R333 closure and the archived compiler delta. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The benchmark source, bytecode, allocator, limits, tables, capacities and complete-call boundary are pinned.

## Use one load for every admitted offset

The compiler reuses the combined-index proof. It verifies a wrapping subtraction of one from the same current unsigned word, a shift with exactly two proved counts and a threshold equal to the bank length. It prepares two extra U32 cells from the current cost snapshot. The high out-of-range index clamps to the first extra cell. The low zero-wrap index clamps to the second. Both use the same load as the mapped cells. The compiler keeps the original checked path when the proof declines and preserves the existing current-view and profitability admission.

| Linked route | R333 instructions | R340 instructions | R342 instructions |
|---|---:|---:|---:|
| Mapped cost lookup | 14 | 10 | 14 |
| Default cost lookup | 14 | 15 | 14 |
| Valid record through short dispatch | 28 | 24 | 28 |
| Singleton through next record | 52 | 48 | 52 |
| Two candidates through next record | 70 | 66 | 70 |
| Prepare one regular cell in each bank | 13 | 13 | 12 |

The mapped route drops one branch and keeps its stack-base reload. It repeats the split comparison before the two selections. The continuing scalar visit remains 16 instructions. Table setup grows by three instructions outside the fill loop and removes the cost-base reload from each of its 256 visits. The pinned reference uses a variable shift, a slot load and a cost load. The C binary uses a dense slot map followed by a cost load. The Rust scalar fixture selects and clamps the same cell with one split comparison.

The composed phase changes from 1,520 to 1,520 code bytes and from 2,272 to 2,272 stack bytes. Four of 351 bodies change. All four are composed phases. The root, matcher and 345 other bodies agree. The original and decoded images match. The two extra cells fit in the existing frame. Metadata agrees with R333 in every form, and the flag-disabled Object reproduces R333. The trial’s scratch_bytes diagnostic reports 2,048. The emitted CLIF reserves 2,056, and the reported frame size remains exact.

## Matched measurements

| Input | R333 / Roc | R342 / Roc | R333 / libdeflate | R342 / libdeflate | Matched change | Primary change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 2.103989× | 2.074944× | 0.016890× | 0.016657× | -1.380% | +0.000% |
| one-stored | 2.046928× | 2.030119× | 0.017962× | 0.017814× | -0.821% | +0.000% |
| seven-stored-prefix | 2.254143× | 2.243237× | 0.018759× | 0.018668× | -0.484% | +0.000% |
| eight-literals | 0.185229× | 0.184405× | 2.355256× | 2.344775× | -0.445% | -0.354% |
| reymont-prefix-32769 | 1.260165× | 1.269688× | 1.148514× | 1.157194× | +0.756% | +1.080% |
| heldout-window-32768 | 1.261781× | 1.268932× | 1.149282× | 1.155796× | +0.567% | +1.419% |
| prefix-natural-32767 | 1.252636× | 1.267121× | 1.147084× | 1.160348× | +1.156% | +1.065% |
| prefix-natural-32768 | 1.259040× | 1.268788× | 1.147097× | 1.155978× | +0.774% | +0.555% |
| prefix-natural-65537 | 1.266671× | 1.280714× | 1.066305× | 1.078127× | +1.109% | +1.123% |
| prefix-natural-131072 | 1.270421× | 1.284515× | 1.041667× | 1.053223× | +1.109% | +1.058% |
| prefix-natural-262144 | 1.276885× | 1.291867× | 1.031795× | 1.043901× | +1.173% | +1.239% |
| prefix-natural-1048576 | 1.283107× | 1.300585× | 1.035975× | 1.050087× | +1.362% | +1.431% |
| prefix-random-16384 | 1.070350× | 1.063172× | 2.893570× | 2.874164× | -0.671% | -0.453% |
| prefix-random-1048576 | 1.594834× | 1.592418× | 2.251802× | 2.248390× | -0.152% | +0.279% |
| prefix-periodic-1048576 | 0.710087× | 0.712900× | 1.201515× | 1.206276× | +0.396% | -0.059% |
| prefix-repeated-1048576 | 0.710050× | 0.710025× | 1.114357× | 1.114317× | -0.004% | -0.011% |

154,752 balanced complete calls. The screen gates pass 16/16 pooled and 16/16 block. The primary paired geometric mean changes +0.521%. The highest matched change is +1.362% on prefix-natural-1048576.

| Input | R333 instructions | R342 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 715,875.5 | 715,834.0 | -0.006% | -0.358% |
| reymont-prefix-32769 | 70,038,852.0 | 70,036,190.0 | -0.004% | +0.516% |
| prefix-natural-262144 | 693,755,820.0 | 693,751,546.0 | -0.001% | +1.026% |
| prefix-natural-1048576 | 2,488,102,484.0 | 2,488,070,082.5 | -0.001% | +0.937% |
| prefix-random-16384 | 10,198,122.0 | 10,198,481.5 | +0.004% | -0.598% |
| prefix-random-1048576 | 576,166,929.5 | 576,167,679.0 | +0.000% | +0.175% |
| prefix-periodic-1048576 | 488,913,266.5 | 488,910,618.5 | -0.001% | -0.008% |
| prefix-repeated-1048576 | 496,683,686.5 | 496,684,814.0 | +0.000% | +0.076% |

The six heap distributions match on 16/16 cases. The separately sampled profiles use the exact candidate and include validation.

| Candidate profile input | Running samples | Composed phase linked PCs |
|---|---:|---:|
| eight-literals | 5,198 | 0 |
| prefix-natural-262144 | 5,355 | 3,084 |
| reymont-prefix-32769 | 4,665 | 2,413 |

## Qualification

Warnings-as-errors builds pass. All 53 CFG tests pass, including four focused tests for exact current words, projection width, shift masks and the table extent. The linked oracle checks all 65,536 offsets on each of two current cost snapshots and both image forms for 262,144 results. It includes both default branches, unchanged backing with new costs, the source rejection origin and successful retry. The 324 whole-compressor checks retain exact streams, independent inflation, capacities, held aliases and cleanup. The benchmark uses the compact diagnostic entry.

The retained all-81 scoreboard remains R333. The archive preserves every pilot result.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r342-prepared-default-cells-c7cf26101e71.tar.gz). SHA-256 c7cf26101e71454a7dca606db6276b618cc7e4a2c94fc97d695b221482838dd4.
