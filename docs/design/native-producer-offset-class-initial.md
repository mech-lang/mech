# R331 — Measure the initial producer classification pilot

**Keep R327 with the R272 runtime.** The initial classification source pilot reproduces all 81 streams. On 256 KiB natural text it takes 2.165× R327 elapsed in the matched cohort and 2.167× in the primary screen. The emitted image loses all four native parser phases. This timing includes the fallback.

Measured compiler ba8f02f61b3735a2160cc6177e76c6f7cc59cf7b. Published parent bb2b7c64fda075eba6e595ced51e4137fcee6fa8. PR base 92f75db84e2523693648084bea16cd52ad60777a. Same backend and runtime. Diagnostic source representation only.

Each of the two match writers classifies the packed distance once. The class fits bits 9–13 of a match word and the original length fits bits 0–8. Header fields, carryover copies, capacity and match order retain their original behavior. The parser reads the current cost for the saved class. The symbolic check covers all 65,536 packed offsets and lengths 0 through 258.

The old slot table was still forwarded through the parser carrier. Its last selected use was removed. The numeric proof requires an operation witness for each retained array and declines this carrier. The next witness removes the unused forwarded argument with the same representation.

The independent CPU cohort counts 745,162,213.0 to 1,902,618,508.5 instructions and 191,050,772.0 to 411,771,418.0 cycles. The root accounts for 4,339 of 5,360 running samples in the candidate 256 KiB profile, which includes validation.

| Input | R327 / Roc | R331 / Roc | R327 / libdeflate | R331 / libdeflate | Primary elapsed change |
|---|---:|---:|---:|---:|---:|
| empty | 2.117540× | 2.035232× | 0.016629× | 0.015983× | +0.000% |
| one-stored | 2.017043× | 1.931421× | 0.017783× | 0.017029× | +0.000% |
| seven-stored-prefix | 2.223736× | 2.143979× | 0.018443× | 0.017782× | +0.000% |
| eight-literals | 0.181381× | 0.182860× | 2.313789× | 2.332659× | +1.109% |
| reymont-prefix-32769 | 1.290746× | 2.780398× | 1.171282× | 2.523060× | +115.061% |
| heldout-window-32768 | 1.286902× | 2.770390× | 1.172176× | 2.523413× | +115.345% |
| prefix-natural-32767 | 1.289736× | 2.770177× | 1.172666× | 2.518726× | +115.412% |
| prefix-natural-32768 | 1.290217× | 2.775188× | 1.173608× | 2.524368× | +115.262% |
| prefix-natural-65537 | 1.303133× | 2.836815× | 1.094072× | 2.381707× | +118.002% |
| prefix-natural-131072 | 1.305241× | 2.829917× | 1.073014× | 2.326422× | +116.957% |
| prefix-natural-262144 | 1.309771× | 2.835909× | 1.059146× | 2.293257× | +116.675% |
| prefix-natural-1048576 | 1.310444× | 2.748342× | 1.059406× | 2.221850× | +109.836% |
| prefix-random-16384 | 1.076907× | 1.615706× | 2.910440× | 4.366592× | +51.230% |
| prefix-random-1048576 | 1.594326× | 2.337316× | 2.252565× | 3.302307× | +46.632% |
| prefix-periodic-1048576 | 0.709362× | 1.442147× | 1.204162× | 2.448086× | +103.672% |
| prefix-repeated-1048576 | 0.713287× | 1.385049× | 1.117852× | 2.170626× | +94.888% |

154,752 balanced timed calls. 162 original linked calls verify exact streams, independent inflation, capacities, held aliases and complete cleanup. The timing screen determines the disposition.

[Verified patch, linked machine code and pilot observations](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r331-produced-match-class-51b9f4235f87.tar.gz). SHA-256 51b9f4235f87814687b607e1c60ee330c69087cea6eb2589e6690e93d5d073ed.
