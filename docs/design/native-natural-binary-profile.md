# Natural-text linked binary comparison

Mech R350 takes 1.222×, 1.220× and 1.235× Roc’s elapsed time on natural 128 KiB, 256 KiB and 1 MiB. Roc takes 0.823×, 0.811× and 0.806× libdeflate’s elapsed time. The existing matched runs use level 12, byte-identical streams, the same input and output capacities, and fresh libdeflate compressor allocation and release in the timed call. The gap requires removing 18.2%, 18.0% and 19.0% of current Mech elapsed time. [Matched campaign and counter evidence](native-ordered-result-extension.md)

| Natural input | Mech / Roc elapsed | Mech / libdeflate elapsed | Mech / Roc instructions | Mech / Roc cycles |
|---|---:|---:|---:|---:|
| 128 KiB | 1.221807× | 1.005364× | 1.320580× | 1.218472× |
| 256 KiB | 1.220096× | 0.988997× | 1.321517× | 1.222103× |
| 1 MiB | 1.234952× | 0.994907× | 1.371156× | 1.235528× |

Mech retires 75.8M, 154.2M and 620.1M more instructions per call while sustaining 8–11% higher instructions per cycle in the separate hardware-counter cohort. The hardware probes use instrumented workers and cover Mech and Roc. This comparison reused that counter cohort and the existing balanced elapsed measurements.

## Fresh profiles

Nine independent Instruments captures use the exact three binaries from the balanced timing. Each capture records five seconds and validates two complete calls. The 47,675 running timer PCs include validation and harness code. The parser and matcher columns are shares of total running timer PCs. The matcher column groups match search and collection in the named routines.

| Natural input | Mech parser / matcher | Roc parser / matcher | libdeflate parser / matcher |
|---|---:|---:|---:|
| 128 KiB | 55.2% / 37.4% | 60.1% / 34.9% | 72.5% / 25.0% |
| 256 KiB | 55.2% / 38.3% | 60.0% / 35.2% | 72.7% / 25.0% |
| 1 MiB | 51.0% / 42.9% | 56.0% / 39.5% | 69.2% / 28.7% |

Scaling the sample shares by the separately measured elapsed ratios locates about half the Mech-to-Roc gap in matching and collection, growing to about 57% at 1 MiB. In units of a complete Roc call, the estimated excess is 0.109× / 0.115× / 0.134× in matching and 0.074× / 0.074× / 0.070× in parsing. At 1 MiB the same estimate puts Mech’s parser at 0.630×, Roc’s at 0.560×, and libdeflate’s at 0.859× of a complete Roc call. These cross-cohort estimates localize the work. Direct phase latency and instruction attribution require separate counters.

## Linked matcher

The two Mech child-update routes at `0x1000682b4–0x10006832f` and their exit predicates at `0x100068330–0x10006835f` account for 484, 568 and 630 running PCs across the three Mech captures. That is 10.2%, 10.6% and 11.8% of all sampled PCs. The initial adjacent tree pair accounts for 23, 23 and 20 PCs.

Mech reloads its tree base from the stack, forms the indexed addresses, and selects the next prefix with a compare, branch and move. At `0x100068330` it materializes the child-range and depth conditions with two `CSET` instructions, combines the Boolean values and compares again. Roc keeps the tree base in `x30` and depth in `x15`, uses indexed halfword accesses and `CSEL`, and tests the exit with `CMP; CCMP; B.cond` at `0x10005c690–0x10005c698` and `0x10005c848–0x10005c850`. The two corresponding Roc tree-store PCs account for 485 samples at 1 MiB. libdeflate carries child pointers and also uses conditional compare in its descent. Prior local branch, conditional-select and child-address transformations showed adverse placement and natural-text timing. The continuing Mech backedge at `0x1000683d8–0x10006840c` also checks the flag again, spills depth and the prior extent, reloads the input pointer and moves five carried values. A conditional-compare selector is a bounded test. Reducing the live state across the full descent loop is the broader target. [Branch experiment](native-direct-predicate-branches.md) · [Child address experiment](native-reused-child-address.md)

## Linked parser

The continuing scalar visit takes 15 instructions in Mech at `0x10006f7d4–0x10006f80c` and 11 in Roc at `0x100064f70–0x100064f98`. Both load two values, add twice, compare the cost and select the cost and winner. Roc carries the actual length, updates the winning values in their existing registers and loops with a countdown branch. Mech rebuilds the length for the winner, copies the minimum and index across the loop edge and jumps back after the exit branch. Mech’s cached offset-cost lookup is five instructions and Roc computes that cost dynamically. libdeflate branches around its winner update when the new cost fails to improve. The Rust scalar control also uses W-register selects. [Parser code and Rust control](native-ordered-result-extension.md)

The earlier equal-work census at natural 1 MiB counted 39.25M candidates in 14.44M nonempty reductions. Four saved instructions on every candidate after the first would cover at most about 99M of the 620M excess instructions before excluding vector paths. The register-join preflight eliminated the explicit widening move while reproducing the earlier widened trial’s hot composed parser machine code, including its extra singleton branch. The preflight passed 162 stream checks and stopped before timing. [Equal-work census](native-work-ledger-prefix-scaling.md)

## Reproduction

The evidence includes capture and sampling scripts, all nine capture and validation receipts, normalized per-PC counts, matched timing and hardware-counter rows, and disassembly of both named routines in each worker. The exact worker hashes are `79193af3953ae66721d8ceb37c36281bb59acaf4362ba3670b00b290dfa25112` (Mech), `ed1ab9175cd175d26d453afc26ba1fa7adb4827ec705aa1cf9bcd231ac84860c` (Roc), and `06e3b4b11fafb5a79ee4a44a0462682662e72f3520b94fd8dd34b6f5e9c2e1d2` (libdeflate).

[Verified profiling and disassembly evidence](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/natural-binary-profile-429eb771f7a2.tar.gz). SHA-256 `429eb771f7a21b6335c22715f2813323f8c98e8547150077d718b21ee74e9cbc`.
