# R313 — Reuse proved matcher window comparisons

**Retain R313 with the R272 runtime.** Across 81 cases the primary paired elapsed change is -0.780%. The pooled and block gates pass 81/81 and 81/81. Mech beats Roc in 45/81 primary cases (55.56%). The matched four-actor run records 3/81 faster than libdeflate and 0/81 faster than both.

The two primary cohorts change -0.759% and -0.801%. The greatest adverse paired movement is +0.989% on eight-overlap. Crossings below parity include none. Crossings above parity include none.

Measured parent 7877f21ecc971fede210201e865d1e0f6a6147c3. Published parent 7877f21ecc971fede210201e865d1e0f6a6147c3. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R312 closure and attached patch. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, and IR/work/ABI/bytecode 215/107/39/14. The pinned source, bytecode, allocator, tables, limits, buffers and full-call timing boundaries are identical.

## Reuse the selected input-window comparisons

The original checked addition and readable-window assertion establish two selected-edge facts for the current scalar inputs. A later unsigned I64 compare can reuse a fact when it has the same current operands and a matching wrapping addition. Reversed operands use the equivalent unsigned condition. A fresh load, signed comparison, different operand, or alternate entry leaves the later check intact. Memory effects stay in their original order.

In the matcher, both repeated checks are reached only after the original successful edges. The proof audit removes each successful block in turn and finds its corresponding repeated check unreachable. The independently required candidate-order comparison and its original failure origin remain in the linked ARM64 code.

The compiler replaces 61 static comparisons in 30 routines. 30 of 347 bodies change. The vector matcher body changes from 13,112 to 12,952 bytes and its stack frame from 656 to 640 bytes. A preflight build still emitted a branch on a constant true Boolean. The measured build redirects only the already-proved direct branch and preserves the candidate-order branch. The preflight assembly is retained with the measured CLIF, VCode and linked ARM64.

The selected extension-entry sequence shrinks from ten ARM64 instructions to three. The whole matcher has 563 to 557 static stack-load instructions and 133 to 122 static stack-store instructions. The running stack-load share inside the matcher is 28.63% and 28.88%. The sampled hot stack reads remain relevant for a separate address-carrying experiment.

## Matched results

| Input | R312 / Roc | R313 / Roc | R312 / libdeflate | R313 / libdeflate | Primary elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.181078× | 0.183156× | 2.306530× | 2.332990× | +0.725% |
| prefix-natural-4096 | 0.989913× | 0.985195× | 1.693191× | 1.685120× | -0.613% |
| prefix-random-16384 | 1.103377× | 1.093201× | 2.976185× | 2.948736× | -0.133% |
| heldout-window-32768 | 1.354765× | 1.347196× | 1.235003× | 1.228103× | -0.753% |
| reymont-prefix-32769 | 1.354036× | 1.348475× | 1.232615× | 1.227553× | -0.518% |
| prefix-natural-32767 | 1.354085× | 1.348764× | 1.234217× | 1.229367× | -0.778% |
| prefix-natural-32768 | 1.358948× | 1.351606× | 1.231854× | 1.225199× | -0.784% |
| prefix-natural-262144 | 1.385548× | 1.377538× | 1.122747× | 1.116256× | -0.362% |
| prefix-natural-1048576 | 1.401785× | 1.392938× | 1.130270× | 1.123136× | -0.650% |
| prefix-periodic-1048576 | 0.740598× | 0.729926× | 1.252752× | 1.234700× | -1.493% |
| prefix-repeated-1048576 | 0.740535× | 0.729368× | 1.163245× | 1.145704× | -1.306% |

The separate four-actor cohort has 245,664 timed calls and exact raw stream agreement across all 81 cases. Each libdeflate call includes a fresh level-12 compressor. The 15-case pilot measured -0.424% by paired block-median ratios.

## Cases where Roc is the faster peer

Roc is faster than libdeflate in 17/81 matched cases. The nearest substantial inputs are natural text around 32 KiB. The frontier uses the same balanced blocks for all three ratios.

| Input | Mech / Roc | Mech / libdeflate | Elapsed cut to Roc |
|---|---:|---:|---:|
| heldout-window-32768 | 1.347196× | 1.228103× | 25.77% |
| reymont-prefix-32769 | 1.348475× | 1.227553× | 25.84% |
| prefix-natural-32767 | 1.348764× | 1.229367× | 25.86% |
| prefix-natural-32769 | 1.349039× | 1.229892× | 25.87% |
| prefix-natural-32768 | 1.351606× | 1.225199× | 26.01% |
| prefix-natural-33027 | 1.351751× | 1.233190× | 26.02% |
| heldout-window-32767 | 1.353272× | 1.233899× | 26.11% |
| prefix-natural-65537 | 1.354609× | 1.141497× | 26.18% |

## Independent counters and profiles

The six Mech payload and heap families agree across all 81 cases. CPU counter measurements and profile samples were collected in separate runs. The profile includes validation.

| Input | R312 instructions | R313 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 716,153.5 | 716,183.5 | +0.004% | +1.455% |
| prefix-natural-4096 | 9,489,069.5 | 9,421,946.0 | -0.707% | -0.258% |
| prefix-random-16384 | 10,670,944.0 | 10,569,863.0 | -0.947% | -1.264% |
| heldout-window-32768 | 79,475,928.0 | 78,588,294.5 | -1.117% | -0.463% |
| reymont-prefix-32769 | 79,723,255.0 | 78,838,902.0 | -1.109% | -0.413% |
| prefix-natural-32767 | 79,473,209.5 | 78,586,859.0 | -1.115% | -0.201% |
| prefix-natural-32768 | 79,473,423.5 | 78,587,704.0 | -1.114% | -0.325% |
| prefix-natural-262144 | 803,462,360.5 | 794,323,109.5 | -1.137% | -0.152% |
| prefix-natural-1048576 | 2,899,517,043.5 | 2,862,320,391.5 | -1.283% | -0.425% |
| prefix-periodic-1048576 | 515,351,678.5 | 503,841,439.5 | -2.233% | -2.916% |
| prefix-repeated-1048576 | 523,078,534.5 | 511,617,438.5 | -2.191% | -2.513% |

| Profile | Running samples | Leading linked leaf | Samples |
|---|---:|---|---:|
| candidate-profiles-screen/eight-literals | 5252 | mech_compact_closed_v2 | 1629 |
| candidate-profiles-screen/prefix-natural-262144 | 5346 | mech_compact_numeric_15296_no_growth | 3018 |
| candidate-profiles-screen/reymont-prefix-32769 | 4364 | mech_compact_numeric_15296_no_growth | 2151 |

## Qualification

The compiler and focused Rust tests use warnings as errors. Fifty-two CFG tests and two packet tests pass. The new test executes 60 pairs of before/after JIT calls, including successful windows, overflow and bounds failure order, an intervening store, a fresh load, a bypass, a changed extent, and signed ordering. The 324 linked original and decoded calls agree on exact output, independent inflate, aliases, capacities and zero final private payload and owner records. The two image forms have identical Object and metadata hashes.

The first build reached a full volume while writing its Rust archive. After reclaiming superseded builds with verified receipts, the same build completed. No timing ran during those builds.

The measured compiler patch retains the exact source inputs used for these Objects. A separate rustfmt-checked review patch reformats one call and its trailing comma.

The next matcher pilot targets the two initial scalar reads. In the current search, the selected offset is strictly shorter than the checked full extent. Every incoming saturated candidate base is either zero or a nonwrapping subtraction from the current input position. The current captured input length and both selected original guards remain part of that proof.

The replay checks all four balanced cohorts and the 81 matched cases from raw per-call samples. It regenerates the timing gates, hardware counters, dual-peer frontier and this report.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r313-dominating-window-facts-d8272c2cd309.tar.gz). SHA-256 d8272c2cd309faf5353c275a9713da8f094a89c5048cf1192f5faaf18af106a6.
