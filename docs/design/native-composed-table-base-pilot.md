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

## Compiler-generated hoist

The first Cranelift experiment asked the existing e-graph elaborator to move a colocated, non-TLS symbol address outside every enclosing loop. Both settings re-emitted the exact same 1,216-byte body. The retained CLIF defines the address at entry, but e-graph elaboration rebuilds it at its use in block 169. Loop analysis puts that block in loop 1 inside loop 0. The elaborator visits a side exit first, pops both loop entries, then reaches block 169 with an empty hoist stack. The pinned dominator walk and optimized CLIF reproduce this behavior.

The second experiment derives placement from the current block's analyzed loop membership and the outermost loop header's immediate dominator. It moves both symbol relocations from function offsets 700 and 704 to 164 and 168. The table base stays live in x5. The allocator spills the packet word at input offset 88, adding a stack reload on the route that prices the prior record and stores when it rebinds that word. The parser body grows from 1,216 to 1,232 bytes while retaining its 224-byte frame. Four composed bodies change, along with the generated root, whose stack frame grows from 49,600 to 49,792 bytes. The hand-edited trial used x16, which Cranelift reserves for spill addressing. The compiler experiment did not change that register policy.

| Input | Screen control → trial / Roc | Confirmation control → trial / Roc | Confirmation control → trial / libdeflate |
| --- | ---: | ---: | ---: |
| Eight literals | 0.160929× → 0.162393× | 0.162687× → 0.163469× | 2.064410× → 2.074332× |
| Natural 16 KiB | 1.038578× → 1.036727× | 1.033900× → 1.038358× | 1.041194× → 1.045683× |
| Reymont 16 KiB | 1.034621× → 1.038061× | 1.036694× → 1.036483× | 1.041369× → 1.041156× |
| Reymont 32,769 bytes | 1.062716× → 1.067450× | 1.061016× → 1.066701× | 0.962750× → 0.967908× |
| Natural 256 KiB | 1.080545× → 1.091558× | 1.080017× → 1.085861× | 0.871967× → 0.876685× |

The screen and repeat take **1.001244×** and **1.002543×** paired control. Both timing gates pass in all 26 comparisons and the peer win sets agree. The separate counter cohort shows natural 16 KiB retiring 28,426,065 → 28,149,980 instructions while cycles hold at 7,160,387 → 7,160,844. Natural 256 KiB retires 544,448,281 → 537,229,303 instructions while cycles rise from 156,043,927 to 156,663,758. The exact-worker profiles record 2,165/4,391, 2,647/5,363, and 3,031/5,357 running timer PCs in the main parser at natural 16 KiB, Reymont 32,769 bytes, and natural 256 KiB respectively.

Compiler base `0d9dce53b0fd5fc63ea3aff8a4e3f49c1a1a17cc` with the archived experimental patch. With the hoist disabled, all 357 bodies and the complete Object match the retained binary. Candidate Object `a7f15ca0f0465786d35f1466db06a651584adf5c4d2c6a2661d526d1c405730c` and linked worker `d890034f420c0dc87ff9c993e376df090e7d47d1481d0144bc523f1253acb8b4`. Both descriptor variants pass 162 linked checks each, and 14,592 timed calls verify exact streams, independent inflation, capacities and aliases. The qualified build remains in use. Original-form and full-matrix qualification were skipped after both timing cohorts regressed. Archive SHA-256 `bd4476e1c29892060d8c681a7fb0b087e7d3ab664fc479a2047c827a06b29f35`. [Both compiler patches, machine code, raw timing observations, counters, and three profiles](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-922503c653-second-private-write-table-base-compiler.tar.gz).
