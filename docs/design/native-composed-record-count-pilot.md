# Carry a remaining count through the match-record loop

The trial carries a 16-bit count instead of the current record ordinal. The admitted start is `header - count` in U64 arithmetic. While the count is positive, `header - remaining` reconstructs the old ordinal. Every completed record decrements the count. The original checked reads, zero defaults, child reductions, strict selection, writes and exits stay in order. The candidate removes the recurring header subtraction from the exit comparison and introduces a recurring subtraction to form the record address. This change trades a value in the termination test for a dependency before the load.

| Input | Screen control → trial / Roc | Confirmation control → trial / Roc | Confirmation control → trial / libdeflate |
| --- | ---: | ---: | ---: |
| Eight literals | 0.162011× → 0.163063× | 0.162670× → 0.161592× | 2.081917× → 2.068129× |
| Natural 16 KiB | 1.034457× → 1.057739× | 1.036377× → 1.054688× | 1.045551× → 1.064023× |
| Reymont 16 KiB | 1.037770× → 1.060013× | 1.035039× → 1.058448× | 1.040632× → 1.064167× |
| Natural 256 KiB | 1.081363× → 1.111696× | — | — |
| Random 1 MiB | 0.966215× → 0.964571× | 0.986275× → 0.987907× | 1.399588× → 1.401903× |
| Repeated 16 KiB | — | 0.438679× → 0.427265× | 1.003790× → 0.977673× |

The 18-case screen takes 1.005682× paired control. The eight-case confirmation takes 1.000580×. Both gates pass on all 26 comparisons and the 14,592 calls preserve the exact streams, independent inflation, capacities, aliases and tracked private payload measures. Natural and Reymont 16 KiB are the same bytes under two fixture names. They regress in both runs. The confirmation's repeated-input win is recorded alongside the adverse natural results. The previous compiler is retained.

The measured compiler is `170401d69aaeded4304b2992763dd5dfd7e9aabf`. The source and revision-14 bytecode pins start `331e659e9218` and `51de51658c1d`. The single ARM64 body stays at 1,216 bytes and its local stack adjustment shrinks from 128 to 112 bytes. The control body is independently reproduced from the captured CLIF before replacement. Its two table relocations and all 1,276 linked symbol addresses agree. The candidate Object is `6e3134541f7a7d03cd8a19fbcfa0289b54746f90c62bc97bd04dc1e946bbc3b2` and the worker is `dd32eb3f7a74cb7946cc5dc2eaa6e1b5a59f2c7e588ef2b418e92886aaa09a57`. All 162 linked checks pass. This is a fixed-address diagnostic. The compiler-source patch, hardware counters, and full timing matrix were skipped.

The exact candidate parser accounts for 2,140/4,239 running timer PCs at natural 16 KiB, 2,360/4,730 at Reymont 32,769 bytes and 3,078/5,375 at natural 256 KiB in the separate profiles. Those profiles include validation. The first profiling launch was blocked by the sandbox's process inspection rule. The approved repeat captured all three profiles.

The public download verifies all 106 checksummed files and reconstructs both timing cohorts and three exact-worker profiles. Archive SHA-256 `29ad91f3178709650a1a3c210400d3fa7245a3da2fb6ae321fa2d31719e9c142`. [Timing, profiles, and linked machine code](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-922503c653-second-private-write-composed-record-count.tar.gz).
