# Fresh executable profile — round 49

Complete eight-byte level-12 compression: **14.306172× → 14.259172× Roc**, one shared fresh 0.293 ms reference. Elapsed **4.1917085 → 4.1779375 ms (−0.32853%)**; all four preparations / 40 of 64 descriptive blocks improve. Four prepared pairs, 3,968 primary calls. Do not infer an additional gain from a changed reference median relative to earlier rounds.

| Separate diagnostic medians | Roc | Mech before | Mech after |
| --- | ---: | ---: | ---: |
| Instructions/call | 1,364,372 | 46,929,766.5 | 46,770,583 |
| Cycles/call | 931,466.5 | 13,359,339.5 | 13,469,479.5 |
| Allocations/call | 95 | 13,484 | 13,484 |
| Requested bytes/call | 9,021,923 | 98,529,723 | 98,529,723 |
| Prepared tracked heap bytes | 32 | 142,829,248 | 142,829,248 |
| Additional peak tracked heap bytes | 9,257,328 | 20,575,320 | 20,575,320 |
| Peak tracked heap bytes | 9,257,360 | 163,404,568 | 163,404,568 |
| Resident bytes after call | 14,778,368 | 320,126,976 | 319,733,760 |
| Physical bytes after call | 14,435,264 | 229,148,416 | 228,935,424 |

Instructions fall **159,183.5 / 0.33920%**. The separate single-preparation CPU diagnostic shows **cycles increasing 0.82444%**; preserve this adverse reading and do not claim a cycle gain. It does not override or replace the separately matched elapsed result. All hardware rows have zero page-ins. Hardware boundary overhead is uncorrected; the single diagnostic preparation is not four independent cycle comparisons.

This is a CPU cleanup improvement with **no allocation or tracked-heap reduction**. Source buffers and all 50 large requests remain unchanged. The post-call OS readings vary slightly; they are not OS peak RSS or proof of reduced physical data storage. Mech retains compiler/catalog/setup state, so its prepared heap is not minimum standalone runtime RAM. Input is a whole eight-byte buffer prepared outside the timer; no streaming or large-file claim. Total copied bytes, branch misses and cache misses remain unmeasured; realloc counts and their narrow relocation-copy upper bound are unchanged.

A separate coherent counter overlay confirms **13,857 omitted arena effects / 26,565 attempts in 590 paid batches**, retaining 12,708 arena effects and every original logical charge. The effect count is not a percentage speedup. Main release builds contain neither overlay counters nor test-only effect counters.

Fresh serialized samples: before 1,311 exclusive weights, after 1,305, 695 complete diagnostic calls each. After ancestry weights: frame actions **317 / 24.29%**; tail cleanup **81 / 6.21%**; construction/admission **54 / 4.14%**; projection accounting **59 / 4.52%**; thread-local lookup **42 / 3.22%**; schema closure **18 / 1.38%**. These groups overlap and include surrounding harness. Sampling is too coarse to attribute a 0.33% gain; the increased cleanup sample share is not hidden or treated as proof of regression.

The generated entry has 151 exclusive after weights. Roc's dominant opaque native symbol has 1,431/1,515 weights (94.46%); the next has 48 (3.17%). No unsupported source names are assigned to hashes. Generated Object/text/stack are unchanged; the host cleanup helper has more static branch sites (616 → 676), while the measured executed work falls.

Frame actions and ordinary helper/visit costs remain large. The next bounded investigation is to prepare original logical prices separately from surviving physical cleanup, with successful-child emptiness requiring returned-value exceptions, no revival and allocation-epoch proof. Shared shadow storage, shared native code and scalar/window facts remain separate; no speedup is promised for those candidates.
