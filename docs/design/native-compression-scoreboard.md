# Current compression scoreboard

**Retained experimental executable: R245. 11 of25 complete compression cases are below1x Roc.** All rows use the same fresh R245 matched cohort. R242 is the prior retained experimental compiler; R237 is preserved separately in the detailed report. Historical denominators remain in their original reports.

R245 improves periodic33 from1.620851x to1.582628x, with2.22%/2.55% lower balanced block elapsed in the initial/confirmation cohorts. Eleven pooled block means worsen elsewhere; every original1.05 point/block gate passes. There are no new or lost below1x cases. Eight literals remains0.495073x; its adverse initial pair and small instruction/cycle increase remain in the detailed report.

These are complete level12 experimental native executable runs with runtime inputs and matched output ownership. OrdinaryIR215 and the standalone's omitted additional contracts remain unchanged. The tested experimental compiler source/patch is in the linked evidence packet. All25 inputs/sizes are timed after every candidate. Ratios use pooled observation medians; changes use balanced block means, so directions can differ. Below1x is a point estimate. Empty/one/seven-byte calls remain near clock resolution.

## All current wins

| Case | Input bytes | Retained R245 / Roc |
|---|---:|---:|
| eight-overlap | 8 | **0.493131x** |
| eight-literals | 8 | **0.495073x** |
| eight-literals-prefix | 8 | **0.506138x** |
| 128-biased-dynamic | 128 | **0.518163x** |
| 32-high-entropy-stored | 32 | **0.549802x** |
| 320-overlap | 320 | **0.557606x** |
| random-64 | 64 | **0.569464x** |
| repeated-1024 | 1024 | **0.648246x** |
| random-256 | 256 | **0.671579x** |
| random-1024 | 1024 | **0.763209x** |
| reymont-prefix-1024 | 1024 | **0.923515x** |

## Every case and regression

| Case | Input bytes | Prior R242 / Roc | Retained R245 / Roc | Block change | Instruction change |
|---|---:|---:|---:|---:|---:|
| empty | 0 | 1.976190x | 1.976190x | -3.9764% | -0.8176% |
| one-stored | 1 | 1.976190x | 1.976190x | -3.1882% | -0.7996% |
| seven-stored-prefix | 7 | 2.000000x | 1.976190x | -5.1893% | -0.7934% |
| eight-literals | 8 | 0.504641x | 0.495073x | -1.7804% | +0.2882% |
| eight-overlap | 8 | 0.490414x | 0.493131x | +0.8276% | +0.2891% |
| 320-overlap | 320 | 0.559828x | 0.557606x | -0.4831% | -0.1150% |
| eight-literals-prefix | 8 | 0.513316x | 0.506138x | -0.7094% | +0.2963% |
| 128-biased-dynamic | 128 | 0.516865x | 0.518163x | +1.0346% | +0.1265% |
| 32-high-entropy-stored | 32 | 0.546704x | 0.549802x | +0.5212% | +0.2783% |
| 16384-distribution-change | 16384 | 2.738145x | 2.734700x | -0.1258% | -0.7906% |
| 33027-periodic-window | 33027 | 1.620851x | 1.582628x | -2.3582% | -0.9039% |
| random-64 | 64 | 0.572755x | 0.569464x | +1.4666% | +0.2079% |
| random-256 | 256 | 0.664982x | 0.671579x | +1.3265% | +0.0344% |
| random-1024 | 1024 | 0.770629x | 0.763209x | -0.2430% | -0.5776% |
| repeated-1024 | 1024 | 0.659273x | 0.648246x | -1.9465% | -0.6142% |
| random-32769 | 32769 | 2.757593x | 2.729007x | -1.0366% | -1.6695% |
| repeated-32769 | 32769 | 1.523382x | 1.499992x | -1.5354% | -0.9070% |
| reymont-prefix-1024 | 1024 | 0.928711x | 0.923515x | -0.4684% | -0.7207% |
| reymont-prefix-16384 | 16384 | 2.185549x | 2.199768x | +0.6506% | -2.0641% |
| reymont-prefix-32769 | 32769 | 2.230560x | 2.240494x | +0.4454% | -2.7654% |
| heldout-window-32767 | 32767 | 2.227567x | 2.239588x | +0.5396% | -2.7692% |
| heldout-window-32768 | 32768 | 2.225616x | 2.237501x | +0.5340% | -2.7724% |
| heldout-slides-65537 | 65537 | 2.301132x | 2.304383x | +0.1413% | -3.3400% |
| heldout-random-65537 | 65537 | 3.019573x | 2.983332x | -1.2002% | -1.2646% |
| heldout-mixed-131073 | 131073 | 2.469678x | 2.502989x | +1.3488% | -1.2800% |

[All absolute times, both cohorts, instruction/cycle/memory measurements, code/stack costs, source pins, tests, skipped coverage and disposition](native-vector-phase-specialization.md). Its public evidence retains every observation and numerical replay. benchmarks/mech-deflate/experiments.json records every round and retain/archive decision. Earlier checkpoint reports remain historical evidence.
