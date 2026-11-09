# Current compression scoreboard

Measured checkpoint **R242**, revision `6fe370c1cedb7a0ea26b11501e242f89d6aa852c`. **11 of 25 cases are below 1x Roc**; 14 remain above. These are complete level-12 compression runs of the experimental executable with the matched runtime-input/output-ownership comparator, not component timings. Ordinary IR215 and the standalone's omitted additional contracts remain unchanged.

All25 inputs and sizes are timed after each candidate, in two balanced cohorts. Each new result updates this table, records crossings below/above1x, and preserves regressions. This checkpoint gained 0 below1x cases and lost 0 against its matched control. 11 pooled block means are slower, even though the periodic target improves. Initial eight-byte block latency +5.3197% fails the historical1.05 gate and remains in the complete report.

Ratios use pooled observation medians against the same current reference cohort. The change column uses medians of balanced block means, so the two statistics can have different directions. Below1x is an observed point estimate, not a confidence claim. Empty/one/seven-byte calls are near clock resolution (about83-84ns vs42ns); their approximately2x ratios are reported with that limitation.

## Cases currently faster than Roc

| Case | Input bytes | Mech / Roc |
|---|---:|---:|
| eight-overlap | 8 | **0.495983x** |
| eight-literals-prefix | 8 | **0.499211x** |
| 128-biased-dynamic | 128 | **0.507024x** |
| eight-literals | 8 | **0.522129x** |
| 32-high-entropy-stored | 32 | **0.531638x** |
| 320-overlap | 320 | **0.561549x** |
| random-64 | 64 | **0.565899x** |
| repeated-1024 | 1024 | **0.651061x** |
| random-256 | 256 | **0.661644x** |
| random-1024 | 1024 | **0.753119x** |
| reymont-prefix-1024 | 1024 | **0.929778x** |

## Complete matrix, including regressions

| Case | Input bytes | Control / Roc | Current / Roc | Below1x | Block elapsed change |
|---|---:|---:|---:|---|---:|
| empty | 0 | 1.976190x | 1.976190x | no | +2.0429% |
| one-stored | 1 | 1.976190x | 1.976190x | no | -1.8564% |
| seven-stored-prefix | 7 | 2.000000x | 2.000000x | no | -0.6599% |
| eight-literals | 8 | 0.510449x | 0.522129x | yes | +1.5578% |
| eight-overlap | 8 | 0.495838x | 0.495983x | yes | +0.8878% |
| 320-overlap | 320 | 0.562933x | 0.561549x | yes | +0.4363% |
| eight-literals-prefix | 8 | 0.497989x | 0.499211x | yes | +1.1984% |
| 128-biased-dynamic | 128 | 0.505874x | 0.507024x | yes | +0.4566% |
| 32-high-entropy-stored | 32 | 0.536067x | 0.531638x | yes | -0.1658% |
| 16384-distribution-change | 16384 | 2.748297x | 2.753283x | no | +0.1814% |
| 33027-periodic-window | 33027 | 1.652601x | 1.627365x | no | -1.5270% |
| random-64 | 64 | 0.564475x | 0.565899x | yes | +0.5095% |
| random-256 | 256 | 0.655508x | 0.661644x | yes | +1.8336% |
| random-1024 | 1024 | 0.749071x | 0.753119x | yes | +0.1051% |
| repeated-1024 | 1024 | 0.661149x | 0.651061x | yes | -1.7432% |
| random-32769 | 32769 | 2.758147x | 2.751177x | no | -0.2527% |
| repeated-32769 | 32769 | 1.552739x | 1.525077x | no | -1.7815% |
| reymont-prefix-1024 | 1024 | 0.931764x | 0.929778x | yes | +0.5273% |
| reymont-prefix-16384 | 16384 | 2.198784x | 2.187019x | no | -0.5351% |
| reymont-prefix-32769 | 32769 | 2.245602x | 2.235129x | no | -0.4664% |
| heldout-window-32767 | 32767 | 2.237491x | 2.230366x | no | -0.3185% |
| heldout-window-32768 | 32768 | 2.241366x | 2.230940x | no | -0.4652% |
| heldout-slides-65537 | 65537 | 2.307768x | 2.298387x | no | -0.4065% |
| heldout-random-65537 | 65537 | 3.018658x | 3.008478x | no | -0.3372% |
| heldout-mixed-131073 | 131073 | 2.473993x | 2.465542x | no | -0.3416% |

The control column is this checkpoint's matched before-executable, using the fresh common reference denominator. Older published ratios stay in their original reports and are not mixed into this cohort. The pinned source, full algorithm, tables, search limits, real scratch sizes, allocator, allocation sites and timing endpoints are unchanged.

[All25 absolute times, both cohorts, hardware/memory counters, source pins, correctness and failure disposition](native-phase-exit-owner-transfer.md). The hashed public evidence linked there retains every observation and the report replayers. The experiment ledger in benchmarks/mech-deflate/experiments.json retains every round and retain/reject decision. All earlier checkpoint reports remain historical evidence.
