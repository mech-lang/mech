# R345 — Fold branches that converge through empty blocks

**Retain R343 with the R272 runtime.** The 81-case primary geometric mean changes -0.153%. The fixed 17-case priority group changes +0.543% in the four-actor comparison.

In the matched run, the candidate wins 46/81 against Roc (56.79%), 3/81 against libdeflate and 0/81 against both.

reymont-prefix-32769 misses the witness requirement in matched (+0.229%).

prefix-natural-262144 misses the witness requirement in combined primary (+0.279%), matched (+0.126%), primary-a (+0.019%), primary-b (+0.540%), primary-b block median (+0.251%).

## Full qualification

The baseline is the exact R343 executable retained by R344. Both Mech launches use the same startup descriptor after verification of the complete input and output layouts.

368,496 complete calls across two fresh primary cohorts and 245,664 calls in a separate four-actor comparison. The comparisons fix level 12, runtime inputs, scratch construction and cleanup inside the call, warmup placement, and output ownership. Independent inflation and validation are outside the timing interval. Each libdeflate call constructs a fresh compressor using the pinned allocator.

Pooled gates pass 81/81. Block-median gates pass 81/81. All six heap distributions match on 81/81 cases. The highest primary movement is +0.390% on prefix-repeated-32769. The highest matched movement is +4.774% on empty.

| Input | R343 / Roc | R345 / Roc | R343 / libdeflate | R345 / libdeflate | Primary A | Primary B | Matched change |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| reymont-prefix-32769 | 1.223491× | 1.226296× | 1.118830× | 1.121396× | -0.170% | -0.171% | +0.229% |
| prefix-natural-262144 | 1.237737× | 1.239299× | 1.000393× | 1.001655× | +0.019% | +0.540% | +0.126% |
| prefix-natural-131072 | 1.230737× | 1.235703× | 1.015090× | 1.019186× | -0.030% | +0.080% | +0.403% |
| prefix-natural-1048576 | 1.250156× | 1.249969× | 1.008342× | 1.008191× | +0.219% | +0.034% | -0.015% |
| eight-literals | 0.183177× | 0.182707× | 2.332984× | 2.327002× | -0.155% | -0.388% | -0.256% |
| prefix-random-16384 | 1.069467× | 1.061731× | 2.896211× | 2.875260× | +0.017% | -0.302% | -0.723% |
| heldout-window-32768 | 1.218231× | 1.220905× | 1.115606× | 1.118054× | -0.100% | -0.059% | +0.219% |
| heldout-window-32767 | 1.218472× | 1.223084× | 1.114568× | 1.118786× | -0.069% | -0.040% | +0.378% |
| prefix-repeated-32769 | 0.623141× | 0.629652× | 1.213256× | 1.225933× | +0.369% | +0.412% | +1.045% |
| empty | 2.147148× | 2.249644× | 0.016712× | 0.017510× | +0.340% | -0.650% | +4.774% |

Matched crossings below Roc parity are none. Crossings above are none.

Matched crossings below libdeflate parity are none. Crossings above are none.

## Empty input with shared metadata

| Empty call cohort | R343 block median | R345 block median | Change |
| --- | ---: | ---: | ---: |
| primary-a | 70.350 ns | 70.951 ns | +0.855% |
| primary-b | 71.143 ns | 70.935 ns | -0.292% |
| combined | 70.723 ns | 70.935 ns | +0.300% |

The linked root entry moves from 0x100094900 to 0x100094700. The emitted root body is byte-identical before linking. The placement receipt records the other linked addresses. The campaign does not isolate a timing effect from placement.


## Independent counter cohort

| Input | R343 instructions | R345 instructions | Instruction change | Cycle change |
| --- | ---: | ---: | ---: | ---: |
| reymont-prefix-32769 | 67,173,103 | 66,633,182 | -0.804% | +0.021% |
| prefix-natural-262144 | 660,945,838 | 655,041,758 | -0.893% | +0.083% |
| prefix-natural-1048576 | 2,381,688,112 | 2,362,272,549 | -0.815% | +0.125% |
| eight-literals | 715,888 | 715,944 | +0.008% | +2.231% |
| prefix-random-1048576 | 576,155,720 | 576,149,060 | -0.001% | +0.036% |

## Exact candidate profile after timing

Running timer samples include validation. They are a separate cohort from both elapsed time and hardware counters.

| Input | Running PCs | Matcher | Parser | Composed parser | Other |
| --- | ---: | ---: | ---: | ---: | ---: |
| prefix-natural-1048576 | 5352 | 2257 | 2737 | 2737 | 358 |
| prefix-natural-262144 | 5589 | 2101 | 3105 | 3105 | 383 |
| prefix-random-1048576 | 5352 | 3282 | 458 | 0 | 1612 |
| prefix-random-16384 | 5230 | 2291 | 482 | 0 | 2457 |

The retained scoreboard uses the accepted R344 campaign. The next experiment tests whether reading the proved immutable cost pointer at its use reduces register pressure around the dependent lookup.

## Screen, machine code, and correctness evidence

**The screen advanced to the full matrix.** Natural 256 KiB changes -0.018% in the matched comparison and -0.040% in the primary screen. Reymont changes -0.160% and -0.194% in those comparisons.

Measured parent 111baac8428e6681113b75268dda9d913cefffb7. Published parent 111baac8428e6681113b75268dda9d913cefffb7. PR base 92f75db84e2523693648084bea16cd52ad60777a. The two workers share the exact benchmark descriptor used in R344. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R343 closure and the archived compiler delta. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The benchmark source, bytecode, allocator, tables, capacities and complete-call boundary are pinned.

## Recognize one continuation reached through either empty route

After read-only lookup composition, both arms of the inspected cache comparison jump through empty blocks to the same parameter-free continuation. The proof follows only acyclic jumps with no instructions before them and no arguments. A different destination, an effect, arguments, or a cycle prevents folding. The rewrite retains the predicate producers until ordinary dead-value cleanup can establish they are unused. It runs on the four admitted immutable-map phases and folds two branches per phase.

| Linked route | R343 instructions | R345 instructions |
|---|---:|---:|
| Offset and cost lookup | 6 | 6 |
| Valid record through short dispatch | 21 | 20 |
| Singleton through next record | 45 | 44 |
| Two candidates through next record | 62 | 61 |
| Continuing scalar visit | 16 | 16 |

The retained native code executes a cache comparison after reading the current cost. The next comparison overwrites its flags before any consumer. R345 proceeds directly to the length extraction and check. The linked record counts include the cursor update, register move and branch on the common backedge. The lookup still reloads both bases from the stack. The pinned libdeflate excerpt reads the offset, slot and cost with register-resident bases. The pinned Roc excerpt computes the class from the same offset through a variable shift and literal table read. The Rust scalar control has the two indexed loads and no separate cache comparison after the cost load.

The composed phase changes from 1,424 to 1,408 code bytes and uses a 224-byte frame. Four of 351 object bodies change, all composed phases. The linked address receipt tracks the resulting placement changes. Both Object forms and metadata agree. Disabling the trial reproduces the R343 Object and metadata byte for byte. The linked bodies differ from their Objects only at the two map address words.

## Matched measurements

| Input | R343 / Roc | R345 / Roc | R343 / libdeflate | R345 / libdeflate | Matched change | Primary change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 2.150503× | 2.121128× | 0.017067× | 0.016833× | -1.366% | +0.000% |
| one-stored | 2.054755× | 2.032706× | 0.018228× | 0.018033× | -1.073% | +0.000% |
| seven-stored-prefix | 2.213630× | 2.248414× | 0.018840× | 0.019136× | +1.571% | +0.000% |
| eight-literals | 0.182785× | 0.182341× | 2.327994× | 2.322338× | -0.243% | +0.657% |
| reymont-prefix-32769 | 1.225247× | 1.223289× | 1.115552× | 1.113769× | -0.160% | -0.194% |
| heldout-window-32768 | 1.221673× | 1.219152× | 1.115947× | 1.113644× | -0.206% | +0.011% |
| prefix-natural-32767 | 1.223106× | 1.222630× | 1.116623× | 1.116187× | -0.039% | -0.090% |
| prefix-natural-32768 | 1.226512× | 1.225004× | 1.115921× | 1.114549× | -0.123% | -0.528% |
| prefix-natural-65537 | 1.223974× | 1.227085× | 1.035174× | 1.037805× | +0.254% | -0.081% |
| prefix-natural-131072 | 1.231120× | 1.230006× | 1.014289× | 1.013371× | -0.090% | -0.110% |
| prefix-natural-262144 | 1.235511× | 1.235290× | 0.999760× | 0.999582× | -0.018% | -0.040% |
| prefix-natural-1048576 | 1.249084× | 1.248086× | 1.009136× | 1.008329× | -0.080% | -0.088% |
| prefix-random-16384 | 1.069283× | 1.062676× | 2.905152× | 2.887202× | -0.618% | -0.614% |
| prefix-random-1048576 | 1.593050× | 1.588981× | 2.252254× | 2.246501× | -0.255% | +0.007% |
| prefix-periodic-1048576 | 0.709291× | 0.708428× | 1.204503× | 1.203038× | -0.122% | -0.043% |
| prefix-repeated-1048576 | 0.710392× | 0.710304× | 1.117326× | 1.117187× | -0.012% | +0.058% |

154,752 balanced complete calls. The screen gates pass 16/16 pooled and 16/16 block. The primary paired geometric mean changes -0.066%. The priority-subset matched mean changes -0.123%. The highest matched change is +1.571% on seven-stored-prefix.

| Input | R343 instructions | R345 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 715,918.5 | 715,827.5 | -0.013% | -2.994% |
| reymont-prefix-32769 | 67,174,414.0 | 66,634,294.0 | -0.804% | +0.044% |
| prefix-natural-262144 | 661,223,488.0 | 655,275,708.5 | -0.900% | -0.048% |
| prefix-natural-1048576 | 2,381,102,003.5 | 2,361,585,690.5 | -0.820% | -0.014% |
| prefix-random-16384 | 10,199,069.5 | 10,196,535.5 | -0.025% | -0.296% |
| prefix-random-1048576 | 576,160,410.0 | 576,162,138.5 | +0.000% | +0.263% |
| prefix-periodic-1048576 | 488,786,402.0 | 488,758,909.5 | -0.006% | +0.015% |
| prefix-repeated-1048576 | 496,560,212.5 | 496,534,229.5 | -0.005% | -0.022% |

The six heap distributions match on 16/16 cases. The separately sampled profiles use the exact candidate and include validation.

| Candidate profile input | Running samples | Composed phase linked PCs |
|---|---:|---:|
| eight-literals | 5,223 | 0 |
| prefix-natural-262144 | 5,336 | 2,973 |
| reymont-prefix-32769 | 4,334 | 2,141 |

## Qualification

The compiler and producer build with warnings as errors. The first test build encountered missing inherited fixtures in the sparse source overlay. A separate test tree restores the prior hash-checked fixtures. All five identity-route tests pass. The focused test checks effect-free convergence, a store, distinct continuation arguments, and a cycle. The linked oracle checks 262,144 offset results across both image forms and two cost versions. It exercises both default ranges, cost changes, the source rejection origin and retry. The 324 whole-compressor checks retain exact streams, independent inflation, capacities, held aliases and cleanup. The benchmark uses the compact diagnostic entry.

The archive preserves every pilot result.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r345-converging-empty-routes-f84a262e63b2.tar.gz). SHA-256 f84a262e63b2011900a95fe044fe1d92de8458891fc5e67def7d33103b52e862.
