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

## Share the proved narrow ordinal

The third compiler variant uses the existing `start < stop` edge and a proved bound on the current stop plus the forward-window bias. The admitted first ordinal then fits U32. One narrow addition supplies both the zero-extended address ordinal and the packed item. The compiler retains the shifted OR and one cost compare. It still materializes the reverse address before the singleton exit, compares the start and stop separately from the span subtraction, and emits an extra register transfer for the continuing cursor. Four composed bodies change with the original 1,216-byte size and 224-byte frame. The actual eleven-instruction continuation remains present. The other 353 audited machine bodies agree.

| Input | Full control → trial / Roc | Full control → trial / libdeflate | Trial / control |
| --- | ---: | ---: | ---: |
| Eight literals | 0.163092× → 0.161962× | 2.069420× → 2.055081× | 0.993071× |
| Natural 16 KiB | 1.035464× → 1.036615× | 1.044039× → 1.045200× | 1.001111× |
| Reymont 16 KiB | 1.032257× → 1.031258× | 1.041420× → 1.040411× | 0.999032× |
| Reymont 32,769 bytes | 1.059379× → 1.058756× | 0.967566× → 0.966997× | 0.999412× |
| Natural 256 KiB | 1.083299× → 1.082002× | 0.878140× → 0.877089× | 0.998803× |
| Random 1 MiB | 0.992081× → 0.987996× | 1.398926× → 1.393166× | 0.995883× |

The screen, confirmation and full matrix take 0.999216×, 0.998952× and 0.999061× paired control. Both full-matrix arms win 62/81 against Roc, 33/81 against libdeflate, 16/81 against both and 79/81 against at least one. The full block gate passes 81/81. The pooled gate passes 80/81 because empty input measures 42 ns in the control and 83 ns in the candidate. Its balanced-block estimate is 1.005792× control. Repeated 4 KiB has the largest adverse paired estimate at 1.021818×. Both 16 KiB fixtures remain above both peers. The measured benefit is preserved and the previously qualified compiler is retained.

| Independent counter case | Instructions / control | Cycles / control | Lifetime physical peak before and after |
| --- | ---: | ---: | ---: |
| Eight literals | 1.000066× | 1.080300× | 44,794,816 B |
| Natural 16 KiB | 0.995406× | 0.994283× | 45,810,624 B |
| Reymont 32,769 bytes | 0.995031× | 1.000176× | 45,810,624 B |
| Natural 256 KiB | 0.993810× | 0.998087× | 50,725,824 B |
| Random 1 MiB | 1.000004× | 1.001066× | 53,085,120 B |

Eight literals also takes 1.072942× control in the single-call counter cohort. The primary full matrix measures 0.993071× control on that input. These are separate cohorts. The three exact elapsed-worker profiles place 2,146/4,407 running timer PCs in the composed parser at natural 16 KiB, 2,653/5,356 at Reymont 32,769 bytes and 3,028/5,359 at natural 256 KiB. Samples include validation. The counter wrapper initially expected a flags field on an older link receipt. It reused the receipt's original default when rerun.

Measured parent `060119341b46aa106f208912727ea2893f7fbc54` and source patch SHA-256 `daa587941179b90dc78877fe256bf7634a2fe9c4423a49a6f9abb3f3acf3cf87`. The six-feature optimized warnings-as-errors decoded emission takes 63.49 seconds and produces Object `c7b4f95697a390bbb7286510c28d0ab3b91208e1972ab0077a482b5e702f2b17` and exact worker `c9ce409ccf31d8eb0fa97d33c68c8d8587421c8e28c784d5799f1caa7dac2432`. The 324 linked checks and 260,256 timed calls preserve exact streams, independent inflate, capacities, held aliases and cleanup. Source-form emission, flag-off qualification and a full repeat were skipped for this trial. The public replay verifies 242 files, including all full-matrix observations, the five hardware cases and three profiles. Archive SHA-256 `50fc012626340958b2342cecd3be259a811973775abf6cabaa31dd3a82f5ed0c`. [Compiler patch, matched timing, counters and profiles](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-922503c653-second-private-write-short-geometry-compiler-v3.tar.gz).
