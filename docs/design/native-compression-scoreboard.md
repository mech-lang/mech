# Current compression scoreboard

**Retained experimental compiler: R253.** All 81 balanced elapsed medians and independent instruction/cycle medians improve over the matched R250 control. Every unchanged 1.05 gate passes in both confirmation cohorts, and eight bytes remains below its 1x ceiling. The original 25 retain 11 below-reference configurations; the 56 prefixes have six, including duplicate inputs. The largest winning size remains 4 KiB, now periodic and repeated.

The generic current-SSA predicate proof eliminates impossible Boolean branches and their cleanup paths. Root code shrinks 5,623,612 -> 940,264 bytes; the frame shrinks 4,576 B. All 136 existing routines and seven vector bodies stay identical. Additional cleanup alone restores the entire baseline executable, confirming the new proof is required.

Current matched ratios: eight **0.430586x**, random 64 KiB **2.797023x**; at 1 MiB text **2.174189x**, random **3.083160x**, periodic **1.690930x**, repeated **1.596171x**. The larger-input gap remains. Historical ratios keep their original denominators; both columns below share the fresh R253 denominator.

Complete level-12 computation, runtime input, matched live borrowed output and full scratch construction/output transfer/private cleanup. CPU/heap cohorts remain independent of elapsed. Ordinary IR215 and the compact entry’s additional omitted managed contract remain unchanged. [Design, source, current profile and every validation receipt](native-current-predicates.md).

## Every current below-reference configuration

| Group | Case | Bytes | R253 / Roc |
|---|---|---:|---:|
| original25 | eight-literals | 8 | **0.430586x** |
| original25 | eight-overlap | 8 | **0.428488x** |
| original25 | 320-overlap | 320 | **0.476647x** |
| original25 | eight-literals-prefix | 8 | **0.430005x** |
| original25 | 128-biased-dynamic | 128 | **0.436179x** |
| original25 | 32-high-entropy-stored | 32 | **0.450353x** |
| original25 | random-64 | 64 | **0.478032x** |
| original25 | random-256 | 256 | **0.554668x** |
| original25 | random-1024 | 1024 | **0.656141x** |
| original25 | repeated-1024 | 1024 | **0.569490x** |
| original25 | reymont-prefix-1024 | 1024 | **0.807463x** |
| prefix56 | prefix-natural-1024 | 1024 | **0.804925x** |
| prefix56 | prefix-random-1024 | 1024 | **0.658209x** |
| prefix56 | prefix-periodic-1024 | 1024 | **0.584372x** |
| prefix56 | prefix-periodic-4096 | 4096 | **0.896756x** |
| prefix56 | prefix-repeated-1024 | 1024 | **0.566308x** |
| prefix56 | prefix-repeated-4096 | 4096 | **0.858050x** |

## All 81 matched measurements

Ratios use pooled observation medians; changes use balanced block means. Positive changes would be regressions. All individual observations, including near-resolution empty/one/seven-byte cases, remain in the public packet.

| Case | Bytes | R250 / Roc | Retained R253 / Roc | Elapsed change | Instructions change |
|---|---:|---:|---:|---:|---:|
| empty | 0 | 1.976190x | 1.976190x | -13.0590% | -0.9192% |
| one-stored | 1 | 2.000000x | 1.976190x | -23.8669% | -1.2862% |
| seven-stored-prefix | 7 | 1.976190x | 1.976190x | -18.8236% | -1.2761% |
| eight-literals | 8 | 0.496470x | 0.430586x | -12.4937% | -4.3235% |
| eight-overlap | 8 | 0.490952x | 0.428488x | -13.0673% | -4.2322% |
| 320-overlap | 320 | 0.544897x | 0.476647x | -12.6865% | -4.1314% |
| eight-literals-prefix | 8 | 0.487295x | 0.430005x | -11.0230% | -4.3234% |
| 128-biased-dynamic | 128 | 0.498919x | 0.436179x | -12.5528% | -4.5889% |
| 32-high-entropy-stored | 32 | 0.523856x | 0.450353x | -14.7901% | -5.1060% |
| 16384-distribution-change | 16384 | 2.688227x | 2.323779x | -13.5572% | -4.9444% |
| 33027-periodic-window | 33027 | 1.534159x | 1.424118x | -7.1727% | -4.1944% |
| random-64 | 64 | 0.560038x | 0.478032x | -14.9124% | -5.4364% |
| random-256 | 256 | 0.654488x | 0.554668x | -14.9916% | -6.1342% |
| random-1024 | 1024 | 0.737837x | 0.656141x | -11.1018% | -5.3625% |
| repeated-1024 | 1024 | 0.634346x | 0.569490x | -10.6173% | -3.8866% |
| random-32769 | 32769 | 2.668885x | 2.518187x | -5.6465% | -4.9941% |
| repeated-32769 | 32769 | 1.457184x | 1.362171x | -6.5203% | -4.1784% |
| reymont-prefix-1024 | 1024 | 0.915964x | 0.807463x | -12.0746% | -4.2161% |
| reymont-prefix-16384 | 16384 | 2.178044x | 2.060267x | -5.4075% | -2.1837% |
| reymont-prefix-32769 | 32769 | 2.220723x | 2.130488x | -4.0633% | -1.9323% |
| heldout-window-32767 | 32767 | 2.227375x | 2.132613x | -4.2544% | -1.9363% |
| heldout-window-32768 | 32768 | 2.227672x | 2.135918x | -4.1188% | -1.9372% |
| heldout-slides-65537 | 65537 | 2.293668x | 2.205655x | -3.8372% | -1.8154% |
| heldout-random-65537 | 65537 | 2.948469x | 2.797023x | -5.1365% | -5.0350% |
| heldout-mixed-131073 | 131073 | 2.442930x | 2.270648x | -7.0523% | -3.3384% |
| prefix-natural-1024 | 1024 | 0.915214x | 0.804925x | -11.9433% | -4.2162% |
| prefix-natural-4096 | 4096 | 1.720220x | 1.569873x | -8.7400% | -2.5199% |
| prefix-natural-8192 | 8192 | 2.003041x | 1.856638x | -7.3090% | -2.2761% |
| prefix-natural-16384 | 16384 | 2.190738x | 2.056135x | -6.1442% | -2.1836% |
| prefix-natural-32767 | 32767 | 2.231778x | 2.134866x | -4.3424% | -1.9358% |
| prefix-natural-32768 | 32768 | 2.229170x | 2.131487x | -4.3820% | -1.9351% |
| prefix-natural-32769 | 32769 | 2.228753x | 2.131146x | -4.3795% | -1.9359% |
| prefix-natural-33027 | 33027 | 2.225623x | 2.130753x | -4.2626% | -1.9312% |
| prefix-natural-65535 | 65535 | 2.201721x | 2.123837x | -3.5374% | -1.5582% |
| prefix-natural-65536 | 65536 | 2.201227x | 2.124178x | -3.5003% | -1.5578% |
| prefix-natural-65537 | 65537 | 2.243909x | 2.166866x | -3.4334% | -1.5511% |
| prefix-natural-131072 | 131072 | 2.201440x | 2.128889x | -3.2956% | -1.4612% |
| prefix-natural-262144 | 262144 | 2.205956x | 2.140922x | -2.9481% | -1.4108% |
| prefix-natural-1048576 | 1048576 | 2.240265x | 2.174189x | -2.9495% | -1.5313% |
| prefix-random-1024 | 1024 | 0.737984x | 0.658209x | -10.5867% | -5.3646% |
| prefix-random-4096 | 4096 | 1.358550x | 1.210273x | -10.9144% | -5.3357% |
| prefix-random-8192 | 8192 | 1.726901x | 1.564482x | -9.4052% | -5.2385% |
| prefix-random-16384 | 16384 | 2.281403x | 2.112230x | -7.4153% | -5.0863% |
| prefix-random-32767 | 32767 | 2.692352x | 2.529481x | -6.0494% | -5.0174% |
| prefix-random-32768 | 32768 | 2.707886x | 2.538700x | -6.2479% | -5.0173% |
| prefix-random-32769 | 32769 | 2.684537x | 2.523804x | -5.9874% | -4.9917% |
| prefix-random-33027 | 33027 | 2.702402x | 2.529977x | -6.3804% | -4.9910% |
| prefix-random-65535 | 65535 | 2.957766x | 2.797302x | -5.4252% | -4.9543% |
| prefix-random-65536 | 65536 | 2.962113x | 2.799693x | -5.4833% | -4.9509% |
| prefix-random-65537 | 65537 | 2.948459x | 2.792447x | -5.2913% | -4.9375% |
| prefix-random-131072 | 131072 | 3.104938x | 2.924923x | -5.7977% | -5.0019% |
| prefix-random-262144 | 262144 | 3.143963x | 3.000630x | -4.5590% | -4.9437% |
| prefix-random-1048576 | 1048576 | 3.221790x | 3.083160x | -4.3029% | -5.0483% |
| prefix-periodic-1024 | 1024 | 0.657210x | 0.584372x | -11.0898% | -4.1396% |
| prefix-periodic-4096 | 4096 | 1.018763x | 0.896756x | -11.9760% | -4.0072% |
| prefix-periodic-8192 | 8192 | 1.176310x | 1.077666x | -8.3859% | -3.9579% |
| prefix-periodic-16384 | 16384 | 1.370205x | 1.260670x | -7.9941% | -4.2239% |
| prefix-periodic-32767 | 32767 | 1.536621x | 1.430592x | -6.9001% | -4.2206% |
| prefix-periodic-32768 | 32768 | 1.532381x | 1.428118x | -6.8040% | -4.2206% |
| prefix-periodic-32769 | 32769 | 1.533775x | 1.428978x | -6.8326% | -4.1942% |
| prefix-periodic-33027 | 33027 | 1.531628x | 1.430057x | -6.6316% | -4.1944% |
| prefix-periodic-65535 | 65535 | 1.655486x | 1.555904x | -6.0153% | -4.2054% |
| prefix-periodic-65536 | 65536 | 1.652601x | 1.552138x | -6.0791% | -4.2054% |
| prefix-periodic-65537 | 65537 | 1.655574x | 1.553409x | -6.1710% | -4.1920% |
| prefix-periodic-131072 | 131072 | 1.715754x | 1.623045x | -5.4034% | -4.1946% |
| prefix-periodic-262144 | 262144 | 1.749652x | 1.662526x | -4.9796% | -4.1927% |
| prefix-periodic-1048576 | 1048576 | 1.776343x | 1.690930x | -4.8084% | -4.1889% |
| prefix-repeated-1024 | 1024 | 0.630400x | 0.566308x | -9.9666% | -3.8869% |
| prefix-repeated-4096 | 4096 | 0.970059x | 0.858050x | -11.5467% | -3.9190% |
| prefix-repeated-8192 | 8192 | 1.143431x | 1.048841x | -8.2725% | -3.9092% |
| prefix-repeated-16384 | 16384 | 1.305312x | 1.207345x | -7.5053% | -4.1950% |
| prefix-repeated-32767 | 32767 | 1.456847x | 1.373327x | -5.7329% | -4.2053% |
| prefix-repeated-32768 | 32768 | 1.463455x | 1.369818x | -6.3984% | -4.2049% |
| prefix-repeated-32769 | 32769 | 1.466358x | 1.369173x | -6.6277% | -4.1784% |
| prefix-repeated-33027 | 33027 | 1.464600x | 1.372711x | -6.2740% | -4.1786% |
| prefix-repeated-65535 | 65535 | 1.569091x | 1.482657x | -5.5086% | -4.1974% |
| prefix-repeated-65536 | 65536 | 1.571538x | 1.484218x | -5.5563% | -4.1974% |
| prefix-repeated-65537 | 65537 | 1.581430x | 1.486302x | -6.0153% | -4.1839% |
| prefix-repeated-131072 | 131072 | 1.631247x | 1.545218x | -5.2738% | -4.1949% |
| prefix-repeated-262144 | 262144 | 1.659798x | 1.573378x | -5.2066% | -4.1916% |
| prefix-repeated-1048576 | 1048576 | 1.633536x | 1.596171x | -2.2874% | -4.0864% |
