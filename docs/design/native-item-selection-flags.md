# R285 — Share the comparison for cost and item selection

**Retain R285 with the R272 runtime.** Candidate paired elapsed changes -0.328% across 81 fixed level-12 cases. The pooled and block gates pass 81/81 and 81/81. R284 beats Roc on 45/81 cases and R285 beats it on 45/81 (55.56%). The independent four-actor cohort gives R285 3/81 cases below libdeflate.

The two primary cohorts change elapsed by -0.325% and -0.331%. The largest slowdown is +1.190% on seven-stored-prefix. Cases crossing below 1× Roc are none. Cases crossing above are none.

Pooled gate misses are none. Block gate misses are none.

Published parent 9cd5c06901de0be02d3e0c5a3ecb2ac4b9a4e3a5. Measured parent 9cd5c06901de0be02d3e0c5a3ecb2ac4b9a4e3a5. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R284 Mech source closure and two changes in the pinned backend. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The candidate uses the pinned input programs, level-12 tables, capacities, limits, R272 runtime and full-call timing boundary.

## Compiler rule and emitted code

The AArch64 emitter already tracked an exact physical-register comparison across known moves and a conditional select. It can now carry that comparison across a single-instruction immediate add or shifted OR. Both forms preserve the flags. A write to either compared register invalidates the fact, including a 32-bit write to the same 64-bit register. The existing barriers include block entries, flag-setting instructions and calls.

In both scalar parser routes the two selections now use one comparison. Four of 347 machine bodies change, with two comparisons removed from each. The changed bodies each occupy 1,744 bytes with a 272-byte frame. The root and matcher are byte-identical. All 227 linked compact entry addresses match, as do the metadata and worker file size.

The pinned Roc loop uses one comparison for both conditional selections. Libdeflate uses a strict branch to commit an improvement. The fresh equivalent scalar C witness emits one comparison and two selections. The pinned Rust witness emits two comparisons and two selections. All are included as assembly evidence.

The initial 27-case screen changes paired elapsed by -0.040%. The screen records 0 pooled and 0 block misses.

## Headline matched ratios

| Input | R284 / Roc | R285 / Roc | R284 / libdeflate | R285 / libdeflate | R285 / R284 elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.206688× | 0.205531× | 2.655055× | 2.640191× | +0.015% |
| prefix-random-8192 | 0.895898× | 0.897192× | 3.244231× | 3.248917× | -0.135% |
| prefix-natural-4096 | 1.085670× | 1.082966× | 1.857962× | 1.853335× | -1.443% |
| prefix-natural-1048576 | 1.493619× | 1.486272× | 1.203342× | 1.197423× | -0.516% |
| prefix-random-1048576 | 1.816989× | 1.818336× | 2.567494× | 2.569397× | -0.196% |
| prefix-periodic-1048576 | 0.898477× | 0.898940× | 1.519825× | 1.520608× | -3.546% |
| prefix-repeated-1048576 | 0.885730× | 0.887026× | 1.391239× | 1.393273× | -0.041% |

The fresh four-actor cohort contains 245,664 measured calls. The same-case Mech and peer ratios use its balanced blocks. The final elapsed-change column and the parity score use the two primary cohorts. The library cohort also records an empty-case ratio of 0.9462× R284.

## Correctness and independent counters

- 324 complete original and decoded linked calls check all 81 exact streams, capacities, independent inflation, retained input/output aliases, and final cleanup. The original and decoded Object and metadata match byte-for-byte.
- The current and candidate backends each pass 3,840 native same-cost and changed-cost checks and 27,104 hazard checks. The emitted same-cost witness loses one comparison in both optimization profiles. The changed-cost witness retains both. Eight tracker tests and 42 selected compiler cleanup tests pass with warnings as errors.
- The logical payload count, requested bytes, copy and zeroing counters, peak private payload and outstanding private bytes agree for all 81 cases.

| Input | R284 instructions | R285 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 813,091.0 | 813,052.0 | -0.005% | -2.119% |
| prefix-random-8192 | 6,534,669.0 | 6,536,133.5 | +0.022% | +0.218% |
| prefix-natural-4096 | 10,895,818.5 | 10,792,058.5 | -0.952% | -0.443% |
| prefix-natural-1048576 | 3,340,885,258.5 | 3,309,060,857.0 | -0.953% | -0.638% |
| prefix-random-1048576 | 748,028,399.0 | 748,023,873.0 | -0.001% | -0.044% |
| prefix-periodic-1048576 | 642,828,596.0 | 642,800,755.5 | -0.004% | -0.004% |
| prefix-repeated-1048576 | 650,469,649.5 | 650,438,040.5 | -0.005% | +0.002% |

The 1 MiB natural-text CPU worker reports a lifetime peak physical footprint of 54,543,296 bytes before and 54,592,512 bytes after. These process-wide samples include the harness.

The 1 MiB periodic timing pairs differ across the two primary cohorts. Their elapsed changes are -0.005% and -6.963%. The independent instruction change is -0.004% and the cycle change is -0.004%. The report includes both complete timing cohorts.

## Exact candidate profile

These are CPU-time samples from the exact linked candidate in a separate cohort. They include harness and validation work.

| Input | Running samples | Path parser samples | Vector matcher samples | Samples at the second matcher reload |
|---|---:|---:|---:|---:|
| eight-literals | 5490 | 6 | 695 | 0 |
| prefix-natural-1048576 | 5368 | 2624 | 2346 | 2 |
| prefix-natural-32768 | 5358 | 2599 | 2103 | 5 |
| prefix-natural-4096 | 5607 | 1539 | 1751 | 9 |
| prefix-periodic-1048576 | 5503 | 678 | 3911 | 0 |
| prefix-random-1048576 | 5487 | 447 | 3529 | 322 |
| prefix-repeated-1048576 | 5342 | 616 | 3550 | 0 |

The matcher reloads x0 from the same spill address on the fallthrough after comparing the first load. Its allocated VCode has three intervening jump-only blocks. Each reached block through the second load has one incoming edge. The audit identifies an exact allocator investigation and supplies the sampled PC. These samples include harness work. The compact timing worker omits the ordinary path’s additional managed admission, accounting and recovery.

The closest candidate case above parity is prefix-natural-4096 at 1.091349× Roc. Trace the two reloads to allocator edits and carry the existing copy facts through the proved chain of blocks with one incoming edge each. Verify refusals at joins and clobbers and then compare emitted code and complete-call timings.

## All primary timings

| Input | Bytes | R284 / Roc | R285 / Roc | R285 / R284 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.120950× | 2.116290× | 0.997803× | pass/pass |
| one-stored | 1 | 1.997652× | 2.002310× | 1.002332× | pass/pass |
| seven-stored-prefix | 7 | 2.236566× | 2.263178× | 1.011899× | pass/pass |
| eight-literals | 8 | 0.199338× | 0.199367× | 1.000147× | pass/pass |
| eight-overlap | 8 | 0.187926× | 0.186909× | 0.994586× | pass/pass |
| 320-overlap | 320 | 0.218244× | 0.217989× | 0.998835× | pass/pass |
| eight-literals-prefix | 8 | 0.197505× | 0.197347× | 0.999198× | pass/pass |
| 128-biased-dynamic | 128 | 0.269541× | 0.269845× | 1.001128× | pass/pass |
| 32-high-entropy-stored | 32 | 0.269928× | 0.268848× | 0.995999× | pass/pass |
| 16384-distribution-change | 16384 | 1.566676× | 1.566322× | 0.999774× | pass/pass |
| 33027-periodic-window | 33027 | 0.789493× | 0.785669× | 0.995157× | pass/pass |
| random-64 | 64 | 0.284320× | 0.284148× | 0.999396× | pass/pass |
| random-256 | 256 | 0.347099× | 0.346336× | 0.997802× | pass/pass |
| random-1024 | 1024 | 0.422452× | 0.422381× | 0.999831× | pass/pass |
| repeated-1024 | 1024 | 0.274914× | 0.273929× | 0.996415× | pass/pass |
| random-32769 | 32769 | 1.474490× | 1.468099× | 0.995665× | pass/pass |
| repeated-32769 | 32769 | 0.767189× | 0.764741× | 0.996810× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.539050× | 0.538246× | 0.998508× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.420322× | 1.412708× | 0.994639× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.466545× | 1.460887× | 0.996142× | pass/pass |
| heldout-window-32767 | 32767 | 1.467781× | 1.458977× | 0.994002× | pass/pass |
| heldout-window-32768 | 32768 | 1.469467× | 1.461543× | 0.994608× | pass/pass |
| heldout-slides-65537 | 65537 | 1.503097× | 1.492547× | 0.992981× | pass/pass |
| heldout-random-65537 | 65537 | 1.638751× | 1.634916× | 0.997660× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.473696× | 1.467352× | 0.995695× | pass/pass |
| prefix-natural-1024 | 1024 | 0.536369× | 0.534861× | 0.997189× | pass/pass |
| prefix-natural-4096 | 4096 | 1.107327× | 1.091349× | 0.985571× | pass/pass |
| prefix-natural-8192 | 8192 | 1.310872× | 1.302933× | 0.993943× | pass/pass |
| prefix-natural-16384 | 16384 | 1.422498× | 1.411736× | 0.992434× | pass/pass |
| prefix-natural-32767 | 32767 | 1.464829× | 1.459058× | 0.996061× | pass/pass |
| prefix-natural-32768 | 32768 | 1.465810× | 1.461114× | 0.996796× | pass/pass |
| prefix-natural-32769 | 32769 | 1.467545× | 1.461895× | 0.996150× | pass/pass |
| prefix-natural-33027 | 33027 | 1.466456× | 1.460110× | 0.995672× | pass/pass |
| prefix-natural-65535 | 65535 | 1.467914× | 1.461056× | 0.995328× | pass/pass |
| prefix-natural-65536 | 65536 | 1.471177× | 1.456690× | 0.990153× | pass/pass |
| prefix-natural-65537 | 65537 | 1.460895× | 1.455221× | 0.996116× | pass/pass |
| prefix-natural-131072 | 131072 | 1.473230× | 1.464830× | 0.994298× | pass/pass |
| prefix-natural-262144 | 262144 | 1.473548× | 1.469762× | 0.997431× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.494812× | 1.487101× | 0.994842× | pass/pass |
| prefix-random-1024 | 1024 | 0.421117× | 0.421872× | 1.001793× | pass/pass |
| prefix-random-4096 | 4096 | 0.703033× | 0.699416× | 0.994855× | pass/pass |
| prefix-random-8192 | 8192 | 0.896769× | 0.895560× | 0.998652× | pass/pass |
| prefix-random-16384 | 16384 | 1.229068× | 1.224597× | 0.996363× | pass/pass |
| prefix-random-32767 | 32767 | 1.484007× | 1.477375× | 0.995531× | pass/pass |
| prefix-random-32768 | 32768 | 1.484409× | 1.480348× | 0.997264× | pass/pass |
| prefix-random-32769 | 32769 | 1.481077× | 1.479493× | 0.998930× | pass/pass |
| prefix-random-33027 | 33027 | 1.485476× | 1.478458× | 0.995276× | pass/pass |
| prefix-random-65535 | 65535 | 1.664476× | 1.659695× | 0.997128× | pass/pass |
| prefix-random-65536 | 65536 | 1.663465× | 1.659270× | 0.997478× | pass/pass |
| prefix-random-65537 | 65537 | 1.659347× | 1.654986× | 0.997372× | pass/pass |
| prefix-random-131072 | 131072 | 1.753953× | 1.750953× | 0.998290× | pass/pass |
| prefix-random-262144 | 262144 | 1.794555× | 1.807834× | 1.007400× | pass/pass |
| prefix-random-1048576 | 1048576 | 1.818927× | 1.815362× | 0.998040× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.336768× | 0.336400× | 0.998909× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.487714× | 0.480672× | 0.985562× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.575149× | 0.571875× | 0.994308× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.687088× | 0.680778× | 0.990817× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.772769× | 0.771331× | 0.998139× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.775344× | 0.772811× | 0.996732× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.778224× | 0.776187× | 0.997383× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.788797× | 0.790948× | 1.002728× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.839359× | 0.837085× | 0.997290× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.840860× | 0.838730× | 0.997468× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.839927× | 0.838921× | 0.998802× | pass/pass |
| prefix-periodic-131072 | 131072 | 0.873200× | 0.872191× | 0.998843× | pass/pass |
| prefix-periodic-262144 | 262144 | 0.889661× | 0.891204× | 1.001734× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 0.924540× | 0.891752× | 0.964536× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.274876× | 0.274398× | 0.998264× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.437888× | 0.433224× | 0.989351× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.533150× | 0.531791× | 0.997452× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.649120× | 0.648978× | 0.999781× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.742281× | 0.740483× | 0.997577× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.742234× | 0.739123× | 0.995809× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.763933× | 0.763308× | 0.999182× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.783866× | 0.783290× | 0.999266× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.831295× | 0.829695× | 0.998075× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.829307× | 0.827749× | 0.998121× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.830920× | 0.829368× | 0.998132× | pass/pass |
| prefix-repeated-131072 | 131072 | 0.869031× | 0.867751× | 0.998528× | pass/pass |
| prefix-repeated-262144 | 262144 | 0.882622× | 0.882168× | 0.999486× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 0.885927× | 0.885568× | 0.999595× | pass/pass |

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r285-reuse-item-selection-flags-0dd371b3a4c2.tar.gz). SHA-256 0dd371b3a4c298e6e2c7e24f346b0dc1cd8925fc112657758f0ece07b2b7c94e.
