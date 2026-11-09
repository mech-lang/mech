# Current compression scoreboard

**Retained executable: R242. 11 of25 complete compression cases are below1x Roc.** This table uses the fresh R244 matched cohort, comparing retained R242, archived R243 and candidate R244 against the same reference. New denominators can change ratios without a compiler change; earlier reports retain their original observations.

R244 is archived. It recovers most of R243's large-case penalty, but periodic is1.625343x versus retained1.617537x (+0.4826% balanced block elapsed) and does not improve in both cohorts. The retained compiler remains R242. All three variants beat the reference on the same11 cases, with no threshold wins gained/lost. Nine R244 pooled block means are slower than R242; all pooled historical1.05 gates pass. Passing those gates alone does not establish a target win.

These are complete level-12 experimental-executable runs with matched runtime inputs/output ownership. OrdinaryIR215 and standalone omitted additional contracts remain unchanged. All25 sizes are timed in two balanced cohorts after every candidate. Ratios below use pooled observation medians; changes use medians of balanced block means and can have a different direction. Below1x is a point estimate. Empty/one/seven-byte calls are near clock resolution and retain that limitation.

## Current retained wins

| Case | Input bytes | R242 / Roc |
|---|---:|---:|
| eight-literals | 8 | **0.486901x** |
| eight-overlap | 8 | **0.488129x** |
| eight-literals-prefix | 8 | **0.495990x** |
| 128-biased-dynamic | 128 | **0.507794x** |
| 32-high-entropy-stored | 32 | **0.527496x** |
| 320-overlap | 320 | **0.557249x** |
| random-64 | 64 | **0.559874x** |
| repeated-1024 | 1024 | **0.650560x** |
| random-256 | 256 | **0.658935x** |
| random-1024 | 1024 | **0.747988x** |
| reymont-prefix-1024 | 1024 | **0.925885x** |

## Every case and candidate regression

| Case | Input bytes | Retained R242 / Roc | R243 / Roc | Candidate R244 / Roc | R244 block vs R242 |
|---|---:|---:|---:|---:|---:|
| empty | 0 | 1.976190x | 1.976190x | 1.976190x | -1.0784% |
| one-stored | 1 | 1.976190x | 1.976190x | 1.976190x | -0.4084% |
| seven-stored-prefix | 7 | 2.000000x | 2.000000x | 2.000000x | -0.7628% |
| eight-literals | 8 | 0.486901x | 0.495208x | 0.493434x | +0.2959% |
| eight-overlap | 8 | 0.488129x | 0.493422x | 0.486842x | -0.2318% |
| 320-overlap | 320 | 0.557249x | 0.562240x | 0.552951x | -0.6848% |
| eight-literals-prefix | 8 | 0.495990x | 0.499856x | 0.494271x | +0.7518% |
| 128-biased-dynamic | 128 | 0.507794x | 0.504919x | 0.505855x | -0.3521% |
| 32-high-entropy-stored | 32 | 0.527496x | 0.529853x | 0.528493x | -0.9779% |
| 16384-distribution-change | 16384 | 2.742602x | 2.834176x | 2.740287x | -0.0844% |
| 33027-periodic-window | 33027 | 1.617537x | 1.654599x | 1.625343x | +0.4826% |
| random-64 | 64 | 0.559874x | 0.554677x | 0.559730x | -0.0734% |
| random-256 | 256 | 0.658935x | 0.652929x | 0.655863x | -1.1164% |
| random-1024 | 1024 | 0.747988x | 0.747581x | 0.742712x | -1.0504% |
| repeated-1024 | 1024 | 0.650560x | 0.658024x | 0.646894x | -0.1696% |
| random-32769 | 32769 | 2.746343x | 2.838488x | 2.746032x | -0.0113% |
| repeated-32769 | 32769 | 1.523366x | 1.562943x | 1.528157x | +0.3145% |
| reymont-prefix-1024 | 1024 | 0.925885x | 0.924520x | 0.925885x | +0.3271% |
| reymont-prefix-16384 | 16384 | 2.181973x | 2.188586x | 2.182951x | +0.0448% |
| reymont-prefix-32769 | 32769 | 2.222992x | 2.229990x | 2.223049x | +0.0026% |
| heldout-window-32767 | 32767 | 2.226325x | 2.238016x | 2.229299x | +0.1336% |
| heldout-window-32768 | 32768 | 2.225359x | 2.233924x | 2.226467x | +0.0498% |
| heldout-slides-65537 | 65537 | 2.299736x | 2.305275x | 2.296964x | -0.1206% |
| heldout-random-65537 | 65537 | 3.016067x | 3.103847x | 2.998607x | -0.5789% |
| heldout-mixed-131073 | 131073 | 2.464219x | 2.523235x | 2.458190x | -0.2447% |

[All absolute times, both cohorts, instructions/cycles/memory, source pins, tests and dispositions](native-constant-helper-geometry.md). Its public evidence retains every observation and numerical replay. benchmarks/mech-deflate/experiments.json records every experiment and retain/archive decision; previous checkpoint reports remain historical evidence.
