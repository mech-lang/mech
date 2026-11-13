# Register allocation in the continuing ordered reduction

The compiler trial takes **0.994941× control time** over 81 cases. All fourteen natural prefixes improve. Both 1.05 gates pass for every case. Repeated 65,535 bytes crosses above libdeflate in the full and confirmation runs. The grouped histogram compiler remains the retained baseline.

| Input | Trial / control | Control / Roc | Trial / Roc | Control / libdeflate | Trial / libdeflate |
| --- | ---: | ---: | ---: | ---: | ---: |
| Eight literals | 1.001888× | 0.172002× | 0.172327× | 2.175073× | 2.179179× |
| Natural 8 KiB | 0.983694× | 1.024725× | **1.008016×** | 1.271701× | 1.250965× |
| Natural 32 KiB | 0.983865× | 1.147299× | 1.128787× | 1.041673× | 1.024866× |
| Natural 256 KiB | 0.978045× | 1.169499× | 1.143824× | 0.948647× | **0.927820×** |
| Natural 1 MiB | 0.977451× | 1.180623× | 1.154001× | 0.952555× | **0.931076×** |
| Held-out random 65,537 bytes | 0.994351× | 1.035296× | 1.029447× | 1.749937× | 1.740051× |
| Random 1 MiB | 1.003040× | 1.161868× | 1.165400× | 1.641682× | 1.646672× |
| Periodic 1 MiB | 1.000464× | 0.581609× | 0.581879× | 0.985298× | 0.985755× |
| Repeated 65,535 bytes | 1.002882× | 0.565084× | 0.566713× | 0.999673× | 1.002554× |

The full cohort contains 245,664 timed calls, 24 balanced order blocks per case, and 47 improved paired estimates. The matched Roc score remains 52/81 (64.2%). The libdeflate score moves from 15 to 14, and the count faster than both moves from six to five. Periodic 8 KiB has the largest adverse paired result at 1.018299×. The ten-case confirmation repeats the repeated 65,535-byte crossing at 0.997380× to 1.004869× libdeflate and puts natural 8 KiB at 1.005777× Roc. Both ten-case screens include 7,008 timed calls each. The stored, empty and small-input metrics remain in the full evidence.

## Compiler and machine evidence

The trial starts at `335e58a455521e9997cf3fbfac0fdbaea8f96446`. The seven-file compiler patch has SHA-256 `017ff13e3b379133d95f5c9e1deb38d2382009ef5f11554b47d648168a1c9d60`. The original and decoded compiler runs use nightly 2026-03-03, release optimization, warnings as errors, and the `aot,canonical-native,canonical-native-program,canonical-native-program-runtime,canonical-native-runtime,jit` feature closure. The two trial flags are `MECH_REDUCTION_COUNTDOWN=1` and `MECH_ARM64_LOOP_LIVE_RANGES=1`.

The countdown reuses the previously proved immutable windows, U32 cost arithmetic and strict minimum selection. Before allocation, the ARM64 backend shares identical comparison flags, delays the index increment past its last old-index consumer and fuses adjacent subtraction and comparison with equal operands. The VCode rewrite admits a direct or single-edge loop, checks resolved virtual operands, crosses only recognized pure instructions, requires common source locations and skips functions with debug value labels or user stack maps. The original check and load order remain at the source level. Eight ordered reduction bodies change out of 357. Each composed loop goes from 16 to 11 linked instructions and drops three backedge register moves. The four composed frames stay at 224 bytes and the four ordinary frames shrink from 272 to 240 bytes.

Natural 8 KiB retires 15.310M → 14.707M instructions and spends 3.673M → 3.603M cycles in the separate counter cohort. Natural 1 MiB retires 2,089.72M → 2,019.13M instructions and spends 607.26M → 595.11M cycles. Eight literals retire effectively equal instructions and spend 186,579 → 196,775 cycles in that cohort. Lifetime physical peaks and the six private-payload fields agree between workers. Exact linked profiles for three natural sizes still place 2,447 of 4,436 running timer PCs in three composed parser bodies at 8 KiB and 1,236 in the matcher. The sample denominator includes validation.

## Compilation time

The complete source-to-object build takes **617.427 s**, followed by a **1.422 s** worker link. A single executable serves all cases.

| Original compile stage | Seconds |
| --- | ---: |
| Source frontend, including parsing and canonical artifact construction | 56.251 |
| Canonical lowering and native plan preparation | 75.051 |
| Native module setup and numeric routines | 11.991 |
| Operation recipes to source CFG | **433.460** |
| Source CFG, SSA finalization and verification | 25.163 |
| Root control-flow cleanup | 11.998 |
| Root machine code generation | 2.763 |

The producer records stage transitions. A two-second compiler stack sample during the 433-second interval directly attributes 151 of 154 compiler-thread samples to branch-aware buffer last-use analysis. It repeatedly walks the canonical graph to prove when a reference can move while aliases stay valid. At each buffer site, the consumer and the finish-source permission can both ask the same query. The sample locates the costly work. The interval also contains other ownership and phase preparation.

## Qualification and disposition

Original and decoded emission produce identical Object and metadata with hashes beginning `02bf7940d9e0` and `4f75b18869c0`. The exact original worker begins `11103102420b`. The source and revision-14 bytecode remain pinned at `331e659e9218` and `51de51658c1d`. Five worker/descriptor combinations pass 810 exact linked calls with independent inflation, capacities, aliases and zero private owners at cleanup. The direct phase oracle covers 2,128 calls and 136,192 positions across original and decoded. The focused JIT witness passes 29,925 cases per worker with the register allocator checker, including unchanged code on the different-operand and exit-threshold refusals. The separately built managed guard preserves all 13 generated bundle hashes, passes 88 exact outputs, and runs at 0.701731× and 0.696560× Rust.

The managed launcher retry recovered an out-of-space strip failure after reclaiming superseded generated outputs. A first timing attempt stopped before sampling because its receipt had the wrong schema. The corrected receipt reuses the same launcher and pinned Rust binary. The remaining question is whether function placement explains the repeated-input regression. A separate layout experiment can hold the matcher and unrelated functions fixed while keeping the compiled reduction.

[Compiler patch, compile-stage receipts, raw timing blocks, counters and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-335e58a455-reduction-loop-registers.tar.gz). SHA-256 `1e7c87992b116af7d683e59330d7b289441ae1277e1e38cd240df0528923abd6`. Run `python3 -B verify.py` after unpacking.
