# Interpreter integration audit

The incoming fixed-shape f32 batch executor was integrated with input-view compatibility and remainder semantics. Its 14 integration tests and signed-zero/remainder test pass. These ten CLBG workloads use the unchanged fused-f64, collection and control interpreter paths, rather than the new f32 executor. No benchmark sources or dispatch paths changed.

All 15 source profiles were measured before and after, three canonical full-workload samples per phase: 90 valid complete-output runs on Apple M3, AC power, Low Power Mode off, sequential under the shared timing mutex. Loaded interpreter execution includes input, computation, formatting, output and flush. Preparation and whole-process time are retained separately.

The table compares medians; ×Mech AOT uses the existing published fastest AOT endpoint, measured in its own cohort. It is a descriptive cross-cohort comparison. The fresh before/after ratios are the matched comparison. The explorer uses the complete post-integration cohort for all 15 interpreted candidates and preserves each prior measurement in its audit; it does not select whichever cohort is faster.

| Program | LOC | Before (s) | After (s) | After / before | After / Mech AOT |
|---|---:|---:|---:|---:|---:|
| nbody:pairwise-mass-tails | 25 | 15.738515 | 15.769531 | 1.002× | 15.171× |
| nbody:compact-baseline | 20 | 16.157657 | 16.134761 | 0.999× | 15.522× |
| mandelbrot:mech-program-simd-loop-fma | 29 | 11.814035 | 11.567850 | 0.979× | 26.306× |
| mandelbrot:mech-program-compact | 20 | 51.937095 | 51.431613 | 0.990× | 116.959× |
| knucleotide:mech-compact-final-std1-simd0-p4 | 17 | 0.600608 | 0.590323 | 0.983× | 1.149× |
| spectralnorm:mech-integer-simd1-u4-r4-p8-g8 | 12 | 0.487126 | 0.497019 | 1.020× | 5.774× |
| revcomp:mech-revcomp-optimized | 7 | 0.232491 | 0.220872 | 0.950× | 1.480× |
| fannkuchredux:mech-fannkuch-even | 59 | 80.358808 | 79.539164 | 0.990× | 46.720× |
| fannkuchredux:mech-fannkuch-default | 39 | 60.702544 | 64.304455 | 1.059× | 37.772× |
| fasta:mech-fasta-final-expanded-v2 | 43 | 18.851154 | 18.808041 | 0.998× | 34.753× |
| fasta:mech-fasta-final-compact-v2 | 32 | 22.293861 | 22.896034 | 1.027× | 42.307× |
| binarytrees:mech-binarytrees-parallel | 26 | 10.445468 | 10.304381 | 0.986× | 26.126× |
| regexredux:mech-regex-final-reuse-signed | 24 | 0.475689 | 0.480343 | 1.010× | 0.981× |
| pidigits:mech-pi-final-advance-signed | 38 | 0.307906 | 0.308233 | 1.001× | 1.027× |
| pidigits:mech-pi-final-compact-signed | 35 | 0.404718 | 0.404442 | 0.999× | 1.347× |

## Confirmed fannkuch signal

The default fannkuch form slowed by 5.93% in the main matched cohort. An independent three-round randomized confirmation used the same frozen before/after binaries and unchanged source at the full n=12 workload. Every sample passed the complete output check. Its medians were **56.117233 s before and 59.300908 s after: 1.056733×, or 5.67% longer**. Neither confirmation samples nor faster historical measurements replace the main post-integration cohort.

The new f32 executor is uncalled on these workloads, but adding it can still change compilation of used functions. Read-only disassembly of the exact binaries shows the unchanged control interpreter's `Machine::call` growing from 11,692 to 11,840 bytes. Static stack loads using SP or the frame pointer remain 265; stack-store instructions rise from 93 to 119. These include cold branches and are not dynamic spill counts. LLVM's different codegen-unit partition and wrapper inlining make a code-generation explanation plausible, but the measurements and disassembly do not establish a specific cause.

A same-package control rebuilt the current code with the original driver package and executable names. Cargo reused all 169 unchanged library archives, and canonical n=7 output passed. Its `Machine::call` body is byte-identical to the pinned after binary, at the same address. Driver package identity therefore does not explain this body change. The [control report](package-control.json) and [reproduction script](package_identity_control.py) retain the build, source and library pins. The [after reference](codegen-package-control-after-reference-machine-call.txt) and [same-package control](codegen-package-control-control-machine-call.txt) disassemblies are retained alongside the report. This was a static control, not another performance sample.

Evidence: [confirmation report](fannkuch-confirmation.json), [confirmation runner](confirm_fannkuch.py), [static inspection](codegen-inspection.json), and the [before](codegen-before-machine-call.txt) / [after](codegen-after-machine-call.txt) disassemblies. The timing cohorts should not be described as performance-neutral solely because the new executor is uncalled.

## Provenance limitations

The baseline pins application sources, before/after executables, driver source and the relevant compiler/interpreter source files. The imported Python guard scripts and full input/reference files were not pinned at execution time in this audit. The importer independently verifies full output records and takes explicitly labelled verification-time helper snapshots; those are not retrospective execution pins. Future resumes reconcile all immutable saved row fields and workloads against the baseline. Ambient environment variables were inherited with pinned per-row overrides, so there is no claim of a fully isolated process environment.
