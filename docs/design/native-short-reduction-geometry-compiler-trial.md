# Compile shared short reduction geometry

The first compiler trial reuses the admitted span and the biased forward ordinal on the continuation after a short reduction's first visit. It emits `remaining - 1` and `first_index + 1` on the edge that has at least two visits. The packed reverse read, same current windows, original cost, strict choice, vector path and failure order remain in place.

The generated body still subtracts the stop and start separately from the entry comparison. Register-width selection also duplicates the first ordinal for the packed item. The first compiler emission captures only the continuation part of the [fixed-address trial](native-short-reduction-geometry-pilot.md). Four composed bodies change among 357 with the same 1,216-byte bodies and 224-byte frames. The ordinary sparse parser and matcher agree byte for byte.

| Input | Screen control → trial / Roc | Confirmation control → trial / Roc | Confirmation control → trial / libdeflate |
| --- | ---: | ---: | ---: |
| Eight literals | 0.164074× → 0.164692× | 0.162418× → 0.163072× | 2.078155× → 2.086520× |
| Natural 16 KiB | 1.032775× → 1.031182× | 1.032310× → 1.033549× | 1.038481× → 1.039728× |
| Reymont 16 KiB | 0.965911× → 1.079378× | 1.033622× → 1.034813× | 1.040877× → 1.042076× |
| Natural 256 KiB | 1.082159× → 1.080381× | 1.078121× → 1.080450× | 0.871961× → 0.873846× |

The 18-case screen takes 1.006480× paired control, and the eight-case confirmation takes 0.999626×. Both existing median gates pass. The individual Reymont 16 KiB paired estimate rises to 1.117471× in the screen and 1.001152× in confirmation. That screen fixture uses the same input bytes as natural 16 KiB, whose paired estimate is 0.998458×. Both observations are retained. The screen loses one win against each peer on that same fixture. The confirmation's win sets agree. This first compiler trial is declined.

The measured compiler parent is `330006c192c085989d8ac2dd0fa1e0c27d5af1e0` with the retained executable from `170401d69aaeded4304b2992763dd5dfd7e9aabf`. The source and bytecode pins are unchanged. Nightly 2026-03-03 builds the six-feature compiler with warnings denied. Decoded emission takes 62.79 seconds and produces Object `82711d6a7fc8702dcc8f7b1a9c8fe4b12874e32563045224a645d1a33e47f4e5`. The exact worker is `3d4184436685c49a1cbf3e28f644b23f2cde32cb0b266fe47c95b5d1b3a5760f`. It passes 324 complete linked checks across the exact and shared descriptors. The 14,592 timed calls preserve the expected streams, independent inflate, capacities, held aliases and cleanup. Source-form emission, full-matrix timing and independent hardware counters were skipped for this variant.

The three exact-worker profiles include validation. They place 2,274/4,640 running timer PCs in the main composed parser at natural 16 KiB, 2,629/5,348 at Reymont 32,769 bytes and 3,007/5,350 at natural 256 KiB. The public download verifies all 105 files and both timing cohorts. Archive SHA-256 `b415b4e3d94f924cbeb180bd69918e694dd572bcda09b97e7e84f0ccde988a7b`. [Generated code, timing observations and profiles](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-922503c653-second-private-write-short-geometry-compiler.tar.gz).

## Reuse the full-width ordinal

The second variant proves the selected ordinal fits U32 before omitting the duplicate narrow calculation. It carries the full-width ordinal into the packed item. The unsigned start range could not be established before the loop admission, so the separate entry comparison stays. AArch64 emits a shift, a width-extension move and a register-register OR where the fixed-address trial uses one shifted OR. The current comparison-reuse rule treats that register-register OR as a barrier and emits a second cost compare. The four bodies grow from 1,216 to 1,232 bytes and keep their 224-byte frames. The next compiler experiment should target those concrete machine differences.

| Input | Screen control → trial / Roc | Confirmation control → trial / Roc | Confirmation control → trial / libdeflate |
| --- | ---: | ---: | ---: |
| Eight literals | 0.162016× → 0.162527× | 0.161390× → 0.162034× | 2.076049× → 2.084329× |
| Natural 16 KiB | 1.036321× → 1.045080× | 1.030957× → 1.048267× | 1.039854× → 1.057313× |
| Reymont 16 KiB | 1.036296× → 1.050088× | 1.039047× → 1.047783× | 1.044661× → 1.053443× |
| Natural 256 KiB | 1.080507× → 1.093927× | 1.083661× → 1.094928× | 0.872305× → 0.881375× |

The 18-case screen and eight-case confirmation take 1.005472× and 1.006982× paired control. All timing gates pass and the peer win sets agree within both cohorts. Both 16 KiB cases slow in both runs. The compiler patch is declined and the previously qualified image remains the baseline.

The measured source parent is `71ccb4a2db9bc897ebbc3627386aaf8b8d8d7ff1` with the two-file trial based on its byte-identical compiler. The first build failed when a query moved a value from a shared reference. Borrowing it fixes that error. The optimized warnings-as-errors decoded build takes 62.40 seconds, producing Object `747ff8781f5a400fe3d5be8915e92ffe88d0cdbfb3e05f44badb8c5b05c8ddcb` and worker `f712e4ec330139eee45a1282aab33870ca56819b0107443d163fcd0a67d3467d`. The two descriptor launches pass 324 complete checks and the 14,592 timed calls preserve reference streams, independent inflation, capacities, aliases and cleanup. Source-form emission, hardware counters and the full matrix were skipped for the declined patch.

The exact linked parser accounts for 2,303/4,723 running timer PCs on natural 16 KiB, 2,645/5,344 on Reymont 32,769 bytes, and 3,038/5,358 on natural 256 KiB. These profiles include validation. The public replay verifies 110 files. Archive SHA-256 `e26b92ba22cd47064f1b520b94787c727b28dd798ef63290de17c6e9f9af612a`. [Compiler patch, timings, generated code and profiles](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-922503c653-second-private-write-short-geometry-compiler-v2.tar.gz).
