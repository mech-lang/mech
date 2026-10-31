# Fresh complete-program profile

All time values are complete eight-byte level-12 compression. The primary uses one shared fresh reference median, **0.285 ms**, from 20,000 reference calls. Four independently prepared executable pairs alternate order across 16 blocks each, 31 steady calls per variant per block: **3,968 primary calls**, plus 16 excluded warm/profile calls. Every output/work/alias/cleanup check passes.

| Metric | Pinned reference | Before | After | Within-cohort change |
|---|---:|---:|---:|---:|
| Primary time | 1× (0.285 ms) | 13.841374× (3.944792 ms) | 13.556068× (3.863480 ms) | −2.06125% |
| Instructions | 1,364,372 | 45,055,355.5 (33.022779×) | 43,759,276.5 (32.072834×) | −2.87664% |
| Cycles | 938,262.5 | 12,539,255.5 | 12,214,862.5 | −2.58702% |
| Allocation requests | 95 | 13,484 (141.936842×) | 10,119 (106.515789×) | −24.95550% |
| Reallocations | 0 | 1,213 | 540 | −55.48228% |
| Requested bytes | 9,021,923 | 98,480,739 | 98,329,987 | −0.15308% |
| Prepared tracked heap | 32 | 142,982,576 | 142,982,576 | unchanged |
| Incremental tracked peak | 9,257,328 | 20,517,976 | 20,517,976 | unchanged |
| Total tracked peak | 9,257,360 | 163,500,552 | 163,500,552 | unchanged |
| Post-call OS RSS | 14,761,984 | 319,799,296 | 319,717,376 | −0.02562% |
| Post-call physical footprint | 14,402,432 | 228,935,424 | 228,935,424 | unchanged |

Time improves in all four prepared pairs (ratios 0.976362, 0.983042, 0.979720, 0.977990) and 63/64 blocks. Blocks within a preparation are descriptive rather than independent experiments. The instruction/cycle figures are a separate 32-steady-call cohort per executable; allocation figures are separate cohorts of 32 reference and eight steady Mech calls per executable. All jobs are serialized. No timing is inferred from allocation tracing. No cross-round multiplication or comparison of different reference clocks is used.

This removes many small temporary allocations. Requested bytes are cumulative allocator requests, not copied or simultaneously resident bytes. Relocating reallocations remain 458, with the observed realloc-copy upper bound 13,905 bytes; that bound is not all program copying. Large path-capacity requests remain, as do retained constructor owners. The current 10,119 requests are still 106.516 times the reference count. The host retains compiler, catalog and image state; post-call RSS is not peak RSS or minimum standalone memory, and the tiny input does not establish larger-input streaming behavior.

The fresh all-request census exactly isolates the change: typed matrix construction **5,384 → 2,019 requests**, all remaining category counts and bytes unchanged. Constant/setup admission remains **1,500**, execution-time admission **1,528**, private frame preparation **227**, registration removal **168**, other immutable owners **3**, separate import lookup **2**, and unclassified **4,672**. These categories partition requests using ordered stack predicates. They are not timing shares. Every event has a complete stack; raw same-build symbolization is preserved.

Fresh serial samples have 1,252 before, 1,309 after and 1,513 reference exclusive weights. Canonical construction/admission ancestry is 88/1,252 → 41/1,309; schema closure 21/1,252 → 2/1,309. Frame actions remain large (308/1,252 → 335/1,309), while projection accounting rises 34/1,252 → 48/1,309. These overlapping two-second ancestry groups include surrounding harness work and cannot be added or treated as exact timer percentages. Contrary movements and complete symbol lists remain in profile-numbers.json. The native Object, image, metadata, text and stack sizes are identical; the gains come from linked runtime work.
