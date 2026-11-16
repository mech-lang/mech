# R294 — Reuse geometry facts for immutable captures

**Retain R294 with the R272 runtime.** Candidate paired elapsed changes -0.729% across 81 fixed level-12 cases. The pooled and block gates pass 81/81 and 81/81. R293 beats Roc on 45/81 cases and R294 beats it on 45/81 (55.56%). The separate four-actor cohort gives R294 3/81 cases below libdeflate.

The two primary cohorts change elapsed by -0.649% and -0.809%. The largest paired slowdown is +0.658% on seven-stored-prefix. Cases crossing below 1× Roc are none. Cases crossing above are none.

Pooled gate misses are none. Block gate misses are none.

Published and measured parent a499471188a0f9f3a2662d11ad9883ae460aa657. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R293 closure and a one-file compiler patch. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The level-12 source, inputs, tables, capacities, allocator runtime and complete-call timer are pinned.

## Current proof and generated code

The captured-window proof applies to a read-only, non-appending view. The capture gate reuses a flat-view fact already proved for the same current array in the enclosing scope. A different or unproved view still receives the original row and column check. The gate retains live lengths, offset overflow and the original scalar range checks. General addressing and source-ordered checked fallbacks remain available.

Two of 347 bodies change. The general matcher goes from 51,024 to 50,840 code bytes and from 1,040 to 1,008 frame bytes. The vector matcher goes from 14,308 to 14,184 code bytes and from 800 to 784 frame bytes. Its eight remaining row and column check pairs are removed. The 29 packed loads and 17 source width guards remain in the emitted IR. The root and parser instruction encodings agree with the control.

| Vector matcher instruction class | R293 | R294 |
|---|---:|---:|
| Stack loads | 734 | 726 |
| Stack stores | 146 | 138 |
| Other loads and stores | 467 | 467 |

The exact linked reference loops use flat buffers. The Roc excerpt compares current scalar bounds and uses paired word reads with no shape dispatch. The libdeflate excerpt uses indexed word loads from its two bases. A separate safe Rust witness slices both ranges before reading them. The packet includes the pinned ARM64 excerpts and source for that witness.

The 27-case screen changes paired elapsed by -1.003%. Both timing gates pass all 27 screen cases. The packet preserves every screened and full measurement.

## Headline matched ratios

| Input | R293 / Roc | R294 / Roc | R293 / libdeflate | R294 / libdeflate | R294 / R293 elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.199245× | 0.197678× | 2.509578× | 2.489839× | -1.016% |
| prefix-random-8192 | 0.827767× | 0.834092× | 2.911053× | 2.933296× | -0.618% |
| prefix-natural-4096 | 1.044694× | 1.037683× | 1.794161× | 1.782121× | -0.080% |
| prefix-natural-32768 | 1.419489× | 1.411154× | 1.295558× | 1.287950× | -0.534% |
| prefix-natural-1048576 | 1.453693× | 1.446452× | 1.172063× | 1.166225× | -0.438% |
| prefix-random-1048576 | 1.707284× | 1.711133× | 2.410358× | 2.415792× | +0.329% |
| prefix-periodic-1048576 | 0.822271× | 0.811852× | 1.389592× | 1.371985× | -1.280% |
| prefix-repeated-1048576 | 0.813534× | 0.804603× | 1.278850× | 1.264811× | -1.199% |

The four-actor cohort contains 245,664 measured calls. The reference ratios use its same-case balanced blocks. The paired elapsed-change column and parity score use the two primary cohorts.

## Qualification and independent counters

- All 324 complete original and decoded calls produce the expected byte stream and independently inflate. Capacities and held aliases agree. The two forms have identical Objects and metadata. Both end with zero live private payload and owners.
- Two compiled gate tests check 3,443 inputs, including 2,688 with a selective flat-view fact. Controls cover current row and column views, different-view geometry, rectangular refusal and offset/length overflow. Six composed tests cover nested captured offsets, shape changes, aliases, handoffs and recovery.
- The 87 direct parser controls and 44 cleanup tests pass with warnings as errors. The existing packed-window test rejects missing, stale, wrapping and different-view proofs. The 43,923 window and 3,840 continuation calls pass.
- The logical payload count, requested bytes, copy and zeroing counters, peak private payload and outstanding private bytes agree for all 81 cases.

The initial gate-test filter selected zero cases. The corrected filter executes two tests. The first campaign setup looked for the allocator library in the immediate parent round. The corrected setup verifies the same linked allocator in its original R292 directory. Both attempts are included.

An offline calculation from the completed primary timings briefly overlapped the first CPU counter run. The table uses the isolated repeat. Both complete counter cohorts are included in the packet.

| Input | R293 instructions | R294 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 778,137.5 | 778,192.5 | +0.007% | -1.350% |
| prefix-random-8192 | 5,842,299.5 | 5,851,565.5 | +0.159% | -0.495% |
| prefix-natural-4096 | 10,100,134.0 | 10,060,086.5 | -0.397% | -0.647% |
| prefix-natural-32768 | 85,098,110.0 | 84,361,245.0 | -0.866% | -0.992% |
| prefix-natural-1048576 | 3,095,802,998.0 | 3,061,120,423.0 | -1.120% | -1.263% |
| prefix-random-1048576 | 659,307,962.5 | 661,214,985.0 | +0.289% | +0.222% |
| prefix-periodic-1048576 | 581,436,659.0 | 574,115,546.0 | -1.259% | -1.200% |
| prefix-repeated-1048576 | 588,859,934.5 | 581,572,832.0 | -1.237% | -1.089% |

The natural 1 MiB process lifetime peak physical footprint moves from 54,592,448 to 54,592,448 bytes. This includes the harness.

Timing directions differ between cohorts on 12 cases. The packet retains both observations.

## Exact executable profiles

The CPU-time profiles include harness and validation samples. The counts are locations observed during sampling, independently of the retired-instruction and cycle cohorts.

| Input | Running samples before / after | Vector matcher before / after | Matcher stack loads before / after | Matcher stack stores before / after |
|---|---:|---:|---:|---:|
| eight-literals | 5366 / 5358 | 667 / 638 | 10 / 21 | 2 / 1 |
| prefix-natural-1048576 | 5380 / 5362 | 2273 / 2257 | 808 / 632 | 59 / 66 |
| prefix-natural-32768 | 5375 / 5381 | 2044 / 2032 | 692 / 614 | 79 / 86 |
| prefix-natural-4096 | 5579 / 5605 | 1537 / 1605 | 468 / 447 | 88 / 99 |
| prefix-periodic-1048576 | 5366 / 5377 | 3693 / 3684 | 1995 / 1929 | 207 / 218 |
| prefix-random-1048576 | 5361 / 5373 | 3374 / 3375 | 1491 / 1866 | 486 / 411 |
| prefix-repeated-1048576 | 5365 / 5375 | 3430 / 3418 | 1863 / 1813 | 235 / 200 |

The linked random 1 MiB matcher still loads recurring state from the stack. The most sampled load slot is offset 464, with 759 samples. The count is a sampling observation, independent of dynamic instruction counts.

The 4 KiB natural-text parser is the next reduction target. The R293 profile attributes 2,754 of 5,579 samples to capacity-proved parser routines. The hottest parser has 74 samples at the first tag check and 65 at the second. The packet includes the exact linked addresses and source SSA definitions. The parser body encodings agree between R293 and R294.

The closest candidate case above parity is prefix-natural-4096 at 1.037513× Roc. Preserve an exact Boolean value when all incoming edges pass the same literal. In the current capacity-proved parser, the seed and latch both pass zero to the tag argument using distinct SSA definitions. The generated code still checks it twice. A focused rule should retire the tag and keep the current cursor branch and publication.

The timing scope is the experimental compact entry. Ordinary managed integration remains open.

## All primary timings

| Input | Bytes | R293 / Roc | R294 / Roc | R294 / R293 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.142602× | 2.097770× | 0.979076× | pass/pass |
| one-stored | 1 | 2.031707× | 2.034965× | 1.001603× | pass/pass |
| seven-stored-prefix | 7 | 2.247209× | 2.261996× | 1.006580× | pass/pass |
| eight-literals | 8 | 0.197770× | 0.195761× | 0.989841× | pass/pass |
| eight-overlap | 8 | 0.185397× | 0.183819× | 0.991489× | pass/pass |
| 320-overlap | 320 | 0.213366× | 0.211466× | 0.991096× | pass/pass |
| eight-literals-prefix | 8 | 0.196026× | 0.193984× | 0.989580× | pass/pass |
| 128-biased-dynamic | 128 | 0.257846× | 0.256178× | 0.993532× | pass/pass |
| 32-high-entropy-stored | 32 | 0.262438× | 0.259955× | 0.990540× | pass/pass |
| 16384-distribution-change | 16384 | 1.465412× | 1.462617× | 0.998093× | pass/pass |
| 33027-periodic-window | 33027 | 0.728035× | 0.715516× | 0.982805× | pass/pass |
| random-64 | 64 | 0.275926× | 0.273521× | 0.991284× | pass/pass |
| random-256 | 256 | 0.330454× | 0.328331× | 0.993577× | pass/pass |
| random-1024 | 1024 | 0.387725× | 0.387664× | 0.999842× | pass/pass |
| repeated-1024 | 1024 | 0.261410× | 0.258718× | 0.989706× | pass/pass |
| random-32769 | 32769 | 1.376407× | 1.375393× | 0.999263× | pass/pass |
| repeated-32769 | 32769 | 0.715960× | 0.700999× | 0.979104× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.507496× | 0.506090× | 0.997231× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.368898× | 1.361395× | 0.994519× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.418193× | 1.410986× | 0.994918× | pass/pass |
| heldout-window-32767 | 32767 | 1.420354× | 1.412746× | 0.994644× | pass/pass |
| heldout-window-32768 | 32768 | 1.419311× | 1.410802× | 0.994005× | pass/pass |
| heldout-slides-65537 | 65537 | 1.454091× | 1.447248× | 0.995294× | pass/pass |
| heldout-random-65537 | 65537 | 1.524920× | 1.522209× | 0.998222× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.407811× | 1.402389× | 0.996149× | pass/pass |
| prefix-natural-1024 | 1024 | 0.506335× | 0.506202× | 0.999737× | pass/pass |
| prefix-natural-4096 | 4096 | 1.038340× | 1.037513× | 0.999204× | pass/pass |
| prefix-natural-8192 | 8192 | 1.252625× | 1.247295× | 0.995745× | pass/pass |
| prefix-natural-16384 | 16384 | 1.364797× | 1.359751× | 0.996303× | pass/pass |
| prefix-natural-32767 | 32767 | 1.419150× | 1.412523× | 0.995331× | pass/pass |
| prefix-natural-32768 | 32768 | 1.420542× | 1.412962× | 0.994664× | pass/pass |
| prefix-natural-32769 | 32769 | 1.418116× | 1.408275× | 0.993061× | pass/pass |
| prefix-natural-33027 | 33027 | 1.420004× | 1.411705× | 0.994156× | pass/pass |
| prefix-natural-65535 | 65535 | 1.431990× | 1.425646× | 0.995570× | pass/pass |
| prefix-natural-65536 | 65536 | 1.428670× | 1.422659× | 0.995792× | pass/pass |
| prefix-natural-65537 | 65537 | 1.424167× | 1.417876× | 0.995583× | pass/pass |
| prefix-natural-131072 | 131072 | 1.436767× | 1.430377× | 0.995552× | pass/pass |
| prefix-natural-262144 | 262144 | 1.438130× | 1.433414× | 0.996721× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.453501× | 1.447134× | 0.995619× | pass/pass |
| prefix-random-1024 | 1024 | 0.392996× | 0.391859× | 0.997105× | pass/pass |
| prefix-random-4096 | 4096 | 0.651144× | 0.648023× | 0.995207× | pass/pass |
| prefix-random-8192 | 8192 | 0.820368× | 0.815297× | 0.993818× | pass/pass |
| prefix-random-16384 | 16384 | 1.136221× | 1.134993× | 0.998919× | pass/pass |
| prefix-random-32767 | 32767 | 1.379775× | 1.375132× | 0.996634× | pass/pass |
| prefix-random-32768 | 32768 | 1.371892× | 1.373504× | 1.001175× | pass/pass |
| prefix-random-32769 | 32769 | 1.374992× | 1.376794× | 1.001311× | pass/pass |
| prefix-random-33027 | 33027 | 1.377685× | 1.377428× | 0.999813× | pass/pass |
| prefix-random-65535 | 65535 | 1.554430× | 1.554915× | 1.000312× | pass/pass |
| prefix-random-65536 | 65536 | 1.551131× | 1.549867× | 0.999185× | pass/pass |
| prefix-random-65537 | 65537 | 1.548967× | 1.547714× | 0.999191× | pass/pass |
| prefix-random-131072 | 131072 | 1.646129× | 1.643035× | 0.998120× | pass/pass |
| prefix-random-262144 | 262144 | 1.688698× | 1.691058× | 1.001398× | pass/pass |
| prefix-random-1048576 | 1048576 | 1.707465× | 1.713076× | 1.003286× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.322173× | 0.319845× | 0.992773× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.459765× | 0.451027× | 0.980995× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.533869× | 0.529015× | 0.990909× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.633365× | 0.625566× | 0.987686× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.706514× | 0.699262× | 0.989736× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.709043× | 0.700901× | 0.988517× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.716446× | 0.705978× | 0.985389× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.723869× | 0.716097× | 0.989262× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.770344× | 0.759683× | 0.986161× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.770230× | 0.758114× | 0.984269× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.772530× | 0.759589× | 0.983248× | pass/pass |
| prefix-periodic-131072 | 131072 | 0.801895× | 0.790430× | 0.985702× | pass/pass |
| prefix-periodic-262144 | 262144 | 0.812891× | 0.802237× | 0.986894× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 0.821278× | 0.810770× | 0.987205× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.262194× | 0.259742× | 0.990645× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.404740× | 0.403664× | 0.997342× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.497641× | 0.489569× | 0.983779× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.602037× | 0.594461× | 0.987417× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.683527× | 0.675801× | 0.988697× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.685723× | 0.677403× | 0.987866× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.708080× | 0.699108× | 0.987330× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.723928× | 0.715791× | 0.988761× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.765500× | 0.754279× | 0.985342× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.766659× | 0.755707× | 0.985714× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.764158× | 0.754253× | 0.987038× | pass/pass |
| prefix-repeated-131072 | 131072 | 0.799007× | 0.787758× | 0.985922× | pass/pass |
| prefix-repeated-262144 | 262144 | 0.813459× | 0.803720× | 0.988027× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 0.813678× | 0.803919× | 0.988007× | pass/pass |

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r294-captured-flat-facts-1bb98517f870.tar.gz). SHA-256 1bb98517f870584bd60305f1775a2946b457e4c2a3a477f7a4c0c027de7752d8.
