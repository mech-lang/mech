# R346 — Read the current cost pointer at its use

**Retain R343 with the R272 runtime.** The compiler trial stops at the 16-case screen. Natural 256 KiB changes +1.781% in the matched comparison and +1.944% in the primary screen. Reymont changes +2.203% and +2.219% in those comparisons.

Measured parent 4ee0140fb8fc3c1c829df4fa2813dca355ba7bf3. Published parent 4ee0140fb8fc3c1c829df4fa2813dca355ba7bf3. PR base 92f75db84e2523693648084bea16cd52ad60777a. The two workers share the exact benchmark descriptor used in R344. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R343 closure and the archived compiler delta. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The benchmark source, bytecode, allocator, tables, capacities and complete-call boundary are pinned.

## Read one proved packet lane at the use

The compact caller initializes an immutable input packet for the synchronous numeric phase. The existing packet proof verifies that the pointer itself stays local and that all direct reads use initialized word offsets. The experiment places the cost-pointer load at the lookup when its exact word is in that verified read set. The existing cost extent admission, immutable offset-code table, current contents, checked fallback and output behavior all stay in force.

| Linked route | R343 instructions | R346 instructions | R343 stack reads | R346 stack reads |
|---|---:|---:|---:|---:|
| Offset and cost lookup | 6 | 6 | 2 | 1 |
| Valid record through short dispatch | 21 | 21 | 2 | 1 |
| Singleton through next record | 45 | 45 | 2 | 1 |
| Two candidates through next record | 62 | 62 | 2 | 1 |
| Continuing scalar visit | 16 | 16 | 0 | 0 |

The retained phase loads the current cost pointer at entry and spills it to byte 112. The candidate reads byte 144 of the input packet before the two indexed loads. It removes the entry load and spill. The recurring route still executes four loads, including one for the map base on the stack. The linked record counts include the cursor update, register move and branch on the same backedge. Libdeflate keeps both bases in registers. The pinned Roc excerpt computes the class with a variable shift and a literal table read. The Rust scalar control uses two indexed loads with register-resident bases.

The composed phase changes from 1,424 to 1,408 code bytes and from 224 to 208 frame bytes. Four of 351 object bodies change, all composed phases. The linked address receipt tracks the placement changes. Original and decoded Objects and metadata agree. Disabling the trial reproduces the R343 Object and metadata. The linked bodies differ from their Objects only at the two map address words.

## Matched measurements

| Input | R343 / Roc | R346 / Roc | R343 / libdeflate | R346 / libdeflate | Matched change | Primary change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 2.030959× | 2.028652× | 0.016476× | 0.016458× | -0.114% | +0.000% |
| one-stored | 1.996858× | 2.014796× | 0.017597× | 0.017755× | +0.898% | +0.000% |
| seven-stored-prefix | 2.254291× | 2.212949× | 0.018600× | 0.018259× | -1.834% | +0.000% |
| eight-literals | 0.184270× | 0.183901× | 2.335433× | 2.330759× | -0.200% | -0.961% |
| reymont-prefix-32769 | 1.225805× | 1.252812× | 1.114750× | 1.139311× | +2.203% | +2.219% |
| heldout-window-32768 | 1.222666× | 1.249562× | 1.112900× | 1.137381× | +2.200% | +1.896% |
| prefix-natural-32767 | 1.218451× | 1.244722× | 1.113572× | 1.137581× | +2.156% | +2.206% |
| prefix-natural-32768 | 1.222769× | 1.251514× | 1.111384× | 1.137510× | +2.351% | +2.072% |
| prefix-natural-65537 | 1.227366× | 1.249636× | 1.036422× | 1.055228× | +1.814% | +2.311% |
| prefix-natural-131072 | 1.231831× | 1.250875× | 1.015411× | 1.031109× | +1.546% | +1.845% |
| prefix-natural-262144 | 1.238368× | 1.260425× | 1.000679× | 1.018503× | +1.781% | +1.944% |
| prefix-natural-1048576 | 1.248259× | 1.268545× | 1.008332× | 1.024719× | +1.625% | +1.659% |
| prefix-random-16384 | 1.052431× | 1.052024× | 2.864528× | 2.863420× | -0.039% | -0.099% |
| prefix-random-1048576 | 1.595095× | 1.594804× | 2.251816× | 2.251404× | -0.018% | +0.439% |
| prefix-periodic-1048576 | 0.708580× | 0.708660× | 1.204903× | 1.205039× | +0.011% | +0.056% |
| prefix-repeated-1048576 | 0.710365× | 0.711034× | 1.117215× | 1.118267× | +0.094% | -0.081% |

154,752 balanced complete calls. The screen gates pass 16/16 pooled and 16/16 block. The primary paired geometric mean changes +0.963%. The priority-subset matched mean changes +1.322%. The highest matched change is +2.351% on prefix-natural-32768.

The matched screen counts 3 to 3 cases faster than Roc, 3 to 3 faster than libdeflate, and 0 to 0 faster than both.

| Input | R343 instructions | R346 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 717,574.5 | 717,475.0 | -0.014% | -0.413% |
| reymont-prefix-32769 | 67,212,098.0 | 67,212,022.0 | -0.000% | +2.089% |
| prefix-natural-262144 | 661,258,577.0 | 661,199,591.0 | -0.009% | +1.795% |
| prefix-natural-1048576 | 2,381,272,001.0 | 2,381,228,731.0 | -0.002% | +1.706% |
| prefix-random-16384 | 10,197,621.5 | 10,198,762.0 | +0.011% | +0.000% |
| prefix-random-1048576 | 576,164,117.0 | 576,157,179.0 | -0.001% | -0.498% |
| prefix-periodic-1048576 | 488,789,724.0 | 488,780,755.0 | -0.002% | -0.097% |
| prefix-repeated-1048576 | 496,563,411.0 | 496,555,102.5 | -0.002% | -0.034% |

The six heap distributions match on 16/16 cases. The separately sampled profiles use the exact candidate and include validation.

| Candidate profile input | Running samples | Composed phase linked PCs |
|---|---:|---:|
| eight-literals | 5,266 | 0 |
| prefix-natural-262144 | 5,358 | 3,029 |
| reymont-prefix-32769 | 4,447 | 2,240 |

## Retained executable and peer profiles on random inputs

| Input | Worker | Running PCs | Main linked locations |
|---|---|---:|---|
| prefix-random-16384 | mech-r343 | 5,272 | `_mech_compact_numeric_837_vector` 2,313, `_mech_compact_numeric_15737` 1,165, `_mech_compact_numeric_15296_no_growth` 538 |
| prefix-random-16384 | roc | 5,214 | `_roc__p82cb10df89b60108bc0589036ee8d0c3.fast` 3,263, `_roc__p251d42c9b11261f4b6c38feff48bfd2d.fast` 1,314, `_roc__pe21b3db7436f4bcdc21a284b2c2e5c88.fast` 289 |
| prefix-random-16384 | libdeflate | 5,300 | `_deflate_compress_near_optimal` 2,348, `_deflate_find_min_cost_path` 1,880, `_deflate_optimize_and_flush_block` 416 |
| prefix-random-1048576 | mech-r343 | 5,346 | `_mech_compact_numeric_837_vector` 3,261, `_mech_compact_numeric_15737` 1,056, `_mech_compact_numeric_15296_no_growth` 457 |
| prefix-random-1048576 | roc | 5,354 | `_roc__p82cb10df89b60108bc0589036ee8d0c3.fast` 3,071, `_roc__p251d42c9b11261f4b6c38feff48bfd2d.fast` 1,687, `_roc__pe21b3db7436f4bcdc21a284b2c2e5c88.fast` 320 |
| prefix-random-1048576 | libdeflate | 5,340 | `_deflate_compress_near_optimal` 3,633, `_deflate_find_min_cost_path` 1,345, `_deflate_optimize_and_flush_block` 218 |

The captures include validation and run outside the timing and counter cohorts. The libdeflate matcher samples include enclosing collection work. The Mech matcher and tally bodies are separate symbols. The exact retained tally repeats a cursor test after its indexed update. This is a distinct loop-state target from relocating the parser cost-pointer load.

## Qualification

The compiler, producer and focused tests build with warnings as errors. All 53 CFG tests and four immutable-view tests pass. The linked oracle checks 393,216 offset results across both image forms and three cost snapshots. It exercises the two default ranges, cost changes on one backing buffer, rebinding to a different buffer, rejection origin 15525 and recovery. The 324 complete-stream checks retain exact bytes, independent inflation, capacities, aliases and cleanup. The benchmark uses the compact diagnostic entry.

The initial audit receipt collided with the table-verification output filename. The audit aborted before timing. Separate receipt names preserve both the original failure and the successful rerun.

The archive preserves every pilot result.

The retained all-81 scoreboard remains the R344 campaign with the R343 compiler.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r346-current-cost-pointer-8096282dcd22.tar.gz). SHA-256 8096282dcd226823a6f6fe282d3be2b482b7c109ee59d28a49867606540bf4a6.
