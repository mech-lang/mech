# Enter the scalar countdown at the first short visit

Keep the [composed countdown baseline](native-composed-countdown-qualification.md). The trial takes **1.002170×** paired control time across all 81 cases. Both 1.05 gates pass everywhere. Natural 256 KiB and 1 MiB slow in the screen, full matrix and fixed-placement diagnostic. Preserving the control's function addresses does not restore their time.

| Input | Trial / control | Control / Roc | Trial / Roc | Control / libdeflate | Trial / libdeflate |
| --- | ---: | ---: | ---: | ---: | ---: |
| Eight literals | 0.999151× | 0.171395× | 0.171250× | 2.189150× | 2.187292× |
| Natural 8 KiB | 1.002010× | 0.994515× | 0.996514× | 1.238818× | 1.241308× |
| Natural 32 KiB | 1.009653× | 1.123827× | 1.134676× | 1.021251× | 1.031109× |
| Natural 256 KiB | 1.007999× | 1.146154× | 1.155322× | 0.928878× | 0.936308× |
| Natural 1 MiB | 1.009375× | 1.153502× | 1.164316× | 0.929247× | 0.937959× |
| Repeated 32 KiB | 1.005428× | 0.507269× | 0.510022× | 0.995650× | 1.001054× |
| Repeated 65,537 bytes | 0.999776× | 0.570390× | 0.570262× | 1.004803× | 1.004578× |
| Held-out slides 65,537 bytes | 1.010099× | 1.152700× | 1.164341× | 1.004349× | 1.014492× |

The two full-matrix actors each win 53/81 against Roc (65.4%). In this cohort both beat Roc on natural 8 KiB; in the earlier retained comparison that case was 1.001108×. The 16-case screen starts with the control at 0.999660× Roc and puts the trial at 1.016386×. The direct entry adds no repeatable Roc crossing. In the full matrix Mech's libdeflate win count goes from 16 to 15 and wins over both go from seven to six; repeated 32 KiB crosses upward. Thirty-one cases improve, with the worst increase on periodic 4 KiB at 1.029749× control. The 16-case screen takes 1.003841× control. Both order halves for natural 32 KiB, 256 KiB, and 1 MiB are adverse in the full matrix.

The existing proof admits a positive span and the paired affine reads. Instead of peeling a first visit and branching around scalar setup for a singleton, the trial starts the existing countdown with the original initial best, current item, biased forward ordinal and complete remaining span. The reverse seed is one stride above the first address; the current body subtracts before each load, so the seed is never dereferenced. U32 wrapping, ties, checked fallback, original range proof, vector selection and source work stay unchanged.

Only four composed parser bodies change out of 357. Each shrinks from 1,328 to 1,264 bytes and its frame shrinks from 224 to 208. The continuing scalar loop still has eleven instructions and no explicit stack traffic, matching the inspected Roc loop's instruction count. The root body is identical in the Object. The natural link moves some function addresses. The fixed-placement diagnostic starts with the exact control Object and substitutes the same four bodies with unreachable padding and the equivalent relocation sites. All 1,348 linked text symbol addresses agree with the control. The rest of the executable is byte-identical outside UUID and signature. The eight-case diagnostic takes **1.008887×** control, and natural 256 KiB and 1 MiB are **1.009616×** and **1.009411×** control. This rules out movement of other function addresses as the sole explanation. Internal layout and register allocation still differ.

| Separate hardware counter cohort | Instructions before | Instructions after | After / before instructions | After / before cycles |
| --- | ---: | ---: | ---: | ---: |
| Eight literals | 626,977 | 626,940 | 0.999941× | 1.029623× |
| Natural 8 KiB | 14,708,061 | 14,677,186 | 0.997901× | 1.017011× |
| Natural 256 KiB | 565,704,002 | 564,345,181 | 0.997598× | 1.011018× |
| Natural 1 MiB | 2,019,715,704 | 2,014,567,492.5 | 0.997451× | 1.009741× |

The exact timed-worker profiles place 2,400/4,414 running samples in the composed parser for natural 8 KiB and 3,031/5,350 for natural 256 KiB. No composed body appears in the 5,253 samples for repeated 65,535 bytes. The samples and hardware counters are separate cohorts. Every private payload metric agrees across the full matrix.

Measured parent `9cf7b6fcdaa6ed5792a61375a0fa39552c27b111`; compiler patch SHA-256 `feb1865fc2c1158ec48285f4a6de62e1c78dc181573d856f65903251df268a77`. Nightly 2026-03-03 builds the six-feature optimized compiler with warnings as errors. The unchanged revision-14 bytecode emits Object `8de823f4f6467e91da4e51f3068cd9d660e785273cda34c617413ad62c77fd0d` in 607.18 seconds. The natural and fixed-placement worker hashes are `a2ca308f03448910d021c22b5d36785054dbdc2a9dbfa499035d034bb83a4d43` and `f37ddf73dc51dd45c8dbc86fe453126ac5a6412750e91c92d83a3dbfd4809b09`. The natural build passes 324 linked checks across two descriptors and 68,096 direct oracle positions with original rejection origin and retry. The fixed-placement worker passes 162 linked checks. The three elapsed cohorts retain 56,640, 245,664 and 6,816 timed calls, with exact outputs, independent inflation, capacities, aliases and zero final owners. Original source emission, the managed performance guard and CI qualification were skipped for this declined source patch.

[Compiler patch, all timing observations, linked disassembly and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-9cf7b6fcda-direct-short-countdown.tar.gz). SHA-256 `185295f401e3698a8d4db474dc363619333f795be131278b9bf1c46b5bcce5a5`. Run `python3 -B verify-round.py` after unpacking.
