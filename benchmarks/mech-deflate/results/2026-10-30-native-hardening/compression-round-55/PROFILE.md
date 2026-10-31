# Fresh executable measurements after exclusive registration access

The matched complete eight-byte timing moves **13.315731× → 13.187714×** one fresh 0.290-ms reference (3.861562 → 3.824437 ms, −0.961399%). All four independently prepared pairs and 58/64 blocks improve. The gain is retained. Pair ratios are 0.990637, 0.989030, 0.990620 and 0.990046. Within-pair blocks are descriptive rather than independent repetitions.

| Separate diagnostic per call | Reference | Before | After | Change |
|---|---:|---:|---:|---:|
| Instructions | 1,364,372 | 43,761,251 | 43,125,295.5 | −1.453239% |
| CPU cycles | 910,205 | 12,228,989.5 | 12,343,522.5 | **+0.936570%** |
| Allocation requests | 95 | 10,119 | 8,941 | −11.641467% |
| Requested bytes | 9,021,923 | 98,329,987 | 98,254,595 | −0.076672% |
| Total tracked peak bytes | 9,257,360 | 163,500,552 | 163,467,016 | −0.020511% |
| Incremental tracked peak bytes | 9,257,328 | 20,517,976 | 20,484,504 | −0.163135% |
| Post-call RSS bytes | 14,745,600 | 319,782,912 | 319,651,840 | −0.040988% |
| Post-call physical bytes | 14,402,496 | 228,984,512 | 228,820,736 | −0.071523% |
| Reallocations | 0 | 540 | 540 | unchanged |
| Observed realloc-copy upper bound bytes | 0 | 13,905 | 13,905 | unchanged |

Hardware and heap counter runs are distinct from primary timing and each other. All 32 steady hardware observations per variant and eight steady Mech heap observations are retained; reference heap has 32. Page-ins are zero. The adverse cycle result is retained without a causal explanation or replacement run. Prepared compiler/catalog/image heap is included; OS post-call readings are not peak RSS. Realloc-copy counts exclude ordinary copying.

The all-request trace confirms the exact mechanism: 500 constant-setup, 510 execution-admission and 168 registration-retirement lock allocations disappear, all 64 bytes. Every other size/kind histogram and classified category agrees. Remaining requested bytes are dominated by buffer storage, including sixteen 2,442,056-byte U64 copies (39,072,896 bytes). Allocation count alone does not measure their cost.

Fresh before/after/reference samples carry 1,467/1,307/1,515 weights. Registration ancestry falls 48 → 29; mutex lifecycle 17 → 7. Ordinary frame actions still appear in 324/1,307 after weights (24.79%); tail cleanup in 68 (5.20%), projection accounting 37 (2.83%) and thread-local lookup 39 (2.98%). These include surrounding harness and overlapping ancestry; they are not instruction totals or exact timer percentages. The opaque reference's dominant native symbol has 1,415/1,515 exclusive weights; no unsupported source-function name is assigned to it.

The next investigation must identify which exact frame admission/transport operations still allocate and why the large-copy sources remain shared. Preserve current output, resource/failure tests and this measured baseline when establishing a new generic transfer proof. See [closure and commands](CLOSURE.md), [full scorecard](scorecard.json), [CSV](scorecard.csv), [sample parser](profile_analysis.py) and [all-request attribution](attribution-comparison.json). Every archive manifest pins its raw inputs; [reconstruct.py](reconstruct.py) replays numerical analyses without running a benchmark.
