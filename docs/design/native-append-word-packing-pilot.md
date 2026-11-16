# Append word packing timing

The no-growth append loop reads the upper 16 bits of the current header, shifts them into the packed item and merges a wrapping U32 cost. The original ARM64 loop applies two U32 masks while constructing the packed word. The literal field needs at most 25 bits after its shift and tag. Its placement above the low cost word leaves the existing store and live cost unchanged.

The fixed-layout pilot replaces seven instruction words in each of four functions. Three instructions compute the same packed word and four NOPs preserve the linked addresses. The control and candidate share the same descriptor. The compiler pilot extends the existing unsigned-width proof to remove a mask whose cleared bits are all zero on every incoming edge. It runs in the admitted no-growth phase under `MECH_FOLD_PROVED_MASKS`. Source checks, ownership, capacity and append order remain intact.

| Input | Fixed layout / control | Compiler / control | Compiler / Roc | Compiler / libdeflate |
| --- | ---: | ---: | ---: | ---: |
| Eight literals | 0.992660× | 0.996298× | 0.175986× | 2.190785× |
| Natural 8 KiB | 1.000420× | 1.014268× | 1.040431× | 1.289408× |
| Held-out random 65,537 bytes | 0.995848× | 1.002487× | 1.031239× | 1.746538× |
| Random 1 MiB | 0.995470× | 1.004387× | 1.169479× | 1.653751× |
| Periodic 1 MiB | 0.999195× | 0.997541× | 0.582773× | 0.990082× |
| Repeated 65,537 bytes | 0.997263× | 1.000558× | 0.566193× | 1.000313× |
| Repeated 1 MiB | 0.997522× | 0.995881× | 0.599021× | 0.941729× |

The independent fixed-layout and compiler screens use 24 balanced blocks and 6,912 and 7,008 timed calls. Their geometric means are **0.997540×** and **1.000866×** their direct controls. Both gates pass on 9/9 and 10/10 cases, with equal private payload metrics in each screen. The fixed-layout repeated 65,537-byte result is 0.996371× libdeflate against a control at 0.999105×. Its paired order halves are 1.004224× and 0.990350×. The compiler candidate crosses above libdeflate parity from 0.999756× and natural 8 KiB slows in both orders. Retain the previous compiler.

The compiler removes 39 proved masks in each of four ordinary no-growth phases. These bodies shrink from 1,504 to 1,456 bytes and their frames shrink from 272 to 256 bytes. Four composed siblings shrink from 1,344 to 1,328 bytes with 224-byte frames. The other 349 named bodies retain their Object bytes. The compiler's ordinary packing sequence falls from seven to five instructions. The exact fixed-layout sequence and the compiler's emitted sequence have different register assignments and instruction boundaries.

In the separate hardware cohort, held-out random 65,537 bytes retires 29.00M → 28.60M instructions and uses 6.42M → 6.39M cycles. Natural 8 KiB retires 15.31M → 15.21M instructions and uses 3.69M → 3.72M cycles. Repeated 65,537 bytes retires 24.28M → 24.02M instructions and uses 4.368M → 4.363M cycles. Five cases retain lifetime peak and counter receipts. Sampled PCs are reported in their own cohorts. Before the experiment, 2,411 of 4,426 natural 8 KiB samples land in the three leading composed parser routines. Held-out random and repeated 65,537 bytes place 594 of 4,919 and 515 of 4,809 samples respectively in the ordinary append loop's header and admission instructions. The profiles include validation.

The measured base is `ef7819f6c9c6d2dde555ed635d203f7e7d9c1856`. The patch SHA-256 is `1f7917f7a99508b64e75662a14df776029f4ff8cf69057aee83969a421fbb295`. It builds with nightly 2026-03-03, warnings as errors and the pinned release closure for `aot,canonical-native,canonical-native-program,canonical-native-program-runtime,canonical-native-runtime,jit`. Source and revision-14 bytecode hashes start `331e659e9218` and `51de51658c1d`. The compiler candidate Object and timed worker start `fc2e569af8ae` and `9beeaad0eb5a`. Its timed control links the retained Object with the same library and startup descriptor.

All 810 original linked calls pass exact byte output, independent inflation, capacities, aliases and cleanup. Seven focused width and JIT tests pass, including a wide incoming arm that requires the masks. Nine retained/peer and six candidate profiles are tied to their timed worker hashes. An initial profile-wrapper import, two isolated JIT harness builds and a counter receipt field were corrected before the successful runs. Decoded, full 81-case timing and managed qualification were skipped after the compiler screen.

The primary parser work remains in the admitted read and loop state. The natural composed scalar continuation still uses a 16-instruction loop with two latch transfers. The reference excerpt uses a countdown and an 11-instruction body. The ordinary empty run retains a countdown, header cursor and output length. These are independent targets for the next pilot.

[Raw blocks, matched counters, compiler patch, linked assembly and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-ef7819f6c9-append-word-packing-pilot.tar.gz). SHA-256 `c10b2599ff13a9da9ae04ffcc5548b844f0821f0417244352f1850ebbe254bd4`. Run `python3 -B verify.py` after unpacking.
