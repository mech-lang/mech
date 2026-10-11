# MATLAB, full Benchmarks Game cohort

Twenty local MATLAB candidates: compact and optimized implementations of all ten workloads. These are local ports from the official task descriptions, not programs published or ranked by the Benchmarks Game. The installed runtime is MATLAB R2026b on Apple Silicon. Every benchmark algorithm remains in the displayed `.m` source; ordinary arrays, dictionaries, regex, binary I/O, and Java BigInteger arithmetic provide standard library support. No MEX benchmark kernel, external benchmark binary, or precomputed result is used.

Headless startup works with cached online licensing (`-licmode onlinelicensing -nodisplay`). All twenty small reference-output checks pass. Fresh batch mode disconnects descriptor 0; the pinned stream-only launcher duplicates redirected stdin to descriptor 3, which the MATLAB sources read. Binary output uses a binary `/dev/stdout` stream to preserve every byte. Generic Java BigInteger arithmetic uses a task-local Eclipse Temurin JRE 25; its official archive SHA, runtime and modules are pinned. No global MATLAB Java setting changed. Full workload measurement follows the work audit; unmeasured MATLAB programs do not enter the website.

## Required work

- n-body: all 50,000,000 sequential steps, ten symmetric pair interactions and all five position updates, both energies.
- spectral-norm: four required coefficient/multiplication/transpose/composition functions; recompute every coefficient in all forty N-by-N products. No matrix caching.
- Mandelbrot: all 16000-by-16000 pixels, ordinary recurrence and at most fifty escape-tested iterations, full PBM bytes. No analytic interior or periodicity shortcuts.
- FASTA: all 200,000,000 fresh LCG updates, double division by 139968, unscaled floating cumulative probabilities and complete 250,000,000 bases. The optimized form uses ordinary `discretize` interval search and bounded output buffers.
- k-nucleotide: all seven complete overlapping hash maps for widths 1,2,3,4,6,12,18. Every valid window performs an explicit lookup and counter update. The compact source first initializes all actual keys to zero with the standard dictionary constructor, adding a full insertion pass; the optimized source grows its dictionary with membership tests. Double keys and counters are exact for the required 36-bit keys and 27-bit counts. No histograms or query-only counts.
- reverse-complement: all input records, preserved headers, uppercase complements, sixty-column output.
- binary-trees: every interior and leaf uses a fresh two-slot cell node. Completely build, then walk, then release each tree. Release stretch tree before long-lived allocation; retain the long-lived tree through all temporary work. Optimized constructor unrolling retains every allocation. Default MATLAB memory management, no flat tree or custom pool.
- fannkuch-redux: all 479,001,600 permutations in the canonical rotation order, all prefix flips, alternating checksum and maximum.
- pi-digits: sequential unbounded LFT spigot, arbitrary precision with both extract(3) and extract(4). Java BigInteger only supplies general arithmetic.
- regex-redux: nine complete case-insensitive, non-overlapping searches and five ordered case-sensitive substitutions, full input/output validation.

## Procedure

Before admitting results, run MATLAB syntax/API checks and all small-output checks, then validate complete full outputs. Review required work beyond matching printed answers, including an instrumented allocation/work probe for the cell trees. Pin sources, runtime, dependencies, and controls before timing. Use three shuffled sequential fresh MATLAB processes per candidate, with AC power and Low Power Mode off. Include startup, JIT, complete input, computation, full output to a regular file, and teardown. Hash validation and setup are excluded. Remeasure unchanged Mech AOT controls alongside MATLAB. Never substitute Octave or import published elapsed times.

Select fastest by measured median and fewest LOC by statement-normalized LOC; break ties by LOC or measured time respectively. Only those measured endpoints enter the website. Retain raw candidates and audit records here.

Before any full nucleotide samples, an untimed generic dictionary diagnostic compared fallback lookup, preinitialized indexing, and growing-map membership/indexing for arbitrary small and large key sets. It verified every counter, not just the total. Ordinary double-key indexing was substantially faster through MATLAB's standard JIT, so both pending sources adopted it while retaining all 874,999,961 explicit window-counter updates. This is a general library/API improvement, with no native benchmark executor or input-specific shortcut. `results/dictionary-probe.json` marks the diagnostic as excluded from chart samples. `evidence/dictionary-before-indexing/` archives the original sources and evidence; `rebase_pending.py` verified that all 69 already-saved samples remained unchanged before the sequential suite resumed.

## Long-running measurements

The detached sequential runner saves each validated complete sample atomically in `results/timings.json`; `results/measurement-process.json` identifies its process and `results/measurement.log` records progress. Resume the existing cohort from those checkpoints instead of starting overlapping benchmark processes. `pause-measurements` pauses only between samples so browser verification can run without contaminating a measurement.

`export_chart.py` makes an immutable snapshot containing only benchmark pairs with three complete, validated samples for both MATLAB sources and their Mech control. The chart's coverage label states how many tests are measured; unfinished tests contribute no timing, LOC endpoint or ratio. `chart-export.json` pins the selected export and its audit records, so later timing samples cannot silently change a published mean. The same exporter is used once the full suite finishes.

`finish.py` waits for that existing runner to complete all ten tests. It then independently rechecks statement LOC, validates the full timing cohort, selects measured endpoints, rebuilds the prepared local website, and checks that all 376 earlier endpoint programs, 364 referenced sources and 1,204 timing samples remain unchanged. The immutable earlier chart is recorded in `evidence/chart-before-matlab.json`. `results/completion.json` records each completed verification step. The finishing process performs no benchmark computation and makes no deployment; responsive browser checks and the supported Sites publication remain separate final steps.

## Completed cohort

All twenty MATLAB sources and ten unchanged fastest Mech AOT controls have three validated full-workload samples: ninety saved runs, with none discarded. `results/final-loc-audit.json` independently verifies all source counts; `results/summary.json` records the measured endpoints and direct MATLAB/Mech multipliers. `results/previous-chart-preserved.json` confirms preservation of every earlier program, source and sample.

The supplemental untimed probe `evidence/work_probe_odd.m` uses exact pinned tree constructor bodies, with helper renaming only, at depths 1,3,7,11. Both profiles have the expected number of unique node headers and unique backing stores, including the optimized depth-one branch. `results/final-allocation-branches.json` pins the probe, diagnostic, source files and results. This complements the original even-depth evidence without changing any measured source or pinned timing report.
