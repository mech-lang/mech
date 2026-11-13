# Keep the immutable cost table address live

The source-derived lookup has a fixed symbol address. Cranelift receives it at function entry but emits the address pair on each reached match-record lookup. Libdeflate keeps the comparable table bases in registers through its record walk. The fixed-address trial keeps the Mech table address in an otherwise unused caller-saved register. It uses two dead input-packet load slots for materialization at entry, relocates both table references, and branches directly to the original selected record load or zero-default continuation. The admitted packet, checked data loads, lookup order, table contents and writes are preserved. It executes two fewer instructions per reached record lookup.

| Input | Screen control → trial / Roc | Confirmation control → trial / Roc | Confirmation control → trial / libdeflate |
| --- | ---: | ---: | ---: |
| Eight literals | 0.164378× → 0.163010× | 0.162225× → 0.162858× | 2.067004× → 2.075070× |
| Natural 16 KiB | 1.034085× → 1.032755× | 1.033810× → 1.033173× | 1.041637× → 1.040995× |
| Reymont 16 KiB | 1.035092× → 1.035321× | 1.033563× → 1.031391× | 1.038240× → 1.036058× |
| Reymont 32,769 bytes | 1.060856× → 1.061192× | 1.060941× → 1.058514× | 0.965271× → 0.963062× |
| Natural 256 KiB | 1.080824× → 1.078070× | 1.081720× → 1.073535× | 0.875584× → 0.868959× |

The 18-case screen takes 0.997992× paired control and the eight-case confirmation takes 0.997068×. Both gates pass on all 26 comparisons and the peer win sets agree. Natural 256 KiB improves by 0.25% and 0.76%. The 16 KiB cases stay above both peers, and the eight-literal timing moves in opposite directions. This trial establishes a measured opportunity for a compiler change. The published score still uses the previously qualified executable.

Measured compiler `170401d69aaeded4304b2992763dd5dfd7e9aabf`, candidate Object `2e37825bff6db6b1b66c270e5408d0df7a5652fe69b3d511dda5db8fe75fc3c0` and worker `2b0dfff6d10b6f5927a5d07e0dd12b09df76844492eaaf1eeb66ab4c8488aa31`. The 1,216-byte body retains its 224-byte frame and all 1,276 linked symbol addresses agree. The linked audit checks the new relocation destinations and register operands. All 162 complete linked checks and 14,592 timed calls verify exact streams, independent inflation, capacities, aliases and cleanup. Compiler integration, hardware counters and the full matrix were skipped for the fixed-address trial.

The exact-worker profiles place 2,287/4,663 running timer PCs in the main composed parser at natural 16 KiB, 2,586/5,267 at Reymont 32,769 bytes, and 3,007/5,361 at natural 256 KiB. These samples include validation. The public replay verifies 97 files. Archive SHA-256 `f9f2660064550bdeac48fd0b2fd744eae3df92fdac4f955338c060931a4af705`. [Timing, linked code, and profile replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-922503c653-second-private-write-composed-table-base.tar.gz).
