# R344 — Match benchmark startup metadata

**Retain R343 with the R272 runtime.** The exact R343 candidate was retimed with shared startup metadata. The 81-case primary geometric mean changes -0.673%. The fixed 17-case priority group changes -1.926% in the four-actor comparison.

In the matched run, the candidate wins 46/81 against Roc (56.79%), 3/81 against libdeflate and 0/81 against both.

## Full qualification

The two Mech executables and their complete-call boundaries match the prior campaign. Their launches use one byte-identical retained metadata file after verification of the complete input and output layouts.

368,496 complete calls across two fresh primary cohorts and 245,664 calls in a separate four-actor comparison. The comparisons fix level 12, runtime inputs, scratch construction and cleanup inside the call, warmup placement, and output ownership. Independent inflation and validation are outside the timing interval. Each libdeflate call constructs a fresh compressor using the pinned allocator.

Pooled gates pass 81/81. Block-median gates pass 81/81. All six heap distributions match on 81/81 cases. The highest primary movement is +2.496% on seven-stored-prefix. The highest matched movement is +3.857% on empty.

| Input | R333 / Roc | Candidate / Roc | R333 / libdeflate | Candidate / libdeflate | Primary A | Primary B | Matched change |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| reymont-prefix-32769 | 1.255082× | 1.221637× | 1.149376× | 1.118748× | -2.721% | -2.954% | -2.665% |
| prefix-natural-262144 | 1.274495× | 1.241978× | 1.033060× | 1.006703× | -3.062% | -3.017% | -2.551% |
| prefix-natural-131072 | 1.274992× | 1.233081× | 1.047524× | 1.013090× | -3.685% | -2.985% | -3.287% |
| prefix-natural-1048576 | 1.283349× | 1.252283× | 1.035016× | 1.009961× | -2.658% | -2.476% | -2.421% |
| eight-literals | 0.182455× | 0.182337× | 2.329816× | 2.328317× | +0.230% | -0.064% | -0.064% |
| prefix-random-16384 | 1.066922× | 1.060161× | 2.895325× | 2.876978× | -0.294% | -0.140% | -0.634% |
| heldout-window-32768 | 1.256596× | 1.222991× | 1.148521× | 1.117806× | -2.767% | -2.700% | -2.674% |
| seven-stored-prefix | 2.259729× | 2.262581× | 0.018513× | 0.018537× | +2.111% | +2.882% | +0.126% |
| empty | 2.068621× | 2.148408× | 0.016296× | 0.016925× | +2.564% | -0.312% | +3.857% |

Matched crossings below Roc parity are none. Crossings above are none.

Matched crossings below libdeflate parity are none. Crossings above are none.

## Empty input with shared metadata

| Empty call cohort | R333 block median | R343 block median | Change |
| --- | ---: | ---: | ---: |
| primary-a | 69.955 ns | 71.575 ns | +2.316% |
| primary-b | 72.040 ns | 71.406 ns | -0.880% |
| combined | 71.096 ns | 71.428 ns | +0.467% |

The executable still contains the 64 KiB placement shift tested in the padding control. The repeated acceptance run changes the startup descriptor supplied to the candidate. The compiler output and linked executable are byte-identical to R343.


## Independent counter cohort

| Input | R333 instructions | R343 instructions | Instruction change | Cycle change |
| --- | ---: | ---: | ---: | ---: |
| reymont-prefix-32769 | 70,038,228 | 67,173,288 | -4.091% | -3.454% |
| prefix-natural-262144 | 693,393,390 | 660,923,546 | -4.683% | -3.813% |
| prefix-natural-1048576 | 2,488,595,442 | 2,381,671,508 | -4.297% | -3.480% |
| eight-literals | 715,916 | 715,875 | -0.006% | -1.381% |
| prefix-random-1048576 | 576,152,970 | 576,161,348 | +0.001% | +0.308% |

## Exact candidate profile after timing

Running timer samples include validation. They are a separate cohort from both elapsed time and hardware counters.

| Input | Running PCs | Matcher | Parser | Composed parser | Other |
| --- | ---: | ---: | ---: | ---: | ---: |
| prefix-natural-1048576 | 5361 | 2260 | 2744 | 2744 | 357 |
| prefix-natural-262144 | 5511 | 2063 | 3076 | 3076 | 372 |
| prefix-random-1048576 | 5364 | 3274 | 448 | 0 | 1642 |
| prefix-random-16384 | 5238 | 2282 | 502 | 0 | 2454 |

## Placement control and metadata crossover

R344 first linked the retained Object with one unused 65,536-byte copy of the compiler-derived map. The padded worker and R343 have the same 1,592 shared symbol addresses and byte-identical linked root, text constants and GOT. The complete linked text differs only within the four composed parser bodies.

Three independent launches rotate which compression worker starts first. Each case in each launch covers all 120 actor orders. The three runs contain 720,000 validated calls.

| Launch rotation | Empty R333 padded vs R333 | Empty R343 vs R333 | Padded / Roc | R343 / Roc | Padded / libdeflate | R343 / libdeflate |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | -0.840% | +8.522% | 2.055867× | 2.249960× | 0.016450× | 0.018003× |
| 1 | -0.996% | +5.382% | 2.103063× | 2.238545× | 0.016555× | 0.017621× |
| 2 | -1.686% | +4.279% | 2.083447× | 2.209862× | 0.016388× | 0.017383× |

The padding did not reproduce the slowdown. The next diagnostic crossed both executables with both metadata files in all 720 actor orders per case, with 432,000 validated calls. Each pair for a given executable uses identical machine code. The two metadata files have identical input and output layouts. Their top-level difference is the numeric-phase diagnostics. All three Mech workers read only the input ports and input/output word counts for execution. They parse and retain the full descriptor before the timed calls.

| Input | R343 metadata on padded worker | R343 metadata on R343 worker | R343 vs padded with shared R333 metadata | R343 with shared metadata / Roc | R343 with shared metadata / libdeflate |
| --- | ---: | ---: | ---: | ---: | ---: |
| empty | +3.518% | +4.288% | +0.226% | 2.102599× | 0.018428× |
| one-stored | +3.177% | +4.892% | -0.428% | 1.970316× | 0.019895× |
| seven-stored-prefix | +3.853% | +3.505% | +0.154% | 2.235940× | 0.020818× |
| eight-literals | -0.612% | -0.120% | -0.764% | 0.190838× | 2.441205× |

The confirmation also holds each worker command byte-identical. The same relative descriptor filename resolves through two working directories. Input, prefix, expected output and fixtures use absolute paths. All 432,000 additional calls are retained.

| Input | R343 metadata on padded worker | R343 metadata on R343 worker | R343 vs padded with shared R333 metadata | R343 with shared metadata / Roc | R343 with shared metadata / libdeflate |
| --- | ---: | ---: | ---: | ---: | ---: |
| empty | +1.404% | +2.551% | -0.954% | 2.133818× | 0.018633× |
| one-stored | +2.141% | +3.058% | -0.852% | 2.054723× | 0.020174× |
| seven-stored-prefix | +3.227% | +3.376% | +0.350% | 2.273632× | 0.021023× |
| eight-literals | -0.108% | +0.331% | +0.211% | 0.191209× | 2.438740× |

The crossover reproduces the tiny slowdown by changing the diagnostic metadata while holding the executable fixed. The full acceptance rerun uses the exact retained descriptor for both Mech executables after verifying their ABI matches. This keeps the retained startup input and the complete-call timer boundary. The original failed R343 campaign remains in the log.

## Screen, machine code, and correctness evidence

**The screen advanced to the full matrix.** Natural 256 KiB changes -3.074% in the matched comparison and -2.807% in the primary screen. Reymont changes -2.925% and -2.917% in those comparisons.

Measured parent ebc837def6d5c878cb3ecdeadd0e3f078a3f12e4. Published parent f33d8c1b0e92208b0592964339dc5dabdc78069d. PR base 92f75db84e2523693648084bea16cd52ad60777a. The candidate is the exact R343 executable. Both Mech workers receive the retained descriptor. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R333 closure and the archived compiler delta. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The benchmark source, bytecode, allocator, limits, tables, capacities and complete-call boundary are pinned.

## Share the result of the immutable part of the lookup

The compiler propagates the exact U8 literal bytes only when the same payload reaches the current full-view capture along every admitted transport. A mismatch, unknown capture or unsupported path keeps the existing composed path. The bounded wrapping split-index proof binds both cases to the same unsigned packed word, preserves the zero and upper default cases, and establishes a 16-bit index into one 65,536-byte read-only map. The map is shared by all four eligible phases. The next load reads the current cost view with its original 30-entry lower bound. This preserves cost changes on the same backing allocation and the existing view, ownership and profitability checks.

| Linked route | R333 instructions | R343 candidate instructions |
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

| Input | R333 / Roc | Candidate / Roc | R333 / libdeflate | Candidate / libdeflate | Matched change | Primary change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 2.122931× | 2.150095× | 0.016669× | 0.016882× | +1.280% | +0.000% |
| one-stored | 2.009246× | 2.037336× | 0.017668× | 0.017915× | +1.398% | +0.000% |
| seven-stored-prefix | 2.233456× | 2.281484× | 0.018566× | 0.018965× | +2.150% | +0.000% |
| eight-literals | 0.183157× | 0.183217× | 2.325644× | 2.326399× | +0.032% | +0.836% |
| reymont-prefix-32769 | 1.262615× | 1.225684× | 1.147032× | 1.113482× | -2.925% | -2.917% |
| heldout-window-32768 | 1.261797× | 1.227018× | 1.148038× | 1.116394× | -2.756% | -2.896% |
| prefix-natural-32767 | 1.258989× | 1.222656× | 1.148582× | 1.115435× | -2.886% | -2.778% |
| prefix-natural-32768 | 1.258888× | 1.224377× | 1.148494× | 1.117008× | -2.741% | -2.843% |
| prefix-natural-65537 | 1.267052× | 1.228167× | 1.067712× | 1.034945× | -3.069% | -3.076% |
| prefix-natural-131072 | 1.268346× | 1.232396× | 1.044881× | 1.015265× | -2.834% | -2.910% |
| prefix-natural-262144 | 1.279090× | 1.239766× | 1.032291× | 1.000554× | -3.074% | -2.807% |
| prefix-natural-1048576 | 1.279957× | 1.249138× | 1.033649× | 1.008761× | -2.408% | -2.663% |
| prefix-random-16384 | 1.066761× | 1.060094× | 2.901065× | 2.882934× | -0.625% | -0.377% |
| prefix-random-1048576 | 1.588407× | 1.588974× | 2.246902× | 2.247703× | +0.036% | -0.218% |
| prefix-periodic-1048576 | 0.709019× | 0.708968× | 1.203225× | 1.203139× | -0.007% | +0.119% |
| prefix-repeated-1048576 | 0.710234× | 0.710542× | 1.116064× | 1.116548× | +0.043% | -0.010% |

154,752 balanced complete calls. The screen gates pass 16/16 pooled and 16/16 block. The primary paired geometric mean changes -1.420%. The priority-subset matched mean changes -1.644%. The highest matched change is +2.150% on seven-stored-prefix.

| Input | R333 instructions | R343 candidate instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 715,783.5 | 715,876.5 | +0.013% | +1.615% |
| reymont-prefix-32769 | 70,038,327.5 | 67,173,333.0 | -4.091% | -3.356% |
| prefix-natural-262144 | 693,671,069.5 | 661,209,605.5 | -4.680% | -3.736% |
| prefix-natural-1048576 | 2,487,995,864.0 | 2,381,073,133.5 | -4.298% | -3.528% |
| prefix-random-16384 | 10,199,367.5 | 10,196,272.0 | -0.030% | +0.044% |
| prefix-random-1048576 | 576,165,725.0 | 576,155,628.0 | -0.002% | -0.022% |
| prefix-periodic-1048576 | 488,911,730.5 | 488,788,776.5 | -0.025% | +0.053% |
| prefix-repeated-1048576 | 496,683,131.0 | 496,557,101.0 | -0.025% | -0.025% |

The six heap distributions match on 16/16 cases. The separately sampled profiles use the exact candidate and include validation.

| Candidate profile input | Running samples | Composed phase linked PCs |
|---|---:|---:|
| eight-literals | 5,220 | 0 |
| prefix-natural-262144 | 5,352 | 2,987 |
| reymont-prefix-32769 | 4,877 | 2,434 |

## Qualification

The first compiler build exposed an interval-field mismatch and an incorrect trusted-load argument. Both were corrected. The final compiler and producer builds pass with warnings as errors. All 53 CFG tests and four immutable-view tests pass. The linked oracle checks all 65,536 offsets on each of two current cost snapshots and both image forms for 262,144 results. It includes both default ranges, unchanged backing with new costs, the source rejection origin and successful retry. The 324 whole-compressor checks retain exact streams, independent inflation, capacities, held aliases and cleanup. The benchmark uses the compact diagnostic entry.

The archive preserves every pilot result.

[Verified compiler patch, linked controls, crossover and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r344-shared-benchmark-metadata-8232c7808111.tar.gz). SHA-256 8232c78081111da1f1207bfb1550d61f37d7ce503fc6487c4677d9e842efa5b5.
