# Forward the previous append cost with the original register allocation

The [compiler-emitted previous-cost trial](native-composed-previous-cost-pilot.md) adds a recurring stack reload. This control changes three original machine words and adds nine words in unused function padding. Every regular and sparse append already puts the packed result in `x28`. A first-entry stub seeds that register from the preceding cell if the original count and length are positive. Both loop backedges retain the just-stored value. The original U32 addition consumes its low word directly. A zero count still reaches the original exit without a payload read. All other registers, the 224-byte frame, the two table relocations and all 1,276 named text addresses agree.

| Input | Screen control → trial / Roc | Confirmation control → trial / Roc | Confirmation control → trial / libdeflate |
| --- | ---: | ---: | ---: |
| Eight literals | 0.165234× → 0.166319× | 0.162509× → 0.162729× | 2.073672× → 2.076479× |
| Natural 16 KiB | 1.037260× → 1.047516× | 1.034921× → 1.045064× | 1.042250× → 1.052465× |
| Reymont 16 KiB | 1.036166× → 1.042231× | 1.030262× → 1.044008× | 1.035511× → 1.049327× |
| Reymont 32,769 bytes | 1.066086× → 1.074226× | 1.062281× → 1.072236× | 0.967542× → 0.976609× |
| Natural 256 KiB | 1.081989× → 1.094955× | 1.074715× → 1.100090× | 0.873604× → 0.894231× |

The 18-case screen and eight-case confirmation take 1.002377× and 1.008790× paired control. Both gates pass in every case and the measured win sets agree within each cohort. The 16 KiB slowdown recurs with the original register allocation. The previous compiler is retained. The diagnostic adds a taken jump on the hot route to bypass the old read corridor, so the comparison includes that branch and cannot isolate the effect of the load in a straight-line layout.

The measured compiler is `170401d69aaeded4304b2992763dd5dfd7e9aabf`. The Object is `3e2b99638608b8cc600b1023fd91f484b6d12ced3a8205226086efb08dcfd49f` and the exact worker is `f9e41ff15b5b0e115822b5737ba9af3becdd86ed31ea9068df7f0f553b1ebf4f`. All 162 linked checks and 14,592 timed calls preserve reference bytes, independent inflation, capacities, aliases and cleanup. Two initial preflight assertions misclassified a compare instruction as a branch and included the returning epilogue in the live-register check. Both were corrected before linking. Compiler integration, hardware counters and the full matrix were skipped for this rejected diagnostic.

The parser accounts for 2,330/4,730 running timer PCs at natural 16 KiB, 2,582/5,220 at Reymont 32,769 bytes and 3,061/5,365 at natural 256 KiB. The three profiles use the exact candidate and include validation. The public replay verifies all 103 files and both timing cohorts. Archive SHA-256 `bdef5f849dcb1f7b9bd9d3318f835cbce60f51ac92123d9174a55f41c10bc290`. [Fixed-register machine code, timings, and profiles](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-922503c653-second-private-write-composed-cost-register.tar.gz).
