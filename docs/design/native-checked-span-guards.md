# R300 — Reuse a checked span for fixed-width reads

**Retain R297.** The R300 pilot changes paired elapsed by +1.153% across 27 cases and passes both screen gates on all 27. The 1 MiB random input improves 0.60% and 4 KiB natural text improves 0.69%. Periodic and repeated 1 MiB increase 2.57% and 2.32%.

Measured parent c2aa0ebc8984e608791e0fca42fcc0ea8f44d7b9. Published parent c29fcbaf9acc6e7b87d96a0a92ccdabfc5d8bc99. PR base 92f75db84e2523693648084bea16cd52ad60777a. Apple ARM64 and the same R297 compiler closure, R272 runtime, workload, scratch and complete-call timing boundary. The source patch is against the compiler closure in the R297 evidence packet.

The matcher selects a minimum extent of five, checks that base plus extent does not wrap, and checks that end against the same current input length. The compiler uses those selected facts for the four-byte read at base and the four-byte read at base plus one. The proof follows dominating edges with unique predecessors and exact SSA words. The earlier-match read keeps its guard. Different lengths, a weak extent, an alternate entry and missing overflow evidence keep the existing checks.

One of 347 bodies changes. The vector matcher moves from 13,828 to 13,896 code bytes and from 784 to 768 frame bytes. Across the two selected successful read spans, linked instructions move from 18 to six and stack loads from three to two. This excludes intervening table work. The remaining earlier-match bound is materialized later. The matched C binary directly loads its current and next prefixes after its entry span test. The archived reference and safe Rust controls also identify their current read paths.

The original candidate passes 162 complete linked calls across the unchanged 81 cases, including independent inflate, aliases, capacities and final cleanup. The focused proof test makes 117,128 compiled calls over eight route and geometry controls.

| Paired screen input | R300 / R297 | R300 / Roc | Gates |
|---|---:|---:|---|
| empty | 1.000000× | 1.976190× | pass/pass |
| one-stored | 1.000000× | 1.976190× | pass/pass |
| seven-stored-prefix | 1.000000× | 1.976190× | pass/pass |
| eight-literals | 0.999626× | 0.194117× | pass/pass |
| 33027-periodic-window | 1.017055× | 0.725288× | pass/pass |
| prefix-natural-4096 | 0.993071× | 1.013073× | pass/pass |
| prefix-natural-8192 | 0.999018× | 1.200508× | pass/pass |
| prefix-natural-32768 | 0.997298× | 1.372018× | pass/pass |
| prefix-natural-1048576 | 0.997985× | 1.403071× | pass/pass |
| prefix-random-8192 | 0.994441× | 0.806051× | pass/pass |
| prefix-random-32768 | 0.990595× | 1.364080× | pass/pass |
| prefix-random-1048576 | 0.993963× | 1.706239× | pass/pass |
| prefix-periodic-32767 | 1.027641× | 0.696930× | pass/pass |
| prefix-periodic-32768 | 1.021996× | 0.703035× | pass/pass |
| prefix-periodic-32769 | 1.021813× | 0.707156× | pass/pass |
| prefix-periodic-33027 | 1.012726× | 0.716936× | pass/pass |
| prefix-periodic-65536 | 1.023229× | 0.764208× | pass/pass |
| prefix-periodic-131072 | 1.025544× | 0.791173× | pass/pass |
| prefix-periodic-1048576 | 1.025662× | 0.811556× | pass/pass |
| prefix-repeated-4096 | 1.004610× | 0.404265× | pass/pass |
| prefix-repeated-32767 | 1.022029× | 0.677284× | pass/pass |
| prefix-repeated-32768 | 1.025639× | 0.685611× | pass/pass |
| prefix-repeated-32769 | 1.031055× | 0.704877× | pass/pass |
| prefix-repeated-33027 | 1.020931× | 0.720548× | pass/pass |
| prefix-repeated-65536 | 1.020515× | 0.757718× | pass/pass |
| prefix-repeated-262144 | 1.024016× | 0.804873× | pass/pass |
| prefix-repeated-1048576 | 1.023167× | 0.806152× | pass/pass |

The four time profiles are separate from the elapsed screen and include harness and validation.

| Input | Running samples R297 / R300 | Vector matcher stack-load samples R297 / R300 | Two read spans R297 / R300 |
|---|---:|---:|---:|
| prefix-natural-4096 | 5589 / 5587 | 494 / 523 | 42 / 21 |
| prefix-periodic-1048576 | 5366 / 5430 | 1694 / 2209 | 1 / 0 |
| prefix-random-1048576 | 5389 / 5360 | 1529 / 1487 | 570 / 505 |
| prefix-repeated-1048576 | 5377 / 5411 | 1548 / 2048 | 0 / 0 |

The 1 MiB repeated capture completed. Its first export failed with status 10. Export from that same saved capture succeeded after reclaiming disk space.

The extra stack-load samples in the repetitive cases occur elsewhere in the same matcher body. This pilot stops at the screen. The next pilot reuses the existing checked extent and ordered bases at the repeated immutable scan, where the current body still evaluates four more bounds.

[Verified compiler patch, timing screen and profiles](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r300-checked-span-guards-78b2fdbc00fd.tar.gz). SHA-256 78b2fdbc00fd8990603bb1b4c64282c33438214285042ef89f3bfba0e70eb668.
