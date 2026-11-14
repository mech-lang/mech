# R295 — Preserve agreeing Boolean literals at joins

**Retain R295 with the R272 runtime.** Candidate paired elapsed changes -0.525% across 81 fixed level-12 cases. The pooled and block gates pass 81/81 and 81/81. R294 beats Roc on 45/81 cases and R295 beats it on 45/81 (55.56%). The separate four-actor cohort gives R295 3/81 cases below libdeflate.

The two primary cohorts change elapsed by -0.482% and -0.567%. The largest paired slowdown is +2.691% on empty. Cases crossing below 1× Roc are none. Cases crossing above are none.

Pooled gate misses are none. Block gate misses are none.

Published and measured parent fc5b9d69b91ef9da037a89a4f827b38f5f91c824. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R294 closure and three changed compiler files. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The level-12 source, inputs, tables, capacities, allocator runtime and complete-call timer are pinned.

## Current proof and generated code

The Boolean SSA analysis previously gave every admitted block parameter the range zero through one. It now keeps the exact literal when every incoming edge supplies that value, including a fresh loop visit. Unknown, differing and exceptional values retain the existing range or are declined. The existing branch cleanup consumes the result. The reached cursor test, checked arithmetic, stores and source failure paths continue in source order.

Four capacity-proved parsers shrink from 1,760 to 1,664 body bytes each with 256-byte stack frames. The matcher encodings agree. The shared pass also changes the root from 997,660 to 998,904 body bytes and from a 57,072- to a 57,104-byte frame. Five of 347 bodies change in total.

| Linked reduction handoff | R294 instructions | R295 instructions |
|---|---:|---:|
| Advance and continue at the next match segment | 9 | 6 |
| Advance and store the finished node | 15 | 8 |

The exact 15296 parser carried a known-zero tag in the cursor dispatch and tested it again before constructing the finished node. The current image continues directly through the cursor comparison and node store. The pinned Roc and libdeflate ordered-candidate excerpts and a compiled safe Rust control show direct scalar selection and explicit cursor advancement. The peer excerpts have different boundaries from these Mech handoff spans.

The 27-case screen changes paired elapsed by -0.263%. Its pooled and block misses are 0 and 0. The packet preserves all observations.

## Headline matched ratios

| Input | R294 / Roc | R295 / Roc | R294 / libdeflate | R295 / libdeflate | R295 / R294 elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.194175× | 0.193589× | 2.480856× | 2.473371× | -0.622% |
| prefix-random-8192 | 0.828606× | 0.836781× | 2.966367× | 2.995631× | +0.072% |
| prefix-natural-4096 | 1.037621× | 1.019462× | 1.754280× | 1.723580× | -1.850% |
| prefix-natural-32768 | 1.410064× | 1.387529× | 1.287794× | 1.267213× | -1.940% |
| prefix-natural-1048576 | 1.450186× | 1.424209× | 1.168464× | 1.147533× | -1.596% |
| prefix-random-1048576 | 1.711824× | 1.713373× | 2.418634× | 2.420824× | +0.136% |
| prefix-periodic-1048576 | 0.811171× | 0.810991× | 1.373230× | 1.372925× | +0.057% |
| prefix-repeated-1048576 | 0.804349× | 0.805647× | 1.263745× | 1.265784× | +0.135% |

The four-actor cohort contains 245,664 measured calls. The reference ratios use its same-case balanced blocks. The paired elapsed-change column and parity score use the two primary cohorts.

## Qualification and independent counters

- All 324 complete original and decoded calls produce the expected byte stream and independently inflate. Capacities and held aliases agree. The two forms have identical Objects and metadata. Both end with zero live private payload and owners.
- Two compiled gate tests check 3,443 inputs, including 2,688 with a selective flat-view fact. Controls cover current row and column views, different-view geometry, rectangular refusal and offset/length overflow. Six composed tests cover nested captured offsets, shape changes, aliases, handoffs and recovery.
- The 87 direct parser controls and 45 cleanup tests pass with warnings as errors. The existing packed-window test rejects missing, stale, wrapping and different-view proofs. The 43,923 window and 3,840 continuation calls pass.
- The logical payload count, requested bytes, copy and zeroing counters, peak private payload and outstanding private bytes agree for all 81 cases.

The new cleanup test executes 48 original machine calls across distinct constant definitions, mixed tags, fresh runtime tags, zero trips and success after rejection. It checks the previous output on rejection. The initial composed-address filter selected zero tests. The corrected filter runs both. Two initial disassembly checks missed embedded zero words and alignment outside the body. The corrected audit bounds the body and reconstructs all its bytes against the linked image and Object.

| Input | R294 instructions | R295 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 778,155.5 | 773,027.0 | -0.659% | +3.403% |
| prefix-random-8192 | 5,850,982.0 | 5,846,034.0 | -0.085% | +0.594% |
| prefix-natural-4096 | 10,057,850.0 | 9,748,133.5 | -3.079% | -1.674% |
| prefix-natural-32768 | 84,357,906.5 | 81,295,319.0 | -3.630% | -2.344% |
| prefix-natural-1048576 | 3,061,209,062.5 | 2,952,819,383.0 | -3.541% | -2.023% |
| prefix-random-1048576 | 661,216,144.0 | 661,098,969.5 | -0.018% | +0.697% |
| prefix-periodic-1048576 | 574,114,347.5 | 574,118,615.0 | +0.001% | +1.351% |
| prefix-repeated-1048576 | 581,567,723.5 | 581,876,245.5 | +0.053% | +0.155% |

The natural 1 MiB process lifetime peak physical footprint moves from 54,559,744 to 54,592,512 bytes. This includes the harness.

Timing directions differ between cohorts on 21 cases. The packet retains both observations.

## Exact executable profiles

The CPU-time profiles include harness and validation samples. The counts are locations observed during sampling, independently of the retired-instruction and cycle cohorts.

| Input | Running samples before / after | All parsers before / after | Advance and cursor dispatch before / after | Final store span before / after |
|---|---:|---:|---:|---:|
| eight-literals | 5358 / 5338 | 12 / 6 | 1 / 0 | 0 / 0 |
| prefix-natural-1048576 | 5362 / 5391 | 2795 / 2772 | 683 / 613 | 157 / 148 |
| prefix-natural-32768 | 5381 / 5380 | 2757 / 2699 | 538 / 509 | 198 / 168 |
| prefix-natural-4096 | 5605 / 5638 | 2776 / 2725 | 195 / 150 | 80 / 62 |
| prefix-periodic-1048576 | 5377 / 5387 | 723 / 713 | 4 / 1 | 0 / 2 |
| prefix-random-1048576 | 5373 / 5360 | 426 / 415 | 0 / 0 | 0 / 0 |
| prefix-repeated-1048576 | 5375 / 5366 | 672 / 681 | 0 / 3 | 1 / 0 |

The pinned random 1 MiB profile from R294 has 759 samples on loads of the same entry-defined input length at stack offset 464. The matcher body is identical in the R295 Object. Two hot width-four paths still form the length-minus-width bound after separately testing the length. The next experiment targets that recurring state.

The closest candidate case above parity is prefix-natural-4096 at 1.025466× Roc. Share an exclusive valid start for fixed-width reads from the same entry-defined length. The retained vector matcher still checks the input length before each width-four window and then separately checks its start. Both misses reach the same zero result. Prove the common continuation and share one total length calculation per immutable view and width.

The timing scope is the experimental compact entry. Ordinary managed integration remains open.

## All primary timings

| Input | Bytes | R294 / Roc | R295 / Roc | R295 / R294 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.099307× | 2.155805× | 1.026912× | pass/pass |
| one-stored | 1 | 2.008844× | 2.038365× | 1.014696× | pass/pass |
| seven-stored-prefix | 7 | 2.276614× | 2.283769× | 1.003143× | pass/pass |
| eight-literals | 8 | 0.196834× | 0.195610× | 0.993784× | pass/pass |
| eight-overlap | 8 | 0.185573× | 0.184716× | 0.995383× | pass/pass |
| 320-overlap | 320 | 0.213015× | 0.213315× | 1.001408× | pass/pass |
| eight-literals-prefix | 8 | 0.195920× | 0.196031× | 1.000569× | pass/pass |
| 128-biased-dynamic | 128 | 0.255280× | 0.254873× | 0.998404× | pass/pass |
| 32-high-entropy-stored | 32 | 0.259883× | 0.259819× | 0.999755× | pass/pass |
| 16384-distribution-change | 16384 | 1.472997× | 1.457402× | 0.989413× | pass/pass |
| 33027-periodic-window | 33027 | 0.719516× | 0.715766× | 0.994788× | pass/pass |
| random-64 | 64 | 0.272503× | 0.273036× | 1.001955× | pass/pass |
| random-256 | 256 | 0.328553× | 0.327581× | 0.997043× | pass/pass |
| random-1024 | 1024 | 0.387890× | 0.388696× | 1.002078× | pass/pass |
| repeated-1024 | 1024 | 0.258846× | 0.258719× | 0.999510× | pass/pass |
| random-32769 | 32769 | 1.371540× | 1.372437× | 1.000654× | pass/pass |
| repeated-32769 | 32769 | 0.702022× | 0.698934× | 0.995602× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.506603× | 0.501982× | 0.990879× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.360732× | 1.335669× | 0.981581× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.409321× | 1.382172× | 0.980736× | pass/pass |
| heldout-window-32767 | 32767 | 1.411472× | 1.383639× | 0.980281× | pass/pass |
| heldout-window-32768 | 32768 | 1.409088× | 1.381459× | 0.980392× | pass/pass |
| heldout-slides-65537 | 65537 | 1.447701× | 1.423884× | 0.983548× | pass/pass |
| heldout-random-65537 | 65537 | 1.524923× | 1.526031× | 1.000726× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.402931× | 1.385098× | 0.987289× | pass/pass |
| prefix-natural-1024 | 1024 | 0.506286× | 0.502420× | 0.992363× | pass/pass |
| prefix-natural-4096 | 4096 | 1.044799× | 1.025466× | 0.981496× | pass/pass |
| prefix-natural-8192 | 8192 | 1.257007× | 1.230898× | 0.979229× | pass/pass |
| prefix-natural-16384 | 16384 | 1.361816× | 1.335871× | 0.980948× | pass/pass |
| prefix-natural-32767 | 32767 | 1.414175× | 1.385176× | 0.979494× | pass/pass |
| prefix-natural-32768 | 32768 | 1.413337× | 1.385915× | 0.980598× | pass/pass |
| prefix-natural-32769 | 32769 | 1.412843× | 1.387072× | 0.981759× | pass/pass |
| prefix-natural-33027 | 33027 | 1.416909× | 1.391966× | 0.982397× | pass/pass |
| prefix-natural-65535 | 65535 | 1.427542× | 1.402476× | 0.982441× | pass/pass |
| prefix-natural-65536 | 65536 | 1.425495× | 1.398455× | 0.981032× | pass/pass |
| prefix-natural-65537 | 65537 | 1.418330× | 1.390744× | 0.980550× | pass/pass |
| prefix-natural-131072 | 131072 | 1.428602× | 1.404933× | 0.983432× | pass/pass |
| prefix-natural-262144 | 262144 | 1.433694× | 1.410061× | 0.983516× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.447257× | 1.424161× | 0.984042× | pass/pass |
| prefix-random-1024 | 1024 | 0.391776× | 0.392922× | 1.002926× | pass/pass |
| prefix-random-4096 | 4096 | 0.650900× | 0.646691× | 0.993532× | pass/pass |
| prefix-random-8192 | 8192 | 0.829503× | 0.830100× | 1.000720× | pass/pass |
| prefix-random-16384 | 16384 | 1.134107× | 1.136395× | 1.002018× | pass/pass |
| prefix-random-32767 | 32767 | 1.378048× | 1.380420× | 1.001722× | pass/pass |
| prefix-random-32768 | 32768 | 1.380350× | 1.382947× | 1.001882× | pass/pass |
| prefix-random-32769 | 32769 | 1.376274× | 1.376965× | 1.000502× | pass/pass |
| prefix-random-33027 | 33027 | 1.380855× | 1.380174× | 0.999507× | pass/pass |
| prefix-random-65535 | 65535 | 1.556647× | 1.553489× | 0.997971× | pass/pass |
| prefix-random-65536 | 65536 | 1.554447× | 1.551150× | 0.997879× | pass/pass |
| prefix-random-65537 | 65537 | 1.554520× | 1.551098× | 0.997799× | pass/pass |
| prefix-random-131072 | 131072 | 1.645494× | 1.646759× | 1.000768× | pass/pass |
| prefix-random-262144 | 262144 | 1.692162× | 1.692642× | 1.000283× | pass/pass |
| prefix-random-1048576 | 1048576 | 1.715315× | 1.717656× | 1.001365× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.320271× | 0.320603× | 1.001039× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.457156× | 0.453377× | 0.991735× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.528587× | 0.527664× | 0.998253× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.624588× | 0.623289× | 0.997919× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.700724× | 0.699392× | 0.998100× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.703297× | 0.702360× | 0.998669× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.708715× | 0.706418× | 0.996759× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.718087× | 0.713798× | 0.994027× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.759965× | 0.756960× | 0.996046× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.762575× | 0.760993× | 0.997926× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.763605× | 0.759592× | 0.994744× | pass/pass |
| prefix-periodic-131072 | 131072 | 0.790953× | 0.790362× | 0.999252× | pass/pass |
| prefix-periodic-262144 | 262144 | 0.803472× | 0.803697× | 1.000280× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 0.810731× | 0.811189× | 1.000565× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.257988× | 0.258028× | 1.000155× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.406517× | 0.399727× | 0.983297× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.490978× | 0.488413× | 0.994776× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.597294× | 0.591755× | 0.990727× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.677518× | 0.677354× | 0.999757× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.680477× | 0.681408× | 1.001368× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.700698× | 0.699494× | 0.998281× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.721172× | 0.717859× | 0.995405× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.753116× | 0.751443× | 0.997778× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.757195× | 0.756040× | 0.998474× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.755675× | 0.752687× | 0.996047× | pass/pass |
| prefix-repeated-131072 | 131072 | 0.788744× | 0.789333× | 1.000746× | pass/pass |
| prefix-repeated-262144 | 262144 | 0.802445× | 0.803955× | 1.001881× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 0.804377× | 0.805466× | 1.001355× | pass/pass |

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r295-agreeing-literal-phis-a0dcfa888024.tar.gz). SHA-256 a0dcfa888024ab0f671ea2daf04241c39843a38c613bdfecf32c894bf2cc8328.
