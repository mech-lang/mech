# Erlang local Benchmark Game cohort

The source pool is every program linked by the pinned published [Erlang
measurements page](https://benchmarksgame-team.pages.debian.net/benchmarksgame/measurements/erlang.html):
21 originals across all ten workloads. Published timings are acquisition hints
only; every plotted timing is measured locally on the Apple M3.

`upstream/` retains the original source, page and attribution. `manifest.json`
pins their hashes. Eleven separately named `*-faithful.erl` corrections are in
`sources/`, with reasons and hashes in `corrections.json`. They bring sources
into the existing comparison's common-work policy or repair output handling:

- Binary trees: allocate a fresh leaf tuple, in addition to every interior node.
- FASTA: cumulative unscaled binary64 probabilities and direct division for
  each fresh LCG update, matching the existing cohort policy.
- K-nucleotide: deterministic frequency tie ordering, retaining all seven maps.
- Mandelbrot: compute every row's final pixel and preserve byte-oriented PBM.
- Regex-redux: search and transform the entire cleaned input, with case-sensitive
  ordered substitutions.

These corrections are not a claim that every original violates its published
Benchmark Game rules. The comparison's common-work policy is stricter in some
dimensions, notably random sampling and dynamic tree leaf allocation. All
corrections count toward application LOC and remain distinguishable from their
originals. No specialized host executor is introduced.

`build.py` compiles each variant to its own BEAM directory. The compiler, runtime,
source and BEAM binaries are pinned. Native Erlang/OTP 29.1.1 uses BEAM load-time
JIT and eight online schedulers (`+S 8:8`), with the published task partitioning
and default heap/GC settings. Optimized BEAM assembly proves fresh leaf/interior
allocation, recursive full construction and traversal, and long-lived-root
retention. `source-audit.json` records that evidence and admission decisions.

`run.py --phase small` checks all built candidates; admission requires matching
small outputs and required work. `run.py` collects three sequential, shuffled
fresh-process full-size samples per admitted candidate and ten unchanged Mech
AOT controls. It checks full output after every sample. N-body energies use the
existing 1e-8 tolerance; all other outputs match the existing full references.
Startup, module loading/load-time JIT, input, computation, all output to a regular
file and shutdown are timed. Bytecode compilation, validation, hashes and LOC
audit are excluded. AC power and Low Power Mode off are required; compiler or
power changes discard a sample. Build and browser QA work is kept outside timed
execution. Reports are resumable; completed measurements are never scaled from
smaller inputs or imported from published Linux results.

LOC is nonblank/noncomment physical source plus extra packed independent clause
or block expressions, found through Erlang's parsed source AST. Semantic
module/export/compile/macro directives and every application helper count.
Structural list, tuple, function argument and guard separators do not inflate
the count. `audit_loc_final.py` independently rechecks source hashes and counts;
the compression of source text is not a metric.

`summarize.py` verifies the full cohort and selects the locally fastest and
fewest-LOC programs, with timing breaking LOC ties. The website retains only
those endpoints and their sources; all raw candidate samples remain here.
Controls record the current/previous timing ratio while preserving existing
chart samples.
