# R338 — Combine the first two scalar candidates

**Retain R333 with the R272 runtime.** The compiler experiment stops at the 16-case screen. Natural 256 KiB changes +2.744% in the matched comparison and +2.696% in the primary screen. Reymont changes +2.130% and +2.014% in those comparisons.

Measured compiler ebc837def6d5c878cb3ecdeadd0e3f078a3f12e4. Published parent dbe900ffdb1d94a8c080faea42d0ab61765f5650. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R333 closure and the archived compiler delta. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The benchmark source, bytecode, allocator, limits, tables, capacities and complete-call boundary are pinned.

## Complete the admitted pair before entering the scalar loop

The complete-child proof admits both immutable reads and all reached source checks and work prices. The first cost is evaluated on the original short path. The multi-candidate edge admits the second cost. A stable unsigned comparison selects the earlier of equal costs, then the original strict improvement rule compares the chosen cost with the incoming minimum. The selected-item expression reads only the candidate index and invariant captures. It is emitted once for the pair. Exactly two visits finish directly. Longer intervals continue with the third logical position and the current reverse cursor.

The linked two-candidate path changes from 42 to 38 instructions from short entry through completion. The singleton path changes from 24 to 26. Both preserve the original read counts. The ARM64 scheduler still emits the pair comparison and the incoming comparison twice in the new path. The Rust two-candidate control emits one of each. The pinned reference excerpts show the existing recurring scalar comparisons in both peers.

The composed phase changes from 1,520 to 1,680 code bytes and from 2,272 to 2,288 stack bytes. Eight of 351 bodies change. The root, matcher and 341 other bodies agree. The original and decoded images match. The flag-disabled Object and metadata reproduce R333.

## Matched measurements

| Input | R333 / Roc | R338 / Roc | R333 / libdeflate | R338 / libdeflate | Matched change | Primary change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 2.153337× | 2.100192× | 0.016760× | 0.016346× | -2.468% | +0.000% |
| one-stored | 2.034647× | 1.982009× | 0.017927× | 0.017463× | -2.587% | +0.000% |
| seven-stored-prefix | 2.250676× | 2.207653× | 0.018730× | 0.018372× | -1.912% | +0.000% |
| eight-literals | 0.181907× | 0.183664× | 2.323644× | 2.346081× | +0.966% | +1.189% |
| reymont-prefix-32769 | 1.261222× | 1.288081× | 1.145595× | 1.169992× | +2.130% | +2.014% |
| heldout-window-32768 | 1.257926× | 1.281388× | 1.148015× | 1.169427× | +1.865% | +1.835% |
| prefix-natural-32767 | 1.257686× | 1.283201× | 1.147306× | 1.170581× | +2.029% | +2.440% |
| prefix-natural-32768 | 1.261132× | 1.287912× | 1.148476× | 1.172863× | +2.123% | +2.198% |
| prefix-natural-65537 | 1.264963× | 1.296115× | 1.065140× | 1.091371× | +2.463% | +2.238% |
| prefix-natural-131072 | 1.265826× | 1.298486× | 1.043421× | 1.070343× | +2.580% | +2.274% |
| prefix-natural-262144 | 1.275905× | 1.310913× | 1.030885× | 1.059170× | +2.744% | +2.696% |
| prefix-natural-1048576 | 1.283219× | 1.312038× | 1.035401× | 1.058655× | +2.246% | +2.395% |
| prefix-random-16384 | 1.063628× | 1.063275× | 2.864488× | 2.863536× | -0.033% | -0.007% |
| prefix-random-1048576 | 1.594067× | 1.594225× | 2.248938× | 2.249161× | +0.010% | +0.019% |
| prefix-periodic-1048576 | 0.708369× | 0.706758× | 1.201396× | 1.198663× | -0.227% | -0.184% |
| prefix-repeated-1048576 | 0.710646× | 0.709761× | 1.116295× | 1.114905× | -0.125% | -0.108% |

154,752 balanced complete calls. The screen gates pass 16/16 pooled and 16/16 block. The primary paired geometric mean changes +1.181%. The highest matched change is +2.744% on prefix-natural-262144.

| Input | R333 instructions | R338 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 725,394.5 | 725,619.0 | +0.031% | -3.061% |
| reymont-prefix-32769 | 70,044,875.5 | 72,416,704.0 | +3.386% | +1.401% |
| prefix-natural-262144 | 693,741,181.5 | 719,355,860.5 | +3.692% | +1.806% |
| prefix-natural-1048576 | 2,488,142,928.5 | 2,572,252,384.5 | +3.380% | +1.808% |
| prefix-random-16384 | 10,197,281.5 | 10,198,939.0 | +0.016% | -0.187% |
| prefix-random-1048576 | 576,160,100.5 | 576,190,282.0 | +0.005% | +0.084% |
| prefix-periodic-1048576 | 488,915,541.0 | 489,066,854.5 | +0.031% | -0.130% |
| prefix-repeated-1048576 | 496,687,109.5 | 496,834,054.5 | +0.030% | -0.234% |

The six heap distributions match on 16/16 cases. The separately sampled profiles use the exact candidate and include validation.

| Candidate profile input | Running samples | Composed phase linked PCs |
|---|---:|---:|
| eight-literals | 5,285 | 0 |
| prefix-natural-262144 | 5,355 | 3,126 |
| reymont-prefix-32769 | 5,011 | 2,609 |

## Qualification

Warnings-as-errors builds pass. The linked oracle checks 136,192 candidate positions across both image forms and 68,096 retained positions. It covers zero, one, two and longer intervals, nonzero starts, earliest ties, wrapping costs, vector tails, source rejection origin and retry. The 324 whole-compressor checks retain exact streams, independent inflation, capacities, held aliases and cleanup. The benchmark uses the compact diagnostic entry.

The retained all-81 scoreboard remains R333. The archive preserves every pilot result.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r338-peeled-pair-1130d0e35143.tar.gz). SHA-256 1130d0e351439d8f3bc2278a3a45bac64480c63326f0363389e69606d817b7ff.
