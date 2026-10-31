# Fresh executable profile — round 50

Complete eight-byte level-12 compression improves **14.639474× → 14.065937× Roc**, using the same fresh **0.285 ms** reference for both versions. Mech elapsed falls **4.172250 → 4.008792 ms (−3.91774%)**. All four freshly prepared pairs improve (−3.572%, −4.113%, −3.897%, −4.140%); 63/64 descriptive blocks improve. There are 3,968 primary calls, plus 16 warmup/profile calls. Do not compare these multiples directly with round 49's different 0.293 ms reference; its retained Mech version is measured again here as the matched before.

| Separate diagnostic medians | Roc | Mech before | Mech after |
| --- | ---: | ---: | ---: |
| Instructions/call | 1,364,372 | 46,770,314 | 45,222,691.5 |
| Cycles/call | 933,429 | 13,263,236.5 | 12,658,163 |
| Allocations/call | 95 | 13,484 | 13,484 |
| Requested bytes/call | 9,021,923 | 98,529,723 | 98,529,723 |
| Prepared tracked heap bytes | 32 | 142,829,248 | 142,982,576 |
| Additional peak tracked heap bytes | 9,257,328 | 20,575,320 | 20,575,320 |
| Peak tracked heap bytes | 9,257,360 | 163,404,568 | 163,557,896 |
| Resident bytes after call | 14,761,984 | 319,684,608 | 319,717,376 |
| Physical bytes after call | 14,418,880 | 228,886,272 | 228,722,368 |

Instructions fall **1,547,622.5 / 3.30898%**; separate diagnostic cycles fall **605,073.5 / 4.56204%**. There are 32 steady hardware observations per label, one preparation per diagnostic, with uncorrected counter boundary overhead. All rows have zero page-ins. These counters complement, rather than replace, the four independently prepared elapsed pairs.

This saves CPU work with **unchanged allocation traffic**. The derived descriptor inventories add **153,328 tracked heap bytes** before the call; additional execution peak is unchanged and absolute tracked peak rises by that amount. This tradeoff is retained explicitly. All 50 large allocation requests / 97,129,588 requested bytes retain their exact histogram. Realloc counts and their narrow relocation-copy upper bound are unchanged. Requested bytes are not copied bytes.

Post-call OS readings are not peak RSS or proof of reduced physical storage. Prepared Mech retains compiler/catalog/setup state, so these numbers are not minimum standalone runtime RAM. The whole eight-byte input is prepared outside the timer. No streaming or larger-input claim; total copied bytes, cache misses and branch misses remain unmeasured.

A separate coherent source overlay verifies **14,091 omitted child-slot effects / 26,565 original attempts** per complete call in 590 paid batches. Of those batches 311 use prepared child summaries; 12,474 slot effects remain. Omitted slots include 7,673 frame containers and 6,418 canonical slots. All omitted frame words/owners and canonical entries are checked: **zero unexpected nonempty slots** in eight steady calls. This effect fraction is not a percentage speedup. Original logical work remains 60,260,219,686 and comparison work 47,612. Main release binaries contain none of these counters or test-only emptiness checks.

Fresh serial samples contain 1,257 before / 1,290 after exclusive weights over 695 complete diagnostic calls per version. After ancestry unions: frame actions **292 / 22.64%**, tail cleanup **58 / 4.50%**, construction/admission **75 / 5.81%**, projection accounting **33 / 2.56%**, thread-local lookup **32 / 2.48%**, schema closure **24 / 1.86%**. Groups overlap and include surrounding harness; do not add them or treat sampling as precise attribution of the elapsed gain. The generated entry has 168 exclusive weights, visit 46 and helper 39. Roc's dominant opaque native symbol has 1,442/1,524 weights (94.62%); no unsupported source name is assigned to it.

The generated compressor Object **and owned image are byte-identical**. Text remains 13,885,696 bytes and machine stack 108,400 bytes. Actual host cleanup disassembly grows **676 → 763 static instruction sites** including additional proof/fallback branches. Smaller static code is not claimed; dynamic instructions and matched elapsed fall.
