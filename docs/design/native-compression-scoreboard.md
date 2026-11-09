# Current compression scoreboard

**Retained executable: R242. 11 of25 cases are below1x Roc.** The measured cohort is R243, with git baseline/publication `a2dc7c4d635780c274d7d4ce2fec6910d897f4fa` and exact source/artifact pins in the linked report. The retained executable uses this cohort's **before** column. A new reference denominator can change a ratio without a compiler change; older published measurements remain historical.

Archive R243 as a measured component; keep R242 as the periodic baseline. Preserve the small-case improvements for the next bounded combination, not discard them.

These are complete level-12 compression runs of the experimental executable with matched runtime inputs/output ownership. OrdinaryIR215 and the standalone's omitted additional contracts remain unchanged. All25 inputs and sizes are timed after every candidate, in two balanced cohorts; this table compares both variants and retains regressions. This candidate gains 0 below1x cases and loses 0; 12 pooled block means are slower. All initial/confirmation/pooled historical1.05 gates pass. Passing the historical gates alone does not establish that a candidate should be adopted.

Ratios use pooled observation medians against the same current reference cohort. The change column uses medians of balanced block means, so the statistics can have different directions. Below1x is an observed point estimate, not a confidence claim. Empty/one/seven-byte calls are near clock resolution (about83-84ns vs42ns); their approximately2x ratios retain that limitation.

## Retained cases currently faster than Roc

| Case | Input bytes | Mech / Roc |
|---|---:|---:|
| eight-overlap | 8 | **0.487586x** |
| eight-literals | 8 | **0.491448x** |
| eight-literals-prefix | 8 | **0.493698x** |
| 128-biased-dynamic | 128 | **0.503152x** |
| 32-high-entropy-stored | 32 | **0.526721x** |
| 320-overlap | 320 | **0.550974x** |
| random-64 | 64 | **0.560933x** |
| repeated-1024 | 1024 | **0.641187x** |
| random-256 | 256 | **0.649137x** |
| random-1024 | 1024 | **0.744782x** |
| reymont-prefix-1024 | 1024 | **0.918168x** |

## Complete matrix, including regressions

| Case | Input bytes | Before / Roc | Candidate / Roc | Retained below1x | Candidate block change |
|---|---:|---:|---:|---|---:|
| empty | 0 | 1.976190x | 1.976190x | no | -0.7217% |
| one-stored | 1 | 1.976190x | 1.976190x | no | -1.4206% |
| seven-stored-prefix | 7 | 2.000000x | 1.976190x | no | -1.2994% |
| eight-literals | 8 | 0.491448x | 0.485602x | yes | -1.3725% |
| eight-overlap | 8 | 0.487586x | 0.473116x | yes | -3.4076% |
| 320-overlap | 320 | 0.550974x | 0.545754x | yes | -0.6030% |
| eight-literals-prefix | 8 | 0.493698x | 0.477545x | yes | -3.9672% |
| 128-biased-dynamic | 128 | 0.503152x | 0.496414x | yes | -1.0094% |
| 32-high-entropy-stored | 32 | 0.526721x | 0.514179x | yes | -2.0524% |
| 16384-distribution-change | 16384 | 2.751893x | 2.824210x | no | +2.6279% |
| 33027-periodic-window | 33027 | 1.624810x | 1.648663x | no | +1.4680% |
| random-64 | 64 | 0.560933x | 0.544895x | yes | -3.3415% |
| random-256 | 256 | 0.649137x | 0.630132x | yes | -3.2230% |
| random-1024 | 1024 | 0.744782x | 0.732794x | yes | -1.3891% |
| repeated-1024 | 1024 | 0.641187x | 0.640936x | yes | +0.4820% |
| random-32769 | 32769 | 2.763567x | 2.831847x | no | +2.4707% |
| repeated-32769 | 32769 | 1.531830x | 1.560767x | no | +1.8891% |
| reymont-prefix-1024 | 1024 | 0.918168x | 0.906952x | yes | -1.1547% |
| reymont-prefix-16384 | 16384 | 2.179456x | 2.182403x | no | +0.1352% |
| reymont-prefix-32769 | 32769 | 2.224053x | 2.233209x | no | +0.4117% |
| heldout-window-32767 | 32767 | 2.224811x | 2.235202x | no | +0.4670% |
| heldout-window-32768 | 32768 | 2.223193x | 2.234112x | no | +0.4911% |
| heldout-slides-65537 | 65537 | 2.299134x | 2.305577x | no | +0.2802% |
| heldout-random-65537 | 65537 | 3.005702x | 3.094068x | no | +2.9399% |
| heldout-mixed-131073 | 131073 | 2.465124x | 2.523425x | no | +2.3650% |

Both columns use the same fresh reference denominator. The pinned source, full algorithm, tables, search limits, real scratch sizes, allocator, allocation sites and timing endpoints are unchanged.

[All25 absolute times, both cohorts, hardware/memory counters, source pins, correctness and failure disposition](native-array-helper-geometry.md). The hashed public evidence retains every observation and report replayer. benchmarks/mech-deflate/experiments.json retains every round and its retain/archive decision. Earlier checkpoint reports remain historical evidence.
