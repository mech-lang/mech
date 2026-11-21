# Fixed-width reads from an admitted window

**The qualified selection remains 63,612 B / 81 Roc / 62 libdeflate. There is no qualified 63rd C win.** Source `2dcf8babd9e34e1f2d752edd875e1572f185aef5` removes another collector predicate. The word, probe and packet candidates reach **63,164 B / 63,436 B / 63,020 B**, each with **4,736 B peak stack**. All prior peer wins observed in fourteen complete twenty-one-case discovery orders survive. Full 81-case confidence qualification is outstanding.

The probe takes **0.13–0.38% less time on natural 4 KiB** and **0.44–0.50% less time on random 1 MiB** than its matching clamped-window control. Natural 4 KiB remains **1.010619× / 1.017073× C**. Revisited scorer width, immutable-load placement and masked-hash narrowing preserve the panel wins but do not reach parity. All smaller and slower alternatives remain available for repair.

## Proof and source

The existing caller guard admits an immutable input span and the unit-stride cursor. The compiler has already proved `window <= length - position`. On the selected `window >= 5` edge, the four-byte read at `position + 1` fits:

```text
length - position >= 5
position + 1 < length - 3
```

The proof resolves the original saturated length subtraction, checks constant offset arithmetic for overflow, and follows only the actual selected unsigned edges and the current cursor. It reuses the bounded clamped-window prover. Unknown shapes and signed comparisons retain the checked path. A separate fixture deliberately reads at `position + 2`; five remaining bytes cannot admit that read, and its original error must survive. No new caller runtime guard or cloned body is added.

This extends `MECH_REUSE_ADMITTED_INPUT_WINDOWS=1`; the collector records three window checks removed instead of two. Normal and checked emission agree exactly. Disabling the option reproduces all three artifacts for four preceding recipes. The original 2,719-line Mech codec and canonical bytecode remain unchanged. No LLVM dependency is added.

The minimal compiler SHA is `6d0df92ba888afda48ae12771d1529e4f22fdfa0dba4af18b8159bd3b9e9cc6e`. Exact source inputs and the separate **206.599 s Rust producer rebuild** are archived. That rebuild is not the codec compilation time.

## Read-proof discovery

Each candidate uses its own matching clamped-window feature control, twenty-one cases, both complete case orders and twenty-four balanced blocks per case. Separate cohorts' C ratios do not establish causal differences between candidates.

| Candidate | Encoder text | Natural 4 KiB / C, forward / reverse | Natural 4 KiB / matching base | Random 1 MiB / matching base |
| --- | ---: | ---: | ---: | ---: |
| read-window-word-profiled | 63,164 B | 1.021799× / 1.029441× | 1.000702× / 1.002791× | 0.993953× / 0.994812× |
| read-window-probe-profiled | 63,436 B | 1.010619× / 1.017073× | 0.996165× / 0.998702× | 0.995559× / 0.994982× |
| read-window-packet-profiled | 63,020 B | 1.021471× / 1.037776× | 1.004430× / 1.011959× | 0.995031× / 0.994468× |

The word's natural-data time increases slightly, and the packet candidate regresses against its own matching packet control. Both retain the observed peer wins. Those adverse results remain in the report and raw archive.

## Historical combinations revisited

All four variants share the exact read-proof probe as their control. `wide` removes previous-DP narrow carry, `pinned` removes immutable-load rematerialization, their combination removes both, and `hash32` enables private masked arithmetic narrowing. Unchanged ordered code graphs and literal/padding coordinates permit reusing layout heuristics; the two pinned variants change the collector graph and receive fresh six-case counter profiles. Graph equivalence is not a latency forecast.

| Candidate | Encoder text | Natural 4 KiB / C, forward / reverse | Natural 4 KiB / shared base | Random 1 MiB / shared base |
| --- | ---: | ---: | ---: | ---: |
| tune-read-wide-profiled | 63,420 B | 1.012744× / 1.026412× | 0.999515× / 0.993732× | 0.999079× / 0.999094× |
| tune-read-pinned-profiled | 63,436 B | 1.023535× / 1.022079× | 1.010513× / 0.997372× | 0.998775× / 0.998775× |
| tune-read-wide-pinned-profiled | 63,420 B | 1.018307× / 1.021254× | 0.998569× / 0.996438× | 0.998958× / 1.001677× |
| tune-read-hash32-profiled | 63,452 B | 1.019336× / 1.031846× | 0.995525× / 0.999093× | 1.041575× / 1.041224× |

Hash narrowing makes random 1 MiB about **4.1% slower**. The carry/load combinations offer only small or inconsistent gains. Every observed prior peer win remains a point win; none establishes the additional C win. The candidates are retained, including their adverse cases.

## Actual collector work

Untimed pre-layout diagnostics compare the probe collector before and after the new read proof on natural 4 KiB:

| Metric | Clamped-window proof | Fixed-read proof |
| --- | ---: | ---: |
| Instructions | 1,387,046 | 1,373,811 |
| Branches | 277,690 | 272,766 |
| Stack loads | 101,457 | 105,508 |
| Stack stores | 30,639 | 30,596 |

The proof removes **13,235 instructions and 4,924 branches**, while register allocation adds **4,051 stack loads**. These are diagnostics, not a cycle prediction or the final placed worker's instruction totals. All original counts are preserved. Actual optimized IR inspection reproduces the exact profiled object, complete metadata and bytecode.

## Complete builds on Apple M1

Twelve balanced serial fresh-process trials per actor include source compilation and final executable linking, with compilers and fixed support prebuilt equally. Strip and compiler bootstrap are separate. Every newly emitted Mech object, metadata and bytecode is checked against its recipe.

| Actor | Source through final link | Through strip | Paired build / Roc / C |
| --- | ---: | ---: | ---: |
| mech-control-reference | 2.215530 s | 2.226732 s | 0.740933× / 4.587826× |
| mech-read-word | 2.216883 s | 2.227892 s | 0.740982× / 4.584896× |
| mech-read-probe | 2.217276 s | 2.228221 s | 0.741816× / 4.587254× |
| mech-read-packet | 2.220301 s | 2.231578 s | 0.742320× / 4.581229× |
| roc | 2.990552 s | 3.001706 s | — |
| libdeflate | 0.482895 s | 0.493751 s | — |

The tuning cohort uses sixteen balanced trials per actor and the same policy:

| Actor | Source through final link | Through strip | Paired build / Roc / C |
| --- | ---: | ---: | ---: |
| mech-control-reference | 2.211354 s | 2.222451 s | 0.741370× / 4.606478× |
| mech-read-probe | 2.218175 s | 2.229230 s | 0.742127× / 4.617860× |
| mech-tune-wide | 2.217123 s | 2.228247 s | 0.743203× / 4.618195× |
| mech-tune-pinned | 2.216448 s | 2.227463 s | 0.742456× / 4.615853× |
| mech-tune-wide-pinned | 2.218832 s | 2.229815 s | 0.743013× / 4.617085× |
| mech-tune-hash32 | 2.216642 s | 2.227722 s | 0.742168× / 4.620469× |
| roc | 2.984581 s | 2.995565 s | — |
| libdeflate | 0.480780 s | 0.491915 s | — |

All Mech stripped executables are **276,072 B**, Roc **191,912 B**, and C **155,752 B**. Encoder text for the peers is **57,696 B Roc / 37,080 B C**. There is no established compile-time gain. No runtime timing, build/test job, counter, profile preparation or archive compression overlaps either complete-build cohort.

The read-proof probe's phase medians are **704.530 ms parsing**, **217.474 ms semantics**, **6.849 ms artifact preparation**, **2.554 ms encoding**, **1,223.354 ms native emission**, and **35.321 ms final linking**. Catalog work overlaps parsing; independent medians do not add.

## Validation and reproduction

The source passes **25 relevant repository tests**, including **72,752 native span/window/histogram calls**. The negative fixed-width fixture preserves the real bounds error. All seven candidates pass **162 output/inflation/alias observations, six checked failures, twelve injected failures and twelve exact recoveries**, normal/checked artifact replay and the **5,280 B stack cap**. Peak stack is 4,736 B. Direct scalar emission, error origins, ownership and finite accounting remain maintained. These diagnostic-entry checks do not establish ordinary managed-entry admission/recovery.

Use a minimal compiler built from source `2dcf8babd9e34e1f2d752edd875e1572f185aef5`:

```sh
python3 reproduce.py /path/to/compile_profile /tmp/mech-fixed-read-probe
python3 reproduce.py /path/to/compile_profile /tmp/mech-fixed-read-word --role read-window-word-profiled
python3 verify.py
```

The verifier recomputes all fourteen paired discovery orders, checks every archived file, artifact/source pin, repository test receipt, error/stack/recovery gate, dynamic instruction/branch count and both complete-build cohorts. The full GPU suite is not claimed; the existing missing archived `pinned_allocator.rs` still blocks the repository-wide unsafe audit.

Next, isolate packet-load qualification by phase. It saves 416 B overall but changes both the hot scorer and smaller routines; a held-feature ablation can identify which changes cause the earlier distribution-change slowdown. The goal remains unfinished.
