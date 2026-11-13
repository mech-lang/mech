# Carry the previous append word across matched visits

Retain the existing load in the ordinary parser. A one-function, fixed-address pilot takes **1.005074×** paired control time across 12 cases. Both 1.05 gates pass everywhere, but natural 8 KiB slows 1.65%. Neither peer win count changes in this screen.

| Input | Control / Roc | Pilot / Roc | Control / libdeflate | Pilot / libdeflate | Pilot / control |
| --- | ---: | ---: | ---: | ---: | ---: |
| Eight literals | 0.172514× | 0.172697× | 2.191926× | 2.194256× | 1.001063× |
| Natural 8 KiB | 1.006409× | 1.023000× | 1.252489× | 1.273136× | 1.016485× |
| Natural 32 KiB | 1.132737× | 1.134749× | 1.025279× | 1.027101× | 1.001776× |
| Natural 256 KiB | 1.146637× | 1.154517× | 0.924823× | 0.931179× | 1.006873× |
| Natural 1 MiB | 1.156596× | 1.163108× | 0.932990× | 0.938243× | 1.005630× |
| Held-out slides, 65,537 bytes | 1.158663× | 1.166078× | 1.013310× | 1.019795× | 1.006400× |
| Repeated, 65,537 bytes | 0.558165× | 0.556970× | 0.988239× | 0.986124× | 0.997859× |

The linked libdeflate parser carries the preceding cost in a register. The retained Mech empty-run fast path already forwards its last packed append. On the ordinary route, Mech loads the current view's last U64 with a zero default for an empty view. In the exact capacity-proved composed phase, this diagnostic seeds that value once and carries the word written by the actual append on both returning edges. The backing pointer, store, checked fallback and output packet stay in place. The first diagnostic used the wrong seed packet lane; it failed linked qualification and produced no timing observations. The corrected seed derives the lane from the previous load and append address in the CLIF.

The retained body re-emits byte for byte. The trial replaces that one 1,328-byte body, keeps its 224-byte frame and all linked addresses, and relocates the same two table references. The previous-node bounds and load disappear from the ordinary visit. The carried word now spans the match reduction, and the allocated code adds an ordinary literal-table length reload. The literal-table pointer reload was already present. The result is a real trade in loop state despite fewer instructions at the old read. Do not attribute the measured change to one stack slot alone.

All 162 complete linked checks preserve exact bytes, independent inflation, capacities, aliases and cleanup. The 7,200 timed calls retain the same six private payload metrics. Four separate profiles use the exact trial worker; natural 8 KiB has 1,567/4,368 running PCs in the changed body, and natural 32 KiB has 2,784/5,321. No hardware-counter or full-matrix run was made for this rejected diagnostic. Both early harness pin errors occurred outside the completed timing run and remain in the archive.

Measured base `8ede8f7c4e2f603e53a3a3deeb6838f751add3c2`, linked pilot `f711543c47a1566bb4116768903880c9d314c0d40305631caaeda59953198e3a`. No production compiler rule is proposed from this result.

[Exact CLIF, linked disassembly, timings, profiles and offline replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-8ede8f7c4e-carry-previous-node.tar.gz). SHA-256 `a571625f48e55beb6f628e45b26bc88df273cd4185d377d45921d039dd41d339`.
