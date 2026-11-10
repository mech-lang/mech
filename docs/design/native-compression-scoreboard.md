# Current compression scoreboard

**Retained experimental compiler: R250. Large-input throughput remains the main problem.** All 81 configurations are freshly timed in two matched cohorts. The original 25 retain 11 below-reference cases; the 56 prefixes have five. These groups include duplicate inputs. The largest winning size in this cohort is 4 KiB for repeated data. Periodic 4 KiB is near parity at 1.001507x; its previous below-reference observation remains in R249 history.

Preserving exact non-growing view descriptors across native returns improves random 64 KiB from 2.970036x to 2.940361x Roc (0.9991% lower balanced elapsed), favorable in both cohorts. Eight literals remains 0.502581x. All 81 unchanged point/block 1.05 gates pass in both cohorts. Twenty-five pooled block means worsen; all appear below. Allocation/copy/zero/private-peak and listed whole-heap medians are unchanged.

At 1 MiB: random **3.217791x**, text **2.244741x**, periodic **1.774171x**, repeated **1.659128x**. Random improves 1.0462%; text, periodic and repeated regress 0.1384%, 0.0314% and 0.0662%. The next target is zero-trip identity-state transport on the large inputs; small cases remain regression checks.

Complete fixed-level-12 native execution, runtime input and matched live output ownership. Ordinary IR215 and the standalone extra-contract omissions remain unchanged. Ratios use pooled observation medians; elapsed changes use balanced block means. Near-resolution 0/1/7-byte timings remain visible.

R251 adds [exact zero-route and state-transport attribution](native-zero-trip-state.md) while restoring this complete normal image byte-for-byte. It adds no timing claim; all standings below remain the R250 measurements.

## All current below-reference configurations

| Group | Case | Bytes | R250 / Roc |
|---|---|---:|---:|
| original25 | eight-literals | 8 | **0.502581x** |
| original25 | eight-overlap | 8 | **0.500649x** |
| original25 | 320-overlap | 320 | **0.557766x** |
| original25 | eight-literals-prefix | 8 | **0.513124x** |
| original25 | 128-biased-dynamic | 128 | **0.505385x** |
| original25 | 32-high-entropy-stored | 32 | **0.538958x** |
| original25 | random-64 | 64 | **0.577381x** |
| original25 | random-256 | 256 | **0.687020x** |
| original25 | random-1024 | 1024 | **0.749493x** |
| original25 | repeated-1024 | 1024 | **0.637812x** |
| original25 | reymont-prefix-1024 | 1024 | **0.924184x** |
| prefix56 | prefix-natural-1024 | 1024 | **0.926059x** |
| prefix56 | prefix-random-1024 | 1024 | **0.745358x** |
| prefix56 | prefix-periodic-1024 | 1024 | **0.664719x** |
| prefix56 | prefix-repeated-1024 | 1024 | **0.638578x** |
| prefix56 | prefix-repeated-4096 | 4096 | **0.969152x** |

## Every configuration and candidate regression

All rows use the same fresh R250 matched reference. R249 is remeasured in that same cohort. Historical reports preserve their original observations; changing the reference denominator does not imply a compiler change.

| Case | Bytes | R249 / Roc | R250 / Roc | Block elapsed change | Instruction change |
|---|---:|---:|---:|---:|---:|
| empty | 0 | 1.976190x | 1.976190x | +2.0378% | -1.4832% |
| one-stored | 1 | 1.976190x | 2.000000x | +3.2157% | -1.4508% |
| seven-stored-prefix | 7 | 2.000000x | 2.000000x | -6.6988% | -1.4396% |
| eight-literals | 8 | 0.504949x | 0.502581x | -1.0791% | -0.6382% |
| eight-overlap | 8 | 0.500217x | 0.500649x | -0.0625% | -0.6540% |
| 320-overlap | 320 | 0.565347x | 0.557766x | -1.4170% | -0.5222% |
| eight-literals-prefix | 8 | 0.512908x | 0.513124x | -1.2899% | -0.6364% |
| 128-biased-dynamic | 128 | 0.512132x | 0.505385x | -0.2762% | -0.5848% |
| 32-high-entropy-stored | 32 | 0.539100x | 0.538958x | +0.3384% | -0.5050% |
| 16384-distribution-change | 16384 | 2.720206x | 2.699221x | -0.7714% | -0.0703% |
| 33027-periodic-window | 33027 | 1.538560x | 1.530851x | -0.5010% | -0.1904% |
| random-64 | 64 | 0.574300x | 0.577381x | +0.6957% | -0.4368% |
| random-256 | 256 | 0.672940x | 0.687020x | +1.5344% | -0.2704% |
| random-1024 | 1024 | 0.746043x | 0.749493x | +0.2746% | -0.2226% |
| repeated-1024 | 1024 | 0.634529x | 0.637812x | +0.7683% | -0.3781% |
| random-32769 | 32769 | 2.710992x | 2.671629x | -1.4520% | -0.3736% |
| repeated-32769 | 32769 | 1.462406x | 1.462314x | -0.0063% | -0.1926% |
| reymont-prefix-1024 | 1024 | 0.925415x | 0.924184x | -0.5133% | -0.3333% |
| reymont-prefix-16384 | 16384 | 2.189679x | 2.186600x | -0.1406% | -0.2421% |
| reymont-prefix-32769 | 32769 | 2.224810x | 2.224029x | -0.0351% | -0.2392% |
| heldout-window-32767 | 32767 | 2.226804x | 2.222766x | -0.1813% | -0.2359% |
| heldout-window-32768 | 32768 | 2.228070x | 2.230066x | +0.0896% | -0.2384% |
| heldout-slides-65537 | 65537 | 2.296276x | 2.299819x | +0.1543% | -0.2370% |
| heldout-random-65537 | 65537 | 2.970036x | 2.940361x | -0.9991% | -0.4332% |
| heldout-mixed-131073 | 131073 | 2.448860x | 2.441313x | -0.3082% | -0.1664% |
| prefix-natural-1024 | 1024 | 0.931719x | 0.926059x | -0.6201% | -0.3347% |
| prefix-natural-4096 | 4096 | 1.722769x | 1.703354x | -1.1270% | -0.2687% |
| prefix-natural-8192 | 8192 | 1.997591x | 1.997480x | -0.0055% | -0.2409% |
| prefix-natural-16384 | 16384 | 2.185716x | 2.180219x | -0.2515% | -0.2420% |
| prefix-natural-32767 | 32767 | 2.232155x | 2.230916x | -0.0555% | -0.2364% |
| prefix-natural-32768 | 32768 | 2.225039x | 2.222479x | -0.1151% | -0.2388% |
| prefix-natural-32769 | 32769 | 2.220557x | 2.221931x | +0.0619% | -0.2355% |
| prefix-natural-33027 | 33027 | 2.223841x | 2.219283x | -0.2050% | -0.2385% |
| prefix-natural-65535 | 65535 | 2.203302x | 2.206415x | +0.1413% | -0.2054% |
| prefix-natural-65536 | 65536 | 2.202691x | 2.208548x | +0.2659% | -0.2059% |
| prefix-natural-65537 | 65537 | 2.235467x | 2.234307x | -0.0519% | -0.2063% |
| prefix-natural-131072 | 131072 | 2.200669x | 2.205618x | +0.2249% | -0.2004% |
| prefix-natural-262144 | 262144 | 2.205837x | 2.211156x | +0.2412% | -0.1972% |
| prefix-natural-1048576 | 1048576 | 2.241639x | 2.244741x | +0.1384% | -0.2165% |
| prefix-random-1024 | 1024 | 0.742688x | 0.745358x | +0.3336% | -0.2173% |
| prefix-random-4096 | 4096 | 1.394145x | 1.359188x | -2.5075% | -0.2514% |
| prefix-random-8192 | 8192 | 1.797708x | 1.741983x | -3.0998% | -0.2826% |
| prefix-random-16384 | 16384 | 2.311120x | 2.259152x | -2.2486% | -0.3182% |
| prefix-random-32767 | 32767 | 2.732192x | 2.688414x | -1.6023% | -0.3751% |
| prefix-random-32768 | 32768 | 2.738365x | 2.698794x | -1.4450% | -0.3749% |
| prefix-random-32769 | 32769 | 2.713077x | 2.686229x | -0.9896% | -0.3731% |
| prefix-random-33027 | 33027 | 2.735747x | 2.691885x | -1.6033% | -0.3739% |
| prefix-random-65535 | 65535 | 2.984584x | 2.952039x | -1.0904% | -0.4240% |
| prefix-random-65536 | 65536 | 2.989730x | 2.953112x | -1.2248% | -0.4274% |
| prefix-random-65537 | 65537 | 2.983336x | 2.947082x | -1.2152% | -0.4231% |
| prefix-random-131072 | 131072 | 3.128898x | 3.094665x | -1.0941% | -0.4310% |
| prefix-random-262144 | 262144 | 3.180297x | 3.144221x | -1.1344% | -0.4551% |
| prefix-random-1048576 | 1048576 | 3.251810x | 3.217791x | -1.0462% | -0.5083% |
| prefix-periodic-1024 | 1024 | 0.661034x | 0.664719x | +0.1450% | -0.3478% |
| prefix-periodic-4096 | 4096 | 0.996553x | 1.001507x | +0.4971% | -0.2441% |
| prefix-periodic-8192 | 8192 | 1.176100x | 1.167607x | -0.7222% | -0.2093% |
| prefix-periodic-16384 | 16384 | 1.366129x | 1.360797x | -0.3903% | -0.2031% |
| prefix-periodic-32767 | 32767 | 1.527999x | 1.526427x | -0.1028% | -0.1917% |
| prefix-periodic-32768 | 32768 | 1.533288x | 1.528650x | -0.3025% | -0.1892% |
| prefix-periodic-32769 | 32769 | 1.535096x | 1.522379x | -0.8284% | -0.1903% |
| prefix-periodic-33027 | 33027 | 1.535069x | 1.529053x | -0.3919% | -0.1902% |
| prefix-periodic-65535 | 65535 | 1.652883x | 1.651325x | -0.0943% | -0.1849% |
| prefix-periodic-65536 | 65536 | 1.654237x | 1.650441x | -0.2294% | -0.1849% |
| prefix-periodic-65537 | 65537 | 1.647772x | 1.647594x | -0.0108% | -0.1866% |
| prefix-periodic-131072 | 131072 | 1.715809x | 1.713796x | -0.1173% | -0.1849% |
| prefix-periodic-262144 | 262144 | 1.745634x | 1.748351x | +0.1556% | -0.1802% |
| prefix-periodic-1048576 | 1048576 | 1.773614x | 1.774171x | +0.0314% | -0.1798% |
| prefix-repeated-1024 | 1024 | 0.634182x | 0.638578x | +0.6985% | -0.3797% |
| prefix-repeated-4096 | 4096 | 0.981718x | 0.969152x | -1.2800% | -0.2557% |
| prefix-repeated-8192 | 8192 | 1.141722x | 1.147352x | +0.4931% | -0.2158% |
| prefix-repeated-16384 | 16384 | 1.309819x | 1.307220x | -0.1984% | -0.2073% |
| prefix-repeated-32767 | 32767 | 1.463230x | 1.453551x | -0.6615% | -0.1938% |
| prefix-repeated-32768 | 32768 | 1.461309x | 1.451493x | -0.6718% | -0.1938% |
| prefix-repeated-32769 | 32769 | 1.466813x | 1.456783x | -0.6838% | -0.1926% |
| prefix-repeated-33027 | 33027 | 1.462116x | 1.460256x | -0.1272% | -0.1924% |
| prefix-repeated-65535 | 65535 | 1.567145x | 1.563789x | -0.2141% | -0.1862% |
| prefix-repeated-65536 | 65536 | 1.568104x | 1.564592x | -0.2239% | -0.1860% |
| prefix-repeated-65537 | 65537 | 1.563703x | 1.567036x | +0.2132% | -0.1855% |
| prefix-repeated-131072 | 131072 | 1.623119x | 1.622337x | -0.0482% | -0.1830% |
| prefix-repeated-262144 | 262144 | 1.654014x | 1.655311x | +0.0784% | -0.1799% |
| prefix-repeated-1048576 | 1048576 | 1.658031x | 1.659128x | +0.0662% | -0.1802% |

[Exact times, both cohorts, every regression, instructions/cycles/memory, current profiles, execution counts, qualification and public evidence](native-phase-view-identity.md). [R249 history](native-private-read-intervals.md). [Prior work/scaling ledger](native-work-ledger-prefix-scaling.md). [Preserved phase-only component](native-phase-only-loop-threading.md). Every round remains in benchmarks/mech-deflate/experiments.json.
