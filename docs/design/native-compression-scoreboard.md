# Current compression scoreboard

**Retained experimental executable: R245. 11 of25 complete compression cases are below1x Roc.** R246 is archived as a measured combination; its repeatable natural-text gain is preserved. The retained compiler has not changed. This table uses both variants against the same fresh R246 reference cohort; previous reports keep their original observations.

R246 combines the existing R237 loop-exit threader with R245 vector phases. Natural32 improves2.239774x->2.141492x (-4.3880% block elapsed), but periodic worsens1.599350x->1.660024x (+3.7937%) in both cohorts. Eight stays below1x while its confirmation historical1.05 gates fail. Pooled gates pass25/25; confirmation passes24/25. Fifteen pooled block means worsen. No below1x case is gained/lost.

These are complete level12 experimental native runs with runtime inputs and matched output ownership. OrdinaryIR215 and standalone extra-contract omissions remain unchanged. Every25-case row is retained after every candidate. Ratios use pooled observation medians; changes use balanced block means, so their directions can differ. Fresh reference denominators can move ratios without a compiler change. Below1x is a point estimate;0/1/7-byte calls remain near clock resolution.

## All current retained wins

| Case | Bytes | Retained R245 / Roc |
|---|---:|---:|
| eight-literals-prefix | 8 | **0.503478x** |
| eight-overlap | 8 | **0.510621x** |
| 128-biased-dynamic | 128 | **0.510730x** |
| eight-literals | 8 | **0.513988x** |
| 32-high-entropy-stored | 32 | **0.540318x** |
| 320-overlap | 320 | **0.559668x** |
| random-64 | 64 | **0.568511x** |
| repeated-1024 | 1024 | **0.649719x** |
| random-256 | 256 | **0.658766x** |
| random-1024 | 1024 | **0.747621x** |
| reymont-prefix-1024 | 1024 | **0.934397x** |

## Every case and candidate regression

| Case | Bytes | Retained R245 / Roc | Candidate R246 / Roc | Candidate block change | Instruction change |
|---|---:|---:|---:|---:|---:|
| empty | 0 | 1.976190x | 1.976190x | -0.7388% | +0.0317% |
| one-stored | 1 | 1.976190x | 1.976190x | +1.5796% | +0.0310% |
| seven-stored-prefix | 7 | 1.976190x | 1.976190x | -1.2639% | +0.0308% |
| eight-literals | 8 | 0.513988x | 0.524576x | +3.0194% | -0.1589% |
| eight-overlap | 8 | 0.510621x | 0.516733x | +0.8530% | -0.1653% |
| 320-overlap | 320 | 0.559668x | 0.577550x | +2.7044% | -1.2792% |
| eight-literals-prefix | 8 | 0.503478x | 0.515872x | +2.3792% | -0.1591% |
| 128-biased-dynamic | 128 | 0.510730x | 0.516820x | +0.5566% | -0.7264% |
| 32-high-entropy-stored | 32 | 0.540318x | 0.543990x | -0.1391% | -0.2564% |
| 16384-distribution-change | 16384 | 2.741179x | 2.721304x | -0.7251% | -5.0266% |
| 33027-periodic-window | 33027 | 1.599350x | 1.660024x | +3.7937% | -2.1247% |
| random-64 | 64 | 0.568511x | 0.572825x | +0.5709% | -0.3868% |
| random-256 | 256 | 0.658766x | 0.674708x | +2.3127% | -0.9156% |
| random-1024 | 1024 | 0.747621x | 0.763402x | +1.7509% | -1.9629% |
| repeated-1024 | 1024 | 0.649719x | 0.669698x | +2.9488% | -2.4610% |
| random-32769 | 32769 | 2.747608x | 2.805254x | +2.0981% | -3.9017% |
| repeated-32769 | 32769 | 1.509380x | 1.561093x | +3.4261% | -2.1284% |
| reymont-prefix-1024 | 1024 | 0.934397x | 0.922571x | -1.3825% | -3.0944% |
| reymont-prefix-16384 | 16384 | 2.200813x | 2.095515x | -4.7845% | -5.8829% |
| reymont-prefix-32769 | 32769 | 2.239774x | 2.141492x | -4.3880% | -5.6047% |
| heldout-window-32767 | 32767 | 2.248910x | 2.144908x | -4.6246% | -5.6153% |
| heldout-window-32768 | 32768 | 2.245089x | 2.141979x | -4.5927% | -5.6152% |
| heldout-slides-65537 | 65537 | 2.306179x | 2.216222x | -3.9007% | -5.1625% |
| heldout-random-65537 | 65537 | 2.987819x | 3.062057x | +2.4847% | -3.9498% |
| heldout-mixed-131073 | 131073 | 2.503778x | 2.508725x | +0.1976% | -5.6785% |

[All absolute times, both cohorts, counters/memory, code/frame costs, fresh profiles, exact source, fresh/reused validation and failure disposition](native-loop-threading-combination.md). The public packet retains every observation and replay. [Last retained compiler checkpoint](native-vector-phase-specialization.md). benchmarks/mech-deflate/experiments.json preserves every round and retain/archive decision.
