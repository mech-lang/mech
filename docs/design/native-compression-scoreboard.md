# Current compression scoreboard

**Retained compiler: R250, remeasured in the R252 cohort. The broad R252 variant is archived.** Large random/text throughput remains the priority. The original 25 retain 11 below-reference cases; the 56 prefixes have five, including duplicate inputs. The largest winning retained size is 4 KiB repeated.

The archived variant reduces root code 32.97% and improves eight/short repetitive cases, but increases both elapsed and instructions on the difficult large random/text cases. All 81 unchanged 1.05 gates pass in both cohorts; that does not justify retaining it against the current priority. Every candidate result, including improvements and regressions, remains below.

Current matched retained ratios: eight **0.508766x**, random 64 **2.960736x**; at 1 MiB text **2.237932x**, random **3.221988x**, periodic **1.779520x**, repeated **1.680464x**. These use a fresh shared reference denominator and the unchanged R250 binary. Historical R250/R251 ratios remain in their original reports.

Complete fixed-level 12 computation, runtime input, matched live borrowed output, full scratch construction/output transfer/private cleanup. CPU/heap cohorts are independent of primary timings. Ordinary IR215 and the experimental compact entry’s additional omitted contract remain unchanged. [R252 design, complete evidence and archive decision](native-identity-route-forwarding.md).

## Every retained below-reference configuration

| Group | Case | Bytes | Retained R250 / Roc |
|---|---|---:|---:|
| original 25 | eight-literals | 8 | **0.508766x** |
| original 25 | eight-overlap | 8 | **0.502017x** |
| original 25 | 320-overlap | 320 | **0.556349x** |
| original 25 | eight-literals-prefix | 8 | **0.503846x** |
| original 25 | 128-biased-dynamic | 128 | **0.510124x** |
| original 25 | 32-high-entropy-stored | 32 | **0.540998x** |
| original 25 | random-64 | 64 | **0.561477x** |
| original 25 | random-256 | 256 | **0.656791x** |
| original 25 | random-1024 | 1024 | **0.740323x** |
| original 25 | repeated-1024 | 1024 | **0.636514x** |
| original 25 | reymont-prefix-1024 | 1024 | **0.926660x** |
| prefix 56 | prefix-natural-1024 | 1024 | **0.925241x** |
| prefix 56 | prefix-random-1024 | 1024 | **0.754210x** |
| prefix 56 | prefix-periodic-1024 | 1024 | **0.672202x** |
| prefix 56 | prefix-repeated-1024 | 1024 | **0.661030x** |
| prefix 56 | prefix-repeated-4096 | 4096 | **0.977098x** |

## All 81 current measurements

Ratios use pooled observation medians from the same R252 reference. Changes are balanced block means against the concurrently remeasured R250 control. The right-hand candidate is archived, not the default. Near-resolution empty/one/seven-byte observations remain visible.

| Case | Bytes | Retained R250 / Roc | Archived R252 / Roc | Candidate elapsed change | Candidate instructions change |
|---|---:|---:|---:|---:|---:|
| empty | 0 | 1.976190x | 1.976190x | -7.3316% | -0.0634% |
| one-stored | 1 | 2.000000x | 1.976190x | -5.9946% | -0.2634% |
| seven-stored-prefix | 7 | 1.976190x | 2.000000x | +1.7290% | -0.2614% |
| eight-literals | 8 | 0.508766x | 0.493174x | -1.3536% | -1.3074% |
| eight-overlap | 8 | 0.502017x | 0.483795x | -4.5947% | -1.2794% |
| 320-overlap | 320 | 0.556349x | 0.545266x | -1.9148% | -1.0313% |
| eight-literals-prefix | 8 | 0.503846x | 0.493347x | -1.8768% | -1.3072% |
| 128-biased-dynamic | 128 | 0.510124x | 0.502946x | -1.2182% | -0.9027% |
| 32-high-entropy-stored | 32 | 0.540998x | 0.520356x | -3.1076% | -1.3670% |
| 16384-distribution-change | 16384 | 2.710542x | 2.660476x | -1.8471% | +0.7260% |
| 33027-periodic-window | 33027 | 1.546264x | 1.514774x | -2.0365% | -0.4582% |
| random-64 | 64 | 0.561477x | 0.543710x | -2.7018% | -1.3978% |
| random-256 | 256 | 0.656791x | 0.633416x | -3.5603% | -1.0761% |
| random-1024 | 1024 | 0.740323x | 0.744087x | +0.3049% | +0.7346% |
| repeated-1024 | 1024 | 0.636514x | 0.632211x | -1.1198% | -0.7696% |
| random-32769 | 32769 | 2.681532x | 2.720398x | +1.4494% | +2.6743% |
| repeated-32769 | 32769 | 1.468270x | 1.442728x | -1.7395% | -0.4498% |
| reymont-prefix-1024 | 1024 | 0.926660x | 0.923020x | -0.2713% | +0.1244% |
| reymont-prefix-16384 | 16384 | 2.183037x | 2.185118x | +0.0953% | +0.6964% |
| reymont-prefix-32769 | 32769 | 2.222817x | 2.224440x | +0.0730% | +0.7439% |
| heldout-window-32767 | 32767 | 2.223614x | 2.221926x | -0.0759% | +0.7415% |
| heldout-window-32768 | 32768 | 2.228023x | 2.227961x | -0.0028% | +0.7412% |
| heldout-slides-65537 | 65537 | 2.299596x | 2.287167x | -0.5405% | +0.5998% |
| heldout-random-65537 | 65537 | 2.960736x | 3.008642x | +1.6181% | +2.6840% |
| heldout-mixed-131073 | 131073 | 2.441508x | 2.442603x | +0.0449% | +0.5811% |
| prefix-natural-1024 | 1024 | 0.925241x | 0.919934x | -0.5845% | +0.1229% |
| prefix-natural-4096 | 4096 | 1.709284x | 1.676193x | -1.9360% | +0.6045% |
| prefix-natural-8192 | 8192 | 1.995019x | 1.970393x | -1.2344% | +0.6701% |
| prefix-natural-16384 | 16384 | 2.185602x | 2.177246x | -0.3823% | +0.7033% |
| prefix-natural-32767 | 32767 | 2.232872x | 2.231034x | -0.0823% | +0.7449% |
| prefix-natural-32768 | 32768 | 2.238170x | 2.230699x | -0.3338% | +0.7446% |
| prefix-natural-32769 | 32769 | 2.220182x | 2.221197x | +0.0457% | +0.7445% |
| prefix-natural-33027 | 33027 | 2.218018x | 2.216092x | -0.0869% | +0.7432% |
| prefix-natural-65535 | 65535 | 2.196571x | 2.205958x | +0.4274% | +0.7508% |
| prefix-natural-65536 | 65536 | 2.197010x | 2.207424x | +0.4740% | +0.7506% |
| prefix-natural-65537 | 65537 | 2.236179x | 2.240498x | +0.1932% | +0.6636% |
| prefix-natural-131072 | 131072 | 2.198196x | 2.221252x | +1.0489% | +0.8204% |
| prefix-natural-262144 | 262144 | 2.201946x | 2.213402x | +0.5203% | +0.6806% |
| prefix-natural-1048576 | 1048576 | 2.237932x | 2.251347x | +0.5995% | +0.7189% |
| prefix-random-1024 | 1024 | 0.754210x | 0.743429x | -1.1166% | +0.7471% |
| prefix-random-4096 | 4096 | 1.400565x | 1.326690x | -5.2747% | +1.9102% |
| prefix-random-8192 | 8192 | 1.755160x | 1.751713x | -0.1964% | +2.5106% |
| prefix-random-16384 | 16384 | 2.306531x | 2.314051x | +0.3260% | +2.6784% |
| prefix-random-32767 | 32767 | 2.713510x | 2.739740x | +0.9666% | +2.6840% |
| prefix-random-32768 | 32768 | 2.701142x | 2.736192x | +1.2976% | +2.6908% |
| prefix-random-32769 | 32769 | 2.694064x | 2.732786x | +1.4373% | +2.6838% |
| prefix-random-33027 | 33027 | 2.707628x | 2.728205x | +0.7600% | +2.6770% |
| prefix-random-65535 | 65535 | 2.961548x | 3.024576x | +2.1282% | +2.6098% |
| prefix-random-65536 | 65536 | 2.960115x | 3.013432x | +1.8012% | +2.6087% |
| prefix-random-65537 | 65537 | 2.961248x | 3.015735x | +1.8400% | +2.6041% |
| prefix-random-131072 | 131072 | 3.092855x | 3.206768x | +3.6831% | +2.7744% |
| prefix-random-262144 | 262144 | 3.152504x | 3.213137x | +1.9233% | +2.4869% |
| prefix-random-1048576 | 1048576 | 3.221988x | 3.293046x | +2.2054% | +2.5057% |
| prefix-periodic-1024 | 1024 | 0.672202x | 0.656998x | -2.6211% | -0.7935% |
| prefix-periodic-4096 | 4096 | 1.039828x | 0.985353x | -5.2388% | -0.5488% |
| prefix-periodic-8192 | 8192 | 1.178681x | 1.157408x | -1.8048% | -0.5012% |
| prefix-periodic-16384 | 16384 | 1.378267x | 1.352093x | -1.8990% | -0.4957% |
| prefix-periodic-32767 | 32767 | 1.542416x | 1.526270x | -1.0468% | -0.4639% |
| prefix-periodic-32768 | 32768 | 1.538564x | 1.516256x | -1.4500% | -0.4627% |
| prefix-periodic-32769 | 32769 | 1.540062x | 1.510660x | -1.9092% | -0.4672% |
| prefix-periodic-33027 | 33027 | 1.540711x | 1.510597x | -1.9546% | -0.4592% |
| prefix-periodic-65535 | 65535 | 1.655958x | 1.633212x | -1.3736% | -0.4475% |
| prefix-periodic-65536 | 65536 | 1.656272x | 1.636949x | -1.1667% | -0.4517% |
| prefix-periodic-65537 | 65537 | 1.653918x | 1.631818x | -1.3362% | -0.4468% |
| prefix-periodic-131072 | 131072 | 1.719799x | 1.701982x | -1.0360% | -0.4389% |
| prefix-periodic-262144 | 262144 | 1.749992x | 1.788525x | +2.2019% | -0.0715% |
| prefix-periodic-1048576 | 1048576 | 1.779520x | 1.786825x | +0.4105% | -0.3315% |
| prefix-repeated-1024 | 1024 | 0.661030x | 0.647771x | -3.1616% | -0.7623% |
| prefix-repeated-4096 | 4096 | 0.977098x | 0.926900x | -5.1375% | -0.5301% |
| prefix-repeated-8192 | 8192 | 1.147509x | 1.113228x | -2.9875% | -0.4737% |
| prefix-repeated-16384 | 16384 | 1.318874x | 1.295250x | -1.7913% | -0.4788% |
| prefix-repeated-32767 | 32767 | 1.472379x | 1.449311x | -1.5667% | -0.4593% |
| prefix-repeated-32768 | 32768 | 1.468606x | 1.454764x | -0.9425% | -0.4600% |
| prefix-repeated-32769 | 32769 | 1.488649x | 1.453128x | -2.3861% | -0.4537% |
| prefix-repeated-33027 | 33027 | 1.475066x | 1.441852x | -2.2517% | -0.4519% |
| prefix-repeated-65535 | 65535 | 1.556018x | 1.525856x | -1.9384% | -0.4460% |
| prefix-repeated-65536 | 65536 | 1.573924x | 1.553098x | -1.3232% | -0.4498% |
| prefix-repeated-65537 | 65537 | 1.615177x | 1.572438x | -2.6461% | -0.4459% |
| prefix-repeated-131072 | 131072 | 1.616565x | 1.605348x | -0.6939% | -0.4382% |
| prefix-repeated-262144 | 262144 | 1.652213x | 1.692036x | +2.4102% | +0.0552% |
| prefix-repeated-1048576 | 1048576 | 1.680464x | 1.692097x | +0.6922% | -0.3305% |
