# Function application benchmark migration

The completed [results and timing table](RESULTS.md) cover all ten benchmarks:
15 selected endpoints, 81 input cases, and 174 variant comparisons pass,
including every full workload. Application-only changes preserve the native
compute objects. The ordinary-matrix spectral norm is 13 → 9 lines and has a
confirmed 4.8% arithmetic cost; the report explains that distinction and links
all raw samples. [Exact summary values](application-v3/summary.json) are saved
alongside the reports.

This audit preserves sources and executable binaries before the named
expression-body function and application syntax migration. It does not modify
historical timing reports, website data, or charts.

`baseline.json` contains the 15 selected Mech fastest/fewest-lines endpoints
across all ten benchmarks, copied original executables, hashes, physical LOC,
original generation/build/run commands, optimization policies, and existing
correctness pins. `current-sources/` contains the 34 current benchmark sources
saved before edits; `published-sources/` contains exactly the chart-pinned
sources. The recovered binary trees executable matches the published report's
exact hash; its already-rebuilt current canonical executable is saved separately
under `current-binaries/`. `input-pins.json` pins the existing small/full FASTA inputs and the
remaining benchmark reference outputs. Copies were made before source edits.

`migration.json` records source hashes, physical LOC before/after, and the new
expression functions and application stages. No compressed-source metric is
used. All computation remains visible in Mech; operations are
general compiler/runtime/stdlib operations.

## Source changes

| Benchmark | Current source before → after LOC | Application feature |
|---|---:|---|
| N-body | 32 / 37 → unchanged | Existing reactive array expressions have no function wrappers to remove. |
| Spectral norm | 13 → 9 | Ordinary nested comprehensions, transpose, matrix multiplication, captured matrix in expression function, and function application. |
| Mandelbrot | 20 / 29 → 20 / 29 | PBM conversion and output application pipeline. |
| K-nucleotide | 17 / 24 → 17 / 24 | Input selection/encoding and query packing pipelines. |
| Reverse complement | 10 → 7 | Reverse/translate/wrap and record-map/output pipelines. |
| Fannkuch redux | 43 / 64 → 39 / 59 | Concise predicate functions; multi-output state transitions stay explicit. |
| FASTA | 33 / 44 → 32 / 43 | Concise search/count predicates. |
| Binary trees | 29 → 26 | Concise allocation reduction and tree-build/count composition. |
| Regex redux | 24 → 24 | Ordered replacement applications retain named stages. |
| Pi digits | 39 / 42 → 35 / 38 | Concise label/no-label and loop-predicate functions. |

Spectral norm is the ordinary matrix-expression example implemented by the
compiler integration work. Its current source hashes and physical LOC are
included in `migration.json`; the regression phases compare it to the preserved
coefficient-callback version.

### Binary trees source distinction

The published chart endpoint was the 50-line callback program, while the
current canonical source already used 29-line match/comprehension syntax before
this migration. Those are distinct changes. The source migration reduces the
current canonical form from 29 to 26 lines. The published callback experiments
remain unchanged; `migrated-endpoints/binarytrees/` contains equivalent callback
copies reduced from 50 to 42 lines for a comparison changing only application
syntax.

## Reproducible validation

Run from the worktree root after the compiler has been rebuilt:

```sh
python3 benchmarks/clbg/verify_function_application.py --run application-next --build
python3 benchmarks/clbg/verify_function_application.py --run application-next --verify --full
python3 benchmarks/clbg/verify_function_application.py --run application-next --inspect
python3 benchmarks/clbg/verify_function_application.py --run application-next --measure --samples 5
```

Build and timing phases are separate. The harness builds the pinned old syntax
and migrated current syntax with the same new compiler, and retains the old
published binary as a third comparison. Binary trees also builds the migrated
callback source. Unchanged source bytes can reuse the same emitted binary.
Binary trees also builds the saved 29-line current canonical source and compares
its saved executable separately from the published 50-line executable.
N-body generation uses its general numeric graph driver with the recorded
optimization flags and no benchmark-specific computation added.

Verification checks edge sizes and full published workloads, hashes complete
stdout, and compares it to the saved original binaries and existing full-output
pins. N-body uses the existing energy tolerance of 1e-8. Timing runs sequential
fresh processes with eight requested workers, randomized complete rounds, AC
power and Low Power Mode disabled, and rejects samples with competing compiler
activity or a power-state change. Stdout is written to regular files; builds and
output comparison are outside the timed interval.

`generic-examples/` also checks evaluation once for repeated argument holes:
an effectful input prints once per loop iteration, and a shared allocation uses
one slot in a one-slot arena even when passed to two parameters. These examples
are built and verified with the benchmark regression phases.

Results are written to a new audit run directory (`application-v1/` by default).
Use `--run another-name` after compiler/source changes. `--only` accepts a
comma-separated benchmark subset. Each saved build pins the compiler, source,
executable, object/descriptor artifacts, and recorded build commands.
`application-v3/` is the completed frozen-compiler run; use a fresh name for
future compiler/source changes. A focused repeated comparison can retain the
same built binaries with, for example:

```sh
python3 benchmarks/clbg/verify_function_application.py --run application-next --only spectralnorm --measure --samples 10 --timing-variants before,after --timing-report spectral-confirmation-10.json
```
