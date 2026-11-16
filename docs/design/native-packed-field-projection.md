# R348 — Compose unsigned packed fields

**Retain R343 with the R272 runtime.** The compiler trial stops at the 16-case screen. Natural 256 KiB changes +0.164% in the matched comparison and -0.022% in the primary screen. Reymont changes +0.169% and +0.226% in those comparisons.

Measured parent 34f35828318404255eb21c0c2114f376bfb0acab. Published parent 34f35828318404255eb21c0c2114f376bfb0acab. PR base 92f75db84e2523693648084bea16cd52ad60777a. The two workers share the exact benchmark descriptor used in R344. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R343 closure and the archived compiler delta. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The benchmark source, bytecode, allocator, tables, capacities and complete-call boundary are pinned.

## Compose constant unsigned projections

The compiler traces a low-bit field back through logical right shifts and masks that preserve every requested bit. It combines the shifts within the original word width and drops a mask when the shift already clears those bits. Intermediate values retain the meaning of their remaining uses. Dynamic shifts and masks that clear requested bits keep their own SSA definition.

| Linked route | R343 instructions | R348 instructions | Conditional branches in each |
|---|---:|---:|---:|
| Literal tally backedge | 25 | 24 | 5 |
| Matched-length tally backedge | 47 | 46 | 8 |

Each route uses a shift by 41 on the original packed word. The current histogram bounds, rejection origins and checked writes stay in place. The linked tally still materializes two eight-bit masks on the literal route. The linked reference paths use 16 instructions in Roc and 14 in libdeflate. The Rust literal-only control uses an admitted histogram view and a UBFX for its eight-bit field.

The tally body changes from 640 to 632 code bytes and has a 192-byte frame. Both variants receive three projection rewrites in each of three phases. Six of 351 Object bodies change. The root body and 4,600-call dispatch inventory match. The original and decoded Objects and metadata agree. Disabling the trial reproduces the retained Object and metadata byte for byte.

## Matched measurements

| Input | R343 / Roc | R348 / Roc | R343 / libdeflate | R348 / libdeflate | Matched change | Primary change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 2.188558× | 2.101210× | 0.017125× | 0.016441× | -3.991% | +0.000% |
| one-stored | 2.059380× | 1.984473× | 0.018207× | 0.017545× | -3.637% | +0.000% |
| seven-stored-prefix | 2.280088× | 2.235561× | 0.018841× | 0.018473× | -1.953% | +0.000% |
| eight-literals | 0.184188× | 0.184015× | 2.346073× | 2.343868× | -0.094% | +0.093% |
| reymont-prefix-32769 | 1.225085× | 1.227157× | 1.116023× | 1.117911× | +0.169% | +0.226% |
| heldout-window-32768 | 1.222207× | 1.222960× | 1.114579× | 1.115267× | +0.062% | +0.328% |
| prefix-natural-32767 | 1.224564× | 1.224897× | 1.112198× | 1.112501× | +0.027% | +0.219% |
| prefix-natural-32768 | 1.226221× | 1.225875× | 1.115560× | 1.115245× | -0.028% | +0.143% |
| prefix-natural-65537 | 1.229532× | 1.231856× | 1.034818× | 1.036773× | +0.189% | +0.487% |
| prefix-natural-131072 | 1.228730× | 1.231963× | 1.013817× | 1.016485× | +0.263% | +0.234% |
| prefix-natural-262144 | 1.237152× | 1.239182× | 0.999215× | 1.000854× | +0.164% | -0.022% |
| prefix-natural-1048576 | 1.248967× | 1.249998× | 1.007889× | 1.008722× | +0.083% | +0.089% |
| prefix-random-16384 | 1.081117× | 1.062319× | 2.949247× | 2.897969× | -1.739% | -0.976% |
| prefix-random-1048576 | 1.590710× | 1.584829× | 2.253144× | 2.244815× | -0.370% | -0.149% |
| prefix-periodic-1048576 | 0.709166× | 0.709360× | 1.204186× | 1.204515× | +0.027% | -0.090% |
| prefix-repeated-1048576 | 0.711848× | 0.710932× | 1.118161× | 1.116722× | -0.129% | -0.127% |

154,752 balanced complete calls. The screen gates pass 16/16 pooled and 16/16 block. The primary paired geometric mean changes +0.028%. The priority-subset matched mean changes -0.799%. The highest matched change is +0.263% on prefix-natural-131072.

The matched screen counts 3 to 3 cases faster than Roc, 4 to 3 faster than libdeflate, and 0 to 0 faster than both.

| Input | R343 instructions | R348 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 715,757.0 | 715,890.0 | +0.019% | -0.610% |
| reymont-prefix-32769 | 67,173,010.0 | 67,145,319.0 | -0.041% | -0.031% |
| prefix-natural-262144 | 661,215,554.5 | 660,988,955.0 | -0.034% | -0.020% |
| prefix-natural-1048576 | 2,381,173,186.0 | 2,380,449,182.5 | -0.030% | -0.035% |
| prefix-random-16384 | 10,198,399.0 | 10,146,775.0 | -0.506% | -0.597% |
| prefix-random-1048576 | 576,158,629.0 | 573,163,784.0 | -0.520% | -0.187% |
| prefix-periodic-1048576 | 488,783,159.0 | 488,787,044.5 | +0.001% | +0.063% |
| prefix-repeated-1048576 | 496,557,246.0 | 496,548,291.0 | -0.002% | +0.030% |

The six heap distributions match on 16/16 cases. The separately sampled profiles use the exact candidate and include validation.

| Candidate profile input | Running PCs | Tally PCs | Matcher PCs |
|---|---:|---:|---:|
| eight-literals | 5,268 | 0 | 683 |
| prefix-natural-262144 | 5,365 | 97 | 2,019 |
| prefix-random-1048576 | 5,258 | 1,017 | 3,231 |
| prefix-random-16384 | 5,261 | 1,170 | 2,300 |
| reymont-prefix-32769 | 4,740 | 122 | 1,824 |

## Qualification

The compiler, producer and 54 CFG tests build with warnings as errors. The focused test executes 8,576 native controls over both versions, including 32- and 64-bit words, wrapped counts, overlapping and nonoverlapping masks, an unknown shift, and an alternate use of the intermediate value. The 324 complete-stream checks retain exact bytes, independent inflation, capacities, aliases and cleanup across both artifact forms. The benchmark uses the compact diagnostic entry.

The archive preserves every pilot result.

The retained all-81 scoreboard remains the R344 campaign with the R343 compiler.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r348-packed-field-projection-3253de31f36f.tar.gz). SHA-256 3253de31f36f6e971ce176c8dfaa4353e897198dd379253b10d9a12290dcd603.
