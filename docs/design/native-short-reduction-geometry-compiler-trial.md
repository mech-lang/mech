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
