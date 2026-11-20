# Combined packet selection and conditional winner reconstruction

**The qualified selection remains 63,612 B / 81 Roc / 62 libdeflate. There is no 63rd C win.** This study keeps all twenty complete discovery orders, including the slower conditional alternatives. The smaller combined word candidate is **62,796 B**, or **1.088× Roc / 1.694× libdeflate encoder text**, but remains unpromoted pending full qualification. The original 2,719-line Mech codec and canonical bytecode are unchanged. No LLVM dependency is added.

## Combined existing optimizations

The word combination adds scorer packet qualification to the nine-family selection. It saves another **48 B**, takes about **0.98–1.09% less time on distribution change**, and leaves natural 4 KiB nearly unchanged against its exact 62,844 B control. It remains **1.025099× / 1.034749× C** on natural 4 KiB. The same combination is slower in the probe recipe; both remain available.

## Conditional reconstruction and repair

Source `6bf204ff21e32c9f1f6d479948ee4b92eda92bd8` adds `MECH_ORDERED_SKIP_UNCHANGED_VECTOR_WINNER=1`. Vector costs are seeded with the incoming best, and indices change only on strict improvement. Therefore an unchanged minimum can retain the existing missing sentinel without reconstructing the winning index. The source keeps the scalar remainder, stable earliest-index ties, errors and exact settled accounting. Sixteen optimized tests pass, including **8,192 native fixture calls**; disabling the option reproduces all three artifacts for eight reference recipes.

That first version grows each encoder by 32 B and keeps the 224 B scorer frame. It skips the expected 2,101 of 3,550 index decodes on natural 4 KiB, but its extra move, comparison and branch cancel the instruction saving. Its natural-data latency is worse against each matching control. It is retained, not silently discarded.

Source `46dc3230fce4cdf4d184fedf261b854b5c8f59fd` repairs the same candidate with `MECH_ORDERED_SKIP_UNCHANGED_VECTOR_WINNER=item`. The selected item is pure and source-observable only on a strict winner edge. Its expression now runs there, while the unchanged edge carries the complete old item directly. This also removes the redundant changed-item comparison and selection on the winner edge. Existing mode `1` retains its original artifacts.

Seventeen optimized tests pass, including **16,384 native calls** checking unchanged full-width items, unsigned extremes, earliest cross-lane ties and guarded storage. Disabled and older index-only modes reproduce all three artifacts for twelve recipes. The repair recovers all 32 B of growth and lowers the scorer frame to **208 B**. Overall measured peak stack remains **4,736 B**. It reduces machine work but still does not establish a natural-data latency win or additional C win.

## Runtime discovery

Every candidate uses twenty-one cases, both complete orders and twenty-four balanced blocks per case. Each row has its matching held-feature control. Comparisons against C from separate cohorts do not establish causal differences between variants. All original observations remain; every previously required peer win observed in these twenty orders remains a point win. Full 81-case confidence qualification is outstanding.

| Candidate | Encoder text | Natural 4 KiB / C, forward / reverse | Natural 4 KiB / matching base | Distribution change / matching base |
| --- | ---: | ---: | ---: | ---: |
| select-packet-word-combined-profiled | 62,796 B | 1.025099× / 1.034749× | 0.999921× / 0.998282× | 0.990221× / 0.989089× |
| select-packet-probe-combined-profiled | 63,068 B | 1.024299× / 1.037499× | 1.006983× / 1.013806× | 1.035476× / 1.037230× |
| defer-winner-probe-profiled | 63,468 B | 1.024120× / 1.042944× | 1.003058× / 1.025479× | 1.043044× / 1.023337× |
| defer-winner-word-profiled | 62,876 B | 1.045650× / 1.038635× | 1.016230× / 1.011602× | 1.026156× / 1.034607× |
| defer-winner-combined-word-profiled | 62,828 B | 1.039624× / 1.037577× | 1.017935× / 1.006192× | 1.037709× / 1.044754× |
| defer-winner-combined-probe-profiled | 63,100 B | 1.029916× / 1.040215× | 1.009327× / 1.011332× | 1.009503× / 0.997796× |
| mapped-item-probe-profiled | 63,436 B | 1.017515× / 1.034193× | 1.011305× / 1.013784× | 1.032676× / 1.028297× |
| mapped-item-word-profiled | 62,844 B | 1.031252× / 1.054411× | 1.009674× / 1.007031× | 1.014452× / 1.019222× |
| mapped-item-combined-word-profiled | 62,796 B | 1.028957× / 1.040244× | 1.014396× / 1.011589× | 1.034517× / 1.038061× |
| mapped-item-combined-probe-profiled | 63,068 B | 1.025981× / 1.032432× | 0.999945× / 1.009185× | 0.997470× / 0.995880× |

## Actual machine work

These are untimed **pre-layout word-scorer** diagnostics on natural 4 KiB, not cycle forecasts or the placed workers' instruction totals:

| Metric | Matching base | Index-only guard | Complete-item guard |
| --- | ---: | ---: | ---: |
| Instructions | 3,407,185 | 3,407,330 | **3,387,758** |
| Branches | 569,130 | 572,680 | 572,680 |
| Horizontal minima | 7,100 | 4,999 | 4,999 |
| Winner comparisons | 3,550 | 1,449 | 1,449 |
| Stack loads | 17,305 | 17,305 | 17,300 |
| Stack stores | 18,470 | 18,470 | **17,305** |

The full repair saves **19,427 instructions and 1,165 stack-store operations** against its matching base, but adds **3,550 conditional branches**. The latency results show why fewer instructions alone are insufficient. Fresh source-bound layouts train on all six declared cases. Cases with no scorer calls remain archived with zero training weight; the initial normalization failure and its fix are preserved.

## Complete builds on Apple M1

Fourteen balanced serial fresh-process trials per actor include source compilation and final executable linking. Compilers and fixed support are prebuilt equally; bootstrap and strip are separate. The latest compiler reproduces older candidates exactly, including the index-only mode. Every newly emitted object, complete metadata and bytecode is hash checked. No runtime acquisition, test/build job, counter, profile preparation or archive compression overlaps this cohort.

| Actor | Source through final link | Through strip | Paired build / Roc / C |
| --- | ---: | ---: | ---: |
| mech-control-reference | 2.212275 s | 2.223217 s | 0.740639× / 4.587332× |
| mech-combined-word | 2.216470 s | 2.227483 s | 0.741495× / 4.600308× |
| mech-index-only | 2.217570 s | 2.228535 s | 0.742017× / 4.603943× |
| mech-mapped-probe | 2.218824 s | 2.229899 s | 0.742600× / 4.606841× |
| mech-mapped-word | 2.215644 s | 2.226644 s | 0.741685× / 4.605130× |
| roc | 2.990601 s | 3.001537 s | — |
| libdeflate | 0.481984 s | 0.492765 s | — |

All Mech stripped executables remain **276,072 B**, Roc **191,912 B**, and C **155,752 B**. Peer encoder text is **57,696 B Roc / 37,080 B C**. No compile-time improvement is established. Compiler phase profiles and the separate Rust producer rebuilds are archived; those rebuilds are not codec compilation times.

## Branchless packed-key prototype

A separate standalone Cranelift prototype passes **8,192 native calls**. Under explicit **best ≤ 65,535** and **stop ≤ 65,535** preconditions, it represents a candidate by `(cost << 16) | (index + 1)`. The initial key is `best << 16`: its zero ordinal retains the old choice on an equal-cost tie. Unsigned saturating narrowing makes larger candidate costs safely uncompetitive, and the smallest key chooses the earliest index among minimum costs.

The prototype emits **one horizontal minimum instead of two**, using `uqxtn`, `zip1` and `umin` in the packed loop. Its source, assembly, build pins and boundary tests are archived. It has **not been integrated into the codec**, and establishes **no speed claim**. Production integration must admit these bounds and retain the existing vector path when they fail; larger inputs or costs cannot be dropped. This is the next experiment because it avoids the unpredictable winner/no-winner branch without adding a SIMD operation to each group.

## Validation and reproduction

All ten candidate workers pass **162 output/inflation/alias observations, six checked failures, twelve injected failures and twelve exact recoveries**, normal/checked artifact identity and the **5,280 B stack cap**. Diagnostic-entry qualification does not establish ordinary managed-entry admission/recovery. Direct scalar emission, ownership, error origins and finite accounting remain maintained. The full GPU suite is not claimed; the pre-existing missing archived `pinned_allocator.rs` still blocks the repository-wide unsafe audit.

Use the minimal compiler built from source `46dc3230fce4cdf4d184fedf261b854b5c8f59fd`:

```sh
python3 reproduce.py /path/to/compile_profile /tmp/mech-mapped-word
python3 reproduce.py /path/to/compile_profile /tmp/mech-combined-word --role select-packet-word-combined-profiled
python3 verify.py
```

The verifier checks ten source/artifact bindings, both optimized test receipts, all exact-mode replays, all output/error/stack/recovery gates, dynamic instruction/branch totals, twenty raw paired orders, fourteen complete-build trials per actor and the packed prototype's proof/assembly pins. The qualified histogram retains its original full confidence evidence unchanged. The goal remains **1× libdeflate performance**, with **63/81 C wins** as the next milestone while preserving every previous C win and all 81 Roc wins.
