# R350 — Reuse zero-extended results in ordered selection

**Retain R350 with the R272 runtime.** The 81-case primary geometric mean changes -0.663%. The fixed 17-case priority group changes -0.719% in the four-actor comparison.

In the matched run, the candidate wins 46/81 against Roc (56.79%), 5/81 against libdeflate and 0/81 against both.

## Full qualification

The two Mech executables and their complete-call boundaries match the prior campaign. Their launches use one byte-identical retained metadata file after verification of the complete input and output layouts.

368,496 complete calls across two fresh primary cohorts and 245,664 calls in a separate four-actor comparison. The comparisons fix level 12, runtime inputs, scratch construction and cleanup inside the call, warmup placement, and output ownership. Independent inflation and validation are outside the timing interval. Each libdeflate call constructs a fresh compressor using the pinned allocator.

Pooled gates pass 81/81. Block-median gates pass 81/81. All six heap distributions match on 81/81 cases. The highest primary movement is +1.388% on prefix-repeated-65536. The highest matched movement is +1.664% on prefix-repeated-4096.

| Input | R349 / Roc | Candidate / Roc | R349 / libdeflate | Candidate / libdeflate | Primary A | Primary B | Matched change |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| reymont-prefix-32769 | 1.214641× | 1.202774× | 1.111259× | 1.100402× | -1.317% | -1.239% | -0.977% |
| prefix-natural-262144 | 1.233715× | 1.220096× | 1.000036× | 0.988997× | -0.788% | -1.097% | -1.104% |
| prefix-natural-131072 | 1.230305× | 1.221807× | 1.012356× | 1.005364× | -1.132% | -1.203% | -0.691% |
| prefix-natural-1048576 | 1.247569× | 1.234952× | 1.005072× | 0.994907× | -0.982% | -1.049% | -1.011% |
| eight-literals | 0.183507× | 0.183772× | 2.320180× | 2.323534× | -0.173% | -0.620% | +0.145% |
| prefix-random-16384 | 1.015657× | 1.015327× | 2.776338× | 2.775436× | -0.621% | -0.855% | -0.032% |
| prefix-natural-65537 | 1.219281× | 1.209794× | 1.025773× | 1.017791× | -1.180% | -0.995% | -0.778% |
| prefix-natural-65535 | 1.226353× | 1.215890× | 1.031519× | 1.022718× | -1.512% | -1.323% | -0.853% |
| heldout-window-32768 | 1.218098× | 1.204642× | 1.112260× | 1.099973× | -1.263% | -1.239% | -1.105% |
| prefix-repeated-65536 | 0.659746× | 0.662026× | 1.162167× | 1.166182× | +2.931% | -0.132% | +0.345% |
| prefix-repeated-4096 | 0.358886× | 0.364857× | 1.360094× | 1.382721× | -2.952% | -1.632% | +1.664% |

Matched crossings below libdeflate parity include prefix-natural-262144, prefix-natural-1048576.

## Empty input with shared metadata

| Empty call cohort | R349 block median | R350 block median | Change |
| --- | ---: | ---: | ---: |
| primary-a | 72.868 ns | 69.219 ns | -5.008% |
| primary-b | 71.046 ns | 69.029 ns | -2.838% |
| combined | 71.538 ns | 69.122 ns | -3.377% |

Both executables use the shared R344 startup descriptor. Their root Object body and numeric dispatch agree. The linked placement receipt records the shifted parser bodies.


## Independent counter cohort

| Input | R349 instructions | R350 instructions | Instruction change | Cycle change |
| --- | ---: | ---: | ---: | ---: |
| reymont-prefix-32769 | 66,841,598 | 64,661,719 | -3.261% | -1.122% |
| prefix-natural-262144 | 658,253,239 | 633,718,518 | -3.727% | -0.889% |
| prefix-natural-1048576 | 2,371,772,144 | 2,290,900,490 | -3.410% | -0.741% |
| eight-literals | 716,244 | 716,374 | +0.018% | +0.487% |
| prefix-random-1048576 | 557,569,493 | 557,569,187 | -0.000% | +0.209% |

## Exact candidate profile after timing

Running timer samples include validation. They are a separate cohort from both elapsed time and hardware counters.

| Input | Running PCs | Matcher | Parser | Composed parser | Other |
| --- | ---: | ---: | ---: | ---: | ---: |
| prefix-natural-1048576 | 5336 | 2288 | 2700 | 2700 | 348 |
| prefix-natural-262144 | 5555 | 2117 | 3075 | 3075 | 363 |
| prefix-random-1048576 | 5365 | 3392 | 490 | 0 | 1483 |
| prefix-random-16384 | 5235 | 2385 | 542 | 0 | 2308 |

## Sampled routes in the main composed parser

The retained and candidate profiles each use their own verified executable addresses. These are timer samples from the linked instruction ranges, including validation.

| Input | Image | Lookup | Record control | First short visit | Continuing scalar | Completion | Other in phase | All running PCs |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| prefix-natural-262144 | R349 | 215 | 564 | 360 | 511 | 623 | 863 | 5589 |
| prefix-natural-262144 | R350 | 235 | 464 | 493 | 484 | 524 | 867 | 5555 |
| prefix-natural-1048576 | R349 | 220 | 483 | 343 | 442 | 467 | 704 | 5354 |
| prefix-natural-1048576 | R350 | 193 | 389 | 420 | 386 | 476 | 757 | 5336 |

The scalar completion path still copies the stop index even after the final increment reaches that same value. The next pilot passes the computed exit cursor through the existing checked completion.

## Screen, machine code, and correctness evidence

**The screen advanced to the full matrix.** Natural 1 MiB changes -0.834% in the matched comparison and -1.167% in the primary screen. Natural 256 KiB changes -0.876% and -1.198%. Reymont changes -0.689% and -0.830% in those comparisons.

Measured parent 0f7dc3bd104d85718526ed504590467f8127676d. Published parent 0f7dc3bd104d85718526ed504590467f8127676d. PR base 92f75db84e2523693648084bea16cd52ad60777a. The workers share the R344 startup descriptor. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R349 closure and the archived Mech and ARM64 selector deltas. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. Source, allocator, bytecode, tables, capacities and timing boundaries are pinned.

## Reuse the physical result of the existing 32-bit write

The complete-child proof identifies the pure selected item under a low-word conversion. The emitter projects addition, bit operations and a literal left shift whose wrapped count is below 32. These operations commute with projection. Other expressions use their full result before truncation. Strict comparison still chooses both the cost and the item. The current incumbent and full output packing follow the existing paths.

The ARM64 selector recognizes an unsigned extension of a scalar I32 OR. Existing scalar OR lowering writes a W register, which zeros the upper half. The widening use borrows the same physical definition, including when other uses keep the low word live. A single-use-only combining rule worked in a local test and declined in the actual parser. The selected item feeds multiple continuation edges, which makes the use state of its operands Multiple.

| Linked route | R349 instructions | R350 instructions | Stack reads in each |
|---|---:|---:|---:|
| Offset and cost lookup | 6 | 5 | 2 |
| Valid record through short dispatch | 21 | 19 | 2 |
| Singleton through next record | 45 | 41 | 2 |
| Two candidates through next record | 62 | 57 | 2 |
| Continuing scalar visit | 16 | 15 | 0 |

The linked parser uses W-register OR on the singleton, scalar continuation and vector completion routes and then uses the same value in its wider selection. The offset register copy also disappears from the mapped lookup. Roc uses W-register OR and W-register conditional selection in its scalar minimum loop. The C binary combines the W-register item and stores it on a strict win. The Rust control uses one W-register OR and shares its cost comparison between two selections.

The composed phase moves from 1,424 to 1,376 code bytes with the same 224-byte frame. Eight of 351 bodies change across four parser copies. The root Object, 4,600 call count and 170 vector choices agree. The changed composed bodies differ from their linked Objects only at the mapped table relocation words. The two candidate image forms agree byte for byte. Disabling both deltas reproduces the exact retained image. The selector-only control changes the same eight bodies, with 1,408 composed code bytes, and passes 162 original-image stream checks.

## Matched measurements

| Input | R349 / Roc | R350 / Roc | R349 / libdeflate | R350 / libdeflate | Matched change | Primary change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 2.133382× | 2.115154× | 0.016699× | 0.016556× | -0.854% | +0.000% |
| one-stored | 2.019218× | 2.021061× | 0.017796× | 0.017813× | +0.091% | +0.000% |
| seven-stored-prefix | 2.266711× | 2.279960× | 0.018704× | 0.018813× | +0.584% | +0.000% |
| eight-literals | 0.184357× | 0.183642× | 2.336810× | 2.327745× | -0.388% | +0.376% |
| reymont-prefix-32769 | 1.222880× | 1.214458× | 1.113284× | 1.105617× | -0.689% | -0.830% |
| heldout-window-32768 | 1.220641× | 1.207325× | 1.112459× | 1.100323× | -1.091% | -0.760% |
| prefix-natural-32767 | 1.222202× | 1.213636× | 1.112081× | 1.104286× | -0.701% | -0.940% |
| prefix-natural-32768 | 1.223814× | 1.210936× | 1.112152× | 1.100449× | -1.052% | -1.070% |
| prefix-natural-65537 | 1.229685× | 1.215332× | 1.034111× | 1.022040× | -1.167% | -1.092% |
| prefix-natural-131072 | 1.230844× | 1.215854× | 1.012758× | 1.000424× | -1.218% | -1.128% |
| prefix-natural-262144 | 1.235940× | 1.225114× | 0.996445× | 0.987717× | -0.876% | -1.198% |
| prefix-natural-1048576 | 1.246663× | 1.236261× | 1.005945× | 0.997552× | -0.834% | -1.167% |
| prefix-random-16384 | 1.023453× | 1.019080× | 2.781392× | 2.769509× | -0.427% | -0.209% |
| prefix-random-1048576 | 1.539909× | 1.540937× | 2.177343× | 2.178797× | +0.067% | +0.320% |
| prefix-periodic-1048576 | 0.689181× | 0.688533× | 1.171126× | 1.170025× | -0.094% | -0.166% |
| prefix-repeated-1048576 | 0.693356× | 0.692778× | 1.089413× | 1.088506× | -0.083% | -0.104% |

154,752 balanced complete calls. The screen gates pass 16/16 pooled and 16/16 block. The primary paired geometric mean changes -0.499%. The priority-subset matched mean changes -0.711%. The highest matched change is +0.584% on seven-stored-prefix.

The matched screen counts 3 to 3 cases faster than Roc, 4 to 5 faster than libdeflate, and 0 to 0 faster than both.

| Input | R349 instructions | R350 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 719,501.5 | 719,548.0 | +0.006% | -2.713% |
| reymont-prefix-32769 | 66,838,391.0 | 64,658,224.0 | -3.262% | -0.996% |
| prefix-natural-1048576 | 2,371,090,626.5 | 2,290,167,877.5 | -3.413% | -0.812% |
| prefix-natural-262144 | 658,559,552.0 | 634,019,501.5 | -3.726% | -0.965% |
| prefix-random-16384 | 9,849,251.5 | 9,845,894.5 | -0.034% | -0.453% |
| prefix-random-1048576 | 557,571,882.5 | 557,570,316.5 | -0.000% | +0.039% |
| prefix-periodic-1048576 | 480,623,859.0 | 480,515,205.5 | -0.023% | +0.037% |
| prefix-repeated-1048576 | 488,715,498.0 | 488,608,699.5 | -0.022% | +0.028% |

The six heap distributions match on 16/16 cases. The separately sampled profiles use the exact candidate and include validation.

| Candidate profile input | Running PCs | Tally PCs | Matcher PCs |
|---|---:|---:|---:|
| eight-literals | 5,232 | 0 | 666 |
| prefix-natural-1048576 | 5,367 | 89 | 2,290 |
| prefix-natural-262144 | 5,360 | 102 | 2,043 |
| prefix-random-1048576 | 5,365 | 1,017 | 3,396 |
| prefix-random-16384 | 5,244 | 1,149 | 2,400 |
| reymont-prefix-32769 | 4,326 | 95 | 1,684 |

## Qualification

Compiler, selector and producer builds pass with warnings as errors. The two selector tests exercise 768 native calls across three shifts, high-bit inputs and retained low-word uses. The linked oracle checks 136,192 reduction positions across the two candidate forms and 68,096 retained positions. This covers strict ties, wrapping costs, short and long intervals, vector tails, source rejection and recovery. Both forms also pass 393,216 packed-offset checks and 324 complete-stream checks with independent inflation, capacities, aliases and cleanup. The benchmark uses the compact diagnostic entry.

The first linked width projection and the single-use selector produced the extra extension in the real parser. Their exact code and receipts remain available. The backend rebuild reconstructed the pinned sparse frontend from its original crate and archived patch after a local dependency copy was found incomplete.

The archive preserves every pilot result.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r350-ordered-result-extension-d7da9c0764c4.tar.gz). SHA-256 d7da9c0764c44999f179b653c373c2afbd6a73fd75fc88b48157e272c5927190.
