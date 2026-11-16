# Compiler reduction countdown timing

The complete-child proof already admits both current immutable reads and establishes the nonempty unsigned interval. The compiler pilot uses that proof to carry the biased forward index and remaining count in the scalar continuation. It requires the selected item to use the same biased index, with every other item and cost input independent of the source index. The actual packed item has both a U32 conversion and a low-word mask. The revised recognizer accepts that expression and keeps its strict cost selection, wrapping arithmetic and exact terminal cursor.

The exact composed phase still executes 16 recurring instructions. Its two loads include the same reverse writeback and neither accesses the stack. It emits three backedge moves, and the decrement and loop comparison are separate instructions. The fixed-layout control achieved 11 instructions by using the same physical registers across the backedge and a flag-setting decrement. Changing the order of the selects in the phase IR produced identical VCode. Packing cost and item into one selected value added packing and unpacking instructions.

| Input | Compiler / control | Control / Roc | Compiler / Roc | Compiler / libdeflate |
| --- | ---: | ---: | ---: | ---: |
| Eight literals | 1.003863× | 0.172637× | 0.173304× | 2.200139× |
| Natural 8 KiB | 1.006097× | 1.016235× | 1.022431× | 1.270271× |
| Natural 32 KiB | 0.995130× | 1.150701× | 1.145097× | 1.038332× |
| Natural 256 KiB | 0.995464× | 1.169674× | 1.164368× | 0.941528× |
| Natural 1 MiB | 0.995796× | 1.177970× | 1.173018× | 0.947001× |
| Held-out random 65,537 bytes | 0.998050× | 1.031280× | 1.029269× | 1.735041× |
| Periodic 1 MiB | 1.004719× | 0.584724× | 0.587483× | 0.996650× |
| Repeated 65,537 bytes | 1.002621× | 0.567735× | 0.569224× | 1.004358× |

The ten-case screen takes **0.999994×** its direct control across 7,008 timed calls. Both gates pass 10/10, all six private payload metrics agree, and neither peer gains or loses a crossing. Natural 32 KiB, 256 KiB and 1 MiB improve in both order halves. Periodic 1 MiB and repeated 65,537 bytes slow in both halves. The independent fixed-layout cohort takes 0.995484× control and brings natural 8 KiB to 1.009634× Roc. Keep the retained compiler because the linked variant loses the 8 KiB gain and costs time on repetitive inputs.

The two-file patch starts from `0882815df6b7103a84f17e55f93c17f91126c21f`. Patch SHA-256 `43db02d35f4736d10efeb0e5da03a55c9580c315603d06a9de8796f22c5eb071`. The compiler uses nightly 2026-03-03 with warnings as errors and the pinned `aot,canonical-native,canonical-native-program,canonical-native-program-runtime,canonical-native-runtime,jit` closure. Source and revision-14 bytecode hashes begin `331e659e9218` and `51de51658c1d`. The Object and timed worker hashes begin `21b343230255` and `1e4831676d2c`. The paired workers share the same runtime archive and ABI-verified startup descriptor.

Eight of 357 Object bodies change. Four ordinary no-growth bodies shrink from 1,504 to 1,456 bytes and their frames from 272 to 240 bytes. Four composed bodies grow from 1,344 to 1,360 bytes and keep their 224-byte frame. The first recognizer missed the actual outer mask and its emitter was stopped after inspecting the unchanged loop. Two phase-only probes capture the corrected code. The full original build reproduces the composed VCode exactly.

The 486 original linked checks preserve exact output, independent inflation, capacities, held aliases and cleanup. The direct ordinary and composed phase oracle passes 1,064 calls covering 68,096 positions, including wrap, ties, zero and nonzero starts, vector tails, rejection origin and recovery. Decoded, managed and full 81-case qualification were skipped after the screen. Compiler-planning quarantine, native host catalog and production resident routing pass. The formatting issue in `checked_increment.rs`, warning-policy failure in `native_tagged_map_allocations.rs` and unsafe-audit failure in `check-measurement.rs` are inherited from the retained tree.

Separate counter medians put natural 1 MiB at 2,089.77M → 2,090.41M retired instructions and 608.05M → 604.06M cycles. The 8 KiB counter cohort moves 15.310M → 15.296M instructions and 3.666M → 3.655M cycles. Its elapsed direction differs from the primary screen. The 8 KiB exact-worker profile maps 2,383 of 4,302 running timer PCs to the three leading composed parser routines. The profiles include validation.

[Compiler patch, raw timing blocks, counter and profile receipts, and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-0882815df6-reduction-countdown-compiler-pilot.tar.gz). SHA-256 `5c09629982720e62ab2a948764fb13f32029efd36b6d57e0c8173a06692266d7`. Run `python3 -B verify.py` after unpacking.
