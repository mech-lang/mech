# Compile the first packed reverse read

The compiler-emitted trial takes **1.000810×** the paired control across all 81 cases. Both 1.05 timing gates pass throughout. The natural-text target improves across the screen, full matrix and confirmation. The full run gains two libdeflate wins with no losses; neither crossing held in every cohort.

| Input | Control / Roc | Trial / Roc | Control / libdeflate | Trial / libdeflate | Trial / control |
| --- | ---: | ---: | ---: | ---: | ---: |
| Eight literals | 0.170963× | 0.172446× | 2.166049× | 2.184845× | 1.008677× |
| Natural 8 KiB | 1.005475× | 1.009695× | 1.252597× | 1.257854× | 1.004197× |
| Natural 32 KiB | 1.125112× | 1.116304× | 1.024015× | 1.015999× | 0.992172× |
| Held-out slides, 65,537 bytes | 1.155412× | 1.142990× | 1.009884× | 0.999027× | 0.989249× |
| Held-out mixed, 131,073 bytes | 1.030667× | 1.022496× | 1.193679× | 1.184215× | 0.992072× |
| Natural 256 KiB | 1.146571× | 1.133146× | 0.929870× | 0.918982× | 0.988291× |
| Natural 1 MiB | 1.154470× | 1.145485× | 0.929910× | 0.922673× | 0.992217× |
| Periodic 256 KiB | 0.568479× | 0.567149× | 1.001801× | 0.999458× | 0.997661× |
| Repeated, 65,537 bytes | 0.557827× | 0.559919× | 0.986369× | 0.990068× | 1.003750× |

The matched control and trial each win 52/81 (64.2%) against Roc. Libdeflate wins move from 18 to 20/81 (24.7%); both from nine to ten, and either from 61 to 62/81 (76.5%). The historical retained campaign recorded 19 libdeflate wins. Its denominator is separate. The 20-case screen, eight-case confirmation and ten-case adverse-case check take 0.998183×, 0.995486× and 0.997340× control. The four elapsed cohorts contain 57,024, 245,664, 55,872 and 7,008 timed calls.

Held-out slides takes 0.999027× and 0.998062× libdeflate in the full matrix and adverse-case check, and 1.000196× and 1.003214× in the screen and confirmation. It is consistently 0.78–1.08% faster than its paired Mech control. Periodic 256 KiB returns to 1.003013× libdeflate in the adverse check, losing the narrow win there. Natural 8 KiB reaches 1.000663× Roc in that same check; it is 1.002529× in confirmation. The largest full-matrix slowdown, periodic 8 KiB, falls from +2.97% to +0.73% in the adverse check. Eight literals slows 0.21–1.26% across the cohorts and remains below 0.177× Roc. All original gate outcomes and both order halves remain in the archive.

The compiler implements the [single-function pilot](native-first-packed-reverse-pilot.md) using the existing current-window and complete-child proofs. It requires an admitted 32-bit forward window and 64-bit reverse window with the same affine bias, a projected low-word cost and an index-independent other item operand. The reverse address now uses the scale-eight native load for the full initialized cell and immediately projects its low word. Forward load order, strict unsigned ties, source failure and accounting remain unchanged. Only the short first visit changes in the emitted IR. The compiler enables the experiment through `MECH_FIRST_PACKED_REVERSE_READ=1`.

The linked byte comparison confines all differences to the four composed parser bodies, UUID and signature. All 1,348 named text addresses, 1,328-byte bodies and 224-byte frames stay fixed. The first visit loses one instruction without a new recurring stack transfer. The continuing scalar loop retains 11 instructions; the pinned Roc loop also has 11. The libdeflate scalar path keeps indexed addresses and branches on a strict improvement. Exact assembly for the retained and candidate parser and peers is in the measurement packet.

| Separate hardware counter cohort | Trial / control instructions | Trial / control cycles |
| --- | ---: | ---: |
| Eight literals | 0.999500× | 1.097415× |
| Natural 8 KiB | 0.994674× | 0.999080× |
| Natural 32 KiB | 0.993489× | 0.992871× |
| Natural 256 KiB | 0.992502× | 0.990907× |
| Natural 1 MiB | 0.992755× | 0.993274× |
| Held-out slides, 65,537 bytes | 0.993639× | 0.991290× |

The eight-literal counter excursion is from a separate single-call hardware cohort. The packed path does not run there. It is recorded rather than treated as an explained elapsed cost. The six private payload metrics agree on all 81 cases. The exact candidate's main composed parser accounts for 1,694/4,840 running timer PCs on natural 8 KiB, 2,950/5,360 on natural 256 KiB and 2,517/5,361 on held-out slides. The matcher accounts for 1,354, 2,009 and 2,267 in the same captures. Profiles include validation and are separate from the counters and timing.

The measured compiler base is `8ede8f7c4e2f603e53a3a3deeb6838f751add3c2`. The two-file patch SHA-256 is `4e28cb63884dd1e3c31a8b746e0164799056381ad3035b319ea7427b76b90904`. The optimized six-feature build keeps nightly 2026-03-03 and warnings as errors. Pinned revision-14 bytecode emits Object `71efb70e6aad57131a522e4771fa5bbaa62fb779947b15a3ac73a9f77ebb5476` and worker `b728bb65f848ab068ec9d87f12e92259a257f5b61af4e25dc29852aacdef6931`. The measured worker passes 324 complete linked checks, 68,096 direct phase positions, and the focused upper-word poison comparison (68,096 positions per implementation), with original rejection origin and recovery. Source and decoded revision-14 bytecode reproduce this exact Object and worker. With the option unset, the same producer restores Object `dea88d78758f9d72251d7bbbb41bedcd9cabed2d143cb092b8cbf89625ed9ea1` and the retained decoded metadata and bytecode exactly.

The `build.py` original, decoded and flag-off receipts record the producer command and feature settings. Across the two input forms and both descriptor modes, linked checks pass **648/648** complete streams with exact reference bytes, independent inflation and intact input and borrowed output aliases. The linked phase witness checks **136,192** positions in total, retains rejection origin 15525 and recovers on retry. The separate managed benchmark passes **88/88** exact output checks and preserves all 13 bundle hashes; its primary and confirmation times are 0.710479× and 0.704092× Rust. Changed-file rustfmt and the resident routing, host catalog, interactive architecture and compiler planning quarantine checks all pass. The public qualification packet records the commands and counts and replays with `python3 -B verify-round.py`.

Both peers were sampled separately at their exact timed executable hashes. On natural 8 KiB, the parser accounts for 1,992/4,319 Roc samples and 3,746/5,260 libdeflate samples; on held-out slides the counts are 2,774/5,336 and 3,592/5,367. These samples include validation. The matcher remains the other major hot region.

Retain the option for the repeated gains on larger natural text and held-out slides. The full suite is effectively flat (+0.081%), and the eight-literal and natural 8 KiB regressions remain in the record. Leave the option disabled by default.

[Compiler patch, all elapsed observations, counter and profile receipts, and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-8ede8f7c4e-compiler-packed-first-reverse.tar.gz). SHA-256 `251014dd698086dd3a22c556c66bbb7ae21691829ccaa08c4d35a32e0c9b07d6`.

[Source, flag-off, managed and exact-peer qualification packet](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-8ede8f7c4e-packed-first-reverse-qualification.tar.gz). SHA-256 `d110ba75bd517ac7eebeb55d989ee6cb3a3a7a443f00e4761d8e961b31902bf1`; all 95 files verified after downloading the public archive.
