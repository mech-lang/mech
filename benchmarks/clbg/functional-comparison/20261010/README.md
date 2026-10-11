# Haskell and OCaml comparison

This cohort considers the 38 Haskell and 22 OCaml sources listed in the pinned
Benchmarks Game measurement pages. Published Linux timings serve only as source
discovery hints. All chart timings are measured locally on the Apple M3, on AC
power with Low Power Mode disabled.

The source pool includes published compact and optimized submissions. The chart
retains the fastest measured source and the fewest-LOC admitted source for each
language and benchmark. Full workloads, three shuffled sequential fresh-process
samples per source, small/full output validation and unchanged Mech AOT controls
are recorded in `results/timings.json`. Startup, complete input, computation,
complete stdout to a regular file and process teardown are timed. Compilation,
reference checks and hashing are excluded.

## Toolchains

- Haskell: GHC 9.10.3, LLVM 19.1.7, `-O2 -fllvm -threaded -rtsopts`, plus
  published source extensions. Published parallel variants use eight runtime
  capabilities; serial variants retain their default settings. Tree sources
  additionally disable common-subexpression elimination and full laziness to
  preserve independent dynamic allocation; no GC heap-tuning flags are used.
- OCaml: 5.4.1 with Flambda enabled, `-O3 -unsafe -noassert -nodynlink
  -inline 100 -ccopt -fPIC`. Published process/task partitioning is retained.
- Ordinary language and library packages provide arrays, maps, arbitrary-precision
  arithmetic and regex operations. No native benchmark executor supplies an
  application algorithm.

`results/builds.json` records exact commands, compiler and library hashes, and
diagnostics. Task-local Cabal and OPAM stores are under `toolchains/`.

## Source and work review

Original sources and pages remain in `upstream/` and `sources/`. Corrections are
separate `-faithful` sources, explicitly labelled by `corrections.json`. Their
complete application code counts toward LOC.

Corrections preserve the common cohort workload: binary64 random FASTA sampling
with direct division and unscaled cumulative probabilities; every overlapping
nucleotide window with hash maps growing from small initial sizes; complete
Mandelbrot output; correct fannkuch identity and terminal permutation handling;
and current library APIs. Pi sources retain the unbounded LFT spigot with both
digit extractions. Spectral norm evaluates every coefficient during each multiply.

Binary trees require complete independent interior and leaf allocation before
traversal. Opaque compiler barriers prevent shared constant leaves and constructor
elimination. The long-lived tree stays rooted through temporary-tree work and is
checked afterward. One Haskell submission uses the standard `GHC.Compact` library
for its long-lived tree; temporary trees use ordinary default GC. Optimized GHC
STG and OCaml assembly evidence under `results/` verifies dynamic leaf allocation,
separate child construction, complete temporary-tree work and root retention.

`source-audit.json` records eligibility and work review for all originals and
corrected sources. The timing runner rejects a candidate that fails output
validation and records its reason. Compiler activity or power changes invalidate
a sample rather than becoming chart data. Source, build, library and audit pins
are rechecked throughout the run.

LOC excludes blank and comment-only lines, includes semantic compiler pragmas,
and counts independent statements or bindings packed onto one line separately,
including nested `let` bindings and adjacent module declarations.
Structural list/record separators do not inflate LOC. Full source and the
normalization expansions are retained; names are unchanged and compressed size
is not a chart metric.

## Reproduction and outputs

The scripts are ordered `fetch.py`, `prepare.py`, `build.py`, `audit.py`, `run.py`,
`audit_loc_final.py`, then `summarize.py`. The first four prepare the pinned source pool and compile
with the recorded task-local toolchains. `run.py` resumes existing validated
samples without rerunning them. Keep pinned files unchanged during a cohort.

After all ten benchmarks complete, `audit_loc_final.py` checks packed/nested
bindings against the same immutable sources and writes `results/final-loc-audit.json`.
It changes no samples or binaries. `summarize.py` verifies source/binary hashes,
small/full checks and every sample, then writes `results/summary.json` containing
twenty compact/fastest endpoint pairs and ten Mech controls. The website importer
requires this passed summary and complete measurement coverage before adding
either language.
