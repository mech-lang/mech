# Check the composed record span at entry

The fixed-address diagnostic takes **0.999889×** the paired control across 81 cases, with both 1.05 gates passing. It makes natural 256 KiB about 1% faster but repeatedly slows natural 8 KiB and costs one Roc win in the full run. Keep the existing compiler. This commit records the experiment and leaves the [retained scoreboard](native-compression-scoreboard.md) unchanged.

| Cohort | Cases | Timed calls | Trial / control | Roc wins, control → trial | libdeflate wins, control → trial | Either peer, control → trial |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Screen | 16 | 7,584 | 0.997281× | 4 → 3 | 9 → 12 | 11 → 13 |
| Full | 81 | 245,664 | 0.999889× | 53 → 52 | 22 → 22 | 66 → 65 |
| Confirmation | 8 | 6,816 | 0.998551× | 2 → 2 | 3 → 5 | 5 → 7 |
| Adverse check | 10 | 7,008 | 1.000378× | 5 → 5 | 5 → 5 | 9 → 8 |

The full cohort wins against both peers on 9/81 cases in each arm. A fresh reference denominator makes the control's full-cohort counts differ from the published scoreboard. The only full-cohort libdeflate changes at 1× trade natural 33,027 bytes for natural 32,769 bytes. Held-out window 32,767 bytes and natural 33,027 bytes cross under 1× libdeflate in the screen and confirmation but stay above it in the full run.

| Full-cohort input | Control → trial / Roc | Control → trial / libdeflate | Trial / control |
| --- | ---: | ---: | ---: |
| Eight literals | 0.173962× → 0.174371× | 2.180789× → 2.185905× | 1.002346× |
| Natural 8 KiB | 0.997681× → 1.009245× | 1.248386× → 1.262856× | 1.011591× |
| Natural 32,769 bytes | 1.102114× → 1.098897× | 1.000448× → 0.997528× | 0.997081× |
| Natural 33,027 bytes | 1.101395× → 1.102867× | 0.999217× → 1.000553× | 1.001337× |
| Natural 256 KiB | 1.119342× → 1.108160× | 0.900761× → 0.891762× | 0.990010× |
| Natural 1 MiB | 1.124993× → 1.116475× | 0.908976× → 0.902093× | 0.992428× |
| Held-out window, 32,767 bytes | 1.104664× → 1.101628× | 1.004230× → 1.001470× | 0.997252× |
| Held-out mixed, 131,073 bytes | 1.011813× → 1.012260× | 1.173499× → 1.174018× | 1.000442× |
| Periodic 8 KiB | 0.398795× → 0.409946× | 1.266686× → 1.302105× | 1.027963× |

Natural 8 KiB is +0.88% in confirmation and +0.39% in the adverse check. Natural 256 KiB is −0.96% and −0.88% respectively. Periodic 8 KiB is +1.08% in the adverse check. Each set has 24 balanced blocks per case and passes both elapsed gates. The full-run natural 8 KiB direction holds in both order halves (+1.96% and +0.36%).

A header load returns zero on an unreadable ordinal, so a nonzero packed record count implies a readable header. A nonwrapping subtraction proves the preceding consecutive record ordinals are readable in the same cache view. The timed diagnostic checks once for subtraction underflow, reads each record of the proved span directly and forwards the original default record on a wrapped start. That default still visits the original cost reads and malformed-record rejection. A second readability branch with identical effect-free successors is removed. The 1,328-byte body grows to 1,360 bytes, with 16 → 13 static stack references and 23 → 21 conditional branches in VCode. The entry underflow check still uses `SUBS; CSET; UXTB; CBNZ`. The original two GOT relocations move; 1,276 named text addresses are fixed and other linked bytes are verified apart from the UUID and signature. This isolates one composed routine and does not establish general compiler eligibility.

In the separate counter cohort, natural 256 KiB retires 1.004069× instructions in 0.991102× cycles. Natural 8 KiB retires 1.005726× instructions in 0.996432× cycles there, despite its adverse elapsed results. On natural 8 KiB and 256 KiB, the exact trial worker's parser accounts for 1,772/4,976 and 2,966/5,335 running timer samples respectively; those profiles include validation. The counter and sample cohorts are not elapsed replicates.

All 162 linked stream checks, 68,096 direct phase positions and 288 boundary pairs pass. The boundary pairs cover wrapped and exact spans, empty count, unreadable header, malformed length, original rejection origin 15525, unchanged previously published payload and cache, and successful retry. Timings preserve the pinned runtime inputs, Mech/Roc exact bytes, independent inflation, input and output aliases, real capacities, allocator, and complete compact call boundary. Ordinary compiler integration and managed qualification were skipped for this rejected diagnostic.

Measured compiler `6e7dd1bed4614034887d11b574251fa06cc0dff5`; source `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; bytecode `51de51658c1d8d0be66567400e382ef5c0f5789baf317c53a8cd56d1df2dd0ca`. Trial Object `3a5e6ed1ea8f8cab6529871b5707b5280622587faaf7569091570368b657c9d9`; timed worker `c8f44b0e8f6e0256a6798a9719852fd5b72a79c82c5762b6c6a1c324b2597fc8`.

[Matched observations, counters, profile, emitted code and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-6e7dd1bed4-composed-record-span.tar.gz). SHA-256 `679be8f651e937a0105a0c84565eef34c63646478d2368507f00c2959b11ad25`; all 243 files verified after public download.
