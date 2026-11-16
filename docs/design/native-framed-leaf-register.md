# R317 — Allocate the saved link register in framed leaf functions

**Retain R317 with the R272 runtime.** Across 81 cases the candidate paired elapsed change is +0.010%. The pooled and block gates pass 81/81 and 81/81. The candidate beats Roc in 46/81 primary cases (56.79%). The matched four-actor run records 3/81 faster than libdeflate and 0/81 faster than both in the candidate.

The measured improvement on the priority natural inputs supports retention. The complete set moves +0.010% with opposite signs in the two primary cohorts. The largest adverse movement is +1.442% on prefix-periodic-1048576.

Candidate elapsed changes +0.064% and -0.044%. Crossings below parity include none. Crossings above parity include none.

Both individual cohorts pass the block-mean gate on all cases.

Measured parent 78eaf501bf8d899dc4e058a9bce6580871630864. Published parent 31349f67fb5f4fa2fd96c274e04f13608e4f4c74. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R315 closure and attached patch. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, and IR/work/ABI/bytecode 215/107/39/14. The pinned source, bytecode, allocator, tables, limits, buffers and full-call timing boundaries are identical.

## Use the saved link register inside eligible leaf functions

The linked reference matcher keeps its tree base in x30. The retained matcher and path parser sign and save the link register before their bodies and restore it at each return. Both are leaves. Cranelift previously reserved x30 throughout those bodies.

The ARM64 selector checks the already-lowered instructions before operand collection. The ordinary register environment remains in use for other calling conventions, unframed code, stack probes, stack limits, normal or tail calls, and explicit return-address instructions. An eligible Apple ARM64 leaf can use x30 as a non-preferred register because its guaranteed frame holds the signed return address. The same chosen environment is used for operand collection, allocation and checking.

42 of 347 emitted bodies change. The root with host calls and the matcher CLIF are byte-identical. The matcher has 58 static x30 operands in its body. Every authenticated return restores x30. The matcher still reloads the tree base on all four direction updates. This experiment made a register available and let the allocator choose its use.

The matcher body changes from 12,776 to 12,904 bytes and its frame from 640 to 608 bytes. Static stack reads change from 547 to 570. The sampled stack-load share inside the matcher changes from 24.64% to 28.58%. These are running-sample shares from separate profiling runs.

## Matched results

| Input | R315 / Roc | R317 / Roc | R315 / libdeflate | R317 / libdeflate | Primary elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.183182× | 0.183064× | 2.317330× | 2.315841× | +0.171% |
| prefix-natural-4096 | 0.980266× | 0.974934× | 1.681774× | 1.672626× | -0.724% |
| prefix-random-16384 | 1.095690× | 1.095046× | 2.935115× | 2.933389× | -0.551% |
| heldout-window-32768 | 1.327940× | 1.314469× | 1.213931× | 1.201616× | -1.335% |
| reymont-prefix-32769 | 1.329434× | 1.315550× | 1.216897× | 1.204189× | -1.390% |
| prefix-natural-32767 | 1.339033× | 1.319552× | 1.217873× | 1.200154× | -1.198% |
| prefix-natural-32768 | 1.339640× | 1.322234× | 1.219235× | 1.203393× | -1.247% |
| prefix-natural-65537 | 1.344949× | 1.327774× | 1.128332× | 1.113923× | -1.338% |
| prefix-natural-262144 | 1.361582× | 1.340684× | 1.102998× | 1.086069× | -1.494% |
| prefix-natural-1048576 | 1.367430× | 1.347037× | 1.105167× | 1.088685× | -1.623% |
| prefix-periodic-1048576 | 0.709254× | 0.719817× | 1.200946× | 1.218832× | +1.442% |
| prefix-repeated-1048576 | 0.711228× | 0.721120× | 1.116707× | 1.132238× | +1.326% |

The separate four-actor cohort has 245,664 timed calls and exact raw stream agreement across all 81 cases. Each libdeflate call includes a fresh level-12 compressor. The 15-case pilot measured -0.514% by paired block-median ratios.

## Cases where Roc is the faster peer

Roc is faster than libdeflate in 17/81 matched cases. The nearest substantial inputs are natural text around 32 and 64 KiB. The frontier uses the same balanced blocks for all three ratios.

| Input | Mech / Roc | Mech / libdeflate | Elapsed cut to Roc |
|---|---:|---:|---:|
| heldout-window-32768 | 1.314469× | 1.201616× | 23.92% |
| reymont-prefix-32769 | 1.315550× | 1.204189× | 23.99% |
| prefix-natural-32767 | 1.319552× | 1.200154× | 24.22% |
| heldout-window-32767 | 1.320595× | 1.204264× | 24.28% |
| prefix-natural-32769 | 1.321113× | 1.199918× | 24.31% |
| prefix-natural-32768 | 1.322234× | 1.203393× | 24.37% |
| prefix-natural-65537 | 1.327774× | 1.113923× | 24.69% |
| prefix-natural-65536 | 1.330601× | 1.120505× | 24.85% |

## Independent counters and profiles

The six Mech payload and heap families agree across all 81 cases. CPU counter measurements and profile samples were collected in separate runs. The profile includes validation.

| Input | R315 instructions | R317 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 716,201.5 | 716,103.0 | -0.014% | -3.604% |
| prefix-natural-4096 | 9,274,209.5 | 9,332,661.5 | +0.630% | -0.366% |
| prefix-random-16384 | 10,494,238.0 | 10,567,010.0 | +0.693% | +0.517% |
| heldout-window-32768 | 76,080,362.5 | 76,840,759.5 | +0.999% | -1.342% |
| reymont-prefix-32769 | 76,329,220.5 | 77,092,900.0 | +1.001% | -1.375% |
| prefix-natural-32767 | 76,079,185.0 | 76,839,554.5 | +0.999% | -1.376% |
| prefix-natural-32768 | 76,080,147.0 | 76,841,085.5 | +1.000% | -1.395% |
| prefix-natural-65537 | 182,555,160.0 | 184,486,140.5 | +1.058% | -0.940% |
| prefix-natural-262144 | 764,651,488.5 | 773,383,386.5 | +1.142% | -0.942% |
| prefix-natural-1048576 | 2,739,399,787.5 | 2,771,773,159.5 | +1.182% | -1.472% |
| prefix-periodic-1048576 | 488,170,064.0 | 500,691,481.5 | +2.565% | +1.484% |
| prefix-repeated-1048576 | 496,002,694.0 | 508,473,542.0 | +2.514% | +1.412% |

| Profile | Running samples | Leading linked leaf | Samples |
|---|---:|---|---:|
| candidate-profiles-screen/reymont-prefix-32769 | 4538 | mech_compact_numeric_15296_no_growth | 2331 |
| candidate-profiles-screen/prefix-natural-262144 | 5360 | mech_compact_numeric_15296_no_growth | 3079 |
| candidate-profiles-screen/eight-literals | 5228 | mech_compact_closed_v2 | 1595 |

## Qualification

The compiler and focused Rust tests use warnings as errors. Fifty-four CFG tests pass. The focused register-allocation checker runs 200 actual JIT calls across signed framed, unframed, ordinary-call, return-address-read and stack-probe cases. The four excluded cases have byte-identical VCode. The 324 linked original and decoded calls agree on exact bytes, independent inflate, held aliases, capacities and final managed cleanup. The two image forms have identical Object, metadata and matcher instruction dumps.

The initial JIT library build ran before its module and native prerequisites. Building in dependency order fixed it. The first profile export returned exit 10. After verified build space was reclaimed, the same capture exported and the other profiles completed. During the full CPU cohort, the disk denied the per-case write after all three workers finished the 36th case. The complete 72 timed calls were recovered from those workers’ flushed journals and the last 45 cases ran with the same commands. Both completed primary cohorts and all counter observations were preserved.

The next experiment will replace the exact ordered merge in the two search and two skip directions with a single unsigned selection. The retained peer comparison uses one conditional select for this step.

The replay checks all four balanced cohorts and the 81 matched cases from raw per-call samples. It regenerates the timing gates, hardware counters, dual-peer frontier and this report.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r317-framed-leaf-44707104f86d.tar.gz). SHA-256 44707104f86dcca75c30062bd0e4b9cf1938c8d0575a037261d12d9f69070dcf.
