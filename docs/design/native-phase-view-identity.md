# R250: preserve current view identity across native phase returns

Retain R250. The large-random64 witness improves **2.970036x -> 2.940361x Roc**, with0.7991% and1.1505% lower balanced elapsed in the two cohorts (0.9991% pooled). Random32 improves1.4520%; random1MiB improves about1%. Periodic33 improves0.5010%, favorable in both cohorts. Eight remains0.502581x. All81 unchanged point/block1.05 gates pass in both cohorts. This is a modest gain; parity remains incomplete.

Git parent `49547c26e237227105f3d32a2d6c054ade716ac6`; PR base `92f75db84e2523693648084bea16cd52ad60777a`. One file changes over the815-file archived R249 GPU source closure: compact/phases.rs. The patch applies to that declared closure, not directly to the maintained tree. Ordinary R215 remains `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`, IR/work/ABI/MBC215/107/39/14. Shared Engine/Core/runtime/dependencies, benchmark source/bytecode, tables, search limits, buffer capacities, source allocation sites, allocator and timing endpoints remain frozen.

## The missing fact and its proof

The current caller reloads six words for every writable descriptor after a successful native phase, even when that phase cannot append or rebind its buffer. It then maps returned frame fields through that packet. The large-random profile puts3351/5437 running samples in the root at R249, with substantial stack-to-stack carrier movement. The actual pinned reference collector/matching binary carries list/table state within its native routine; its existing matching excerpt and fresh collector disassembly are retained. This motivates testing boundary identity reuse, not assuming that all root time is removable transport.

With `MECH_PHASE_INVARIANT_VIEWS=1`, the existing checked numeric proof selects writable arrays with appends=false. That proof rejects array rebinding, swaps or unknown recursive forwarding, checks current schema/provenance and classifies every append/bulk append. The shared numeric emitter changes a descriptor only at its capacity/growth path. Element stores change contents without changing the current six-word view.

The caller therefore keeps that exact input descriptor only on successful phase return and maps its result from the same current words. All result source words are captured before any destination can overwrite them. This preserves parallel transport when results are reordered or aliased. Every appending array still refreshes all six words, even when a particular visit does not allocate. Contents are never cached, uniqueness is not inferred, and no new permission survives a source alias or phase boundary.

All current owner/capacity/geometry/entry-window guards, general matrix fallback, source errors and their lexical origins, settlement/cleanup and success-only output publication stay. No new source operation/phase class is selected; no compressor/input identity selects the optimization. The original callee packet ABI and full output stores stay. This first experiment isolates caller reuse from a future private-ABI change.

Exactly130 phases and179 writable views qualify statically. All136 original numeric/map bodies and7 selected vector bodies are byte-identical to R249, as is every existing numeric phase metadata record after removing the new invariant_input_views field. Root code shrinks5630912 ->5623612 bytes (-7300B), frame55152 ->55136B (-16B). Disabled complete original Object/metadata restore R249 exactly. All four existing growth/live-alias/fanout negative programs also retain exact parent Objects/metadata.

## Fresh complete-call timings and counters

All81 configurations, including the original25 and56 controlled prefixes through1MiB, are remeasured in two primary cohorts. Each has24 balanced blocks, all six parent/candidate/reference orders repeated four times,200ms warmup per actor/case, fixed level12, runtime inputs and matched live borrowed output. Full compression, source scratch construction, output transfer and actual private cleanup are timed. Preparation, validation, logging and prior-output drop stay excluded. CPU and memory are separate24-block cohorts. Builds/tests/diagnostics/profiles do not overlap them.

Ratios use pooled observation medians; elapsed changes use balanced block means. Instruction and cycle cohorts remain separate. Positive changes are retained regressions.

| Case | Roc ms | R249 ms | R250 ms | R249 / Roc | R250 / Roc | Block change | Instructions change | Cycles change |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| eight-literals | 0.290583 | 0.146729 | 0.146041 | 0.504949x | 0.502581x | -1.0791% | -0.6382% | -7.1970% |
| 33027-periodic-window | 1.262125 | 1.941855 | 1.932125 | 1.538560x | 1.530851x | -0.5010% | -0.1904% | -0.3927% |
| repeated-32769 | 1.348291 | 1.971750 | 1.971625 | 1.462406x | 1.462314x | -0.0063% | -0.1926% | -0.5593% |
| random-32769 | 1.066438 | 2.891104 | 2.849125 | 2.710992x | 2.671629x | -1.4520% | -0.3736% | -1.5011% |
| heldout-random-65537 | 1.939104 | 5.759208 | 5.701666 | 2.970036x | 2.940361x | -0.9991% | -0.4332% | -1.0682% |
| reymont-prefix-32769 | 4.373042 | 9.729187 | 9.725771 | 2.224810x | 2.224029x | -0.0351% | -0.2392% | -0.1854% |
| heldout-mixed-131073 | 8.399562 | 20.569355 | 20.505958 | 2.448860x | 2.441313x | -0.3082% | -0.1664% | -0.3763% |
| prefix-natural-1048576 | 161.152250 | 361.245209 | 361.745084 | 2.241639x | 2.244741x | +0.1384% | -0.2165% | +0.1973% |
| prefix-random-1048576 | 28.801437 | 93.656812 | 92.677000 | 3.251810x | 3.217791x | -1.0462% | -0.5083% | -1.1398% |
| prefix-periodic-1048576 | 31.421688 | 55.729937 | 55.747437 | 1.773614x | 1.774171x | +0.0314% | -0.1798% | +0.0799% |
| prefix-repeated-1048576 | 34.326542 | 56.914458 | 56.952124 | 1.658031x | 1.659128x | +0.0662% | -0.1802% | +0.5675% |

All81 descriptive geometric elapsed changes -0.4123%; original large11 -0.3786%. There are25 adverse pooled block means. These averages do not replace the primary individual-case gates. Instructions decrease on all81 measured medians; cycles improve on69 and worsen on12. This does not prove each tiny counter delta is caused by the descriptor mechanism.

Natural1MiB regresses0.1384% in pooled balanced elapsed, with both cohorts adverse. Periodic/repeated1MiB also have small adverse changes (+0.0314%/+0.0662%). Keep them. The large-random instruction reduction is0.4332%, considerably smaller than its elapsed reduction. Eight’s separate cycle reduction is7.1970%, not a7.1970% primary elapsed gain; its initial point/block directions and all original observations remain in the receipts.

All81 payload allocation requests/bytes, initialized copies, zeroing, nonreturned payload, private peaks and listed whole-heap medians remain exactly equal. RSS/physical/lifetime peaks include executable/metadata/harness/input preparation and allocator retention, not just payload. Requested capacity is not bytes copied.

## Actual execution, not just static eligibility

Two observed calls per actor/case compare the retained and candidate phase selections/refusals. All81 repeat exact phase counts and complete output/inflate/alias/capacity/cleanup checks. These324 calls use dedicated observed binaries, never primary timing or hardware cohorts. Logical descriptor-word lanes below are not physical memory traffic or a claimed instruction partition.

| Case | Reused current descriptors per call | Logical descriptor word lanes |
|---|---:|---:|
| eight-literals | 121 | 726 |
| 33027-periodic-window | 33210 | 199260 |
| repeated-32769 | 32990 | 197940 |
| random-32769 | 14357 | 86142 |
| heldout-random-65537 | 40276 | 241656 |
| reymont-prefix-32769 | 54525 | 327150 |
| heldout-mixed-131073 | 130693 | 784158 |
| prefix-natural-1048576 | 1917400 | 11504400 |
| prefix-random-1048576 | 822445 | 4934670 |
| prefix-periodic-1048576 | 1053015 | 6318090 |
| prefix-repeated-1048576 | 1053054 | 6318324 |

Random64 has65533 source search positions but only19852 matching phase1279 visits. Random1MiB has1048572 searches and406968 matching-phase visits. Thus descriptor reuse is not exercised twice for every input byte; a large amount of per-position work stays in the root, including no-match paths. The new census prevents attributing the whole throughput gap to the changed call boundary.

## Every measured configuration

| Case | Bytes | R249 / Roc | R250 / Roc | Block change | Cohort A | Cohort B | Instructions change |
|---|---:|---:|---:|---:|---:|---:|---:|
| empty | 0 | 1.976190x | 1.976190x | +2.0378% | +1.3248% | +2.0188% | -1.4832% |
| one-stored | 1 | 1.976190x | 2.000000x | +3.2157% | +3.6581% | +3.5200% | -1.4508% |
| seven-stored-prefix | 7 | 2.000000x | 2.000000x | -6.6988% | -7.3527% | -5.8723% | -1.4396% |
| eight-literals | 8 | 0.504949x | 0.502581x | -1.0791% | -0.2009% | -2.4257% | -0.6382% |
| eight-overlap | 8 | 0.500217x | 0.500649x | -0.0625% | -0.1794% | -1.4307% | -0.6540% |
| 320-overlap | 320 | 0.565347x | 0.557766x | -1.4170% | -1.0859% | -2.9938% | -0.5222% |
| eight-literals-prefix | 8 | 0.512908x | 0.513124x | -1.2899% | +2.1066% | -5.1491% | -0.6364% |
| 128-biased-dynamic | 128 | 0.512132x | 0.505385x | -0.2762% | +0.8905% | -2.4524% | -0.5848% |
| 32-high-entropy-stored | 32 | 0.539100x | 0.538958x | +0.3384% | +1.1835% | -1.6711% | -0.5050% |
| 16384-distribution-change | 16384 | 2.720206x | 2.699221x | -0.7714% | -0.9943% | -0.5972% | -0.0703% |
| 33027-periodic-window | 33027 | 1.538560x | 1.530851x | -0.5010% | -0.2998% | -0.9346% | -0.1904% |
| random-64 | 64 | 0.574300x | 0.577381x | +0.6957% | +0.1966% | -0.0375% | -0.4368% |
| random-256 | 256 | 0.672940x | 0.687020x | +1.5344% | +1.4667% | +1.0363% | -0.2704% |
| random-1024 | 1024 | 0.746043x | 0.749493x | +0.2746% | +1.8181% | -1.5806% | -0.2226% |
| repeated-1024 | 1024 | 0.634529x | 0.637812x | +0.7683% | +1.3640% | +0.3528% | -0.3781% |
| random-32769 | 32769 | 2.710992x | 2.671629x | -1.4520% | -0.6407% | -1.9189% | -0.3736% |
| repeated-32769 | 32769 | 1.462406x | 1.462314x | -0.0063% | +0.1301% | -0.1951% | -0.1926% |
| reymont-prefix-1024 | 1024 | 0.925415x | 0.924184x | -0.5133% | +0.3804% | -0.7576% | -0.3333% |
| reymont-prefix-16384 | 16384 | 2.189679x | 2.186600x | -0.1406% | +0.0295% | -0.2707% | -0.2421% |
| reymont-prefix-32769 | 32769 | 2.224810x | 2.224029x | -0.0351% | +0.3503% | -0.2623% | -0.2392% |
| heldout-window-32767 | 32767 | 2.226804x | 2.222766x | -0.1813% | -0.0069% | -0.3539% | -0.2359% |
| heldout-window-32768 | 32768 | 2.228070x | 2.230066x | +0.0896% | +0.0578% | +0.1089% | -0.2384% |
| heldout-slides-65537 | 65537 | 2.296276x | 2.299819x | +0.1543% | +0.1057% | +0.1717% | -0.2370% |
| heldout-random-65537 | 65537 | 2.970036x | 2.940361x | -0.9991% | -0.7991% | -1.1505% | -0.4332% |
| heldout-mixed-131073 | 131073 | 2.448860x | 2.441313x | -0.3082% | -0.2357% | -0.4121% | -0.1664% |
| prefix-natural-1024 | 1024 | 0.931719x | 0.926059x | -0.6201% | +0.4475% | -2.0799% | -0.3347% |
| prefix-natural-4096 | 4096 | 1.722769x | 1.703354x | -1.1270% | -0.4099% | -1.4813% | -0.2687% |
| prefix-natural-8192 | 8192 | 1.997591x | 1.997480x | -0.0055% | +0.4065% | -0.5911% | -0.2409% |
| prefix-natural-16384 | 16384 | 2.185716x | 2.180219x | -0.2515% | -0.2934% | -0.2122% | -0.2420% |
| prefix-natural-32767 | 32767 | 2.232155x | 2.230916x | -0.0555% | -0.1060% | -0.0620% | -0.2364% |
| prefix-natural-32768 | 32768 | 2.225039x | 2.222479x | -0.1151% | -0.0253% | -0.1789% | -0.2388% |
| prefix-natural-32769 | 32769 | 2.220557x | 2.221931x | +0.0619% | -0.0087% | -0.0287% | -0.2355% |
| prefix-natural-33027 | 33027 | 2.223841x | 2.219283x | -0.2050% | -0.2273% | -0.1333% | -0.2385% |
| prefix-natural-65535 | 65535 | 2.203302x | 2.206415x | +0.1413% | +0.1953% | +0.0427% | -0.2054% |
| prefix-natural-65536 | 65536 | 2.202691x | 2.208548x | +0.2659% | +0.3204% | +0.2031% | -0.2059% |
| prefix-natural-65537 | 65537 | 2.235467x | 2.234307x | -0.0519% | -0.0325% | -0.0748% | -0.2063% |
| prefix-natural-131072 | 131072 | 2.200669x | 2.205618x | +0.2249% | +0.1060% | +0.3868% | -0.2004% |
| prefix-natural-262144 | 262144 | 2.205837x | 2.211156x | +0.2412% | +0.2298% | +0.2710% | -0.1972% |
| prefix-natural-1048576 | 1048576 | 2.241639x | 2.244741x | +0.1384% | +0.1586% | +0.1175% | -0.2165% |
| prefix-random-1024 | 1024 | 0.742688x | 0.745358x | +0.3336% | +1.3277% | -0.8079% | -0.2173% |
| prefix-random-4096 | 4096 | 1.394145x | 1.359188x | -2.5075% | -3.1066% | -2.4604% | -0.2514% |
| prefix-random-8192 | 8192 | 1.797708x | 1.741983x | -3.0998% | -3.6190% | -2.9920% | -0.2826% |
| prefix-random-16384 | 16384 | 2.311120x | 2.259152x | -2.2486% | -2.5483% | -2.0007% | -0.3182% |
| prefix-random-32767 | 32767 | 2.732192x | 2.688414x | -1.6023% | -1.8160% | -1.2876% | -0.3751% |
| prefix-random-32768 | 32768 | 2.738365x | 2.698794x | -1.4450% | -1.2812% | -1.3746% | -0.3749% |
| prefix-random-32769 | 32769 | 2.713077x | 2.686229x | -0.9896% | -1.0621% | -0.9846% | -0.3731% |
| prefix-random-33027 | 33027 | 2.735747x | 2.691885x | -1.6033% | -1.3547% | -2.2192% | -0.3739% |
| prefix-random-65535 | 65535 | 2.984584x | 2.952039x | -1.0904% | -1.0624% | -1.0712% | -0.4240% |
| prefix-random-65536 | 65536 | 2.989730x | 2.953112x | -1.2248% | -1.2234% | -1.1257% | -0.4274% |
| prefix-random-65537 | 65537 | 2.983336x | 2.947082x | -1.2152% | -1.3600% | -0.8480% | -0.4231% |
| prefix-random-131072 | 131072 | 3.128898x | 3.094665x | -1.0941% | -0.7027% | -1.4423% | -0.4310% |
| prefix-random-262144 | 262144 | 3.180297x | 3.144221x | -1.1344% | -1.0394% | -1.2356% | -0.4551% |
| prefix-random-1048576 | 1048576 | 3.251810x | 3.217791x | -1.0462% | -0.8858% | -1.1503% | -0.5083% |
| prefix-periodic-1024 | 1024 | 0.661034x | 0.664719x | +0.1450% | +2.9333% | -1.7428% | -0.3478% |
| prefix-periodic-4096 | 4096 | 0.996553x | 1.001507x | +0.4971% | +1.3882% | -0.0294% | -0.2441% |
| prefix-periodic-8192 | 8192 | 1.176100x | 1.167607x | -0.7222% | +0.0981% | -1.1769% | -0.2093% |
| prefix-periodic-16384 | 16384 | 1.366129x | 1.360797x | -0.3903% | -0.3854% | -0.6579% | -0.2031% |
| prefix-periodic-32767 | 32767 | 1.527999x | 1.526427x | -0.1028% | -0.5115% | +0.1138% | -0.1917% |
| prefix-periodic-32768 | 32768 | 1.533288x | 1.528650x | -0.3025% | -0.2603% | -0.4822% | -0.1892% |
| prefix-periodic-32769 | 32769 | 1.535096x | 1.522379x | -0.8284% | -0.2717% | -1.3663% | -0.1903% |
| prefix-periodic-33027 | 33027 | 1.535069x | 1.529053x | -0.3919% | -0.3934% | -0.3815% | -0.1902% |
| prefix-periodic-65535 | 65535 | 1.652883x | 1.651325x | -0.0943% | +0.0377% | -0.3272% | -0.1849% |
| prefix-periodic-65536 | 65536 | 1.654237x | 1.650441x | -0.2294% | -0.2515% | -0.1023% | -0.1849% |
| prefix-periodic-65537 | 65537 | 1.647772x | 1.647594x | -0.0108% | +0.1664% | -0.1925% | -0.1866% |
| prefix-periodic-131072 | 131072 | 1.715809x | 1.713796x | -0.1173% | -0.1393% | -0.0243% | -0.1849% |
| prefix-periodic-262144 | 262144 | 1.745634x | 1.748351x | +0.1556% | +0.1533% | +0.2488% | -0.1802% |
| prefix-periodic-1048576 | 1048576 | 1.773614x | 1.774171x | +0.0314% | +0.0314% | +0.0497% | -0.1798% |
| prefix-repeated-1024 | 1024 | 0.634182x | 0.638578x | +0.6985% | +3.0567% | -2.0272% | -0.3797% |
| prefix-repeated-4096 | 4096 | 0.981718x | 0.969152x | -1.2800% | -0.8819% | -1.6201% | -0.2557% |
| prefix-repeated-8192 | 8192 | 1.141722x | 1.147352x | +0.4931% | +0.5462% | -1.0631% | -0.2158% |
| prefix-repeated-16384 | 16384 | 1.309819x | 1.307220x | -0.1984% | +0.4436% | -0.5019% | -0.2073% |
| prefix-repeated-32767 | 32767 | 1.463230x | 1.453551x | -0.6615% | -0.5422% | -0.0998% | -0.1938% |
| prefix-repeated-32768 | 32768 | 1.461309x | 1.451493x | -0.6718% | -0.8544% | -0.5016% | -0.1938% |
| prefix-repeated-32769 | 32769 | 1.466813x | 1.456783x | -0.6838% | -0.9396% | -0.5540% | -0.1926% |
| prefix-repeated-33027 | 33027 | 1.462116x | 1.460256x | -0.1272% | -0.7726% | +0.2707% | -0.1924% |
| prefix-repeated-65535 | 65535 | 1.567145x | 1.563789x | -0.2141% | +0.0802% | -0.4848% | -0.1862% |
| prefix-repeated-65536 | 65536 | 1.568104x | 1.564592x | -0.2239% | -0.0279% | -0.3508% | -0.1860% |
| prefix-repeated-65537 | 65537 | 1.563703x | 1.567036x | +0.2132% | +0.2603% | +0.1527% | -0.1855% |
| prefix-repeated-131072 | 131072 | 1.623119x | 1.622337x | -0.0482% | -0.0291% | -0.0274% | -0.1830% |
| prefix-repeated-262144 | 262144 | 1.654014x | 1.655311x | +0.0784% | +0.0112% | +0.1400% | -0.1799% |
| prefix-repeated-1048576 | 1048576 | 1.658031x | 1.659128x | +0.0662% | +0.0853% | +0.0151% | -0.1802% |

## All current below-reference configurations

Original25 and prefix56 remain separate groups with some duplicate inputs. Below1x is a point estimate, not a new distinct workload or a guarantee for every input at that size. The largest winning size stays4KiB for repetitive families; text/random lose before that.

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

## Fresh exact-executable profiles

Own executable symbols, actual image slide and own Object map these Timer Fired/Running PCs. The profile includes harness/validation; sample shares are not per-function retired instructions or removable-time estimates.

| Case | Running samples | Most sampled symbols (weights) |
|---|---:|---|
| 33027-periodic-window | 5583 | `_mech_compact_closed_v2` 2350; `_mech_compact_numeric_2624_vector` 1672; `_mech_compact_numeric_15296` 807; `_mech_compact_numeric_12868_vector` 169 |
| eight-literals | 5524 | `_mech_compact_closed_v2` 1658; `__RNvNtNtCselbn9KGBQg0_11miniz_oxide7inflate4core9init_tree` 64; `_mech_compact_numeric_17556` 55; `_mech_compact_numeric_29155` 34 |
| heldout-random-65537 | 5470 | `_mech_compact_closed_v2` 3333; `_mech_compact_numeric_15296` 729; `_mech_compact_numeric_15737` 621; `_mech_compact_numeric_1279_vector` 408 |
| reymont-prefix-32769 | 5353 | `_mech_compact_numeric_15296` 2623; `_mech_compact_numeric_1279_vector` 1206; `_mech_compact_closed_v2` 1160; `_mech_compact_numeric_15737` 85 |

## Qualification and exact scope

- Nightly2026-03-03 AArch64 macOS, compiler opt3/test root opt1, warnings-as-errors, six features aot/jit/canonical-native/canonical-native-runtime/canonical-native-program/canonical-native-program-runtime. Every command/environment/dependency hash is retained in build specs and receipts.
-703 maintained tests pass, with the same seven inherited exclusions listed in maintained-native-tests.json. No green full CI is inferred.
- Before functionality, the retained parent fixed-view source witness runs100 native and100 resident comparisons in original/decoded mode. The new dual-buffer witness adds10808 native controls across normal/observed, parent/candidate and original/decoded. It independently verifies current values, rectangular geometry, reordered results, multiple private/shared owners, assertion/bounds/overflow order, held inputs/actual prior returned backing, failure output sentinels, cleanup and recovery. Each observed form includes51 shared fallback then selected-native paths in one invocation.
-1700 fresh unchanged existing matrix native and1700 resident comparisons pass;1700 parent native rows are reused by hash from R249.416 fresh boundary calls and30 original/decoded image pairs pass. Growth and return-fanout negatives preserve the complete parent Object and metadata. Existing allowances/expected values and default-cap denials remain unchanged.
-162 normal complete-call smokes, every all81 retained measurement and324 observed phase-count calls match exact reference bytes, independent inflate, live aliases, capacity and final zero private ownership. Portable deduplicated input/prefix/expected fixtures accompany the packet. Raw profile transport is losslessly compressed locally; no binary/Object/bytecode/raw trace is published.
- The first new dual-buffer test used a tuple-only return and the clean parent selected no numeric phase. It was a valid source program, not a compiler regression or a failed execution. A flat record using the existing pinned representation makes the intended positive class explicit; the original fixture and emission receipt are preserved. No parser/compiler support was added to accommodate the test.
- No introduced semantic failure remains. Seven broad exclusions and the additional full disabled decoded image remain explicit skipped coverage; small original/decoded images and the pinned decoded MBC14 input are qualified. OrdinaryIR215 exact resource/accounting/admission/canonical-publication/recovery remains separate and omitted by this compact entry. No allowance was increased.

Commands: run-spec.py candidate-gpu/candidate-producer/candidate-tests and maintained-native-tests receipt; qualify-writes.py, qualify-exits.py, qualify-two-view-normal.py and dual-view receipts; measure-primary-a.py/measure-primary-b.py, measure-cpu.py/measure-memory.py; measure-phase-transport.py; profile-compact-pcs.py/analyze-pcs.py. Numerical replay executes analyze-primary.py, analyze-counters.py and make-report.py without running a compiler/compressor. Resolve portable fixture paths against the evidence directory for executable reproduction.

## Remaining large-input target

The current random64 profile still puts3333/5470 running samples in generated root code. Text still favors path15296. The current view optimization removes only a small fraction of the remaining large-input cost. Preserve it and every contrary row rather than imply parity.

Next inspect source-to-machine root attribution and the treatment of native-phase refusal paths. The caller creates an unmarked original-source fallback, while the actual reference contains cold shims. Current matching gates have zero refusals in these complete large witnesses, but other phase classes do fall back; any placement experiment must keep those paths executable and test shared/matrix/failure/retry behavior. Marking source fallback cold is not implemented here and has no claimed speedup. Existing local-phi simplification already solves phi-only dependencies; do not propose it again as an unexplored SCC optimization.

Phase-success uniqueness authority and individual Mech per-block history remain unimplemented. Source arrays, algorithm and timing boundaries stay pinned. Continue toward large-input parity with all81 and the eight-byte1x ceiling intact.

## Reproduction identities

- source: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`
- mbc14: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`
- normal_object: `12127a7982dd982db282d993895035656da96eace5f151426436cbd5ac2c290f`
- normal_metadata: `642fd888084b5ba6285a0ef4b9afbf0740c71b9f6a559785963ba7d4dc1af13e`
- normal_worker: `8d9abe2beca132f0d86f1f7119216a0980b5e06b325492ff232b74ce8a1ca81a`
- runtime: `bbc12169d120eaa60af57904de32f599d594e24c80201f1173991a05746ff066`
- compiler_patch: `42491fd9dd6c1d77aea657389f35b4fdd9861da6e24e2356e47912277b1294ca`
- compiler_source_manifest: `787f115edc0016839ffc88c6bd5db3a0e53f7677bb93ce0585ff3e6dd48ca9be`
- reference_worker: `ed1ab9175cd175d26d453afc26ba1fa7adb4827ec705aa1cf9bcd231ac84860c`
- pinned_allocator: `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`

## Published review packet and updated work order

[Verified evidence archive](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r250-phase-view-identity-2dafa325194e.tar.gz) (10,722,927 bytes), SHA-256 `2dafa325194e930c8bee402f924a553f8d2f70ac004c726cc5d1c5e0b36544ff`. The downloaded public packet verifies all 2,542 file hashes and five byte-identical numerical reports. Applying its one-file patch to the declared R249 closure reconstructs all 815 tested compiler source files. Full executable images, bytecode and raw traces are excluded.

The new review brief supersedes the earlier speculative next step in the sealed report: first map the zero-trip skip identity route in source, CLIF and machine code, using paired conditional/always-entered source witnesses and a separate exact route observer. Only implement generic identity-result forwarding if meaningful work survives. Then examine the empty path reduction. Recover the current enclosing collector refusal before a bounded multi-position recurrence experiment; keep the R247 placement control separate. Cold layout remains an unimplemented alternative, not the selected next compiler change.
