# Fold identical routes in composed lookup phases

Keep the [composed countdown baseline](native-composed-countdown-qualification.md). The trial takes **0.998186×** paired control time across all 81 cases. It removes 0.88–1.04% of retired instructions on natural text. Natural 8 KiB and 32 KiB are slower in both elapsed cohorts, and neither peer win count changes within the full comparison. Both 1.05 gates pass on all 81 cases.

| Input | Trial / control | Control / Roc | Trial / Roc | Control / libdeflate | Trial / libdeflate |
| --- | ---: | ---: | ---: | ---: | ---: |
| Eight literals | 1.003900× | 0.170720× | 0.171386× | 2.180349× | 2.188852× |
| Natural 8 KiB | 1.006663× | 1.002128× | 1.008805× | 1.250080× | 1.258409× |
| Natural 32 KiB | 1.004130× | 1.127398× | 1.132053× | 1.023667× | 1.027895× |
| Natural 256 KiB | 0.996652× | 1.146986× | 1.143147× | 0.929065× | 0.925955× |
| Natural 1 MiB | 0.997413× | 1.157240× | 1.154246× | 0.933934× | 0.931519× |
| Periodic 1 MiB | 0.999827× | 0.581781× | 0.581681× | 0.985575× | 0.985405× |
| Repeated 65,535 bytes | 1.000777× | 0.582830× | 0.583283× | 1.026988× | 1.027786× |
| Held-out slides 65,537 bytes | 0.999482× | 1.156788× | 1.156188× | 1.011217× | 1.010693× |

The fresh control and trial each win 52/81 against Roc, 13/81 against libdeflate, and four against both. The retained campaign recorded 16 libdeflate wins in its own cohort. The current pair has no crossings. Forty-three full-suite paired estimates improve. Repeated 8 KiB has the largest increase at 1.024228× control. The largest decrease is repeated 4 KiB, whose order halves are 1.025363× and 0.893548×. The 16-case screen takes 0.998940× control with both gates passing. Natural 256 KiB and 1 MiB improve in the full matrix and slow slightly in the screen. The two runs contain 56,640 and 245,664 timed calls, respectively. All six private payload measures agree on all 81 inputs.

The two bounds predicates in the composed phase that still select an actual load stay in place. Two later branches reach identical endpoints through blocks containing only unconditional jumps. Their endpoint arguments resolve to identical SSA values. The experiment follows those chains and keeps the original first edge when both agree. A block with parameters or another instruction ends the walk, and a cycle declines the fold. The original condition producers stay in the IR for normal effect-aware cleanup. The generic rule runs after immutable lookup composition under the experiment flag.

Four of 357 audited Object bodies change. All 1,348 linked text symbol entries keep their addresses. Each composed body retains its 1,328-byte allocation and 224-byte frame and emits two fewer comparisons. Eight table relocation offsets move by four bytes. The complete linked byte comparison confines the differences to the four bodies, UUID and signature. The continuing scalar loop still uses 11 instructions with zero explicit stack transfers. The pinned Roc scalar loop also uses 11 instructions. The libdeflate parser branches on a strict improvement. The trial changes setup and register allocation inside the composed bodies.

| Separate hardware counter cohort | Instructions before | Instructions after | After / before instructions | After / before cycles |
| --- | ---: | ---: | ---: | ---: |
| Natural 8 KiB | 14,707,441 | 14,578,689 | 0.991246× | 1.006724× |
| Natural 256 KiB | 565,677,153 | 559,808,243 | 0.989625× | 1.001907× |
| Natural 1 MiB | 2,019,124,222 | 2,000,420,060 | 0.990736× | 1.002347× |

Three timer profiles use the exact trial worker. Composed parser bodies account for 2,105/3,864 running PCs on natural 8 KiB, 3,004/5,352 on natural 256 KiB, and zero of 5,280 on repeated 65,535 bytes. Those counts include validation and are separate from the counters. The natural 8 KiB profiles leave the first short visit and the matchfinder as larger targets than the removed comparisons.

The measured parent is `4096fe058eb084ebbf69c262cb5ef930fb0dcf71` and the compiler patch SHA-256 is `006ea1ed92c4ff531c53cb4abfc8e7caebfe9aeb0d81720c3b3d69a1ca967907`. Nightly 2026-03-03 builds the optimized six-feature native compiler with warnings as errors. The revision-14 input produces Object `839e3bd4a67c2f60f399460e5cb821246d8d6fe36b521680cb0282c640d142e8` and linked worker `c50ac99c960b54fd5244d8c66a65de83e3275eb766726e5147c296c708602cd0`. The two descriptor launches pass 324 complete linked checks. The phase oracle checks 68,096 positions with unchanged failure origin and successful retry. Five focused JIT scenarios exercise equal and unequal arguments, a selected store, a trapping condition producer and a cycle. The initial missing trait import and witness link dependency were corrected before successful builds. Source-form emission, the managed guard and full CI were skipped for this declined experiment.

[Compiler patch, raw timings, counters, linked disassembly and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-4096fe058e-composed-route-folding.tar.gz). SHA-256 `ce2f41174c4f499fc2866a850f989de90a71828e545e7b9c7cc627649dbb984d`. Run `python3 -B verify-round.py` after unpacking.
