# Selected byte-pair forwarding pilot

The direct control is the selected-read image from `f3a628b73cb01a7a7d8ee034714ec02b41674a5f`. The trial adapts the archived selected mismatch proof to the current direct unsigned byte load and to mismatch edges that enter a shared continuation directly. It requires compatible plain loads, exact current addresses and readable extents, and a memory-effect-free path. Shared continuation entries without that proof keep their checked reads. The replay on the exact retained matcher IR independently reports two threaded edges, four proved predicates, and 20 repaired block parameters.

The linked mismatch branches go straight to their direction comparison using the original byte values. Each previously reached two more address/bounds checks and two more byte loads. The selected routes have no added spill or reload. Across the whole matcher, the static stack-read count rises from 358 to 366 and the static stack-write count rises from 105 to 111. The matcher grows from 10,284 to 10,308 code bytes and keeps its 576-byte frame. It is the only changed body out of 351.

| Input | Control/Roc | Trial/Roc | Control/libdeflate | Trial/libdeflate | Trial/control |
| --- | ---: | ---: | ---: | ---: | ---: |
| Eight literals | 0.172259× | 0.173218× | 2.205828× | 2.218105× | 1.005566× |
| Natural 8 KiB | 1.058472× | 1.050034× | 1.310644× | 1.300196× | 0.992028× |
| Natural 1 MiB | 1.189993× | 1.183669× | 0.959726× | 0.954625× | 0.994685× |
| Random 1 MiB | 1.409113× | 1.401873× | 1.991035× | 1.980805× | 0.994862× |
| Periodic 1 MiB | 0.592071× | 0.603420× | 1.005191× | 1.024460× | 1.019169× |
| Repeated 256 KiB | 0.598005× | 0.648543× | 0.982623× | 1.065665× | 1.084511× |
| Repeated 1 MiB | 0.593456× | 0.613888× | 0.926586× | 0.958488× | 1.034430× |

The seven-case, 6,720-call pilot measures **1.017441× control**. Both 1.05 gates pass their pooled-median and block-mean estimators on all seven cases. The paired-order estimator records an 8.45% elapsed increase on repeated 256 KiB, which loses its libdeflate win. Three cases improve and four regress. Repeated 256 KiB retires 0.997367× the control's instructions and takes 1.011193× its cycles in the separate counter cohort. The whole-matcher changes make this a failed restoration at the current register allocation. A future retry should eliminate the live continuation transport or demonstrate an improvement under fixed placement.

Each linked worker passes 162 exact-output, independent-inflate, capacity, alias, and cleanup checks. The focused JIT suite passes 180 differential pairs with register allocation checking and warnings as errors. It covers direct and split load forms, direct and corridor mismatch edges, alternate entries with mutations, intervening stores, wrong addresses and lengths, and checked-extent rejection. All measured payload metrics agree. The seven profiles map to the exact timed candidate. The trial is rejected after the targeted screen. Full timing and decoded emission were skipped.

[Patch, replay, linked assembly, and measurement evidence](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-f3a628b73c-selected-byte-pair-pilot.tar.gz). Archive SHA-256 `57ff5fa7ccba343e8a27fe30272fa1221e36db826c0064c7d63bd97c58b5297e`.
