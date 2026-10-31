# Fresh executable profile — round 51

**Retain the small elapsed and storage/code improvement; preserve the adverse instruction count.** Initial complete eight-byte level-12 compression improves **13.799399× → 13.774699× Roc**, using one shared fresh 0.291 ms reference: **4.015625 → 4.0084375 ms (−0.17899%)**. Three of four preparations / 41 of 64 descriptive blocks improve.

The independently prepared longer confirmation improves **14.000365× → 13.915864× Roc**, using its own shared 0.286 ms reference: **4.0041045 → 3.979937 ms (−0.60357%)**. All four preparations / 55 of 64 blocks improve. Preserve both cohorts: **seven of eight prepared pairs improve**, 3,968 initial + 8,064 confirmation primary calls, plus 32 warmup/profile calls. Do not choose only the better cohort, multiply independent results, treat blocks as independent repetitions, or compare multiples across different reference clocks. Only eight-byte complete compression is timed.

| Separate diagnostic medians | Roc | Mech before | Mech after |
| --- | ---: | ---: | ---: |
| Instructions/call | 1,364,372 | 45,224,746 | 45,494,546 |
| Cycles/call | 909,094.5 | 12,702,555 | 12,680,797 |
| Allocation requests/call | 95 | 13,484 | 13,484 |
| Requested bytes/call | 9,021,923 | 98,529,723 | 98,480,739 |
| Prepared tracked heap bytes | 32 | 142,982,576 | 142,982,576 |
| Additional peak tracked heap bytes | 9,257,328 | 20,575,320 | 20,517,976 |
| Peak tracked heap bytes | 9,257,360 | 163,557,896 | 163,500,552 |
| Resident bytes after call | 14,794,752 | 319,651,840 | 320,110,592 |
| Physical bytes after call | 14,451,648 | 228,656,832 | 229,328,576 |

Instructions **increase 269,800 / 0.59658%**; cycles decrease **21,758 / 0.17129%**. Preserve both readings. There are 32 steady hardware observations per label from one preparation per diagnostic, with uncorrected boundary overhead and zero page-ins. These are separate from the eight elapsed pairs; no instruction reduction or cache-miss attribution is claimed.

Requested bytes fall **48,984**; the directly identified native word-bank request falls **483,344 → 439,808 bytes (−43,536)**. All other large request sizes/counts agree; smaller allocation bookkeeping contributes the remaining aggregate difference, without an assigned call site. Requests remain 13,484; realloc counts and their narrow relocation-copy upper bound are unchanged. Peak tracked heap falls **57,344 bytes**. Prepared tracked heap is unchanged. Requested bytes are not copied bytes. Post-call OS readings increased slightly; they are not OS peak RSS. The harness retains compiler/catalog/setup state, so prepared heap is not minimum standalone runtime RAM. Whole input is prepared outside the timer; no streaming, large-file, total-copy-volume, branch-miss or cache-miss claim.

Fresh actual descriptors have **239 numeric regions**. Their original scratch inventory is **5,491 words**, maximum **49**; the after image assigns all 239 the same 49-word bank. All **717 readiness/fuel words remain disjoint**, and no scratch word overlaps them. Physical closed-tail storage falls **6,208 → 766 words**, while logical setup storage remains **6,208**. Original work **60,260,219,686**, comparison work **47,612**, exact raw bytes/inflate, input contents/aliases, capacity 13 and final cleanup zero agree in both cohorts.

The fresh emitted Object text falls **13,885,696 → 13,046,668 bytes (−839,028 / 6.04239%)**; machine stack **108,400 → 108,368 bytes**. The new actual Object and owned image are archived losslessly in compiled-artifact. Code size alone does not establish the reason for the small elapsed gain.

Fresh serial samples: before **1,287**, after **1,137** exclusive weights, 695 complete diagnostic calls per version. After ancestry unions: frame actions **276 / 24.27%**, cleanup **58 / 5.10%**, construction/admission **68 / 5.98%**, projection accounting **29 / 2.55%**, thread-local lookup **32 / 2.81%**, schema closure **22 / 1.93%**. Groups overlap and include surrounding harness; do not add them or attribute sub-percent elapsed changes precisely from sampling. The generated entry has 146 exclusive after weights, frame action 46 and helper 42. The dominant opaque Roc native symbol has 1,427/1,512 weights (94.38%); no unsupported source name is assigned.

The next investigation should identify the call sites behind the unchanged 35 allocation requests of 2,442,056 bytes each, while using the frame-action profile to prioritize compiler changes. These account for 85,471,960 requested bytes, but requested volume alone does not prove copying or a removable cost. No gain is presumed before a diagnostic stack census and a bounded proof.
