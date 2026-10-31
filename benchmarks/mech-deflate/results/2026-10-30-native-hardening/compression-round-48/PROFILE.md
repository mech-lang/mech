# Fresh executable profile — round 48

Complete abcdefgh level-12 only. Elapsed remains approximately **14.5× Roc**. Initial timing is −0.2036%; the independent longer confirmation is +0.0756%; four of eight preparations improve. There is **no confirmed elapsed gain**. Retain the measured allocation/instruction improvement without crediting a new latency baseline.

| Separate diagnostic medians | Pinned Roc | Mech before | Mech after |
| --- | ---: | ---: | ---: |
| Instructions/call | 1,364,372 | 46,989,480 | 46,929,877 |
| Cycles/call | 938,255 | 13,777,822 | 13,276,394 |
| Allocation requests/call | 95 | 13,994 | 13,484 |
| Requested bytes/call | 9,021,923 | 98,586,843 | 98,529,723 |
| Prepared tracked heap bytes | 32 | 142,829,248 | 142,829,248 |
| Additional peak tracked heap bytes | 9,257,328 | 20,575,432 | 20,575,320 |
| Peak tracked heap bytes | 9,257,360 | 163,404,680 | 163,404,568 |
| Resident bytes after call | 14,745,600 | 319,815,680 | 319,881,216 |
| Physical bytes after call | 14,402,496 | 228,804,352 | 228,902,656 |

Instructions fall **59,603 / 0.12684%**; requests fall **510 / 3.64442%**; requested bytes fall **57,120 / 0.05794%**. Peak tracked heap falls only **112 bytes**, since the redundant wrapper was short-lived. Prepared heap and final live heap are unchanged. The observed cycle reduction is 3.6394% in one separate diagnostic preparation per version; mixed elapsed cohorts do not establish that as a general speedup. Hardware reads include uncorrected boundary overhead. All measured hardware rows have zero page-ins. Current post-call resident/physical memory are not OS peak RSS.

Prepared linked Mech retains compiler/catalog/setup state; this is not minimal standalone runtime RAM. Peak heap is measured by allocation hooks; the OS readings include other mappings. Do not equate 142.8 MB of prepared tracked Mech heap with a reference runtime-only comparison. Mech still executes approximately **34.40×** reference instructions per call.

Fresh 695-call isolated Mech samples and a separate 20,000-call Roc sample follow the CPU/allocation runs. Candidate sample ancestry weights: frame actions **300/1,321 = 22.71%**; tail cleanup **88/1,321 = 6.66%**; canonical construction/admission **73/1,321 = 5.53%**; projection accounting **44/1,321 = 3.33%**; thread-local lookup **42/1,321 = 3.18%**; schema closure **16/1,321 = 1.21%**. These groups overlap. They include surrounding harness and are not exact timer percentages or additive attribution. Baseline weights total 1,296, so the sample shifts are not precise mechanism speedup estimates.

The candidate's disjoint groups are: frame actions 300 (22.71%); ordinary visits 203 (15.37%); other helpers 206 (15.59%); selected library calls 128 (9.69%); remaining 484 (36.64%). The direct generated entry accounts for 147 exclusive weights. Roc's dominant opaque native symbol has **1,414/1,517 = 93.21%** exclusive weights; the next has 48 (3.16%). No unsupported source-function names are assigned to those hashes.

Input is a whole eight-byte buffer prepared before evaluate_into; this case does not demonstrate streaming or large-file behavior. All 50 large allocation requests / 97,129,588 requested bytes remain unchanged. Requested capacity is not bytes copied. Total physical copying, branch misses and cache misses are unmeasured; the narrow relocating-realloc copy upper bound remains unchanged. No larger-input throughput or memory extrapolation is justified.

The real generated compressor Object/text/stack are unchanged. Host admission's static 1,119 → 1,168 sites include fallback/error paths and cannot be read as dynamic counts. The allocator delta, focused ownership tests and hardware counts establish the actual mechanism.

The next bounded target is unnecessary physical cleanup, using existing representation proofs and paid retirement loans while preserving original charges, error order and aliases. Finish the worker-capture and generation/epoch audit first. Frame operations and ordinary helper/visit costs remain substantial; no numerical speedup is promised for the next change.
