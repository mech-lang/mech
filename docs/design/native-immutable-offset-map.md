# R343 — Share an immutable offset map

**Retain R333 with the R272 runtime.** The 81-case primary geometric mean changes -0.600%. The fixed 17-case priority group changes -1.698% in the four-actor comparison.

In the matched run, the candidate wins 46/81 against Roc (56.79%), 3/81 against libdeflate and 0/81 against both.

The candidate does not meet every full retention criterion. See the timing gates and witness results below.

## Full qualification

368,496 complete calls across two fresh primary cohorts and 245,664 calls in a separate four-actor comparison. The comparisons fix level 12, runtime inputs, scratch construction and cleanup inside the call, warmup placement, and output ownership. Independent inflation and validation are outside the timing interval. Each libdeflate call constructs a fresh compressor using the pinned allocator.

Pooled gates pass 81/81. Block-median gates pass 80/81. All six heap distributions match on 81/81 cases. The highest primary movement is +6.888% on empty. The highest matched movement is +4.938% on empty.

| Input | R333 / Roc | R343 / Roc | R333 / libdeflate | R343 / libdeflate | Primary A | Primary B | Matched change |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| reymont-prefix-32769 | 1.265426× | 1.226796× | 1.154232× | 1.118997× | -2.883% | -2.966% | -3.053% |
| prefix-natural-262144 | 1.274948× | 1.236181× | 1.034202× | 1.002755× | -3.053% | -3.034% | -3.041% |
| prefix-natural-131072 | 1.271763× | 1.235304× | 1.046173× | 1.016181× | -2.973% | -2.900% | -2.867% |
| prefix-natural-1048576 | 1.282690× | 1.250345× | 1.035074× | 1.008973× | -2.591% | -2.557% | -2.522% |
| eight-literals | 0.182903× | 0.182928× | 2.324084× | 2.324390× | -0.126% | +0.009% | +0.013% |
| prefix-random-16384 | 1.060299× | 1.056038× | 2.870128× | 2.858593× | +0.054% | +0.133% | -0.402% |
| heldout-window-32768 | 1.261189× | 1.222994× | 1.152066× | 1.117176× | -2.707% | -2.981% | -3.028% |
| prefix-natural-32769 | 1.261236× | 1.225536× | 1.149238× | 1.116708× | -2.750% | -2.739% | -2.831% |
| empty | 2.169531× | 2.276662× | 0.016810× | 0.017640× | +7.459% | +6.321% | +4.938% |

Matched crossings below Roc parity are none. Crossings above are none.

Matched crossings below libdeflate parity are none. Crossings above are none.

blockmean_1_05_gate misses empty.

Individual primary cohort block ratios over 1.05 are primary-b empty (1.059345×), primary-b seven-stored-prefix (1.054818×).

## Empty-input timing and linked placement

| Empty call cohort | R333 block median | R343 block median | Change |
| --- | ---: | ---: | ---: |
| primary-a | 69.899 ns | 73.294 ns | +4.856% |
| primary-b | 68.801 ns | 72.884 ns | +5.935% |
| combined | 69.160 ns | 73.178 ns | +5.809% |

The combined pooled median is 83 ns for each image. The candidate changes only the four composed parser bodies at Object emission. The linked address of every shared text symbol agrees. The new read-only data shifts the 14 original constant tables and the GOT by 64 KiB. The linked root changes 159 page-address relocation words for the tables and memmove. The measurement does not isolate an elapsed effect from that data-placement change.


## Independent counter cohort

| Input | R333 instructions | R343 instructions | Instruction change | Cycle change |
| --- | ---: | ---: | ---: | ---: |
| reymont-prefix-32769 | 70,038,326 | 67,173,362 | -4.091% | -3.439% |
| prefix-natural-262144 | 693,439,020 | 660,913,862 | -4.690% | -3.828% |
| prefix-natural-1048576 | 2,488,756,232 | 2,381,725,782 | -4.301% | -3.287% |
| eight-literals | 715,914 | 715,913 | -0.000% | +1.781% |
| prefix-random-1048576 | 576,155,566 | 576,163,024 | +0.001% | +0.100% |

## Exact candidate profile after timing

Running timer samples include validation. They are a separate cohort from both elapsed time and hardware counters.

| Input | Running PCs | Matcher | Parser | Composed parser | Other |
| --- | ---: | ---: | ---: | ---: | ---: |
| prefix-natural-1048576 | 5330 | 2247 | 2730 | 2730 | 353 |
| prefix-natural-262144 | 5536 | 2080 | 3088 | 3088 | 368 |
| prefix-random-1048576 | 5353 | 3270 | 453 | 0 | 1630 |
| prefix-random-16384 | 5269 | 2331 | 520 | 0 | 2418 |

The next isolated test can place the map in its own read-only segment and require the original constant and GOT addresses to match before timing. Keep this gate miss in the record.

## Screen, machine code, and correctness evidence

**Proceed to the full matrix.** Natural 256 KiB changes -2.906% in the matched comparison and -2.905% in the primary screen. Reymont changes -2.997% and -2.514% in those comparisons.

Measured compiler ebc837def6d5c878cb3ecdeadd0e3f078a3f12e4. Published parent 8f743bd3977fa2cfa7025868c599c775d002b870. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R333 closure and the archived compiler delta. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The benchmark source, bytecode, allocator, limits, tables, capacities and complete-call boundary are pinned.

## Share the result of the immutable part of the lookup

The compiler propagates the exact U8 literal bytes only when the same payload reaches the current full-view capture along every admitted transport. A mismatch, unknown capture or unsupported path keeps the existing composed path. The bounded wrapping split-index proof binds both cases to the same unsigned packed word, preserves the zero and upper default cases, and establishes a 16-bit index into one 65,536-byte read-only map. The map is shared by all four eligible phases. The next load reads the current cost view with its original 30-entry lower bound. This preserves cost changes on the same backing allocation and the existing view, ownership and profitability checks.

| Linked route | R333 instructions | R343 instructions |
|---|---:|---:|
| Mapped cost lookup | 14 | 6 |
| Default cost lookup | 14 | 6 |
| Valid record through short dispatch | 28 | 21 |
| Singleton through next record | 52 | 45 |
| Two candidates through next record | 70 | 62 |
| Continuing scalar visit | 16 | 16 |

The linked candidate replaces the per-phase 2 KiB setup and one conditional branch with a read-only map. The lookup has four loads versus two, including two explicit stack reads versus one. The libdeflate excerpt reads the packed offset, map and cost with the two bases resident in registers. The pinned Roc excerpt calculates the class with a variable shift and reads the table. The Rust scalar fixture uses an offset mask and the two indexed loads when the bases arrive in registers.

The composed phase changes from 1,520 to 1,424 code bytes and from 2,272 to 224 stack bytes. Four of 351 bodies change, all composed phases. The original and decoded Objects and metadata agree. With the experiment disabled, the Object and metadata reproduce R333. Each of the four linked bodies differs from its Object at just the two table address relocation words. The shared map adds 65,536 bytes in the read-only section.

## Matched measurements

| Input | R333 / Roc | R343 / Roc | R333 / libdeflate | R343 / libdeflate | Matched change | Primary change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 2.100942× | 2.201056× | 0.016751× | 0.017549× | +4.765% | +0.000% |
| one-stored | 2.029391× | 2.124383× | 0.017842× | 0.018678× | +4.681% | +0.000% |
| seven-stored-prefix | 2.236056× | 2.323981× | 0.018574× | 0.019305× | +3.932% | +0.000% |
| eight-literals | 0.182669× | 0.182026× | 2.342087× | 2.333851× | -0.352% | +0.201% |
| reymont-prefix-32769 | 1.263224× | 1.225360× | 1.147268× | 1.112879× | -2.997% | -2.514% |
| heldout-window-32768 | 1.260632× | 1.222610× | 1.148906× | 1.114254× | -3.016% | -2.784% |
| prefix-natural-32767 | 1.261161× | 1.226373× | 1.147645× | 1.115988× | -2.758% | -2.731% |
| prefix-natural-32768 | 1.261035× | 1.226648× | 1.148963× | 1.117632× | -2.727% | -2.606% |
| prefix-natural-65537 | 1.271504× | 1.232462× | 1.069388× | 1.036552× | -3.071% | -3.344% |
| prefix-natural-131072 | 1.268977× | 1.228980× | 1.045988× | 1.013019× | -3.152% | -2.913% |
| prefix-natural-262144 | 1.277652× | 1.240517× | 1.031587× | 1.001604× | -2.906% | -2.905% |
| prefix-natural-1048576 | 1.282118× | 1.249556× | 1.035306× | 1.009013× | -2.540% | -2.511% |
| prefix-random-16384 | 1.071060× | 1.063120× | 2.915181× | 2.893568× | -0.741% | -0.640% |
| prefix-random-1048576 | 1.589096× | 1.590739× | 2.245828× | 2.248150× | +0.103% | +0.059% |
| prefix-periodic-1048576 | 0.708637× | 0.709481× | 1.202266× | 1.203699× | +0.119% | +0.029% |
| prefix-repeated-1048576 | 0.710740× | 0.711075× | 1.116962× | 1.117488× | +0.047% | -0.004% |

154,752 balanced complete calls. The screen gates pass 16/16 pooled and 16/16 block. The primary paired geometric mean changes -1.426%. The priority-subset matched mean changes -0.943%. The highest matched change is +4.765% on empty.

| Input | R333 instructions | R343 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 717,385.5 | 717,500.5 | +0.016% | -0.946% |
| reymont-prefix-32769 | 70,071,152.5 | 67,209,868.0 | -4.083% | -3.427% |
| prefix-natural-262144 | 693,678,200.5 | 661,236,649.5 | -4.677% | -3.768% |
| prefix-natural-1048576 | 2,488,071,584.5 | 2,381,128,091.0 | -4.298% | -3.459% |
| prefix-random-16384 | 10,196,198.0 | 10,198,830.0 | +0.026% | -0.417% |
| prefix-random-1048576 | 576,163,567.0 | 576,164,970.0 | +0.000% | +0.186% |
| prefix-periodic-1048576 | 488,912,790.0 | 488,786,625.5 | -0.026% | -0.025% |
| prefix-repeated-1048576 | 496,686,170.0 | 496,562,899.5 | -0.025% | +0.003% |

The six heap distributions match on 16/16 cases. The separately sampled profiles use the exact candidate and include validation.

| Candidate profile input | Running samples | Composed phase linked PCs |
|---|---:|---:|
| eight-literals | 5,276 | 0 |
| prefix-natural-262144 | 5,360 | 2,992 |
| reymont-prefix-32769 | 4,740 | 2,370 |

## Qualification

The first compiler build exposed an interval-field mismatch and an incorrect trusted-load argument. Both were corrected. The final compiler and producer builds pass with warnings as errors. All 53 CFG tests and four immutable-view tests pass. The linked oracle checks all 65,536 offsets on each of two current cost snapshots and both image forms for 262,144 results. It includes both default ranges, unchanged backing with new costs, the source rejection origin and successful retry. The 324 whole-compressor checks retain exact streams, independent inflation, capacities, held aliases and cleanup. The benchmark uses the compact diagnostic entry.

The archive preserves every pilot result.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r343-immutable-offset-map-ad2adc483969.tar.gz). SHA-256 ad2adc48396938e4a53bd49d3e84ca1ef9b454cbccd5ff3aa74c22644abcb168.
