# Function application migration results

All 15 selected endpoints across all 10 benchmarks pass their full workloads and edge cases. Application-only changes preserve the native compute objects for all 14 non-spectral endpoints. The 9-line ordinary-matrix spectral norm passes all outputs and costs about 4.8% more time than the previous 13-line source in the separate ten-round confirmation.

The syntax integration and these regressions cover the compiled AOT execution path.

Source counts are physical nonblank, noncomment lines. Both columns below use the same frozen compiler and unchanged full workloads. A ratio above 1 means slower. Values are process-wall-time median ± sample standard deviation, in seconds. Spectral norm uses 10 samples per variant; every other row uses 5.

| Benchmark endpoint | LOC before → after | Before (s ± SD) | After (s ± SD) | After / before | Same native object? |
|---|---:|---:|---:|---:|:---:|
| N-body compact | 32 → 32 | 1.124825 ± 0.001549 | 1.127594 ± 0.003218 | 1.0025 | Yes |
| N-body optimized | 37 → 37 | 1.055480 ± 0.003783 | 1.054417 ± 0.004206 | 0.9990 | Yes |
| Mandelbrot compact | 20 → 20 | 2.256402 ± 0.012454 | 2.255570 ± 0.017301 | 0.9996 | Yes |
| Mandelbrot optimized | 29 → 29 | 0.441074 ± 0.001930 | 0.441356 ± 0.001835 | 1.0006 | Yes |
| K-nucleotide | 17 → 17 | 0.530810 ± 0.010585 | 0.524700 ± 0.017830 | 0.9885 | Yes |
| Spectral norm | 13 → 9 | 0.084689 ± 0.004255 | 0.088730 ± 0.001371 | 1.0477 | No |
| Reverse complement | 10 → 7 | 0.153134 ± 0.002603 | 0.150428 ± 0.001393 | 0.9823 | Yes |
| Fannkuch redux compact | 43 → 39 | 2.838093 ± 0.012714 | 2.830439 ± 0.030235 | 0.9973 | Yes |
| Fannkuch redux optimized | 64 → 59 | 1.728729 ± 0.029842 | 1.729004 ± 0.048224 | 1.0002 | Yes |
| FASTA compact | 33 → 32 | 1.077448 ± 0.004312 | 1.075838 ± 0.003419 | 0.9985 | Yes |
| FASTA optimized | 44 → 43 | 0.573664 ± 0.002077 | 0.573702 ± 0.002274 | 1.0001 | Yes |
| Binary trees current | 29 → 26 | 0.397759 ± 0.001380 | 0.401623 ± 0.007337 | 1.0097 | Yes |
| Regex redux | 24 → 24 | 0.497256 ± 0.004364 | 0.495754 ± 0.021186 | 0.9970 | Yes |
| Pi digits compact | 39 → 35 | 0.396854 ± 0.000703 | 0.396614 ± 0.002244 | 0.9994 | Yes |
| Pi digits optimized | 42 → 38 | 0.304678 ± 0.001512 | 0.305370 ± 0.001337 | 1.0023 | Yes |

The non-spectral paired median changes range from −1.77% to +0.97%, with byte-identical compute objects. These small differences do not establish a performance change. FASTA optimized differs by +0.0065%, and Fannkuch optimized by +0.0159%.

## Spectral norm arithmetic and confirmation

The 10-round confirmation is 84.689312 ± 4.254772 ms before versus 88.730396 ± 1.370639 ms after: **1.0477166×** (+4.77%). The new source is slower in 9 of 10 paired rounds. The one 97.760750 ms old-source sample is retained; no outlier was discarded. The original five-sample batch independently gives 1.0501081×.

The old source divides each vector element by a generated denominator. The new source explicitly forms matrix coefficients as `1.0 / denominator`, then multiplies by the vector element. Preserving that intermediate rounding requires an extra multiply. Generic SIMD lowering now vectorizes the floating-point suffix after the integer-to-float cast; it avoids the scalar-lane division problem found during this work. A representative unchecked matrix kernel has 20 floating divides in both sources, and 0 → 20 floating multiplies; its static instruction count is 463 → 488. Whole-object totals are 13,473 → 13,963 and include multiple checked/unchecked kernels, not just the hot loop.

This is a cost of the new arithmetic expression, not evidence that application syntax slows unchanged kernels. Recompiling the original source with the new compiler gives a five-sample median of 85.773708 ms versus 84.063667 ms for the saved published executable (1.0203422×); the saved executable has a 6.052321 ms sample SD, so that batch does not isolate a separate compiler slowdown. Reciprocal-to-division reassociation was not used because it changes intermediate floating-point rounding.

## Binary trees baseline distinction

The chart-published 50-line callback program and the already-current 29-line match/comprehension program predate different source revisions. The current application migration is **29 → 26 lines**, with identical objects and a paired median ratio of 1.0097144. The application-only callback copy is **50 → 42 lines**, also with identical objects: 0.403344 ± 0.003957 s → 0.402637 ± 0.002777 s, ratio 0.9982447. The historical callback source and executable remain preserved. The earlier 50 → 29 redesign is not attributed to function application.

## Validation and measurement conditions

- 81 input cases and 174 old/new/saved variant comparisons passed across 15 endpoints; all 15 full endpoint workloads passed.
- Complete stdout is compared, including PBM/FASTA binary and large text outputs. N-body retains its existing 1e−8 energy tolerance. Mandelbrot size zero retains the original rejection status; all positive edge sizes pass.
- Two persistent evaluation-once examples pass: repeated `_` arguments print an effect once per loop iteration, and a shared allocation consumes one slot in a one-slot arena.
- The coordinating compiler tests also passed 329 general matrix outputs and expanded capture tests, including nine captured values and aliased vector seeds; spectral correctness spans 13 sizes through 5500.
- 240 recorded process samples in the full five-sample batch, plus 20 in the spectral confirmation. **0 discarded samples**, with no detected competing compiler or power change.
- Timing runs use sequential fresh processes, randomized complete variant rounds, 8 requested workers, AC power, and Low Power Mode disabled. Full stdout goes to regular files; builds, input verification, and output hashing are outside the timer. The monitor checks power and compiler activity at both sample boundaries and every 10 seconds for long runs.
- All compiler, executable, source, input, and report hashes are pinned. Final compiler/driver/source hashes still match the build and migration reports; final power check confirms AC power and Low Power Mode off.
- Five or ten samples are an exploratory comparison, not a formal significance test. SD describes samples, not a confidence bound for the median. Printed-output equality does not assert bitwise equality of unprinted floating-point intermediates. The existing formatter supports precision up to 12; spectral norm uses 9 decimal places.

## Raw evidence

- [Machine-readable summary and exact medians/SD/paired ratios](application-v3/summary.json)
- [Full five-sample batch](application-v3/timings.json)
- [Spectral ten-round confirmation](application-v3/spectral-confirmation-10.json)
- [Full correctness report](application-v3/verification.json)
- [Native object comparison and disassembly links](application-v3/objects.json)
- [Build commands, flags, source/binary hashes](application-v3/builds.json)
- [Before/after source audit](migration.json)
- [Preserved published baseline](baseline.json) and [input/output pins](input-pins.json)

The prior `application-v2/` correctness report also passed, but it predates the SIMD correction and was not timed. `application-v1/` was interrupted for the capture ABI correction and is not a validated result. Historical reports, explorer data, and charts were not modified.
