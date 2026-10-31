# Fresh executable comparison and profile

This accompanies the retained record-prefix change. All samples/counters are separate from elapsed timing; raw observations and exact builds are archived.

| Metric | Roc | Mech before | Mech after |
| --- | ---: | ---: | ---: |
| Complete eight-byte elapsed | **0.289 ms** | 4.2935205 ms (**14.8565×**) | **4.2081035 ms (14.5609×)** |
| Instructions | **1,364,372** | 48,599,979 | **46,993,498.5 (34.4433×)** |
| Cycles | **920,543** | 13,551,678 | **13,282,488.5 (14.4290×)** |
| Allocation requests | **95** | 13,994 | **13,994** |
| Requested bytes | **9,021,923** | 98,586,843 | **98,586,843** |
| Additional peak tracked heap | **9,257,328** | 20,575,432 | **20,575,432** |
| Prepared tracked heap | 32 | 142,706,080 | **142,829,248** |
| Resident memory after | **14,778,368** | 319,668,224 | **319,799,296** |
| Kernel physical footprint after | **14,435,264** | 228,640,512 | **228,771,584** |

The Mech host retains compiler/catalog and decoded program state. Whole-host memory is not a minimal native-runtime comparison; the additional peak during compression is reported separately. Requested capacity is not bytes copied. Relocating reallocation's observed upper bound is **13,905 bytes**, unchanged; total copying, branch misses and cache misses remain unmeasured. Both sides receive the whole input before timing. This is not a streaming test, and eight bytes cannot establish large-input copying behavior.

The fresh Mech samples contain **1,217 before / 1,204 after exclusive weights**, each over 695 diagnostic calls and surrounding harness. Mutually exclusive ancestry categories:

| Category | Before weights | After weights | After share |
| --- | ---: | ---: | ---: |
| Frame actions | 313 | **310** | **25.75%** |
| Ordinary control visits outside helpers above | 174 | **183** | **15.20%** |
| Other invocation helpers | 159 | **174** | **14.45%** |
| Selected library calls | 110 | **133** | **11.05%** |
| Remaining | 461 | **404** | **33.55%** |

Within those categories, **tail cleanup is 72 weights (5.98%)**, **canonical construction/admission 76 (6.31%)**, and **schema closure 19 (1.58%)**. These narrower ancestry groups overlap and must not be summed. Percentages are noisy diagnostic samples, not exact compression-timer fractions; they do not override the measured 1.99% time / 3.31% instruction reduction.

The fresh Roc profile has **1,514 weights**: **1,415 (93.46%)** land in `roc__p5794f552e2e0fcba87a322629fdca7c0.fast`, and **50 (3.30%)** in `roc__p1a804bfda1f5e4235a696be5d06bcb43.fast`. These opaque hashes are retained without unsupported source-name attribution. Its sampling run includes 20,000 level-12 calls and surrounding harness/printing; none enters the primary elapsed comparison.

The next admission pilot has a measured allocation baseline, not a predicted speedup. See [ALLOCATIONS.md](ALLOCATIONS.md). Retained source and algorithm are unchanged; the next change will receive its own before/after timing, hardware, allocator and fresh profile cohorts.
