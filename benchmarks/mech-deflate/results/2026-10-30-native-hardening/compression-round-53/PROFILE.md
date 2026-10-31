# Matched executable measurements

Only complete `abcdefgh` level-12 compression. One shared pinned reference executable and reference timing per cohort; never compare multiples from different clocks as a regression. Four independent prepared pairs per cohort, 16 alternating-order blocks each. Initial: 3,968 primary calls plus 16 excluded warm/profile calls. Longer independent confirmation: 8,064 plus 16. Both cohorts and every observation are retained.

| Cohort | Shared reference | Before | After | Paired change | Improving preparations |
|---|---:|---:|---:|---:|---:|
| Initial | 0.288 ms | 14.683667× (4.228897 ms) | 14.537182× (4.186709 ms) | −0.9976% | 4/4 |
| Longer confirmation | 0.290 ms | 14.527298× (4.212917 ms) | 14.453736× (4.191584 ms) | −0.5064% | 3/4 |

Initial blocks improve 49/64; confirmation improves 41/64. One confirmation preparation is 0.4107% slower. The previous round's current-only timing is a different cohort, not a matched regression comparison. Post-initial environment snapshots show substantial unrelated system activity; they cannot establish load throughout the timing or its cause. No user/system service was stopped.

Separate CPU cohort: 32 steady calls each. Instructions: reference **1,364,372**, before **45,490,459.5**, after **45,057,256** (−0.9523%; after 33.023× reference instructions). Cycles: **904,110**, **13,566,263.5**, **12,494,762** (−7.8983% within this counter cohort). Cycle readings from one preparation are not a universal latency claim. Post-call RSS increases 0.01025%; physical footprint decreases 0.02862%; no page-ins. OS post-call memory is not peak RSS.

Separate allocation cohort: 32 reference and eight steady calls per Mech build. Requests **95 / 13,484 / 13,484**. Requested bytes **9,021,923 / 98,480,739 / 98,480,739**. Prepared tracked heap **32 / 142,982,576 / 142,982,576** bytes; incremental peak **9,257,328 / 20,517,976 / 20,517,976**. Total tracked peak **9,257,360 / 163,500,552 / 163,500,552**. Mech reallocations remain 1,213, moving reallocations 458, observed moving-copy upper bound 13,905 bytes. Host/catalog/image retention is included; these are not standalone RAM requirements. This owner-transfer change removes clone/drop traffic and intentionally does not remove allocations.

The generated Object is byte-identical: SHA `1a8bb46e9f78fec2147b3242fab5651e470f86ba0347f4f11c7f1aedd5008777`, text **13,046,668 bytes**, native stack **108,368 bytes**. Image and metadata pins also match. IR/work/ABI/bytecode remain **132/95/37/14**. The implementation changes linked runtime code, not generated operations or source.

Fresh serial samples: before 1,200 exclusive weights, after 1,185, reference 1,520. Frame-Copy ancestry is 19/1,200 → 9/1,185; all frame actions 283/1,200 → 288/1,185. These two-second samples include surrounding harness work and overlap by ancestry; they are not instruction counts, exact timer percentages or additive savings predictions. All remaining groups and contrary movements are in profile-numbers.json. The next user-directed priority is the allocation-request census in round 54.
