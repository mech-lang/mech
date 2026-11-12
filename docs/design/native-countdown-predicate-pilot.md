# Countdown predicate timing pilot

This compiler-only trial starts at `d24aed711e1f8ee01abe9f72f2c2581fe65c4038`. It replaces the old countdown word at a join with the Boolean result of its only use, an equality comparison against zero. On entry, the next word equals the old word. On the other edge, the next word is the old word minus one modulo 2⁶⁴. The comparison against zero on that edge is equivalent to testing the next word against the maximum unsigned value. Both edges keep the same arithmetic, memory operations, and control-flow destinations.

The original control and trial workers link against the same runtime. The source and bytecode hashes agree with the retained compression image. One of 351 generated bodies changes. The matcher drops from 10,284 to 10,248 code bytes and its stack frame grows from 576 to 592 bytes. On the measured recurring route, it loses the old counter reload and keeps the cache cursor in a register. The linked route goes from eleven instructions to eight. The counter still occupies a stack slot, and the branch now consumes a Boolean materialized by `cset`.

The seven-case trial is rejected. It takes **1.007504×** the control's paired elapsed time across 6,720 measured calls. Both 1.05 timing gates pass all seven cases. Roc wins hold at the same four cases. The trial loses the repeated 256 KiB win over libdeflate. The published 81-case result remains 47 Roc wins, 11 libdeflate wins, two wins over both, and 56 wins over at least one peer.

| Input | Control / Roc | Trial / Roc | Control / libdeflate | Trial / libdeflate | Trial / control |
| --- | ---: | ---: | ---: | ---: | ---: |
| Eight literals | 0.173614× | 0.174431× | 2.224804× | 2.235266× | 1.004702× |
| Natural 8 KiB | 1.053485× | 1.050306× | 1.308365× | 1.304418× | 0.996983× |
| Natural 1 MiB | 1.191308× | 1.191298× | 0.961786× | 0.961778× | 0.999992× |
| Random 1 MiB | 1.411568× | 1.413869× | 1.990440× | 1.993685× | 1.001630× |
| Periodic 1 MiB | 0.590835× | 0.600844× | 1.003015× | 1.020007× | 1.016941× |
| Repeated 256 KiB | 0.608113× | 0.617719× | 0.991038× | 1.006692× | 1.015795× |
| Repeated 1 MiB | 0.604754× | 0.614860× | 0.948514× | 0.964364× | 1.016711× |

In the independent counter cohort, periodic 1 MiB retires 3,145,670.5 fewer instructions and spends 1,094,582 more cycles. The same pattern appears on both repeated cases. The exact timed trial's periodic profile places 3,338 of 5,361 sampled PCs in the matcher, including 136 on the retained countdown store. The profile includes output validation. The periodic profiles of the pinned peer executables show register-held offsets in their hot eight-byte comparisons. Libdeflate also keeps a nearby countdown in a register and branches on the decrement flags. That particular loop branches on the updated word, while the measured Mech join requires the old word's zero test.

The original control and trial each pass 162 full correctness calls with exact bytes, independent inflate, capacity and alias checks, and zero residual private owners at cleanup. All observed payload metrics agree across the seven timing cases. The separate direct JIT test passes 128 calls with both zero-test polarities, wrapping cases, and controls for a different decrement and a live old word. Seven linked profiles and the independent counter cohort complete. Full 81-case timing and decoded trial emission were skipped after the pilot rejection.

The replay archive includes the exact three-file trial patch, receipts, every timing observation, and the linked disassembly. Run `python3 -B verify-countdown.py` after extraction. Archive SHA-256 `a8ecfb77b6519434d34a8a704b7942aab4595c551ec9be46accd6b9105421c69`. [Evidence archive](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-d24aed711e-countdown-predicate-pilot.tar.gz)
