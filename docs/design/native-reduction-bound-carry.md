# Carry the current reduction bound

The compact compiler can represent a recurring unsigned start as `start + bias` in the composed numeric phase. Every reachable incoming value must be a literal that fits after adding the bias, the selected `raw - bias` whose current bounds prove it cannot wrap, or another value in the same verified transfer graph. Selected edges provide the bounds for the current visit. The compiler compares the carried value with raw, subtracts the two directly for the remaining span, and recovers the original value for uses that still require it. The source checks, errors, routes, stores and results retain their original order.

The Apple ARM64 linked executable changes only four composed parser bodies, the Mach-O UUID and signature. Each body shrinks from 1,184 to 1,168 bytes and keeps its 224-byte frame. No linked symbol address moves. The matcher and main entry are byte-identical. The inspected configuration enables `MECH_CARRY_REDUCTION_BOUND=1` alongside the existing level-12 compact options. Disabling it reproduces the previous Object and descriptor exactly.

| Matched cohort | Cases | Timed calls | Elapsed / control | Pooled / block gates | Faster than Roc | Faster than libdeflate | Faster than both |
| --- | ---: | ---: | ---: | --- | ---: | ---: | ---: |
| Full | 81 | 245,664 | 0.995110× | 81 / 81 | 64 → 64 | 32 → 34 | 15 → 17 |
| Full repeat | 81 | 245,664 | 0.993968× | 81 / 81 | 64 → 64 | 33 → 35 | 16 → 18 |

All 81 cases beat at least one peer in both runs. The ratios use balanced block means, with fixed level 12, runtime input, borrowed output, 200 ms warmups, and all compression, scratch construction, output transfer and cleanup inside the same timing boundary.

| Input | Full control → candidate / Roc | Repeat control → candidate / Roc | Full / repeat candidate / libdeflate |
| --- | ---: | ---: | ---: |
| Eight literals | 0.161696× → 0.162826× | 0.160654× → 0.161433× | 2.093832× / 2.055187× |
| Natural 16 KiB | 0.995344× → 0.979267× | 0.995227× → 0.978756× | 0.988480× / 0.989971× |
| Natural 32 KiB | 1.033533× → 1.015619× | 1.034311× → 1.014978× | 0.922921× / 0.920523× |
| Natural 64 KiB | 1.033088× → 1.011236× | 1.032795× → 1.009985× | 0.849657× / 0.850246× |
| Natural 1 MiB | 1.055034× → 1.035682× | 1.055406× → 1.033286× | 0.832094× / 0.832249× |
| Held-out slides | 1.059267× → 1.038485× | 1.057273× → 1.035213× | 0.902557× / 0.904590× |

The first screen measured empty and one stored byte at 1.049786× and 1.040859× their paired controls. In the two full runs the empty-input ratios were 0.994490× and 1.003709×. One stored byte was 1.025479× and 0.978983×. The largest repeat regression was periodic 8 KiB at 1.015222×. The stored cases retire exactly the same instructions before and after. Their code and symbol placement are identical.

In a separate ten-case CPU cohort, natural 1 MiB retires 1,868,566,902 → 1,829,007,036.5 instructions (−2.12%) and uses 543,421,368.5 → 539,781,964.5 cycles (−0.67%). Its pinned reference retires 1,670,771,208 instructions. The held-out slide case retires 107,640,526 → 105,612,979 instructions (−1.88%). The eight-literal counter cohort moves from 170,817 to 177,650 cycles, alongside the 0.5–0.7% elapsed increase in the full runs. All six reported private payload metrics match in a separate ten-case allocation and memory cohort.

On natural 1 MiB, a fresh profile of the measured Mech executable places 2,723 of 5,616 running timer samples in the main composed path parser and 2,399 in the matcher. On the slides case the same two functions account for 4,767 of 5,358 samples. The two corresponding routines in the peer account for 5,131 of 5,354 and 4,964 of 5,298 respectively. The libdeflate path parser and near-optimal compressor account for 5,236 of 5,358 and 5,110 of 5,345. These profiles include validation and are separate from the timed calls and hardware counters.

Two focused compiler tests cover the verified transfers and refusal for an unguarded current bound, an unknown reset and an overflowing seed. Original source and decoded revision-14 bytecode emit the same Object and each pass 162 linked checks, with exact raw bytes, independently inflated output, capacities, aliases and cleanup. The linked parser controls pass 68,096 positions and 288 boundary pairs, including ties, wrapping costs, malformed-record rejection at the same origin, unchanged output and cache, and recovery. The existing compiler-planning, host catalog, interactive architecture, source catalog and production resident-routing checks pass. The unsafe-boundary audit reports the inherited related-post measurement probe outside its allowlist. The checked-in changes are scoped to the compact compiler.

Base `7bd03fbb1f7819eeb6f9f1e734502e89f65c2cf2`. Source SHA-256 `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`, bytecode `51de51658c1d8d0be66567400e382ef5c0f5789baf317c53a8cd56d1df2dd0ca`, resulting Object `b125164d7e8ba9d4e98cb4b467b3194bb14a6d8cafe411600359c6f67943c703`, timed worker `e8308313ae86b282f03119172e1599b66f41eb53c51736a034d82a35d110dab2`. The producer uses nightly 2026-03-03, optimized builds with warnings denied, and the frozen transitive dependency closure.

## Separate ARM64 diagnostic

A fixed-address experiment merges the unsigned comparison and span subtraction at the four new loop entries. It preserves the existing memory accesses and branches to the same zero-trip exit using the flags set by the subtraction. The 17-case screen and repeat take 0.994396× and 0.999085× their already improved paired control. All gates pass and peer win counts agree. Natural 64 KiB improves from 1.017560× to 1.007765× Roc in the screen and from 1.015000× to 1.009817× in the repeat. The experiment changes only the selected instruction spans. Neither its machine edits nor its timing are included in the compiler result.

The [raw observations, patch, counter and sample receipts](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/native-reduction-bound-carry-576ba00e9bfd.tar.gz) verify 354 files. Archive SHA-256 `576ba00e9bfd30252d320aafd5577089deea9ce96ea2e6cc4c7b536b39e12e80`.
