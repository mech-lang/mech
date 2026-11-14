# R347 — Reuse the current loop continuation

**Retain R343 with the R272 runtime.** The compiler trial stops at the 16-case screen. Natural 256 KiB changes +0.025% in the matched comparison and +0.179% in the primary screen. Reymont changes +0.161% and +0.460% in those comparisons.

Measured parent 671803fd07121caa03da37b80fb5244fb9ead48d. Published parent 671803fd07121caa03da37b80fb5244fb9ead48d. PR base 92f75db84e2523693648084bea16cd52ad60777a. The two workers share the exact benchmark descriptor used in R344. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R343 closure and the archived compiler delta. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The benchmark source, bytecode, allocator, tables, capacities and complete-call boundary are pinned.

## Reuse the selected continuation

A dominating branch decides whether the current cursor reached the end. The compiler follows the current Boolean through the completed update and forwards the proved continuing edge to the same header with the current typed arguments. The shortened edge bypasses only pure scalar dispatch. Stores, checked failures, zero-trip execution and fresh header values follow their original order.

| Linked route | R343 instructions | R347 instructions | R343 conditional branches | R347 conditional branches |
|---|---:|---:|---:|---:|
| Literal tally backedge | 25 | 22 | 5 | 4 |
| Matched-length tally backedge | 47 | 47 | 8 | 7 |

The literal route removes the repeat comparison and two moves. An extra bit mask recomputes the step from the packed word after the indexed store. The comparison counts include the loop header and the complete return to it. The linked Roc and libdeflate literal tally paths execute 16 and 14 instructions respectively through their corresponding continuing edge. The safe Rust control admits a fixed-size histogram view once and uses a single UBFX for the packed field. It isolates literal iteration from mixed match items.

The tally body changes from 640 to 644 code bytes and has a 192-byte frame. The first emission activated 16 redundant vector alternatives because only the general variants changed. The corrected trial applies the same proof to both variants. It changes 32 of 351 Object bodies and preserves the 4,600-call dispatch inventory and byte-identical root Object body. The original and decoded Objects and metadata agree. Disabling the trial reproduces the R343 Object and metadata.

## Matched measurements

| Input | R343 / Roc | R347 / Roc | R343 / libdeflate | R347 / libdeflate | Matched change | Primary change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 2.153231× | 2.152176× | 0.016779× | 0.016770× | -0.049% | +0.000% |
| one-stored | 2.040403× | 2.010107× | 0.017988× | 0.017721× | -1.485% | +0.000% |
| seven-stored-prefix | 2.293136× | 2.248016× | 0.018825× | 0.018454× | -1.968% | +0.000% |
| eight-literals | 0.184277× | 0.184765× | 2.347184× | 2.353400× | +0.265% | -0.078% |
| reymont-prefix-32769 | 1.226529× | 1.228500× | 1.113772× | 1.115562× | +0.161% | +0.460% |
| heldout-window-32768 | 1.221493× | 1.223195× | 1.113253× | 1.114804× | +0.139% | -0.114% |
| prefix-natural-32767 | 1.220063× | 1.220230× | 1.114748× | 1.114901× | +0.014% | +0.225% |
| prefix-natural-32768 | 1.224194× | 1.227141× | 1.115553× | 1.118238× | +0.241% | +0.521% |
| prefix-natural-65537 | 1.227005× | 1.228537× | 1.030638× | 1.031924× | +0.125% | +0.343% |
| prefix-natural-131072 | 1.228489× | 1.229915× | 1.013665× | 1.014842× | +0.116% | +0.183% |
| prefix-natural-262144 | 1.240205× | 1.240511× | 1.000229× | 1.000476× | +0.025% | +0.179% |
| prefix-natural-1048576 | 1.262650× | 1.252120× | 1.019532× | 1.011029× | -0.834% | +0.016% |
| prefix-random-16384 | 1.063978× | 1.054938× | 2.857320× | 2.833041× | -0.850% | -0.703% |
| prefix-random-1048576 | 1.762733× | 1.776403× | 2.270647× | 2.288256× | +0.776% | -0.573% |
| prefix-periodic-1048576 | 0.708106× | 0.708621× | 1.203100× | 1.203974× | +0.073% | +0.018% |
| prefix-repeated-1048576 | 0.710109× | 0.710654× | 1.117318× | 1.118174× | +0.077% | -0.057% |

154,752 balanced complete calls. The screen gates pass 16/16 pooled and 16/16 block. The primary paired geometric mean changes +0.026%. The priority-subset matched mean changes -0.322%. The highest matched change is +0.776% on prefix-random-1048576.

The matched screen counts 3 to 3 cases faster than Roc, 3 to 3 faster than libdeflate, and 0 to 0 faster than both.

| Input | R343 instructions | R347 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 715,869.0 | 715,704.5 | -0.023% | -2.427% |
| reymont-prefix-32769 | 67,173,799.0 | 67,135,106.5 | -0.058% | +0.095% |
| prefix-natural-262144 | 661,202,912.5 | 660,995,197.5 | -0.031% | +0.076% |
| prefix-natural-1048576 | 2,381,234,104.5 | 2,380,495,774.0 | -0.031% | +0.089% |
| prefix-random-16384 | 10,198,056.5 | 10,046,697.0 | -1.484% | -1.036% |
| prefix-random-1048576 | 576,160,159.5 | 567,162,538.0 | -1.562% | -0.440% |
| prefix-periodic-1048576 | 488,788,634.5 | 488,791,620.5 | +0.001% | +0.104% |
| prefix-repeated-1048576 | 496,553,256.0 | 496,556,628.5 | +0.001% | -0.035% |

The six heap distributions match on 16/16 cases. The separately sampled profiles use the exact candidate and include validation.

| Candidate profile input | Running PCs | Tally PCs | Matcher PCs |
|---|---:|---:|---:|
| eight-literals | 5,243 | 0 | 698 |
| prefix-natural-262144 | 5,354 | 99 | 2,032 |
| prefix-random-1048576 | 5,356 | 1,042 | 3,295 |
| prefix-random-16384 | 5,265 | 1,142 | 2,299 |
| reymont-prefix-32769 | 4,329 | 111 | 1,656 |

## Qualification

The compiler, producer and 54 CFG tests build with warnings as errors. The focused test executes 810 native controls across the retained and transformed loops, including zero trips, a fresh comparison, an effect in the skipped region, an early return, and wrapping cursor values. The 324 complete-stream checks retain exact bytes, independent inflation, capacities, aliases and cleanup across both artifact forms. The benchmark uses the compact diagnostic entry.

The archive preserves every pilot result.

The retained all-81 scoreboard remains the R344 campaign with the R343 compiler.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r347-selected-loop-continuation-445b3bad08e6.tar.gz). SHA-256 445b3bad08e6c61b3671b0431a68fbb1e7c295c503af5d43dbd2d0ea94b67792.
