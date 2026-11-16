# R340 — Combine the table bank and index

**Retain R333 with the R272 runtime.** The compiler experiment stops at the 16-case screen. Natural 256 KiB changes +1.302% in the matched comparison and +1.359% in the primary screen. Reymont changes +1.939% and +1.241% in those comparisons.

Measured compiler ebc837def6d5c878cb3ecdeadd0e3f078a3f12e4. Published parent 2cd65ad661e0c152e29c86ec585ccc3474a4a6d7. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R333 closure and the archived compiler delta. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The benchmark source, bytecode, allocator, limits, tables, capacities and complete-call boundary are pinned.

## Select one index into the current prepared table

The existing exact selector proof identifies two outcomes of a wrapping subtraction and shift. The compiler additionally checks that the slot index uses the same current unsigned word, subtracts one, and uses the same selector or a mask that retains all its possible bits. The low range covers the first bank and the zero wrap. The high range uses a constant shift and an offset to the second bank. One bound admits the combined index. Both default values, the current cost capture, the sparse route and table preparation remain intact.

| Linked route | R333 instructions | Initial rewrite | R340 instructions |
|---|---:|---:|---:|
| Mapped cost lookup | 14 | 13 | 10 |
| Default cost lookup | 14 | 16 | 15 |
| Valid record through short dispatch | 28 | 27 | 24 |
| Singleton through next record | 52 | 51 | 48 |
| Two candidates through next record | 70 | 69 | 66 |

The initial rewrite kept a separate bank calculation and duplicated the split comparison. R340 uses a single split comparison and a constant shift on the mapped path. The current cost lookup still reloads one stack base. The continuing scalar visit remains 16 instructions. The linked reference uses a variable shift, a slot load and a cost load. The pinned C binary uses a dense slot lookup followed by a cost load. The Rust scalar witness also emits a single split comparison for the combined index.

The composed phase changes from 1,520 to 1,520 code bytes and from 2,272 to 2,272 stack bytes. Four of 351 bodies change. All four are composed phases. The root, matcher and 345 other bodies agree. The original and decoded images match. The metadata matches R333 in every form, and the flag-disabled Object reproduces R333.

## Matched measurements

| Input | R333 / Roc | R340 / Roc | R333 / libdeflate | R340 / libdeflate | Matched change | Primary change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 2.167739× | 2.097753× | 0.016980× | 0.016432× | -3.229% | +0.000% |
| one-stored | 2.046179× | 1.993322× | 0.018212× | 0.017741× | -2.583% | +0.000% |
| seven-stored-prefix | 2.307718× | 2.258553× | 0.018864× | 0.018462× | -2.130% | +0.000% |
| eight-literals | 0.182494× | 0.183241× | 2.345433× | 2.355035× | +0.409% | -0.391% |
| reymont-prefix-32769 | 1.256064× | 1.280425× | 1.144418× | 1.166614× | +1.939% | +1.241% |
| heldout-window-32768 | 1.258132× | 1.274475× | 1.145019× | 1.159893× | +1.299% | +1.555% |
| prefix-natural-32767 | 1.254644× | 1.272024× | 1.146655× | 1.162538× | +1.385% | +1.175% |
| prefix-natural-32768 | 1.258463× | 1.278846× | 1.148215× | 1.166812× | +1.620% | +1.235% |
| prefix-natural-65537 | 1.266337× | 1.280618× | 1.065253× | 1.077267× | +1.128% | +0.653% |
| prefix-natural-131072 | 1.267165× | 1.284429× | 1.043212× | 1.057424× | +1.362% | +1.215% |
| prefix-natural-262144 | 1.276667× | 1.293288× | 1.030361× | 1.043775× | +1.302% | +1.359% |
| prefix-natural-1048576 | 1.282172× | 1.300819× | 1.034758× | 1.049807× | +1.454% | +1.422% |
| prefix-random-16384 | 1.068129× | 1.066282× | 2.886008× | 2.881019× | -0.173% | -0.564% |
| prefix-random-1048576 | 1.589433× | 1.588673× | 2.247894× | 2.246819× | -0.048% | -0.118% |
| prefix-periodic-1048576 | 0.708517× | 0.708504× | 1.203077× | 1.203054× | -0.002% | -0.168% |
| prefix-repeated-1048576 | 0.709272× | 0.711593× | 1.115658× | 1.119309× | +0.327% | +0.035% |

154,752 balanced complete calls. The screen gates pass 16/16 pooled and 16/16 block. The primary paired geometric mean changes +0.538%. The highest matched change is +1.939% on reymont-prefix-32769.

| Input | R333 instructions | R340 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 715,898.0 | 715,983.5 | +0.012% | +0.153% |
| reymont-prefix-32769 | 70,038,555.0 | 68,503,435.0 | -2.192% | +0.109% |
| prefix-natural-262144 | 693,729,631.5 | 676,150,526.0 | -2.534% | +0.378% |
| prefix-natural-1048576 | 2,488,104,764.5 | 2,430,340,178.0 | -2.322% | +0.656% |
| prefix-random-16384 | 10,196,015.0 | 10,198,539.5 | +0.025% | -0.483% |
| prefix-random-1048576 | 576,161,368.5 | 576,159,921.0 | -0.000% | -0.374% |
| prefix-periodic-1048576 | 488,912,209.0 | 488,852,005.5 | -0.012% | +0.043% |
| prefix-repeated-1048576 | 496,682,513.0 | 496,629,715.0 | -0.011% | +0.060% |

The six heap distributions match on 16/16 cases. The separately sampled profiles use the exact candidate and include validation.

| Candidate profile input | Running samples | Composed phase linked PCs |
|---|---:|---:|
| eight-literals | 5,253 | 0 |
| prefix-natural-262144 | 5,347 | 3,083 |
| reymont-prefix-32769 | 4,584 | 2,369 |

## Qualification

Warnings-as-errors builds pass. All 53 CFG tests pass, including four focused tests for exact current words, projection width, shift masks and the table extent. The linked oracle checks all 65,536 offsets on each of two current cost snapshots and both image forms for 262,144 results. It includes both default branches, unchanged backing with new costs, the source rejection origin and successful retry. The 324 whole-compressor checks retain exact streams, independent inflation, capacities, held aliases and cleanup. The benchmark uses the compact diagnostic entry.

The retained all-81 scoreboard remains R333. The archive preserves every pilot result.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r340-two-case-read-index-827f4aec809a.tar.gz). SHA-256 827f4aec809a66b3cf924b1a24d7439595d3b8b128b6d0588641eb6a32e4ee7e.
