# Reuse dominating bounds for selected byte reads

A selected unsigned `index < length` edge establishes two facts. The original index fits the current length, and adding one to that index cannot wrap. The compact numeric compiler reuses those facts when the byte read lowers its one-based index back to `(index + 1) - 1`. The proof requires the same current SSA values for index and length on a dominating selected edge. Signed comparisons, inclusive bounds, other lengths, changed index values, and additional entrances to the selected block leave the later checks in place.

The pass runs in the existing CFG cleanup stage for private vector phases under `MECH_REUSE_SELECTED_READ_BOUND=1`. It folds the redundant predicates at their original locations. The selected edge, address calculation, and byte reads keep their order.

The measured release compiler is `a101b7ff4f1274bc46f067112b8e8378654d0f64` plus the bounds patch. All four compiler files match the integration patch byte for byte. The original and decoded emissions independently produced the same source, bytecode, Object, and metadata. The two candidate forms each passed 162 checks. The maintained scalar-span suite passed both tests, with controls for wrong masks, changed lengths, alternate entries, overflow, and failure order. [Source, commands, and replay archive](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-a101b7ff4-selected-read-bounds.tar.gz)

The initial maintained test link ran out of disk space. A retry with the same command and source passed after old build caches were reclaimed. The changed-file unsafe audit passed. The repository audit reports the inherited missing audit entry in `benchmarks/related-post-gen/check-measurement.rs`, reproduced on the published parent.

| Full comparison | Control | Bounds reuse |
| --- | ---: | ---: |
| Faster than Roc | 47/81 (58.0%) | 47/81 (58.0%) |
| Faster than libdeflate | 10/81 (12.3%) | 11/81 (13.6%) |
| Faster than both | 1/81 | 2/81 |
| Faster than either | 56/81 (69.1%) | 56/81 (69.1%) |

The same 47 cases beat Roc in both builds. Repeated 256 KiB adds the libdeflate win. Across the 81 cases and 245,664 measured calls, the paired elapsed geometric mean is **0.994925× control**. There are 58 faster and 23 slower paired estimates. The largest increases are eight-byte overlap at 1.022837×, eight literals at 1.015877×, and natural 1 MiB at 1.014403×. Both the pooled median and median block-mean 1.05 gates pass all 81 cases. The raw packets retain the 6,378 zero timer samples and all adverse observations.

| Input | Mech/Roc | Mech/libdeflate | Elapsed vs control |
| --- | ---: | ---: | ---: |
| Eight literals | 0.174678× | 2.233684× | 1.015877× |
| Natural 8 KiB | 1.043123× | 1.301655× | 1.001536× |
| Natural 1 MiB | 1.203971× | 0.977727× | 1.014403× |
| Random 1 MiB | 1.409735× | 1.989949× | 0.995542× |
| Periodic 1 MiB | 0.590477× | 1.002281× | 0.981951× |
| Repeated 128 KiB | 0.602855× | 1.006874× | 0.982542× |
| Repeated 256 KiB | 0.608077× | 0.990174× | 0.985644× |
| Repeated 1 MiB | 0.604758× | 0.947619× | 0.981359× |

The seven-case pilot independently put repeated 256 KiB at 0.987413× libdeflate and 0.981114× its control. Both comparisons use the same workers and 24 balanced four-actor orders per case. Allocation, initialized copying, and zeroing metrics agree across all 81 cases.

Four of the 351 generated function bodies change. The linked matcher loses all seven detected duplicate compare-and-branch pairs. Its code drops from 10,616 to 10,284 bytes and it uses a 576-byte stack frame in both builds. The other three bodies retain their frame sizes. The root body has identical Object bytes. The seven independent CPU cohorts show 0.995× control instructions on repeated 256 KiB, 0.989× on natural 1 MiB, and 0.987× on random 1 MiB. Eight literals records 1.030× control cycles in the same independent cohort. Seven profiles identify PCs in the exact timed executable.

Archive SHA-256 `878563903d3a7d32fe704e41f51cb692085db8a279d2af12f3cc2cfcd9849007`.
