# Share the first ordered-reduction ordinal

Retain the current compiler. The trial takes **1.000384×** paired control time across 20 cases. Both 1.05 timing gates pass 20/20. The same-cohort win counts stay at 7/20 against Roc, 8/20 against libdeflate, and 5/20 against both. The retained full 81-case scores are unchanged.

| Input | Control / Roc | Trial / Roc | Control / libdeflate | Trial / libdeflate | Trial / control |
| --- | ---: | ---: | ---: | ---: | ---: |
| Eight literals | 0.185181× | 0.184566× | 2.343823× | 2.336045× | 0.996681× |
| Natural 8 KiB | 1.005867× | 1.008611× | 1.246261× | 1.249662× | 1.002728× |
| Natural 32 KiB | 1.125696× | 1.129403× | 1.022100× | 1.025466× | 1.003293× |
| Natural 256 KiB | 1.145381× | 1.145857× | 0.923482× | 0.923866× | 1.000416× |
| Natural 1 MiB | 1.153829× | 1.159204× | 0.932031× | 0.936373× | 1.004658× |
| Held-out slides, 65,537 bytes | 1.158588× | 1.156432× | 1.012613× | 1.010728× | 0.998139× |
| Held-out random, 65,537 bytes | 1.039332× | 1.034288× | 1.754429× | 1.745914× | 0.995147× |
| Repeated, 65,537 bytes | 0.561205× | 0.559933× | 0.989382× | 0.987140× | 0.997734× |

The existing proof establishes an unsigned ordered reduction over current readable forward and reverse views. It also recognizes an item formed by ORing the biased ordinal with an invariant word. The trial feeds the same biased ordinal to the first forward load and the selected item's low word. The source checks, cost and tie order, continuing scalar loop, vector route, and ordinary fallback remain in place. This is an emitter experiment selected by `MECH_FIRST_BIASED_INDEX=1`; it does not use a program or source-site identity.

Four of 357 linked bodies change, all composed reductions. All 1,348 named text addresses, the 1,328-byte body sizes, and the 224-byte frames match. Each first visit replaces `add base,#12; ldr [adjusted_base,index,lsl #2]` with `add index,#3; ldr [base,adjusted_index,lsl #2]`. The separately emitted `add wIndex,#3` survives for the selected item. The edit saves no instructions. The linked Roc scalar reduction shares a comparison between both choices; libdeflate branches on a strict improvement and carries its ordinal directly into the lookup and winning item. The pinned Mech continuing scalar loop remains at 11 instructions.

| Separate hardware counter cohort | Trial / control instructions | Trial / control cycles |
| --- | ---: | ---: |
| Natural 8 KiB | 0.999937× | 1.006904× |
| Natural 32 KiB | 1.000004× | 0.999507× |
| Natural 256 KiB | 0.999996× | 0.999947× |
| Natural 1 MiB | 0.999987× | 1.000202× |
| Held-out slides, 65,537 bytes | 0.999960× | 1.000628× |

The archive includes all nine counter cases and six exact-worker time profiles. The main composed parser accounts for 1,673/4,671 sampled running PCs on natural 8 KiB, 2,636/5,358 on natural 32 KiB, 3,012/5,363 on natural 256 KiB, and 2,562/5,364 on held-out slides. The last case also has 2,197 matcher PCs. These are samples including validation, independent of both the primary elapsed and hardware-counter cohorts. All six private payload metrics agree in the 57,024 balanced calls. The best held-out random movement and adverse natural movements are kept in the same packet.

Measured parent `41892af837c0b966c0c9c1b43c0c25c55a2b9cba`; the optimized six-feature compiler uses nightly 2026-03-03 and warnings as errors. The unchanged revision-14 bytecode emits Object `04897a26301a5736701a2a263fdc5710e420436d2c99f4fac541b77408b5ff03`; exact worker `665fc7a196e2a17af723ab748e6df310df6e2dfecdf0bca240ff2101e6c94894`. Two descriptor launches pass 324 linked checks with exact bytes, independent inflation, capacities, aliases, and cleanup. The direct oracle passes 68,096 native positions, preserves rejection origin 15525, and recovers after failure. No full timing campaign, source-form emit, or managed performance run was started for this declined change.

[Compiler patch, raw observations, linked disassembly, profiles, and offline replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-41892af837-first-biased-index.tar.gz). SHA-256 `57cc86e6fdeb20caf353ce34d8321c6b9a0a808af12aa5eab5ab8cb878ce489e`.
