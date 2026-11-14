# R288 — Share current packed-window admission checks

**Retain R288 with the R272 runtime.** Candidate paired elapsed changes -0.799% across 81 fixed level-12 cases. The pooled and block gates pass 81/81 and 81/81. R287 beats Roc on 45/81 cases and R288 beats it on 45/81 (55.56%). The independent four-actor cohort gives R288 3/81 cases below libdeflate.

The two primary cohorts change elapsed by -0.734% and -0.864%. The largest paired elapsed ratio has a change of +1.438% on prefix-random-32769. Cases crossing below 1× Roc are none. Cases crossing above are none.

Pooled gate misses are none. Block gate misses are none.

Published parent cc44167b09e3fe8e05ae30e67025a6b6ac416ce2. Measured parent cc44167b09e3fe8e05ae30e67025a6b6ac416ce2. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R287 compiler and one changed gate emitter. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The candidate uses the pinned input programs, level-12 tables, capacities, limits, R272 runtime and full-call timing boundary.

## Compiler rule and emitted code

A captured-window gate reads its current words before selecting the checked or packed child. Identical checks on those words now share one predicate. Distinct bases, limits, widths and current views still have their own checks. Each current view supplies its geometry once, and a check already proved at this initializer contributes no Boolean identity operation. The original checked fallback and funding branches still use the combined condition.

Eight vector matcher gates each retain one overflow check and one flat-view check. The two inspected linked entry spans go from 19 to 15 instructions, each retaining three compares and replacing five bitwise conjunctions with one. Only the general and vector matcher bodies change among 347. The general body shrinks from 52,964 to 51,856 code bytes and its frame from 1,056 to 1,040 bytes. The vector body shrinks from 27,252 to 27,124 bytes and its frame grows from 864 to 880 bytes. It gains one single stack load and nine single stack stores in the whole static body.

The exact linked Roc and libdeflate loops compare the current scalar count and load two words directly from the flat input. The independent safe Rust slice witness uses a single flat slice length for both windows. All three excerpts and the Rust source are archived.

The initial 27-case screen changes paired elapsed by -1.168%. The screen records 0 pooled and 0 block misses.

## Headline matched ratios

| Input | R287 / Roc | R288 / Roc | R287 / libdeflate | R288 / libdeflate | R288 / R287 elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.201114× | 0.200733× | 2.515561× | 2.510802× | +0.217% |
| prefix-random-8192 | 0.853602× | 0.856097× | 3.098573× | 3.107633× | +0.910% |
| prefix-natural-4096 | 1.077144× | 1.069963× | 1.834289× | 1.822061× | +0.288% |
| prefix-natural-1048576 | 1.470482× | 1.465712× | 1.186713× | 1.182864× | -0.143% |
| prefix-random-1048576 | 1.764005× | 1.781920× | 2.496865× | 2.522222× | +0.993% |
| prefix-periodic-1048576 | 0.846275× | 0.825687× | 1.432777× | 1.397920× | -2.500% |
| prefix-repeated-1048576 | 0.837032× | 0.818237× | 1.315791× | 1.286245× | -2.506% |

The fresh four-actor cohort contains 245,664 measured calls. The same-case Mech and peer ratios use its balanced blocks. The final elapsed-change column and the parity score use the two primary cohorts. The library cohort also records an empty-case ratio of 0.9978× R287.

## Correctness and independent counters

- 324 complete original and decoded linked calls check all 81 exact streams, capacities, independent inflation, retained input/output aliases, and final cleanup. The original and decoded Object and metadata match byte-for-byte.
- Forty-three compiler cleanup tests and the focused gate test pass with warnings as errors. The gate test checks 755 current input combinations against an independent predicate. Its five compiled variants check exact duplicate captures, distinct bases, separate views, different induction/limit words, and different widths. Both selected and declined outcomes execute in every variant, including overflow boundaries and changing matrix geometry.
- The logical payload count, requested bytes, copy and zeroing counters, peak private payload and outstanding private bytes agree for all 81 cases.

| Input | R287 instructions | R288 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 812,595.0 | 812,570.0 | -0.003% | -1.478% |
| prefix-random-8192 | 6,345,788.5 | 6,372,970.5 | +0.428% | +1.398% |
| prefix-natural-4096 | 10,687,836.5 | 10,686,305.5 | -0.014% | -0.700% |
| prefix-natural-1048576 | 3,296,000,207.0 | 3,287,450,626.5 | -0.259% | -0.196% |
| prefix-random-1048576 | 724,658,250.5 | 728,444,171.0 | +0.522% | +0.741% |
| prefix-periodic-1048576 | 616,608,872.0 | 605,139,578.5 | -1.860% | -2.517% |
| prefix-repeated-1048576 | 624,296,013.0 | 612,875,034.0 | -1.829% | -2.280% |

The 1 MiB natural-text CPU worker reports a lifetime peak physical footprint of 54,592,512 bytes before and 54,592,448 bytes after. These process-wide samples include the harness.

Cases with opposite timing directions across cohorts total 15. The packet preserves both ratios and every original observation.

## Exact candidate profile

These are CPU-time samples from the exact linked candidate in a separate cohort. They include harness and validation work.

| Input | Running samples | Path parser | Vector matcher | Matcher stack loads / stores |
|---|---:|---:|---:|---:|
| eight-literals | 5338 | 7 | 636 | 23 / 5 |
| prefix-natural-1048576 | 5365 | 2681 | 2297 | 1039 / 173 |
| prefix-natural-32768 | 5366 | 2636 | 2070 | 869 / 192 |
| prefix-natural-4096 | 5562 | 1579 | 1659 | 544 / 200 |
| prefix-periodic-1048576 | 5354 | 709 | 3688 | 1946 / 305 |
| prefix-random-1048576 | 5355 | 434 | 3417 | 1336 / 668 |
| prefix-repeated-1048576 | 5350 | 659 | 3416 | 1750 / 266 |

The packet maps the exact current instruction addresses and the two reduced gate spans. On 1 MiB periodic input the second span accounts for 187 running samples. The compact timing worker runs outside the ordinary path’s managed admission, resource accounting and recovery.

The closest candidate case above parity is prefix-natural-4096 at 1.077436× Roc. Carry the exact unsigned relationship established by a selected edge. The current matcher repeats a checked subtraction after its opposite comparison and checked one-based increments after proving a strict readable bound. Keep the reached bounds, lexical failure order and fallback for unproved paths.

## All primary timings

| Input | Bytes | R287 / Roc | R288 / Roc | R288 / R287 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.199321× | 2.094365× | 0.952278× | pass/pass |
| one-stored | 1 | 2.088011× | 1.996654× | 0.956247× | pass/pass |
| seven-stored-prefix | 7 | 2.286984× | 2.226431× | 0.973523× | pass/pass |
| eight-literals | 8 | 0.197917× | 0.198346× | 1.002169× | pass/pass |
| eight-overlap | 8 | 0.187926× | 0.188025× | 1.000523× | pass/pass |
| 320-overlap | 320 | 0.217470× | 0.217504× | 1.000159× | pass/pass |
| eight-literals-prefix | 8 | 0.199594× | 0.199414× | 0.999094× | pass/pass |
| 128-biased-dynamic | 128 | 0.263282× | 0.263364× | 1.000312× | pass/pass |
| 32-high-entropy-stored | 32 | 0.266796× | 0.265564× | 0.995384× | pass/pass |
| 16384-distribution-change | 16384 | 1.526631× | 1.542177× | 1.010183× | pass/pass |
| 33027-periodic-window | 33027 | 0.745015× | 0.727810× | 0.976906× | pass/pass |
| random-64 | 64 | 0.279986× | 0.280078× | 1.000329× | pass/pass |
| random-256 | 256 | 0.335975× | 0.335461× | 0.998471× | pass/pass |
| random-1024 | 1024 | 0.398890× | 0.399789× | 1.002254× | pass/pass |
| repeated-1024 | 1024 | 0.265086× | 0.263058× | 0.992351× | pass/pass |
| random-32769 | 32769 | 1.435263× | 1.447767× | 1.008712× | pass/pass |
| repeated-32769 | 32769 | 0.731695× | 0.710342× | 0.970818× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.517533× | 0.520549× | 1.005829× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.392225× | 1.389825× | 0.998276× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.441166× | 1.440574× | 0.999589× | pass/pass |
| heldout-window-32767 | 32767 | 1.442405× | 1.442979× | 1.000398× | pass/pass |
| heldout-window-32768 | 32768 | 1.443152× | 1.442729× | 0.999707× | pass/pass |
| heldout-slides-65537 | 65537 | 1.475694× | 1.473970× | 0.998832× | pass/pass |
| heldout-random-65537 | 65537 | 1.582789× | 1.597621× | 1.009371× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.439653× | 1.439948× | 1.000205× | pass/pass |
| prefix-natural-1024 | 1024 | 0.517741× | 0.520225× | 1.004798× | pass/pass |
| prefix-natural-4096 | 4096 | 1.074338× | 1.077436× | 1.002883× | pass/pass |
| prefix-natural-8192 | 8192 | 1.274906× | 1.276232× | 1.001040× | pass/pass |
| prefix-natural-16384 | 16384 | 1.392143× | 1.392292× | 1.000107× | pass/pass |
| prefix-natural-32767 | 32767 | 1.445040× | 1.444104× | 0.999352× | pass/pass |
| prefix-natural-32768 | 32768 | 1.442248× | 1.439376× | 0.998009× | pass/pass |
| prefix-natural-32769 | 32769 | 1.444203× | 1.442474× | 0.998802× | pass/pass |
| prefix-natural-33027 | 33027 | 1.442626× | 1.442673× | 1.000033× | pass/pass |
| prefix-natural-65535 | 65535 | 1.447863× | 1.446655× | 0.999166× | pass/pass |
| prefix-natural-65536 | 65536 | 1.448166× | 1.448602× | 1.000301× | pass/pass |
| prefix-natural-65537 | 65537 | 1.440628× | 1.439904× | 0.999497× | pass/pass |
| prefix-natural-131072 | 131072 | 1.450623× | 1.451663× | 1.000717× | pass/pass |
| prefix-natural-262144 | 262144 | 1.453579× | 1.451530× | 0.998590× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.470069× | 1.467969× | 0.998572× | pass/pass |
| prefix-random-1024 | 1024 | 0.400380× | 0.402872× | 1.006225× | pass/pass |
| prefix-random-4096 | 4096 | 0.673894× | 0.676116× | 1.003297× | pass/pass |
| prefix-random-8192 | 8192 | 0.856098× | 0.863890× | 1.009101× | pass/pass |
| prefix-random-16384 | 16384 | 1.178199× | 1.187788× | 1.008139× | pass/pass |
| prefix-random-32767 | 32767 | 1.430886× | 1.450212× | 1.013506× | pass/pass |
| prefix-random-32768 | 32768 | 1.431371× | 1.450315× | 1.013235× | pass/pass |
| prefix-random-32769 | 32769 | 1.426113× | 1.446617× | 1.014377× | pass/pass |
| prefix-random-33027 | 33027 | 1.440975× | 1.459362× | 1.012760× | pass/pass |
| prefix-random-65535 | 65535 | 1.612465× | 1.625342× | 1.007986× | pass/pass |
| prefix-random-65536 | 65536 | 1.609625× | 1.625306× | 1.009742× | pass/pass |
| prefix-random-65537 | 65537 | 1.610623× | 1.623746× | 1.008148× | pass/pass |
| prefix-random-131072 | 131072 | 1.699802× | 1.711872× | 1.007101× | pass/pass |
| prefix-random-262144 | 262144 | 1.746681× | 1.762052× | 1.008800× | pass/pass |
| prefix-random-1048576 | 1048576 | 1.763609× | 1.781120× | 1.009929× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.330012× | 0.328046× | 0.994042× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.470471× | 0.459384× | 0.976434× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.551771× | 0.539948× | 0.978573× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.651649× | 0.636423× | 0.976635× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.729324× | 0.711995× | 0.976241× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.736272× | 0.714580× | 0.970538× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.735900× | 0.718211× | 0.975964× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.743508× | 0.726447× | 0.977053× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.793155× | 0.774276× | 0.976198× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.793929× | 0.775455× | 0.976732× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.797291× | 0.776198× | 0.973544× | pass/pass |
| prefix-periodic-131072 | 131072 | 0.825543× | 0.802945× | 0.972627× | pass/pass |
| prefix-periodic-262144 | 262144 | 0.838448× | 0.817908× | 0.975502× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 0.845798× | 0.824654× | 0.975001× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.263568× | 0.261662× | 0.992766× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.419099× | 0.411610× | 0.982132× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.514601× | 0.508052× | 0.987274× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.617331× | 0.601887× | 0.974981× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.701091× | 0.686075× | 0.978582× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.703963× | 0.687371× | 0.976431× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.726614× | 0.711652× | 0.979410× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.748067× | 0.726137× | 0.970684× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.787658× | 0.769016× | 0.976333× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.787603× | 0.769972× | 0.977614× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.790127× | 0.770184× | 0.974759× | pass/pass |
| prefix-repeated-131072 | 131072 | 0.821296× | 0.802793× | 0.977471× | pass/pass |
| prefix-repeated-262144 | 262144 | 0.836650× | 0.817561× | 0.977184× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 0.838339× | 0.817326× | 0.974936× | pass/pass |

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r288-shared-window-checks-102ca9982807.tar.gz). SHA-256 102ca9982807fe5cc1442fd0158241a7e342f4ad91075698ef889d19040d4c0f.
