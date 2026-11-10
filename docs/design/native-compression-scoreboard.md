# Current compression scoreboard

**Retained experimental compiler: R256.** The compiler composes fixed-buffer nested computations into one guarded native phase and hands back exact current state before unsupported operations. The source/algorithm and ordinary funding guard stay unchanged. The all 81 descriptive geometric elapsed improvement is18.2622%; the larger controlled-prefix improvement is22.2123% against the freshly remeasured R255 parent.

All 81 pooled point/balanced1.05 gates pass. Primary A passes 81/81; Primary B passes 81/81 point and 78/81 balanced, with explicit misses for empty, one-stored and eight-literals-prefix. No allowances change. Their contrary observations remain in the report. Heap requests, initialized copying, zeroing and private peaks remain equal on all 81. Root code/frame and executable size increase; this is not a memory-size reduction.

There are22 measured below-reference configurations. The largest controlled wins are now16 KiB: periodic0.980901x, repeated0.949197x; random4 KiB0.947134x. At 32 KiB:text1.837344x, random1.828522x, periodic1.090658x, repeated1.048871x. At 1 MiB:text1.908252x, random2.170161x, periodic1.267271x, repeated1.222946x. Eight stays0.414364x. These use a fresh shared reference denominator; historical tables retain their original denominators.

[Compiler patch, exact qualification, profiles, limits and verified evidence](native-private-nested-phases.md). [R255 historical measurements](native-shared-comparison.md). Ordinary IR215 and the compact entry’s additional omitted managed contract remain unchanged. Large-input parity remains incomplete.

## R257 component and timing audit

**R256 remains retained.** The next packed-read geometry experiment is archived with all81 results, including text/random gains, repetitive-input instruction increases and the failed pooled empty timing gate. Code shrinks, but executed work does not improve uniformly. [Complete R257 component, qualification, profiles and all81 observations](native-entry-packed-geometry.md).

The supplemental paired audit finds two R257 sign reversals from pooling actor medians across heterogeneous cohorts. Repeated32KiB's pooled−2.1642% masks individual+0.5778%/+0.3198% slowdowns; comparing within the48 balanced blocks gives+0.3240%. The same audit confirms R256's18.2763% matrix improvement and finds no R256 reversals. Existing observations, pooled ratios and1.05 gates are preserved. The historical R256 table below remains unchanged; pooled medians are descriptive, not paired causal estimates.

## Every retained below-reference configuration

| Group | Case | Bytes | R256 / Roc |
|---|---|---:|---:|
| original25 | eight-literals | 8 | **0.414364x** |
| original25 | eight-overlap | 8 | **0.411438x** |
| original25 | 320-overlap | 320 | **0.447221x** |
| original25 | eight-literals-prefix | 8 | **0.411343x** |
| original25 | 128-biased-dynamic | 128 | **0.409098x** |
| original25 | 32-high-entropy-stored | 32 | **0.422530x** |
| original25 | random-64 | 64 | **0.437030x** |
| original25 | random-256 | 256 | **0.475533x** |
| original25 | random-1024 | 1024 | **0.538481x** |
| original25 | repeated-1024 | 1024 | **0.518242x** |
| original25 | reymont-prefix-1024 | 1024 | **0.669349x** |
| prefix56 | prefix-natural-1024 | 1024 | **0.675251x** |
| prefix56 | prefix-random-1024 | 1024 | **0.537245x** |
| prefix56 | prefix-random-4096 | 4096 | **0.947134x** |
| prefix56 | prefix-periodic-1024 | 1024 | **0.522303x** |
| prefix56 | prefix-periodic-4096 | 4096 | **0.765540x** |
| prefix56 | prefix-periodic-8192 | 8192 | **0.875809x** |
| prefix56 | prefix-periodic-16384 | 16384 | **0.980901x** |
| prefix56 | prefix-repeated-1024 | 1024 | **0.521466x** |
| prefix56 | prefix-repeated-4096 | 4096 | **0.741312x** |
| prefix56 | prefix-repeated-8192 | 8192 | **0.851834x** |
| prefix56 | prefix-repeated-16384 | 16384 | **0.949197x** |

## All 81 matched measurements

Ratios use pooled observation medians. Changes use balanced block means against matched R255. Positive changes are regressions. Instructions/cycles are from a separate cohort. All observations and the three individual Primary B gate misses remain recorded.

| Case | Bytes | R255 / Roc | Retained R256 / Roc | Elapsed change | Instructions change |
|---|---:|---:|---:|---:|---:|
| empty | 0 | 1.976190x | 1.976190x | +4.1367% | +0.3199% |
| one-stored | 1 | 1.976190x | 1.976190x | +3.4691% | +0.3297% |
| seven-stored-prefix | 7 | 1.976190x | 1.976190x | +1.5995% | +0.3271% |
| eight-literals | 8 | 0.427100x | 0.414364x | -3.4678% | -2.9654% |
| eight-overlap | 8 | 0.415224x | 0.411438x | -0.4414% | -2.7329% |
| 320-overlap | 320 | 0.471106x | 0.447221x | -3.7754% | -6.9207% |
| eight-literals-prefix | 8 | 0.419347x | 0.411343x | -1.0045% | -2.9685% |
| 128-biased-dynamic | 128 | 0.430983x | 0.409098x | -4.2641% | -5.8909% |
| 32-high-entropy-stored | 32 | 0.449914x | 0.422530x | -6.6809% | -10.2314% |
| 16384-distribution-change | 16384 | 2.324858x | 1.865948x | -19.7393% | -19.4188% |
| 33027-periodic-window | 33027 | 1.444307x | 1.101319x | -23.7476% | -20.9825% |
| random-64 | 64 | 0.466704x | 0.437030x | -6.7839% | -12.1854% |
| random-256 | 256 | 0.545797x | 0.475533x | -12.2065% | -18.3207% |
| random-1024 | 1024 | 0.647682x | 0.538481x | -15.6985% | -20.9159% |
| repeated-1024 | 1024 | 0.563010x | 0.518242x | -8.0336% | -10.9642% |
| random-32769 | 32769 | 2.523387x | 1.809078x | -28.3075% | -28.0738% |
| repeated-32769 | 32769 | 1.374344x | 1.054159x | -23.2973% | -21.0944% |
| reymont-prefix-1024 | 1024 | 0.802377x | 0.669349x | -16.4497% | -16.6329% |
| reymont-prefix-16384 | 16384 | 2.051386x | 1.757274x | -14.3372% | -15.3122% |
| reymont-prefix-32769 | 32769 | 2.128826x | 1.838233x | -13.6504% | -15.7320% |
| heldout-window-32767 | 32767 | 2.126821x | 1.833910x | -13.7722% | -15.7700% |
| heldout-window-32768 | 32768 | 2.128863x | 1.839295x | -13.6020% | -15.7709% |
| heldout-slides-65537 | 65537 | 2.213204x | 1.902064x | -14.0583% | -15.8723% |
| heldout-random-65537 | 65537 | 2.789476x | 1.975392x | -29.1841% | -30.0327% |
| heldout-mixed-131073 | 131073 | 2.274267x | 1.920398x | -15.5597% | -17.6464% |
| prefix-natural-1024 | 1024 | 0.805746x | 0.675251x | -16.3223% | -16.6298% |
| prefix-natural-4096 | 4096 | 1.558745x | 1.318161x | -15.4344% | -15.8698% |
| prefix-natural-8192 | 8192 | 1.856918x | 1.589342x | -14.4096% | -15.1039% |
| prefix-natural-16384 | 16384 | 2.050931x | 1.762051x | -14.0853% | -15.3119% |
| prefix-natural-32767 | 32767 | 2.131404x | 1.837118x | -13.8071% | -15.7724% |
| prefix-natural-32768 | 32768 | 2.135596x | 1.837344x | -13.9658% | -15.7661% |
| prefix-natural-32769 | 32769 | 2.130578x | 1.835871x | -13.8322% | -15.7383% |
| prefix-natural-33027 | 33027 | 2.132550x | 1.839407x | -13.7461% | -15.7381% |
| prefix-natural-65535 | 65535 | 2.127331x | 1.900385x | -10.6681% | -13.6003% |
| prefix-natural-65536 | 65536 | 2.133617x | 1.908106x | -10.5694% | -13.6004% |
| prefix-natural-65537 | 65537 | 2.125089x | 1.903007x | -10.4505% | -13.5776% |
| prefix-natural-131072 | 131072 | 2.137816x | 1.903422x | -10.9642% | -13.5444% |
| prefix-natural-262144 | 262144 | 2.143698x | 1.908282x | -10.9817% | -13.5030% |
| prefix-natural-1048576 | 1048576 | 2.182609x | 1.908252x | -12.5701% | -14.9740% |
| prefix-random-1024 | 1024 | 0.653959x | 0.537245x | -18.1326% | -21.0675% |
| prefix-random-4096 | 4096 | 1.211736x | 0.947134x | -21.8366% | -23.2881% |
| prefix-random-8192 | 8192 | 1.579874x | 1.166803x | -26.1458% | -25.3070% |
| prefix-random-16384 | 16384 | 2.119998x | 1.541744x | -27.2762% | -26.4495% |
| prefix-random-32767 | 32767 | 2.541201x | 1.826622x | -28.1197% | -28.2248% |
| prefix-random-32768 | 32768 | 2.541539x | 1.828522x | -28.0545% | -28.2249% |
| prefix-random-32769 | 32769 | 2.517614x | 1.813471x | -27.9686% | -28.0679% |
| prefix-random-33027 | 33027 | 2.530290x | 1.811509x | -28.4071% | -28.0961% |
| prefix-random-65535 | 65535 | 2.800912x | 2.000222x | -28.5868% | -29.3728% |
| prefix-random-65536 | 65536 | 2.799132x | 1.993951x | -28.7654% | -29.3728% |
| prefix-random-65537 | 65537 | 2.785346x | 1.994217x | -28.4033% | -29.2923% |
| prefix-random-131072 | 131072 | 2.929944x | 2.081702x | -28.9508% | -30.1548% |
| prefix-random-262144 | 262144 | 2.999684x | 2.188740x | -27.0343% | -29.9102% |
| prefix-random-1048576 | 1048576 | 3.069914x | 2.170161x | -29.3087% | -31.2393% |
| prefix-periodic-1024 | 1024 | 0.577033x | 0.522303x | -9.9486% | -12.0610% |
| prefix-periodic-4096 | 4096 | 0.899704x | 0.765540x | -14.9121% | -17.0162% |
| prefix-periodic-8192 | 8192 | 1.076883x | 0.875809x | -18.6718% | -18.3903% |
| prefix-periodic-16384 | 16384 | 1.267506x | 0.980901x | -22.6117% | -20.7949% |
| prefix-periodic-32767 | 32767 | 1.440622x | 1.092726x | -24.1491% | -21.3065% |
| prefix-periodic-32768 | 32768 | 1.439207x | 1.090658x | -24.2181% | -21.3068% |
| prefix-periodic-32769 | 32769 | 1.435808x | 1.088024x | -24.2222% | -21.1598% |
| prefix-periodic-33027 | 33027 | 1.439778x | 1.098919x | -23.6744% | -20.9820% |
| prefix-periodic-65535 | 65535 | 1.553309x | 1.168122x | -24.7978% | -21.4074% |
| prefix-periodic-65536 | 65536 | 1.554477x | 1.166292x | -24.9721% | -21.4072% |
| prefix-periodic-65537 | 65537 | 1.552722x | 1.166647x | -24.8645% | -21.3321% |
| prefix-periodic-131072 | 131072 | 1.622989x | 1.212178x | -25.3120% | -21.4600% |
| prefix-periodic-262144 | 262144 | 1.663151x | 1.240945x | -25.3859% | -21.5047% |
| prefix-periodic-1048576 | 1048576 | 1.691711x | 1.267271x | -25.0894% | -21.0589% |
| prefix-repeated-1024 | 1024 | 0.562111x | 0.521466x | -6.9248% | -10.9512% |
| prefix-repeated-4096 | 4096 | 0.875654x | 0.741312x | -15.3419% | -16.6166% |
| prefix-repeated-8192 | 8192 | 1.046276x | 0.851834x | -18.5842% | -18.1769% |
| prefix-repeated-16384 | 16384 | 1.214046x | 0.949197x | -21.8154% | -20.6697% |
| prefix-repeated-32767 | 32767 | 1.374672x | 1.053905x | -23.3340% | -21.2420% |
| prefix-repeated-32768 | 32768 | 1.372313x | 1.048871x | -23.5691% | -21.2408% |
| prefix-repeated-32769 | 32769 | 1.370309x | 1.047298x | -23.5721% | -21.0948% |
| prefix-repeated-33027 | 33027 | 1.384601x | 1.075042x | -22.3573% | -20.3153% |
| prefix-repeated-65535 | 65535 | 1.480816x | 1.136948x | -23.2215% | -20.8088% |
| prefix-repeated-65536 | 65536 | 1.479569x | 1.132539x | -23.4548% | -20.8089% |
| prefix-repeated-65537 | 65537 | 1.477570x | 1.130412x | -23.4952% | -20.7362% |
| prefix-repeated-131072 | 131072 | 1.544460x | 1.178993x | -23.6631% | -20.5967% |
| prefix-repeated-262144 | 262144 | 1.569101x | 1.225382x | -21.9055% | -20.2888% |
| prefix-repeated-1048576 | 1048576 | 1.583851x | 1.222946x | -22.7866% | -20.1630% |
