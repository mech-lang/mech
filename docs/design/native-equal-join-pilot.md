# Equal join arguments in the matcher

On every incoming edge to two matcher joins, both cursor slots receive the same value. The common value can vary by edge and invocation. The trial proves equality for the current argument pair at integer comparisons and subtraction, including through an intermediate join. It uses the existing block-argument repair to replace the later formal. A disagreement, an unsupported argument kind, or a cycle without a direct proof keeps the original operands. The checked matcher still performs its tree writes, literal selection, cache capacity check, header write, cursor increment, and old-countdown test.

The separate control is the selected-read image from `f3a628b73cb01a7a7d8ee034714ec02b41674a5f`. Two formal arguments coalesce in the exact matcher IR. The linked branch now tests a constant false value. It always bypasses the last-match load, whose block remains in the executable. The header's zero delta is folded, and its store uses the same admitted cache position. One of 351 bodies changes. The matcher grows from 10,284 to 10,288 code bytes with a 576-byte frame in each build. Across the entire body, static stack reads change from 358 to 363 and writes from 105 to 104.

| Input | Control/Roc | Trial/Roc | Control/libdeflate | Trial/libdeflate | Trial/control |
| --- | ---: | ---: | ---: | ---: | ---: |
| Eight literals | 0.171403× | 0.171033× | 2.191859× | 2.187129× | 0.997842× |
| Natural 8 KiB | 1.046753× | 1.046701× | 1.306673× | 1.306608× | 0.999950× |
| Natural 1 MiB | 1.192341× | 1.192689× | 0.962307× | 0.962587× | 1.000292× |
| Random 1 MiB | 1.410790× | 1.419985× | 1.993638× | 2.006632× | 1.006518× |
| Periodic 1 MiB | 0.591694× | 0.591888× | 1.004379× | 1.004708× | 1.000328× |
| Repeated 256 KiB | 0.607724× | 0.608759× | 0.989977× | 0.991663× | 1.001704× |
| Repeated 1 MiB | 0.605144× | 0.605101× | 0.949872× | 0.949804× | 0.999929× |

The seven-case screen measured **1.000934× control** over 6,720 calls. Both 1.05 gates passed on all seven cases and all peer win sets stayed the same. Periodic and repeated cases retired about 0.8% fewer instructions in the separate counter cohort, with essentially flat cycles. Random 1 MiB retired 0.44% more instructions and spent 0.43% more cycles. The elapsed result and the surviving branch give the next experiment a concrete target. The branch cleanup will run where the equality becomes available.

The original control and candidate passed 162 complete checks each, including exact bytes, independent inflate, capacity, live aliases and cleanup. The JIT suite passed 700 differential pairs with warnings as errors and the register allocator checker. It exercises two destinations from one branch, nested joins, and loop backedges with agreeing, diverging, and unresolved argument pairs. All observed payload metrics agree across the seven cases. The independent counter cohort and seven exact linked profiles completed. This first variant was rejected at the screen. Full timing and decoded emission were skipped.

[Patch, linked assembly, test commands, and timing replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-f3a628b73c-equal-join-pilot.tar.gz). Archive SHA-256 `ae91518f9d3bfd26b22f7a4818c0d8f0c1ffed4147b8af0a64175f305886dc71`.
