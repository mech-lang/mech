# Current compression scoreboard

**Retained experimental compiler: R249. Large-input throughput remains the main problem.** All 81 configurations are freshly timed in two matched cohorts. The original 25 retain 11 below-reference cases; the 56 prefixes have six. These groups include duplicate inputs. The largest current winning size is 4 KiB for periodic/repeated data; natural text and random data are slower at 4 KiB.

The one-file proof extension improves periodic33 from 1.577853x to 1.535339x Roc (-2.6944% balanced elapsed). Eight literals stays 0.493774x. Both cohorts pass all 81 unchanged point/block 1.05 gates. Eight pooled block means regress; all appear below. Requested/copy/zero/peak/private and listed whole-heap medians are unchanged. R247’s natural-text component remains separately archived.

At 1 MiB: random **3.247617x**, text **2.241704x**, periodic **1.774457x**, repeated **1.658908x**. The next target is repeated work per byte in the large inputs. Small cases remain regression checks.

Complete fixed-level12 native execution, runtime input and matched live output ownership. Ordinary IR215 and the standalone extra-contract omissions remain unchanged. Ratios use pooled observation medians; changes use balanced block means. Near-resolution 0/1/7-byte timings remain visible.

## All current below-reference configurations

| Group | Case | Bytes | R249 / Roc |
|---|---|---:|---:|
| original25 | eight-literals | 8 | **0.493774x** |
| original25 | eight-overlap | 8 | **0.489278x** |
| original25 | 320-overlap | 320 | **0.544692x** |
| original25 | eight-literals-prefix | 8 | **0.490593x** |
| original25 | 128-biased-dynamic | 128 | **0.500930x** |
| original25 | 32-high-entropy-stored | 32 | **0.552957x** |
| original25 | random-64 | 64 | **0.576308x** |
| original25 | random-256 | 256 | **0.668728x** |
| original25 | random-1024 | 1024 | **0.750507x** |
| original25 | repeated-1024 | 1024 | **0.641626x** |
| original25 | reymont-prefix-1024 | 1024 | **0.921924x** |
| prefix56 | prefix-natural-1024 | 1024 | **0.924036x** |
| prefix56 | prefix-random-1024 | 1024 | **0.744041x** |
| prefix56 | prefix-periodic-1024 | 1024 | **0.653021x** |
| prefix56 | prefix-periodic-4096 | 4096 | **0.977414x** |
| prefix56 | prefix-repeated-1024 | 1024 | **0.632393x** |
| prefix56 | prefix-repeated-4096 | 4096 | **0.950202x** |

## Every configuration and candidate regression

All rows use the same fresh R249 matched reference. R245 is remeasured in that same cohort. Historical reports preserve their original observations; changed denominators do not imply a compiler change.

| Case | Bytes | R245 / Roc | R249 / Roc | Block elapsed change | Instruction change |
|---|---:|---:|---:|---:|---:|
| empty | 0 | 1.976190x | 1.976190x | -2.5172% | +0.0000% |
| one-stored | 1 | 2.000000x | 1.976190x | -3.1356% | +0.0000% |
| seven-stored-prefix | 7 | 2.976190x | 2.000000x | -7.1451% | +0.0000% |
| eight-literals | 8 | 0.492201x | 0.493774x | -0.0935% | -0.2207% |
| eight-overlap | 8 | 0.485270x | 0.489278x | +0.3961% | -0.2180% |
| 320-overlap | 320 | 0.545932x | 0.544692x | -0.4269% | -0.5574% |
| eight-literals-prefix | 8 | 0.493442x | 0.490593x | -0.7995% | -0.2207% |
| 128-biased-dynamic | 128 | 0.504075x | 0.500930x | -0.8093% | -0.2281% |
| 32-high-entropy-stored | 32 | 0.547541x | 0.552957x | +1.9767% | -0.2198% |
| 16384-distribution-change | 16384 | 2.754320x | 2.730085x | -0.8799% | -0.4091% |
| 33027-periodic-window | 33027 | 1.577853x | 1.535339x | -2.6944% | -1.7640% |
| random-64 | 64 | 0.577605x | 0.576308x | -0.7488% | -0.2139% |
| random-256 | 256 | 0.673010x | 0.668728x | -0.6146% | -0.1933% |
| random-1024 | 1024 | 0.748612x | 0.750507x | +0.3190% | -0.1820% |
| repeated-1024 | 1024 | 0.649621x | 0.641626x | -0.7655% | -0.9239% |
| random-32769 | 32769 | 2.704745x | 2.692052x | -0.4693% | -0.0137% |
| repeated-32769 | 32769 | 1.488572x | 1.453857x | -2.3321% | -1.7688% |
| reymont-prefix-1024 | 1024 | 0.928110x | 0.921924x | -0.2470% | -0.2105% |
| reymont-prefix-16384 | 16384 | 2.188753x | 2.185085x | -0.1676% | -0.0635% |
| reymont-prefix-32769 | 32769 | 2.216387x | 2.211765x | -0.2086% | -0.0449% |
| heldout-window-32767 | 32767 | 2.227906x | 2.220523x | -0.3314% | -0.0477% |
| heldout-window-32768 | 32768 | 2.234075x | 2.224079x | -0.4474% | -0.0508% |
| heldout-slides-65537 | 65537 | 2.302126x | 2.299668x | -0.1068% | -0.0299% |
| heldout-random-65537 | 65537 | 2.972625x | 2.964952x | -0.2581% | -0.0058% |
| heldout-mixed-131073 | 131073 | 2.455329x | 2.447567x | -0.3162% | -0.4070% |
| prefix-natural-1024 | 1024 | 0.927311x | 0.924036x | -0.7725% | -0.2087% |
| prefix-natural-4096 | 4096 | 1.719200x | 1.707442x | -0.6839% | -0.0780% |
| prefix-natural-8192 | 8192 | 2.003134x | 1.993862x | -0.4628% | -0.0652% |
| prefix-natural-16384 | 16384 | 2.188705x | 2.181989x | -0.3068% | -0.0658% |
| prefix-natural-32767 | 32767 | 2.229389x | 2.223846x | -0.2486% | -0.0453% |
| prefix-natural-32768 | 32768 | 2.231282x | 2.223553x | -0.3464% | -0.0513% |
| prefix-natural-32769 | 32769 | 2.227214x | 2.224159x | -0.1372% | -0.0483% |
| prefix-natural-33027 | 33027 | 2.225711x | 2.219863x | -0.2627% | -0.0447% |
| prefix-natural-65535 | 65535 | 2.208166x | 2.205831x | -0.1057% | -0.0268% |
| prefix-natural-65536 | 65536 | 2.213786x | 2.206898x | -0.3111% | -0.0269% |
| prefix-natural-65537 | 65537 | 2.241840x | 2.233469x | -0.3734% | -0.0262% |
| prefix-natural-131072 | 131072 | 2.208308x | 2.204033x | -0.1936% | -0.0258% |
| prefix-natural-262144 | 262144 | 2.215547x | 2.205271x | -0.4638% | -0.0245% |
| prefix-natural-1048576 | 1048576 | 2.250232x | 2.241704x | -0.3790% | -0.0263% |
| prefix-random-1024 | 1024 | 0.742712x | 0.744041x | +0.2552% | -0.1897% |
| prefix-random-4096 | 4096 | 1.405076x | 1.368862x | -2.5774% | -0.0711% |
| prefix-random-8192 | 8192 | 1.774691x | 1.767847x | -0.3856% | -0.0412% |
| prefix-random-16384 | 16384 | 2.311070x | 2.285589x | -1.1025% | -0.0273% |
| prefix-random-32767 | 32767 | 2.722672x | 2.715118x | -0.2774% | -0.0137% |
| prefix-random-32768 | 32768 | 2.730308x | 2.712286x | -0.6601% | -0.0137% |
| prefix-random-32769 | 32769 | 2.701782x | 2.696490x | -0.1959% | -0.0137% |
| prefix-random-33027 | 33027 | 2.735300x | 2.726038x | -0.3386% | -0.0133% |
| prefix-random-65535 | 65535 | 2.989225x | 2.989320x | +0.0032% | -0.0083% |
| prefix-random-65536 | 65536 | 2.981042x | 2.982394x | +0.0454% | -0.0067% |
| prefix-random-65537 | 65537 | 2.961203x | 2.962107x | +0.0305% | -0.0048% |
| prefix-random-131072 | 131072 | 3.115530x | 3.120069x | +0.1457% | -0.0074% |
| prefix-random-262144 | 262144 | 3.169205x | 3.165091x | -0.1298% | -0.0025% |
| prefix-random-1048576 | 1048576 | 3.251823x | 3.247617x | -0.1294% | -0.0034% |
| prefix-periodic-1024 | 1024 | 0.662653x | 0.653021x | -2.3122% | -0.8776% |
| prefix-periodic-4096 | 4096 | 1.014317x | 0.977414x | -3.6382% | -1.3638% |
| prefix-periodic-8192 | 8192 | 1.201361x | 1.179501x | -1.8196% | -1.5102% |
| prefix-periodic-16384 | 16384 | 1.395559x | 1.353868x | -2.9874% | -1.7201% |
| prefix-periodic-32767 | 32767 | 1.555681x | 1.517337x | -2.4648% | -1.7745% |
| prefix-periodic-32768 | 32768 | 1.564099x | 1.529397x | -2.2186% | -1.7743% |
| prefix-periodic-32769 | 32769 | 1.567761x | 1.529268x | -2.4553% | -1.7635% |
| prefix-periodic-33027 | 33027 | 1.557464x | 1.526919x | -1.9612% | -1.7640% |
| prefix-periodic-65535 | 65535 | 1.680979x | 1.646864x | -2.0295% | -1.7972% |
| prefix-periodic-65536 | 65536 | 1.684256x | 1.647663x | -2.1727% | -1.7972% |
| prefix-periodic-65537 | 65537 | 1.680116x | 1.646418x | -2.0057% | -1.7916% |
| prefix-periodic-131072 | 131072 | 1.745373x | 1.712867x | -1.8624% | -1.8064% |
| prefix-periodic-262144 | 262144 | 1.780883x | 1.746732x | -1.9176% | -1.8155% |
| prefix-periodic-1048576 | 1048576 | 1.807059x | 1.774457x | -1.8041% | -1.8163% |
| prefix-repeated-1024 | 1024 | 0.640699x | 0.632393x | -0.7237% | -0.9241% |
| prefix-repeated-4096 | 4096 | 0.975666x | 0.950202x | -2.6100% | -1.3883% |
| prefix-repeated-8192 | 8192 | 1.155618x | 1.136148x | -1.6849% | -1.5250% |
| prefix-repeated-16384 | 16384 | 1.318529x | 1.292547x | -1.9705% | -1.7299% |
| prefix-repeated-32767 | 32767 | 1.484775x | 1.460915x | -1.6069% | -1.7798% |
| prefix-repeated-32768 | 32768 | 1.476909x | 1.445569x | -2.1220% | -1.7799% |
| prefix-repeated-32769 | 32769 | 1.486576x | 1.455550x | -2.0870% | -1.7689% |
| prefix-repeated-33027 | 33027 | 1.489947x | 1.462391x | -1.8494% | -1.7694% |
| prefix-repeated-65535 | 65535 | 1.590879x | 1.559376x | -1.9802% | -1.8001% |
| prefix-repeated-65536 | 65536 | 1.590914x | 1.562201x | -1.8048% | -1.8001% |
| prefix-repeated-65537 | 65537 | 1.590579x | 1.558479x | -2.0182% | -1.7945% |
| prefix-repeated-131072 | 131072 | 1.653954x | 1.624397x | -1.7870% | -1.8108% |
| prefix-repeated-262144 | 262144 | 1.683889x | 1.651828x | -1.9040% | -1.8163% |
| prefix-repeated-1048576 | 1048576 | 1.690971x | 1.658908x | -1.8962% | -1.8242% |

[Exact times, both cohorts, every regression, instructions/cycles/memory, current profiles, execution counts, qualification and public evidence](native-private-read-intervals.md). [Prior work/scaling ledger](native-work-ledger-prefix-scaling.md). [Preserved phase-only component](native-phase-only-loop-threading.md). [R245 control](native-vector-phase-specialization.md). Every round remains in benchmarks/mech-deflate/experiments.json.
