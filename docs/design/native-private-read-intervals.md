# R249: preserve private write permission across a checked scalar read

Retain R249. Periodic33 improves **1.577853x -> 1.535339x Roc**, with 2.7269% and 2.6317% lower elapsed in the two cohorts (2.6944% pooled). Eight literals stays **0.493774x Roc**. All81 unchanged point/block1.05 gates pass in both cohorts. Eight pooled block means worsen; they remain below. R247 phase-only threading and its natural-text gain remain separately archived, not combined.

Git parent `f7cf8c0704ba9b923001c5c8f5ca214b468f6d3f`; PR base `92f75db84e2523693648084bea16cd52ad60777a`; experimental source base R245 `085dce0ee59b2c798a9ed542c62273759414f0cc`. One file changes in the815-file GPU source closure. Ordinary R215 remains `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`, IR/work/ABI/MBC215/107/39/14. The complete source/patch/pins/build commands are in the evidence packet. The patch applies to archived R245, not directly to the maintained IR215 tree.

## Why this compiler change is valid

The pinned skip operation writes two U16 hash heads, reads a scalar from the same current table, then writes the third head. R248 counts32854 skip positions at33027 bytes and1044466 at1MiB. The existing short private-write proof stopped at that read even though it creates no alias. This is repeated compiler work, not extra matching work.

With `MECH_PRIVATE_READ_INTERVALS=1`, derive may retain its existing guarded capability through `Recipe::Read` only when its source is the exact current view and its result is an owner-free scalar. The current nonnull sole-owner guard, consuming/non-growing view proof, empty prechange retirement, unique predecessor, linear control and existing32-site proof-search bound remain. Calls, growth, changed views, aliases, joins and owner-producing/non-scalar reads decline. The32-site bound is compile-time proof search; it does not deny execution.

The shared action emitter still executes the lexical read with its original checked index, current matrix geometry, source errors and canonical ordinal translation. No new source operation becomes native, no runtime permission escapes, no phase boundary is crossed, and no compressor identity/input size/content selects this optimization. All original source arithmetic, tables, allocations, search limits, buffer sizes and timing endpoints are unchanged.

Exactly one real interval extends: first2596, head stores2596/2598, read2602 of current value1960, store2604, exit2605. There are still12 intervals. All136 original numeric/map routines and seven selected vector bodies are byte-identical to R245. The root shrinks5630944 ->5630912 bytes, while its frame **grows55136 ->55152 bytes**. Disabled full original Object and metadata are exactly R245. The additional full disabled decoded image is not emitted; small original/decoded controls and the pinned bytecode input remain qualified.

## Main complete-call measurements

Both primary cohorts remeasure all25 original and56 controlled-prefix configurations. Each uses24 balanced blocks, all six three-actor orders repeated four times and200ms warmup per actor/case. Parent, candidate and reference share fixed level12, runtime input, pinned allocator, actual live borrowed output, Rust Instant, validation placement and complete compression/scratch/output transfer/cleanup boundaries. Preparation, validation, logging and prior-output drop remain excluded. CPU and memory use separate24-block cohorts. No build, test, compressor diagnostic or profile overlaps retained timing/counter cohorts.

Ratios below use pooled observation medians; elapsed changes use balanced block means. CPU counters are a separate cohort, not a partition inferred from sample shares.

| Case | Roc ms | R245 ms | R249 ms | R245 / Roc | R249 / Roc | Elapsed change | Instructions change | Cycles change |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| eight-literals | 0.291125 | 0.143292 | 0.143750 | 0.492201x | 0.493774x | -0.0935% | -0.2207% | -1.2488% |
| 33027-periodic-window | 1.260375 | 1.988687 | 1.935104 | 1.577853x | 1.535339x | -2.6944% | -1.7640% | -2.6635% |
| repeated-32769 | 1.349084 | 2.008208 | 1.961375 | 1.488572x | 1.453857x | -2.3321% | -1.7688% | -1.6757% |
| reymont-prefix-32769 | 4.403083 | 9.758937 | 9.738583 | 2.216387x | 2.211765x | -0.2086% | -0.0449% | -0.1275% |
| random-32769 | 1.066937 | 2.885792 | 2.872250 | 2.704745x | 2.692052x | -0.4693% | -0.0137% | -1.9736% |
| heldout-random-65537 | 1.941354 | 5.770917 | 5.756021 | 2.972625x | 2.964952x | -0.2581% | -0.0058% | -0.4720% |
| heldout-mixed-131073 | 8.403125 | 20.632437 | 20.567208 | 2.455329x | 2.447567x | -0.3162% | -0.4070% | -0.5526% |
| prefix-natural-1048576 | 161.164937 | 362.658479 | 361.284104 | 2.250232x | 2.241704x | -0.3790% | -0.0263% | -0.0276% |
| prefix-random-1048576 | 28.790229 | 93.620729 | 93.499625 | 3.251823x | 3.247617x | -0.1294% | -0.0034% | -0.0817% |
| prefix-periodic-1048576 | 31.405271 | 56.751166 | 55.727312 | 1.807059x | 1.774457x | -1.8041% | -1.8163% | -2.0549% |
| prefix-repeated-1048576 | 34.309729 | 58.016771 | 56.916667 | 1.690971x | 1.658908x | -1.8962% | -1.8242% | -2.0292% |

All81 descriptive geometric elapsed changes -1.1298%; original large11 -0.7503%. These averages do not override the periodic-primary criterion. Instructions decrease on78 cases, tie on3 and increase on0. Cycles improve on72 and worsen on9; every row is retained.

Periodic instructions fall39327707 ->38633961.5 (-1.7640%); cycles fall2.6635%. Natural32 and random64 gain little: their separate instruction reductions are0.0449% and0.0058%. This change does not explain away their much larger remaining throughput gaps.

## Actual eligible/executed work

An independently built observed image counts only entry/refusal of the extended interval. It is never used for primary elapsed or hardware claims. Two calls on every configuration repeat exact counts, outputs/inflate/aliases and final zero ownership. Counts agree with the previously independent skip-position ledger. The counter uses the same existing observer interface; it changes no production emitter or runtime.

| Case | Entered extended private interval per call | Refused | Expected skip positions |
|---|---:|---:|---:|
| eight-literals | 0 | 0 | 0 |
| 33027-periodic-window | 32854 | 0 | 32854 |
| repeated-32769 | 32636 | 0 | 32636 |
| reymont-prefix-32769 | 2 | 0 | 2 |
| random-32769 | 0 | 0 | 0 |
| heldout-random-65537 | 0 | 0 | 0 |
| heldout-mixed-131073 | 32628 | 0 | 32628 |
| prefix-natural-1048576 | 0 | 0 | 0 |
| prefix-random-1048576 | 0 | 0 | 0 |
| prefix-periodic-1048576 | 1044466 | 0 | 1044466 |
| prefix-repeated-1048576 | 1044505 | 0 | 1044505 |

All81 payload requests/bytes, initialized copies, zeroed bytes, nonreturned payload, payload peak and complete listed heap medians stay exactly equal to R245. This is a repeated-check/instruction optimization, not a physical-copy or allocation reduction. RSS/physical/lifetime peaks include executable/metadata/harness/input preparation and allocator retention; they are not payload-copy measurements.

## Every current below-reference configuration

Original25 and prefix56 are separate coverage groups and contain duplicate configurations. A new below-reference prefix point is not a new distinct workload. All are point estimates; tiny0/1/7-byte timing is near clock resolution.

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

## Every measured configuration and contrary result

All81 have fresh exact output/decompression/alias/capacity/cleanup observations in both primary and separate CPU/memory cohorts. Both per-case point/block gates pass81/81 in each primary cohort and pooled. Plus signs below remain regressions even when another metric improves.

| Case | Bytes | R245 / Roc | R249 / Roc | Pooled block change | Cohort A | Cohort B | Instructions change |
|---|---:|---:|---:|---:|---:|---:|---:|
| empty | 0 | 1.976190x | 1.976190x | -2.5172% | -0.4118% | -3.4482% | +0.0000% |
| one-stored | 1 | 2.000000x | 1.976190x | -3.1356% | -3.4121% | -2.9885% | +0.0000% |
| seven-stored-prefix | 7 | 2.976190x | 2.000000x | -7.1451% | -6.8661% | -8.5865% | +0.0000% |
| eight-literals | 8 | 0.492201x | 0.493774x | -0.0935% | +1.0259% | -0.7819% | -0.2207% |
| eight-overlap | 8 | 0.485270x | 0.489278x | +0.3961% | +0.7848% | +0.0001% | -0.2180% |
| 320-overlap | 320 | 0.545932x | 0.544692x | -0.4269% | -0.7526% | -0.2958% | -0.5574% |
| eight-literals-prefix | 8 | 0.493442x | 0.490593x | -0.7995% | -0.2118% | -0.0105% | -0.2207% |
| 128-biased-dynamic | 128 | 0.504075x | 0.500930x | -0.8093% | -0.1761% | -1.6527% | -0.2281% |
| 32-high-entropy-stored | 32 | 0.547541x | 0.552957x | +1.9767% | -0.7011% | +0.8862% | -0.2198% |
| 16384-distribution-change | 16384 | 2.754320x | 2.730085x | -0.8799% | +0.1491% | -1.1611% | -0.4091% |
| 33027-periodic-window | 33027 | 1.577853x | 1.535339x | -2.6944% | -2.7269% | -2.6317% | -1.7640% |
| random-64 | 64 | 0.577605x | 0.576308x | -0.7488% | +0.4332% | -0.6907% | -0.2139% |
| random-256 | 256 | 0.673010x | 0.668728x | -0.6146% | +0.9384% | -2.1085% | -0.1933% |
| random-1024 | 1024 | 0.748612x | 0.750507x | +0.3190% | +0.5457% | +0.1296% | -0.1820% |
| repeated-1024 | 1024 | 0.649621x | 0.641626x | -0.7655% | -0.3351% | -2.0612% | -0.9239% |
| random-32769 | 32769 | 2.704745x | 2.692052x | -0.4693% | -0.4957% | -0.4462% | -0.0137% |
| repeated-32769 | 32769 | 1.488572x | 1.453857x | -2.3321% | -1.8030% | -3.4216% | -1.7688% |
| reymont-prefix-1024 | 1024 | 0.928110x | 0.921924x | -0.2470% | -0.6244% | -0.4311% | -0.2105% |
| reymont-prefix-16384 | 16384 | 2.188753x | 2.185085x | -0.1676% | -0.6861% | +0.2166% | -0.0635% |
| reymont-prefix-32769 | 32769 | 2.216387x | 2.211765x | -0.2086% | -0.4440% | -0.2952% | -0.0449% |
| heldout-window-32767 | 32767 | 2.227906x | 2.220523x | -0.3314% | -0.4726% | -0.3068% | -0.0477% |
| heldout-window-32768 | 32768 | 2.234075x | 2.224079x | -0.4474% | -0.6925% | -0.2580% | -0.0508% |
| heldout-slides-65537 | 65537 | 2.302126x | 2.299668x | -0.1068% | -0.4282% | +0.1718% | -0.0299% |
| heldout-random-65537 | 65537 | 2.972625x | 2.964952x | -0.2581% | -0.5153% | +0.1700% | -0.0058% |
| heldout-mixed-131073 | 131073 | 2.455329x | 2.447567x | -0.3162% | -0.3451% | -0.3097% | -0.4070% |
| prefix-natural-1024 | 1024 | 0.927311x | 0.924036x | -0.7725% | -0.8632% | -0.1547% | -0.2087% |
| prefix-natural-4096 | 4096 | 1.719200x | 1.707442x | -0.6839% | -0.1393% | -2.1102% | -0.0780% |
| prefix-natural-8192 | 8192 | 2.003134x | 1.993862x | -0.4628% | -0.1136% | -0.5902% | -0.0652% |
| prefix-natural-16384 | 16384 | 2.188705x | 2.181989x | -0.3068% | -0.4597% | -0.1556% | -0.0658% |
| prefix-natural-32767 | 32767 | 2.229389x | 2.223846x | -0.2486% | -0.2668% | -0.1665% | -0.0453% |
| prefix-natural-32768 | 32768 | 2.231282x | 2.223553x | -0.3464% | -0.4739% | -0.2049% | -0.0513% |
| prefix-natural-32769 | 32769 | 2.227214x | 2.224159x | -0.1372% | -0.3415% | +0.1820% | -0.0483% |
| prefix-natural-33027 | 33027 | 2.225711x | 2.219863x | -0.2627% | -0.1598% | -0.3651% | -0.0447% |
| prefix-natural-65535 | 65535 | 2.208166x | 2.205831x | -0.1057% | -0.0975% | -0.1921% | -0.0268% |
| prefix-natural-65536 | 65536 | 2.213786x | 2.206898x | -0.3111% | -0.4571% | -0.2659% | -0.0269% |
| prefix-natural-65537 | 65537 | 2.241840x | 2.233469x | -0.3734% | -0.2681% | -0.2825% | -0.0262% |
| prefix-natural-131072 | 131072 | 2.208308x | 2.204033x | -0.1936% | -0.1835% | -0.1508% | -0.0258% |
| prefix-natural-262144 | 262144 | 2.215547x | 2.205271x | -0.4638% | -0.4571% | -0.4638% | -0.0245% |
| prefix-natural-1048576 | 1048576 | 2.250232x | 2.241704x | -0.3790% | -0.4763% | -0.3266% | -0.0263% |
| prefix-random-1024 | 1024 | 0.742712x | 0.744041x | +0.2552% | +0.4726% | +0.2286% | -0.1897% |
| prefix-random-4096 | 4096 | 1.405076x | 1.368862x | -2.5774% | -1.0162% | -1.3261% | -0.0711% |
| prefix-random-8192 | 8192 | 1.774691x | 1.767847x | -0.3856% | +3.0462% | -1.4228% | -0.0412% |
| prefix-random-16384 | 16384 | 2.311070x | 2.285589x | -1.1025% | -0.8587% | -1.4062% | -0.0273% |
| prefix-random-32767 | 32767 | 2.722672x | 2.715118x | -0.2774% | +0.0697% | -0.9638% | -0.0137% |
| prefix-random-32768 | 32768 | 2.730308x | 2.712286x | -0.6601% | -0.0993% | -0.5523% | -0.0137% |
| prefix-random-32769 | 32769 | 2.701782x | 2.696490x | -0.1959% | -0.1752% | -0.2098% | -0.0137% |
| prefix-random-33027 | 33027 | 2.735300x | 2.726038x | -0.3386% | +0.2532% | -0.5802% | -0.0133% |
| prefix-random-65535 | 65535 | 2.989225x | 2.989320x | +0.0032% | +0.1139% | -0.3212% | -0.0083% |
| prefix-random-65536 | 65536 | 2.981042x | 2.982394x | +0.0454% | -0.0021% | -0.0213% | -0.0067% |
| prefix-random-65537 | 65537 | 2.961203x | 2.962107x | +0.0305% | +0.1103% | -0.1107% | -0.0048% |
| prefix-random-131072 | 131072 | 3.115530x | 3.120069x | +0.1457% | +0.0960% | +0.1873% | -0.0074% |
| prefix-random-262144 | 262144 | 3.169205x | 3.165091x | -0.1298% | -0.2367% | +0.0685% | -0.0025% |
| prefix-random-1048576 | 1048576 | 3.251823x | 3.247617x | -0.1294% | -0.1579% | -0.1209% | -0.0034% |
| prefix-periodic-1024 | 1024 | 0.662653x | 0.653021x | -2.3122% | -1.6749% | -3.2131% | -0.8776% |
| prefix-periodic-4096 | 4096 | 1.014317x | 0.977414x | -3.6382% | -3.2854% | -3.8862% | -1.3638% |
| prefix-periodic-8192 | 8192 | 1.201361x | 1.179501x | -1.8196% | -2.7359% | -1.2797% | -1.5102% |
| prefix-periodic-16384 | 16384 | 1.395559x | 1.353868x | -2.9874% | -2.8251% | -3.2345% | -1.7201% |
| prefix-periodic-32767 | 32767 | 1.555681x | 1.517337x | -2.4648% | -1.7562% | -3.3015% | -1.7745% |
| prefix-periodic-32768 | 32768 | 1.564099x | 1.529397x | -2.2186% | -2.3078% | -2.1297% | -1.7743% |
| prefix-periodic-32769 | 32769 | 1.567761x | 1.529268x | -2.4553% | -2.7512% | -2.2038% | -1.7635% |
| prefix-periodic-33027 | 33027 | 1.557464x | 1.526919x | -1.9612% | -2.0872% | -2.0998% | -1.7640% |
| prefix-periodic-65535 | 65535 | 1.680979x | 1.646864x | -2.0295% | -2.2483% | -1.9196% | -1.7972% |
| prefix-periodic-65536 | 65536 | 1.684256x | 1.647663x | -2.1727% | -1.9453% | -2.2555% | -1.7972% |
| prefix-periodic-65537 | 65537 | 1.680116x | 1.646418x | -2.0057% | -1.8718% | -2.1170% | -1.7916% |
| prefix-periodic-131072 | 131072 | 1.745373x | 1.712867x | -1.8624% | -1.8477% | -1.8616% | -1.8064% |
| prefix-periodic-262144 | 262144 | 1.780883x | 1.746732x | -1.9176% | -1.9543% | -1.8005% | -1.8155% |
| prefix-periodic-1048576 | 1048576 | 1.807059x | 1.774457x | -1.8041% | -1.7761% | -1.8464% | -1.8163% |
| prefix-repeated-1024 | 1024 | 0.640699x | 0.632393x | -0.7237% | +1.8224% | -2.7824% | -0.9241% |
| prefix-repeated-4096 | 4096 | 0.975666x | 0.950202x | -2.6100% | -2.7057% | -1.9759% | -1.3883% |
| prefix-repeated-8192 | 8192 | 1.155618x | 1.136148x | -1.6849% | -2.1864% | -1.1166% | -1.5250% |
| prefix-repeated-16384 | 16384 | 1.318529x | 1.292547x | -1.9705% | -1.5339% | -2.0692% | -1.7299% |
| prefix-repeated-32767 | 32767 | 1.484775x | 1.460915x | -1.6069% | -1.8155% | -1.4645% | -1.7798% |
| prefix-repeated-32768 | 32768 | 1.476909x | 1.445569x | -2.1220% | -2.2066% | -2.1916% | -1.7799% |
| prefix-repeated-32769 | 32769 | 1.486576x | 1.455550x | -2.0870% | -1.8301% | -2.1337% | -1.7689% |
| prefix-repeated-33027 | 33027 | 1.489947x | 1.462391x | -1.8494% | -1.4326% | -2.0965% | -1.7694% |
| prefix-repeated-65535 | 65535 | 1.590879x | 1.559376x | -1.9802% | -2.0958% | -2.0336% | -1.8001% |
| prefix-repeated-65536 | 65536 | 1.590914x | 1.562201x | -1.8048% | -1.6748% | -1.9930% | -1.8001% |
| prefix-repeated-65537 | 65537 | 1.590579x | 1.558479x | -2.0182% | -2.0633% | -1.7113% | -1.7945% |
| prefix-repeated-131072 | 131072 | 1.653954x | 1.624397x | -1.7870% | -1.6825% | -1.9249% | -1.8108% |
| prefix-repeated-262144 | 262144 | 1.683889x | 1.651828x | -1.9040% | -1.8859% | -2.0050% | -1.8163% |
| prefix-repeated-1048576 | 1048576 | 1.690971x | 1.658908x | -1.8962% | -1.8390% | -1.9075% | -1.8242% |

Eight literals has contrary cohort block changes +1.0259% and-0.7819%; its pooled point median worsens0.3196% while block mean improves0.0935%. Keep all of these, not a selective small-input improvement claim. The largest adverse pooled block case is32-high-entropy-stored (+1.9767%).

## Current executable profile

These are fresh Timer Fired/Running samples from the exact measured candidate executable. Actual image slides and its own symbols map PCs back to its own Object. Samples include harness/validation; no per-function retired-instruction claim or removable-time fraction is inferred.

| Case | Running samples | Most sampled symbols (weights) |
|---|---:|---|
| 33027-periodic-window | 5539 | `_mech_compact_closed_v2` 2307; `_mech_compact_numeric_2624_vector` 1696; `_mech_compact_numeric_15296` 833; `_mech_compact_numeric_12868_vector` 150 |
| eight-literals | 5472 | `_mech_compact_closed_v2` 1596; `__RNvNtNtCselbn9KGBQg0_11miniz_oxide7inflate4core9init_tree` 73; `_mech_compact_numeric_17556` 56; `_mech_compact_numeric_29155` 39 |
| heldout-random-65537 | 5437 | `_mech_compact_closed_v2` 3351; `_mech_compact_numeric_15296` 714; `_mech_compact_numeric_15737` 621; `_mech_compact_numeric_1279_vector` 390 |
| reymont-prefix-32769 | 5367 | `_mech_compact_numeric_15296` 2616; `_mech_compact_numeric_1279_vector` 1228; `_mech_compact_closed_v2` 1184; `_mech_compact_numeric_15737` 68 |

## Qualification, commands and failure disposition

Compiler opt3/test-root opt1, nightly2026-03-03 AArch64 macOS, warnings-as-errors, features aot/jit/canonical-native/canonical-native-runtime/canonical-native-program/canonical-native-program-runtime. Exact command vectors, linked dependency hashes and environments are retained in `*-build-spec.json` and receipts.

- 703 maintained tests pass; the same seven inherited exclusions stay explicit in maintained-native-tests.json. No green full CI or omitted coverage is inferred.
- Same-view scalar, different-view negative and real rectangular matrix controls execute original/decoded on parent/candidate, with and without observers:48432 fresh native calls. Typed inputs use an independent ordinal/value/error oracle; raw Boolean words preserve clean-parent behavior without claiming external admission. Bounds/overflow/assertion order, equal/repeated writes, current matrix dimensions, held input aliases, actual previously returned backing, failure output sentinels, cleanup and success/failure/retry agree exactly. Different-view Objects and metadata remain byte-identical.
- 1700 fresh existing matrix native controls and1700 fresh resident comparisons pass unchanged;1700 parent native records are reused by hash.416 fresh boundary controls compare parent/candidate/original/decoded. Existing default-cap denials remain expected; no allowances or expected values change. New packed-record read execution is not claimed; non-scalar/owner-producing read refusal is the explicit current type boundary.
- 162 normal complete-call smokes and162 observed full-call interval-count controls pass. All81 inputs now accompany the packet as deduplicated portable fixtures without changing the measured fixture paths.
- The first matrix fixture wrongly assumed a comprehension preserves shape. Clean parent established row-vector collection. Another fixture used U16 operands for a U64 bits operation, then an unsupported narrow scalar lowering path. Preserve those failed commands/fixtures; use existing constant checked indexed replacements to construct real matrices instead. The initial empty-view control also incorrectly expected a private entry for null owner0; the clean parent takes the unchanged fallback. The first producer build omitted the unchanged producer.rs driver; copying that archived driver and rebuilding succeeds. Preserve both initial receipts. These are test-setup/build setup errors or inherited unsupported test paths, not introduced compressor failures.
- The initial smoke script mislabeled its outer count25 although it actually executed162 observations on81 inputs. Preserve its receipt and correct the metadata/replay script; no outcome changes.
- Disk exhaustion stopped the initial second primary cohort while saving logs. Preserve the incomplete files under initial-disk-full-*, retain the complete first cohort, verify/free old completed builds, losslessly compress every saved observation and rerun only the second cohort. Both retained cohorts complete. The environmental failure changes no compiler, expected result, allowance or measurement endpoint.
- Seven broad-suite exclusions and the additional full disabled decoded image are the explicit skipped coverage. The ordinary IR215 exact admission/resource/accounting/canonical-publication/recovery contract remains separate and omitted by this standalone compact entry; no ordinary integration or finite-contract admission at1MiB is claimed.

Command families: run-spec.py candidate-gpu/candidate-producer/candidate-tests; maintained-native-tests receipt; qualify-scalar-matrix.py, qualify-plain-reads.py, qualify-writes.py and qualify-exits.py; measure-primary-a.py/measure-primary-b.py (all81), measure-cpu.py, measure-memory.py; measure-interval-executions.py; profile-compact-pcs.py and analyze-pcs.py. `replay.py` recomputes the retained numerical reports without executing a compiler or compressor. To execute portable fixtures, resolve their three relative input paths against the unpacked evidence directory.

## Brief disposition and next bounded step

Phase-only isolation is published in R247; current independent work and controlled prefixes are published in R248; its fresh exact retained random64 profile is preserved. R249 supplies the requested next bounded compiler experiment, both timing directions and fresh all81 counters. No LLVM dependency or compressor source rewrite is introduced.

Individual Mech block offsets are still not directly sampled. The independent11-category aggregate/reason/span and weighted block-work agreement must not be described as a complete cross-language per-block-history proof. Phase-success ownership propagation is also not implemented by this scalar-read change.

The user now explicitly prioritizes the worsening large-input ratio. The largest current wins reach4KiB only for repetitive inputs; text/random lose before that. The next target is repeated per-position root state/view transport and checks in random64, with64KiB and1MiB throughput/instructions per byte as primary witnesses. The fresh random64 root holds3351/5437 running samples; text has a separate path15296 hot region,2616/5367. These samples locate work, not removable time. Establish one static proof and minimal witness before modifying lowering. Observed unique owners alone are not authority. Keep all81 cases and the eight-byte1x ceiling as regression controls; the ordinary extra contract remains separate.

## Reproduction identities

- source: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`
- mbc14: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`
- normal_object: `da30e96737ec133b6c466c4ad719522a7138f2e308dcf9032bed2f2e7e9653aa`
- normal_metadata: `d658a44bdcad92c14365dd6334d5b197444f2b8eb09f82842690a00ea0191737`
- normal_worker: `0ab9ce8176e59074e068733b30ecb18ee246b6b7aebcd22bab22bfe627f63460`
- runtime: `bbc12169d120eaa60af57904de32f599d594e24c80201f1173991a05746ff066`
- compiler_patch: `77917b5ebf8c123268f5126e756cf3e3a700076d097ce11cc0d8ee750efb3113`
- compiler_source_manifest: `0e962ed1609df6e86b84c6f73ac9ebf01e27a8f467064a694203c0ac42a81b51`
- reference_worker: `ed1ab9175cd175d26d453afc26ba1fa7adb4827ec705aa1cf9bcd231ac84860c`
- pinned_allocator: `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`

## Public review packet

[Source, compiler patch, all81 portable fixtures, timing/counter/qualification receipts and numerical replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r249-private-read-intervals-474fe983ca44.tar.gz). SHA256 `474fe983ca4421cdd8923b4866eb2e644f8be6ef823fff349dec5b1d1201fe06`; 11190750 bytes. The public download verifies2732 file hashes and five numerical reports; the one-file patch reconstructs the815-file source closure. No native executable/Object, bytecode or raw profile stack is published.

Only draft PR973 is updated. Request exact-head review and cancel only exact-head CI by standing instruction; no green full CI or merge readiness is claimed.
