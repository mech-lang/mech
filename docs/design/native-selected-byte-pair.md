# R320 — Reuse the initial byte pair on a selected edge

**Retain R320 with the R272 runtime.** Across 81 cases the candidate paired elapsed change is -0.823%. The pooled and block gates pass 81/81 and 81/81. The candidate beats Roc in 46/81 primary cases (56.79%). The matched four-actor run records 3/81 faster than libdeflate and 0/81 faster than both in the candidate.

The 17 priority cases identified before the run change -0.273% by primary paired elapsed and -0.442% in the four-actor comparison. Their primary block-mean ratios change +0.438% and -0.985%. The largest adverse all-case movement is +1.837% on seven-stored-prefix.

Retention uses the established combined gates and the repeated Reymont and 256 KiB text witnesses. The full priority-group results and individual-cohort misses are reported for review.

Candidate elapsed changes -0.671% and -0.974%. Crossings below parity include none. Crossings above parity include none.

Individual block-mean misses were one-stored in primary-a (+5.494%), seven-stored-prefix in primary-a (+6.675%).

Measured parent 8903df3b33c310ba314500832aab10c9e09345bf. Published parent 52f2e9809df7eb6083b05b752d867ad2bb23f2d2. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R317 closure and attached patch. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, and IR/work/ABI/bytecode 215/107/39/14. The pinned source, bytecode, allocator, tables, limits, buffers and full-call timing boundaries are identical.

## Reuse the original byte pair on the selected mismatch

A reached unsigned window proof establishes the readable index. The selected mismatch edge forwards the exact current SSA words and follows the original checks. The compiler recognizes equal wrapping address expressions, plain byte load attributes, and an effect-free corridor since the first reads. The repeated checked increment and read bounds must be established for the same current indices. Alternate entries keep their checked read paths.

The two routes pass the original widened bytes through the existing typed continuation. The compiler discharges 13 conditions on those routes. It forwards the current live words with 20 repair parameters across both shared destinations. No stores or calls are bypassed.

The linked search and skip paths go from 31 and 36 executed instructions to five and two between the mismatch decision and the direction branch. Both eliminate their two byte reloads. The linked Roc and libdeflate executables reread their bytes on those paths. C uses two direct indexed reads. The reference executable retains its bounds branches.

1 of 347 bodies changes. The matcher shrinks from 12,904 to 12,784 bytes and its frame grows from 608 to 624 bytes. Static stack reads change from 570 to 541.

## Matched results

| Input | R317 / Roc | R320 / Roc | R317 / libdeflate | R320 / libdeflate | Primary elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.183858× | 0.183280× | 2.311610× | 2.304338× | +0.147% |
| prefix-natural-4096 | 0.976297× | 0.972918× | 1.662997× | 1.657240× | -0.928% |
| prefix-random-16384 | 1.081908× | 1.073569× | 2.919012× | 2.896514× | -1.031% |
| reymont-prefix-32769 | 1.315659× | 1.310586× | 1.200439× | 1.195810× | -0.255% |
| heldout-window-32768 | 1.317757× | 1.312644× | 1.198497× | 1.193847× | -0.394% |
| prefix-natural-32767 | 1.320685× | 1.313833× | 1.202193× | 1.195955× | -0.445% |
| prefix-natural-32768 | 1.322085× | 1.319208× | 1.201258× | 1.198644× | -0.421% |
| prefix-natural-65537 | 1.325106× | 1.319838× | 1.113718× | 1.109290× | -0.474% |
| prefix-natural-262144 | 1.342164× | 1.333217× | 1.087771× | 1.080520× | -0.469% |
| prefix-natural-1048576 | 1.346879× | 1.336065× | 1.087282× | 1.078552× | -0.868% |
| prefix-periodic-1048576 | 0.719039× | 0.708879× | 1.219598× | 1.202365× | -1.459% |
| prefix-repeated-1048576 | 0.719976× | 0.710964× | 1.131335× | 1.117174× | -1.391% |

The separate four-actor cohort has 245,664 timed calls and exact raw stream agreement across all 81 cases. Each libdeflate call includes a fresh level-12 compressor. The 15-case pilot measured -0.502% by paired block-median ratios.

## Historical baseline from the supplied brief

A separate 15-case four-actor cohort compares exact R309 at 6120aba1263cab886cd1d66c3c3c8d610c590172 with R320 and the same references across 154,656 calls. This measures the combined compiler changes since R309.

| Input | R309 / Roc | R320 / Roc | R309 / libdeflate | R320 / libdeflate |
|---|---:|---:|---:|---:|
| heldout-window-32768 | 1.362453× | 1.313484× | 1.240299× | 1.195721× |
| prefix-natural-262144 | 1.389471× | 1.336591× | 1.121899× | 1.079203× |
| prefix-natural-1048576 | 1.403250× | 1.335695× | 1.133821× | 1.079237× |

## Cases where Roc is the faster peer

Roc is faster than libdeflate in 17/81 matched cases. The nearest substantial inputs are natural text around 32 and 64 KiB. The frontier uses the same balanced blocks for all three ratios.

| Input | Mech / Roc | Mech / libdeflate | Elapsed cut to Roc |
|---|---:|---:|---:|
| reymont-prefix-32769 | 1.310586× | 1.195810× | 23.70% |
| heldout-window-32768 | 1.312644× | 1.193847× | 23.82% |
| prefix-natural-32767 | 1.313833× | 1.195955× | 23.89% |
| heldout-window-32767 | 1.316052× | 1.198353× | 24.02% |
| prefix-natural-32769 | 1.317656× | 1.198992× | 24.11% |
| prefix-natural-32768 | 1.319208× | 1.198644× | 24.20% |
| prefix-natural-65535 | 1.319330× | 1.107505× | 24.20% |
| prefix-natural-65537 | 1.319838× | 1.109290× | 24.23% |

## Independent counters and profiles

The six Mech payload and heap families agree across all 81 cases. CPU counter measurements and profile samples were collected in separate runs. The profile includes validation.

| Input | R317 instructions | R320 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 715,933.0 | 715,808.0 | -0.017% | -3.982% |
| prefix-natural-4096 | 9,334,361.0 | 9,266,021.5 | -0.732% | -0.738% |
| prefix-random-16384 | 10,566,455.5 | 10,448,951.5 | -1.112% | -1.038% |
| reymont-prefix-32769 | 77,091,413.5 | 75,958,166.0 | -1.470% | -0.836% |
| heldout-window-32768 | 76,840,385.5 | 75,708,955.0 | -1.472% | -0.619% |
| prefix-natural-32767 | 76,840,490.0 | 75,709,491.5 | -1.472% | -0.575% |
| prefix-natural-32768 | 76,840,989.5 | 75,711,350.0 | -1.470% | -0.749% |
| prefix-natural-65537 | 184,347,577.5 | 181,306,375.5 | -1.650% | -0.636% |
| prefix-natural-262144 | 773,289,393.5 | 759,150,787.5 | -1.828% | -0.761% |
| prefix-natural-1048576 | 2,771,693,106.0 | 2,713,273,235.0 | -2.108% | -0.681% |
| prefix-periodic-1048576 | 500,688,091.0 | 489,215,967.5 | -2.291% | -1.462% |
| prefix-repeated-1048576 | 508,474,355.0 | 497,048,211.0 | -2.247% | -1.373% |

| Profile | Running samples | Leading linked leaf | Samples |
|---|---:|---|---:|
| candidate-profiles/eight-literals | 5250 | mech_compact_closed_v2 | 1636 |
| candidate-profiles/prefix-natural-262144 | 5359 | mech_compact_numeric_15296_no_growth | 3098 |
| candidate-profiles/reymont-prefix-32769 | 4392 | mech_compact_numeric_15296_no_growth | 2221 |

## Qualification

The warnings-as-errors build and 55 CFG tests pass. The focused test executes 90 before/after JIT pairs, with changed bounds and addresses, stores before the decision and in the reread route, an alternate entry, and failed checked extents. The 324 linked original and decoded calls agree on exact bytes, independent inflate, held aliases, capacities and final managed cleanup. Both forms have identical Object, metadata, matcher CLIF and matcher VCode.

The shortened path also makes two remaining equality values redundant after the branch. Each is used once after a dominating true edge. The search stores its Boolean and skip keeps its Boolean in a register across extension. The separate audit proves both uses can become constants.

The replay checks all four balanced cohorts and the 81 matched cases from raw per-call samples. It regenerates the timing gates, hardware counters, both matched comparisons, and this report.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r320-selected-byte-pair-e8ebfaee81fb.tar.gz). SHA-256 e8ebfaee81fb739d4f8182bcea00a3cb6c74d10d529566490681a7c1e67f77bf.
