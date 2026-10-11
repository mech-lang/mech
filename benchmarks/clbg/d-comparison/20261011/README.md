# D Benchmark Game comparison

All ten tests have native ARM64 D compact and performance candidates. The cohort contains thirty D artifacts, including two compiler settings for one n-body source, and ten unchanged Mech AOT controls.

D is not currently listed on the Benchmark Game. Historical D submissions are preserved from `kragen/shootout`; newer published implementations are preserved from `hanabi1224/programming-language-benchmarks`. Commit IDs, original sources, application changes and build commands are recorded in the three preparation manifests. Locally authored compact or optimized ports are identified as such. These are locally measured endpoints from the candidate pool, not published Linux timings or a claim about the language's absolute limits.

The compiler is LDC 1.43.0, based on DMD 2.113.0 and LLVM 23.1.2, targeting the native Apple M3. Normal builds use release optimization, disabled bounds checks and native CPU selection. One n-body artifact allows FMA contraction; a precise counterpart is also measured. No approximate reciprocal square root, global fast-math, integer n-body scaling or input-specific shortcuts are used.

## Required work

- N-body: momentum correction, every pair at every one of fifty million steps, position updates and both energies. Binary64 arithmetic.
- Spectral norm: forty complete on-demand matrix-vector products at n=5500. No cached matrix or skipped iteration.
- Mandelbrot: every requested pixel and up to fifty recurrence steps, complete raw PBM output. Vector candidates preserve arbitrary dimensions and clear unused tail bits; independent odd-width pixel checks are retained.
- Fannkuch redux: every permutation at n=12, complete prefix flipping, checksum and maximum. Native NEON shuffle supplies the same byte permutation as the published x86 source.
- FASTA: every fresh LCG update, binary64 direct division and unscaled cumulative-probability comparisons, all output at n=25000000. No period cache or integer-scaled thresholds.
- K nucleotide: every overlapping window in all seven full growable maps of the THREE record. Compact string maps and parallel rolling keys have lexical ties and short-input guards. Natural repetition in the input is not assumed or used to skip updates.
- Reverse complement: every complete record, original headers, full uppercase complement and sixty-column output.
- Binary trees: fresh equal-size leaf/interior allocations, recursive construction and traversal of every tree, and a long-lived tree retained until its final check. Default GC or the general Phobos Region allocator. Instrumented independent checks cover allocated nodes, leaves, traversal calls and live address uniqueness.
- Pi digits: canonical unbounded spigot, both stable-digit extractions and all next-term transformations, complete padding/output. Phobos BigInt or general GMP arithmetic; the algorithm and bindings are D code.
- Regex redux: all nine ordinary nonoverlapping searches and five ordered substitutions over complete input, using the D standard regex engine. Complete count/length output.

The displayed D source contains the application algorithm. Ordinary standard libraries and GMP supply general operations. No native benchmark executor is called. Helpers, SIMD intrinsic declarations, bindings and source corrections count toward D LOC.

The native parallel fannkuch candidate follows its published partitioning assumption of n≥4; prescribed n=7 and n=12 satisfy it. The buffered reverse-complement candidate accepts the Benchmark Game's IUPAC sequence records and headers; semicolon comment records outside that input format are not supported. These applicability limits do not remove any prescribed benchmark work. Mandelbrot and nucleotide parsing were additionally checked beyond the prescribed inputs.

## Measurement and LOC

`source-audit.json` freezes application, native artifact, runtime, library, input and audit hashes. `run.py` runs three sequential, shuffled fresh-process samples at the full unchanged workloads. Startup, runtime libraries, workers, input, computation, complete output to regular files and shutdown are timed. Compilation, source audits, output hashing and validation are outside timing. Every admitted sample passes complete output validation. AC power and Low Power Mode off are required; competing compilers or power changes invalidate a sample.

`loc.py` counts nonblank, noncomment application lines. It expands packed independent statements and initialized bindings, while treating loop-header semicolons as structural. No compressed source metric is used. Structural lines are counted consistently with the existing native-language cohorts.

`results/summary.md`, `.csv` and `.json` contain raw seconds, samples, standard deviations, measured endpoint selection and × Mech comparisons. Time × Mech uses the freshly timed unchanged fastest AOT control for both D profiles. Fastest LOC compares fastest sources; fewest LOC compares fewest sources. Values below 1× mean faster or shorter than Mech. Existing explorer measurements are preserved when D is added.

## Reproduction

Prepare numerical sources with `prepare_numerical.py`; sequence sources with `finalize_sequences.py`. Tree/pi build commands are in `tree-digit-manifest.json`; allocation and padding canaries are under `audit/trees-digits`. All sources and originals are already retained locally.

Run `prepare.py`, `run.py`, and `summarize.py` using a Python environment with Pygments. Source/binary hashes and preparation checks must match the frozen audit. The runner is resumable and never scales a short input's time to represent a full workload.
