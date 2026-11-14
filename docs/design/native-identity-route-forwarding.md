# R252: acyclic identity continuation forwarding — archived broad variant

**Retain R250. Archive this broad variant.** It reduces root code by 32.97% and improves short/repetitive cases, but the targeted large random/text cases get slower in both confirmation cohorts. All 81 existing 1.05 gates pass; passing those gates does not override the current throughput priority. Preserve this component and its contrary results for a narrower follow-up.

Git parent `dbd5b0ecdc2ab2cac53f33580a5eb8e789fb5130`; PR base `92f75db84e2523693648084bea16cd52ad60777a`. Eight changed/new files over the archived R251 815-file GPU closure reconstruct 820 files. This patch does not apply directly to the maintained tree. Ordinary R215 stays `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`, IR/work/ABI/MBC215/107/39/14. Benchmark source/bytecode, algorithm, tables, depth 300, nice 258, near-EOF limits, real buffers, allocation sites, pinned allocator and timing endpoints remain unchanged.

## Transformation and actual machine evidence

The opt-in `MECH_IDENTITY_ROUTE_FORWARDING=1` pass follows acyclic pure scalar SSA routes with a currently selected integer predicate. It forwards typed current values into the supported continuation. Loads, stores, calls, checked arithmetic, source debits and exceptional edges stop the walk. Re-entry/backedges and undominated shared uses decline. Current interval inversion handles normalized Boolean words without assuming raw flags are valid Boolean values. Skipped live definitions receive typed destination parameters, with all original incoming edges and dominated uses repaired. Logical aliases and physical ownership operations stay. Only the compact root invokes this experiment; no ordinary finite-entry path changes.

The actual root changes **5,623,612 -> 3,769,668 bytes**, while its machine frame grows **55,136 -> 56,096 bytes** (+960 B). All 136 existing routines and seven selected vector bodies are byte-identical. The final formatted source, four-test snapshot and source-location annotations reproduce the measured full Object/metadata exactly; disabling the flag restores the R250 full Object/metadata exactly.

The pass selects 1,909 edges, 3,203 conditions and adds 2,201 typed repair parameters. The source 2399 zero edge bypasses the skip identity joins in CLIF and supplies 22 current values at its destination. That is an IR forwarding result, not proof that physical stack transport disappeared. The machine frame grows, random instructions increase, and the new actual hot code still contains long stack-to-stack transfer sequences. No whole zero-route saving is claimed.

The R251 22-instruction relay and 97-instruction root corridor are historical exact-image witnesses. Their addresses cannot be applied to the changed R252 Object. Current disassembly excerpts and aggregate current-image PCs are retained. Source locations denote emission provenance; many inserted moves have the default location. The selected-edge fragment inspected here is sampled on periodic input and not on random/text, so it is not used as proof of the common zero-route cost.

## Complete-call elapsed and independent counters

Both primary cohorts use all 81 frozen configurations, 24 balanced blocks, all six parent/candidate/reference orders repeated four times and 200ms disclosed warmup per actor/case. Input is runtime on both sides, level 12 fixed and output ownership matched/live/borrowed. Compression, scratch construction, output transfer and private cleanup are timed. Validation/logging/preparation/prior-output drop stay outside. CPU and heap are separate 24-block cohorts. Builds, tests, profiles and diagnostic execution do not overlap any of these cohorts.

Ratios below use pooled observation medians against the same fresh Roc denominator. Elapsed changes use balanced block means; instructions/cycles come from their independent cohort. Positive changes are regressions.

| Case | Roc ms | R250 ms | R252 ms | R250 / Roc | R252 / Roc | Elapsed change | Instructions | Cycles |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| eight-literals | 0.290000 | 0.147542 | 0.143020 | 0.508766x | 0.493174x | -1.3536% | -1.3074% | -5.6213% |
| 33027-periodic-window | 1.266291 | 1.958021 | 1.918146 | 1.546264x | 1.514774x | -2.0365% | -0.4582% | -1.3929% |
| repeated-32769 | 1.351562 | 1.984458 | 1.949937 | 1.468270x | 1.442728x | -1.7395% | -0.4498% | -0.6942% |
| random-32769 | 1.068854 | 2.866166 | 2.907709 | 2.681532x | 2.720398x | +1.4494% | +2.6743% | +1.9138% |
| heldout-random-65537 | 1.940854 | 5.746354 | 5.839334 | 2.960736x | 3.008642x | +1.6181% | +2.6840% | +2.5849% |
| reymont-prefix-32769 | 4.391584 | 9.761688 | 9.768812 | 2.222817x | 2.224440x | +0.0730% | +0.7439% | +0.2598% |
| prefix-natural-1048576 | 161.487145 | 361.397209 | 363.563605 | 2.237932x | 2.251347x | +0.5995% | +0.7189% | +0.5740% |
| prefix-random-1048576 | 28.849687 | 92.953354 | 95.003354 | 3.221988x | 3.293046x | +2.2054% | +2.5057% | +2.2355% |
| prefix-periodic-1048576 | 31.397312 | 55.872146 | 56.101500 | 1.779520x | 1.786825x | +0.4105% | -0.3315% | +0.4496% |
| prefix-repeated-1048576 | 34.461104 | 57.910646 | 58.311521 | 1.680464x | 1.692097x | +0.6922% | -0.3305% | +0.3086% |

All 81 descriptive elapsed geometric change is-0.8785%; original large 11 is-0.2760%. There are 29 adverse pooled elapsed rows, 40 instruction increases and 31 cycle increases. The averages do not erase random 1MiB+2.2054% elapsed/+2.5057% instructions or natural 1MiB+0.5994% elapsed/+0.7189% instructions. Eight remains below 1x in both cohorts. Its pooled median ratio change and balanced change differ; do not call its median 3.06% difference the balanced 1.35% result.

All 81 payload allocation/copy/zero/private-peak and listed whole-heap medians are exactly equal. RSS/physical/lifetime values include executable, parsed metadata, harness and allocator retention, not payload alone. The candidate metadata includes its detailed route audit and is larger, which can affect those process measures outside the execution timing. Requested capacity is not initialized bytes copied.

## Actual current-image profile

| Case | Running samples | Root | Numeric phases | Other image/harness/library |
|---|---:|---:|---:|---:|
| eight-literals | 5351 | 1452 | 638 | 3261 |
| prefix-natural-1048576 | 5352 | 910 | 4394 | 48 |
| prefix-periodic-1048576 | 5504 | 2341 | 3085 | 78 |
| prefix-random-1048576 | 5554 | 3541 | 1978 | 35 |

These CPU-time samples include validation and harness. They are neither retired-instruction partitions nor removable-time budgets. Random remains dominated by generated-root work; natural text remains dominated by numeric path phases. The current hottest random root windows still show stack-to-stack carrier traffic. Full code and frame differences do not by themselves identify how much transport moved or establish causality for each elapsed change.

## Qualification and failure disposition

707 maintained tests pass with the same seven inherited exclusions, plus four focused proof tests containing 3,918 before/after native calls. Controls cover current repeated arguments, raw invalid flags, float bit patterns/NaNs, parallel payload swaps, shared joins, captured predicates and re-entry refusals, assertion/load/call effects and arithmetic failure precedence. The focused tests are included in 707, not additional to that count.

3,500 independent source calls across six programs/seven execution variants and original/decoded artifacts pass, including 700 zero trips. They preserve values, current rectangular geometry, aliases, prior published output/backing, cleanup and failure/retry. All seven small image pairs are byte-identical original/decoded. 162 immediate complete-call smokes and all primary/CPU/heap/profile calls repeat exact reference bytes, independent inflate, aliases, output capacity and zero final private payload/owners. Existing R250 extended matrix/buffer/boundary qualification is inherited, not relabeled fresh.

GPU/producer builds retain `-D warnings`, opt 3 and aot/jit/canonical-native/canonical-native-runtime/canonical-native-program/canonical-native-program-runtime. The test root uses opt 1 and the same pinned nightly. Full candidate/disabled decoded compression images are not emitted this round; decoded behavior is exercised by the seven source pairs and maintained suite. No full CI completion or ordinary compact-entry extra-contract restoration is claimed.

Introduced pre-validation setup failures: an unused test import, and an omitted nested test-harness source in the qualification copier. Both are corrected and their failed receipts retained. The copied analysis initially labeled periodic 33 the primary target; only this stale label was corrected to the current large random/text priority, with the original script/report retained and all 81 numerical rows unchanged.

Environmental failures: old allocator/probe source files and 3,161 tracked files plus the linked checkout pointer were absent. Allocator sources were recovered from their exact hash-verified public R163 closure; a rebuilt retained R250 memory worker is byte-identical before counters were collected. Missing checkout files were restored only from unchanged HEAD. The first checkout restore lacked permission to lock the common index; the elevated retry succeeded. No allocator or main-worktree edits were introduced.

Remaining inherited boundaries: seven maintained exclusions, compact-entry ordinary extra-contract omissions and the enclosing mutable-child visit-funding/native-selection refusal from R251. This round does not relax or repair them. The introduced large-input performance regression is unresolved in this variant and is why it is archived. No unresolved introduced semantic failure remains.

## Every measured configuration

| Case | Bytes | R250 / Roc | R252 / Roc | Balanced change | Cohort A | Cohort B | Instructions |
|---|---:|---:|---:|---:|---:|---:|---:|
| empty | 0 | 1.976190x | 1.976190x | -7.3316% | -7.8067% | -6.9554% | -0.0634% |
| one-stored | 1 | 2.000000x | 1.976190x | -5.9946% | -6.3409% | -6.1166% | -0.2634% |
| seven-stored-prefix | 7 | 1.976190x | 2.000000x | +1.7290% | +1.5008% | +3.8430% | -0.2614% |
| eight-literals | 8 | 0.508766x | 0.493174x | -1.3536% | -2.5727% | -0.6757% | -1.3074% |
| eight-overlap | 8 | 0.502017x | 0.483795x | -4.5947% | -4.4543% | -4.6779% | -1.2794% |
| 320-overlap | 320 | 0.556349x | 0.545266x | -1.9148% | -2.7021% | -1.9580% | -1.0313% |
| eight-literals-prefix | 8 | 0.503846x | 0.493347x | -1.8768% | -4.3978% | -0.1356% | -1.3072% |
| 128-biased-dynamic | 128 | 0.510124x | 0.502946x | -1.2182% | -1.4065% | -1.5237% | -0.9027% |
| 32-high-entropy-stored | 32 | 0.540998x | 0.520356x | -3.1076% | -3.6754% | -2.5304% | -1.3670% |
| 16384-distribution-change | 16384 | 2.710542x | 2.660476x | -1.8471% | -1.6598% | -2.0752% | +0.7260% |
| 33027-periodic-window | 33027 | 1.546264x | 1.514774x | -2.0365% | -1.9390% | -2.2761% | -0.4582% |
| random-64 | 64 | 0.561477x | 0.543710x | -2.7018% | -3.3665% | -2.2542% | -1.3978% |
| random-256 | 256 | 0.656791x | 0.633416x | -3.5603% | -3.3076% | -3.0178% | -1.0761% |
| random-1024 | 1024 | 0.740323x | 0.744087x | +0.3049% | +0.2099% | -0.0448% | +0.7346% |
| repeated-1024 | 1024 | 0.636514x | 0.632211x | -1.1198% | +0.0728% | -1.8006% | -0.7696% |
| random-32769 | 32769 | 2.681532x | 2.720398x | +1.4494% | +1.5466% | +1.2471% | +2.6743% |
| repeated-32769 | 32769 | 1.468270x | 1.442728x | -1.7395% | -0.9636% | -2.2975% | -0.4498% |
| reymont-prefix-1024 | 1024 | 0.926660x | 0.923020x | -0.2713% | +1.0309% | -1.1823% | +0.1244% |
| reymont-prefix-16384 | 16384 | 2.183037x | 2.185118x | +0.0953% | -0.0738% | +0.2188% | +0.6964% |
| reymont-prefix-32769 | 32769 | 2.222817x | 2.224440x | +0.0730% | -0.1608% | +0.1242% | +0.7439% |
| heldout-window-32767 | 32767 | 2.223614x | 2.221926x | -0.0759% | -0.2381% | +0.0763% | +0.7415% |
| heldout-window-32768 | 32768 | 2.228023x | 2.227961x | -0.0028% | -0.2615% | +0.3137% | +0.7412% |
| heldout-slides-65537 | 65537 | 2.299596x | 2.287167x | -0.5405% | -0.5462% | -0.5456% | +0.5998% |
| heldout-random-65537 | 65537 | 2.960736x | 3.008642x | +1.6181% | +1.5627% | +1.7152% | +2.6840% |
| heldout-mixed-131073 | 131073 | 2.441508x | 2.442603x | +0.0449% | -0.0451% | +0.0868% | +0.5811% |
| prefix-natural-1024 | 1024 | 0.925241x | 0.919934x | -0.5845% | -1.3903% | -0.1838% | +0.1229% |
| prefix-natural-4096 | 4096 | 1.709284x | 1.676193x | -1.9360% | -2.5165% | -1.0383% | +0.6045% |
| prefix-natural-8192 | 8192 | 1.995019x | 1.970393x | -1.2344% | -1.4660% | -1.1128% | +0.6701% |
| prefix-natural-16384 | 16384 | 2.185602x | 2.177246x | -0.3823% | -0.5541% | -0.0378% | +0.7033% |
| prefix-natural-32767 | 32767 | 2.232872x | 2.231034x | -0.0823% | -0.0694% | -0.0305% | +0.7449% |
| prefix-natural-32768 | 32768 | 2.238170x | 2.230699x | -0.3338% | -0.2577% | +0.0311% | +0.7446% |
| prefix-natural-32769 | 32769 | 2.220182x | 2.221197x | +0.0457% | +0.0562% | +0.0288% | +0.7445% |
| prefix-natural-33027 | 33027 | 2.218018x | 2.216092x | -0.0869% | -0.2903% | +0.0299% | +0.7432% |
| prefix-natural-65535 | 65535 | 2.196571x | 2.205958x | +0.4274% | +0.2867% | +0.4583% | +0.7508% |
| prefix-natural-65536 | 65536 | 2.197010x | 2.207424x | +0.4740% | +0.3148% | +0.6025% | +0.7506% |
| prefix-natural-65537 | 65537 | 2.236179x | 2.240498x | +0.1932% | +0.0343% | +0.2636% | +0.6636% |
| prefix-natural-131072 | 131072 | 2.198196x | 2.221252x | +1.0489% | +1.0871% | +1.0745% | +0.8204% |
| prefix-natural-262144 | 262144 | 2.201946x | 2.213402x | +0.5203% | +0.6577% | +0.4690% | +0.6806% |
| prefix-natural-1048576 | 1048576 | 2.237932x | 2.251347x | +0.5995% | +0.7347% | +0.5665% | +0.7189% |
| prefix-random-1024 | 1024 | 0.754210x | 0.743429x | -1.1166% | -0.4895% | -1.3636% | +0.7471% |
| prefix-random-4096 | 4096 | 1.400565x | 1.326690x | -5.2747% | -5.9304% | -3.9578% | +1.9102% |
| prefix-random-8192 | 8192 | 1.755160x | 1.751713x | -0.1964% | +0.3087% | -0.6195% | +2.5106% |
| prefix-random-16384 | 16384 | 2.306531x | 2.314051x | +0.3260% | +0.3644% | +0.0468% | +2.6784% |
| prefix-random-32767 | 32767 | 2.713510x | 2.739740x | +0.9666% | +1.2600% | +0.9506% | +2.6840% |
| prefix-random-32768 | 32768 | 2.701142x | 2.736192x | +1.2976% | +1.4496% | +1.0960% | +2.6908% |
| prefix-random-32769 | 32769 | 2.694064x | 2.732786x | +1.4373% | +1.3079% | +1.5117% | +2.6838% |
| prefix-random-33027 | 33027 | 2.707628x | 2.728205x | +0.7600% | +1.2328% | +0.6173% | +2.6770% |
| prefix-random-65535 | 65535 | 2.961548x | 3.024576x | +2.1282% | +2.2023% | +2.0537% | +2.6098% |
| prefix-random-65536 | 65536 | 2.960115x | 3.013432x | +1.8012% | +1.6505% | +2.0444% | +2.6087% |
| prefix-random-65537 | 65537 | 2.961248x | 3.015735x | +1.8400% | +1.7919% | +1.8533% | +2.6041% |
| prefix-random-131072 | 131072 | 3.092855x | 3.206768x | +3.6831% | +3.6831% | +3.7058% | +2.7744% |
| prefix-random-262144 | 262144 | 3.152504x | 3.213137x | +1.9233% | +1.8273% | +2.0867% | +2.4869% |
| prefix-random-1048576 | 1048576 | 3.221988x | 3.293046x | +2.2054% | +2.1906% | +2.2498% | +2.5057% |
| prefix-periodic-1024 | 1024 | 0.672202x | 0.656998x | -2.6211% | -0.0194% | -3.4490% | -0.7935% |
| prefix-periodic-4096 | 4096 | 1.039828x | 0.985353x | -5.2388% | -4.5341% | -6.5792% | -0.5488% |
| prefix-periodic-8192 | 8192 | 1.178681x | 1.157408x | -1.8048% | -2.1881% | -1.6199% | -0.5012% |
| prefix-periodic-16384 | 16384 | 1.378267x | 1.352093x | -1.8990% | -2.4863% | -1.4471% | -0.4957% |
| prefix-periodic-32767 | 32767 | 1.542416x | 1.526270x | -1.0468% | -1.4067% | -0.9888% | -0.4639% |
| prefix-periodic-32768 | 32768 | 1.538564x | 1.516256x | -1.4500% | -1.1556% | -1.7715% | -0.4627% |
| prefix-periodic-32769 | 32769 | 1.540062x | 1.510660x | -1.9092% | -2.1310% | -1.4633% | -0.4672% |
| prefix-periodic-33027 | 33027 | 1.540711x | 1.510597x | -1.9546% | -2.0229% | -2.0559% | -0.4592% |
| prefix-periodic-65535 | 65535 | 1.655958x | 1.633212x | -1.3736% | -1.3971% | -1.0401% | -0.4475% |
| prefix-periodic-65536 | 65536 | 1.656272x | 1.636949x | -1.1667% | -1.3297% | -0.8680% | -0.4517% |
| prefix-periodic-65537 | 65537 | 1.653918x | 1.631818x | -1.3362% | -1.4925% | -1.4034% | -0.4468% |
| prefix-periodic-131072 | 131072 | 1.719799x | 1.701982x | -1.0360% | -1.1451% | -1.0924% | -0.4389% |
| prefix-periodic-262144 | 262144 | 1.749992x | 1.788525x | +2.2019% | +2.2337% | +2.1229% | -0.0715% |
| prefix-periodic-1048576 | 1048576 | 1.779520x | 1.786825x | +0.4105% | +0.4142% | +0.4097% | -0.3315% |
| prefix-repeated-1024 | 1024 | 0.661030x | 0.647771x | -3.1616% | -1.9739% | -3.1003% | -0.7623% |
| prefix-repeated-4096 | 4096 | 0.977098x | 0.926900x | -5.1375% | -5.4456% | -5.6300% | -0.5301% |
| prefix-repeated-8192 | 8192 | 1.147509x | 1.113228x | -2.9875% | -5.6009% | -2.3665% | -0.4737% |
| prefix-repeated-16384 | 16384 | 1.318874x | 1.295250x | -1.7913% | -2.2159% | -2.5795% | -0.4788% |
| prefix-repeated-32767 | 32767 | 1.472379x | 1.449311x | -1.5667% | -0.9175% | -2.0117% | -0.4593% |
| prefix-repeated-32768 | 32768 | 1.468606x | 1.454764x | -0.9425% | -1.5579% | -1.0360% | -0.4600% |
| prefix-repeated-32769 | 32769 | 1.488649x | 1.453128x | -2.3861% | -3.0078% | -1.2143% | -0.4537% |
| prefix-repeated-33027 | 33027 | 1.475066x | 1.441852x | -2.2517% | -2.8291% | -2.4350% | -0.4519% |
| prefix-repeated-65535 | 65535 | 1.556018x | 1.525856x | -1.9384% | -0.8596% | -1.1728% | -0.4460% |
| prefix-repeated-65536 | 65536 | 1.573924x | 1.553098x | -1.3232% | -1.3330% | -0.3364% | -0.4498% |
| prefix-repeated-65537 | 65537 | 1.615177x | 1.572438x | -2.6461% | -1.0674% | -1.8323% | -0.4459% |
| prefix-repeated-131072 | 131072 | 1.616565x | 1.605348x | -0.6939% | -1.0944% | -1.0791% | -0.4382% |
| prefix-repeated-262144 | 262144 | 1.652213x | 1.692036x | +2.4102% | +2.0254% | +2.9317% | +0.0552% |
| prefix-repeated-1048576 | 1048576 | 1.680464x | 1.692097x | +0.6922% | +1.0228% | +1.1715% | -0.3305% |

## Remaining brief and next bounded experiment

The identity route is now implemented and measured as a complete compiler experiment, but it has not satisfied the machine/throughput retention criterion. Preserve the useful code-size and short/repetitive-input component. Next test current-interval predicate refinement without introducing transport parameters, then inspect the empty OptimalPathMatches reduction with exact current-image attribution. The supported multi-position recurrence/cold handoff and single R247 placement control remain separate outstanding tasks. Do not repeat phase-only isolation or weaken current-account funding.

The reproducible packet contains the complete 820-file compiler closure, 8-file patch, pins, warning-clean build/test commands, all 81 raw aggregate cohorts/portable fixtures, contrary measurements and current-image aggregate PCs/disassembly. Native binaries/Objects, bytecode, complete compressor descriptors/IR/maps, raw traces/stacks and secrets are excluded. Offline replay reconstructs analysis from committed observations; it does not execute a compressor.

## Public evidence

[Evidence archive](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r252-identity-continuations-99f204a9ca41.tar.gz), 18,490,950 bytes, SHA-256 `99f204a9ca419ae58c6a1b56e5eaafc353ca157e7929628466f81be7ad7afeb7`. The public download verifies all 2,086 hashes, five byte-identical offline reports and reconstruction of all 820 compiler source files from the declared R251 closure. A local prepublication scan rejected historical reference executables accidentally selected as fixture inputs; all were removed before upload. Current raw inputs, expected compressed bytes and matched reference measurements remain intact. Native executables/Objects, bytecode, complete compressor descriptors/IR/maps and raw traces are excluded.

## Selected absolute counters

These medians are from the independent CPU and heap cohorts. They are not multiplied by primary timing or sample shares.

| Case | Roc instructions | Retained R250 instructions | Archived R252 instructions |
|---|---:|---:|---:|
| eight-literals | 1,361,942.0 | 1,225,093.0 | 1,209,076.5 |
| heldout-random-65537 | 21,048,799.0 | 93,190,026.5 | 95,691,264.0 |
| prefix-natural-1048576 | 1,671,024,575.0 | 4,628,762,406.0 | 4,662,036,761.0 |
| prefix-random-1048576 | 320,221,795.5 | 1,514,550,312.0 | 1,552,499,870.5 |

R250 and R252 have identical heap figures below. Requested bytes count allocation capacity; copied bytes count initialized payload copied. Incremental live usable peak is the fixed probe’s heap metric, not operating-system RSS.

| Case | Mech / Roc requests | Mech / Roc requested bytes | Mech / Roc incremental live usable peak | Mech initialized payload copied |
|---|---:|---:|---:|---:|
| eight-literals | 230 / 96 | 9,030,708 / 9,021,944 | 9,192,488 / 9,257,360 | 2,188 |
| prefix-natural-1048576 | 1,556 / 695 | 26,677,303 / 10,390,299 | 10,823,992 / 10,371,440 | 40,676 |
| prefix-random-1048576 | 914 / 373 | 26,493,368 / 10,213,500 | 10,824,168 / 10,371,440 | 15,316 |
