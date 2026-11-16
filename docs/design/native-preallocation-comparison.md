# R335 — Measure comparison reuse before register allocation

**Retain R333 with the R272 runtime.** The backend experiment stops at the 16-case screen. Natural 256 KiB changes +1.864% in the four-actor comparison and +1.719% in the primary screen. Reymont changes +0.981% and +0.618% in those same comparisons.

Measured compiler ebc837def6d5c878cb3ecdeadd0e3f078a3f12e4. Published parent 431b568d80ded9e445e7aa3658b1586a4d0401c7. PR base 92f75db84e2523693648084bea16cd52ad60777a. Mech compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the unchanged R333 closure and a five-file backend delta. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, and IR/work/ABI/bytecode 215/107/39/14. The source, bytecode, allocator, limits, table contents, capacities and complete-call boundary are pinned.

## Shorten the live range before allocation

A block-local AArch64 rewrite recognizes two exact same-width, same-order comparisons with virtual operands. A conditional select must use the first and immediately follow the second. Only known flag-preserving forms may intervene, with either compared operand clobber ending the proof. Replace the second comparison with a zero-byte instruction before collecting allocator operands. The rewrite preserves flags, instruction indices and source locations. The later conditional select can reuse an old comparison operand register.

The retained emitter already shares the cost comparison after allocation. In the retained VCode the second comparison still extends the old minimum live range across the first select. In the revised VCode it is a zero-byte placeholder. The first select writes its result into the old minimum register in the linked loop and the backedge transfer disappears. The loop changes 16 to 15 instructions and 3 to 2 moves. Both forms use two loads and zero explicit stack transfers. The remaining cost comparison, loop comparison and branch remain in the exact binary.

The CLIF matches exactly. Sixteen of 351 bodies change across the parser variants. The inspected composed phase changes 1,520 to 1,504 code bytes. Its frame remains 2,272 bytes. All frame sizes, the root and 334 other bodies agree. Disabling the pass restores the retained Object and metadata byte for byte. The retained Roc scalar reduction uses one cost comparison for both choices and a separate count-down branch. The libdeflate excerpt branches on strict cost improvement.

## Matched measurements

| Input | R333 / Roc | R335 / Roc | R333 / libdeflate | R335 / libdeflate | Matched change | Primary change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 2.212506× | 2.066066× | 0.017605× | 0.016439× | -6.619% | +0.000% |
| one-stored | 2.106737× | 1.988012× | 0.018591× | 0.017543× | -5.636% | +0.000% |
| seven-stored-prefix | 2.248234× | 2.251788× | 0.018766× | 0.018795× | +0.158% | +0.000% |
| eight-literals | 0.181599× | 0.181520× | 2.319593× | 2.318592× | -0.043% | +0.417% |
| reymont-prefix-32769 | 1.259204× | 1.271561× | 1.146441× | 1.157691× | +0.981% | +0.618% |
| heldout-window-32768 | 1.259851× | 1.267696× | 1.147869× | 1.155017× | +0.623% | +0.711% |
| prefix-natural-32767 | 1.255986× | 1.268913× | 1.145635× | 1.157426× | +1.029% | +1.057% |
| prefix-natural-32768 | 1.260661× | 1.274577× | 1.149199× | 1.161884× | +1.104% | +1.045% |
| prefix-natural-65537 | 1.264697× | 1.283011× | 1.068190× | 1.083659× | +1.448% | +1.095% |
| prefix-natural-131072 | 1.267847× | 1.287122× | 1.045998× | 1.061900× | +1.520% | +1.821% |
| prefix-natural-262144 | 1.276194× | 1.299978× | 1.029345× | 1.048529× | +1.864% | +1.719% |
| prefix-natural-1048576 | 1.283762× | 1.302439× | 1.036306× | 1.051383× | +1.455% | +1.484% |
| prefix-random-16384 | 1.091852× | 1.085994× | 2.909099× | 2.893491× | -0.537% | -0.706% |
| prefix-random-1048576 | 1.592754× | 1.596592× | 2.251854× | 2.257280× | +0.241% | -0.187% |
| prefix-periodic-1048576 | 0.708815× | 0.708758× | 1.202914× | 1.202817× | -0.008% | +0.010% |
| prefix-repeated-1048576 | 0.713243× | 0.711322× | 1.120227× | 1.117209× | -0.269% | -0.190% |

154,752 balanced matched calls. The screen timing gates pass 16/16 and 16/16. The primary paired geometric mean changes +0.553%. The largest adverse matched movement is +1.864% on prefix-natural-262144.

The separate 256 KiB hardware cohort measures 693,693,963.5 to 686,797,318.5 instructions (-0.994%). Cycles change +0.726%. The six heap distributions match on 16/16 screen cases.

| Candidate profile input | Running samples | Composed phase linked PCs |
|---|---:|---:|
| eight-literals | 5,226 | 0 |
| prefix-natural-262144 | 5,336 | 3,089 |
| reymont-prefix-32769 | 4,694 | 2,439 |

## Retained random-input profiles

Six separate five-second captures use the exact matched R333, Roc and libdeflate executables. All use level 12 and independently verify the stream and aliases. These running timer samples include validation. The table uses each image’s own linked program counters.

| Input | R333 running PCs | Matcher | Parser | Root | Other generated | Other worker or external |
|---|---:|---:|---:|---:|---:|---:|
| prefix-random-16384 | 5,248 | 2,303 | 516 | 278 | 2,037 | 114 |
| prefix-random-1048576 | 5,362 | 3,283 | 450 | 12 | 1,534 | 83 |

| Input | Executable | Hottest linked function | Samples at function / all running PCs |
|---|---|---|---:|
| prefix-random-16384 | mech-r333 | `_mech_compact_numeric_837_vector` | 2,303 / 5,248 |
| prefix-random-16384 | roc | `_roc__p82cb10df89b60108bc0589036ee8d0c3.fast` | 3,332 / 5,219 |
| prefix-random-16384 | libdeflate | `_deflate_compress_near_optimal` | 2,370 / 5,305 |
| prefix-random-1048576 | mech-r333 | `_mech_compact_numeric_837_vector` | 3,283 / 5,362 |
| prefix-random-1048576 | roc | `_roc__p82cb10df89b60108bc0589036ee8d0c3.fast` | 3,058 / 5,358 |
| prefix-random-1048576 | libdeflate | `_deflate_compress_near_optimal` | 3,613 / 5,342 |

## Qualification

The backend libraries, GPU compiler and producer build with warnings as errors. JIT checks pass 5,832 calls across enabled and disabled variants for U32 and U64. Identical inputs permit comparison reuse. The different-operand controls keep both comparisons and identical machine bytes. The original and decoded linked images agree and pass 324 exact-stream, independent inflation, capacity, alias and cleanup checks. The linked oracle covers 136,192 candidate positions and 68,096 retained positions, including wrapping, strict ties, vector remainders, empty and repeated segments, source error precedence and successful retry. The benchmark comparison uses the compact diagnostic entry.

A temporary cleanup removed the inherited sparse frontend sources, pinned bytecode and a CPU counter helper. The build reconstitutes the frontend from the cached crate and pinned patch and matches the helper and bytecode hashes to prior copies. The first JIT control selected the wrong backend build path. The reported counts come from the rebuilt, pinned candidate. The flag-disabled Object and metadata independently reproduce R333.

The retained 81-case scoreboard remains R333. All 16 pilot timing and counter observations, including every adverse result, are in the replay.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r335-preallocate-comparison-4ca0a9af5bca.tar.gz). SHA-256 4ca0a9af5bcab70fa8c697cd02ed31d47a9c9a6316d9126e5f864d71ed0ed4b8.
