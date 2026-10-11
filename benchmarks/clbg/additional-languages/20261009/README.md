# Mojo and Futhark CLBG CPU comparison

These ports cover all ten Computer Language Benchmarks Game workloads in the
Mech benchmark explorer. They use native CPU execution on Apple M3, at most eight
workers, and the same full inputs and output references as the existing cohort.
Mojo uses version 1.1.0 with MAX 26.6.0; Futhark uses version 0.27.1 and its
multicore C backend. GPU results are outside this comparison.

`provenance.json` pins the online search and source snapshots. Community Mojo
N-body implementations and a scalar Spectral Norm implementation were adapted
to the current API and canonical CLI/output. The other Mojo programs and all
ten Futhark programs are local ports. The official Futhark N-body and Mandelbrot
examples have different input shapes and iteration counts, so they were not
used as CLBG submissions.

The benchmark algorithms are in `sources/`. `stdlib/` contains reusable IO and
formatting, signed unbounded arithmetic, growing hash maps and byte regular
expressions. Mojo uses ordinary native containers and general GMP/PCRE2 bindings.
Futhark implements those data operations in Futhark; its generic C adapter only
reads arguments/stdin, creates a compiler runtime context, calls the generated
entry point and writes its returned bytes. There are no native benchmark
executors. Application LOC excludes this general library and IO support.

LOC counts nonblank, noncomment lines. Independent packed statements are not
permitted; structural array literals, tuples and loop-state patterns remain
single language constructs. Endpoint labels mean the fastest or fewest-LOC
candidate actually measured here, not the global best possible program.

`audit_sources.py` pins every source, compiled binary, compiler and support file
and documents required work. In particular, spectral norm evaluates all
coefficients in all forty products, nucleotide performs all seven counting
passes and every hash update, binary trees creates and traverses every node,
FASTA retains every RNG update and floating CDF comparison, and pi digits uses
the unbounded LFT spigot with dual extraction. Regex Redux has nine separate
searches and five ordered substitutions. Complete output matching is necessary
but does not replace this work review.

`test_stdlib.py` checks general arithmetic against Python integers, regex
features and randomized fixed-width alternatives against Python `re`, and hash
map collisions/growth/full-u64 keys against independent counts. The Futhark
regex engine supports concatenation, alternatives, byte classes/ranges,
escapes, dot and greedy `*`, `+`, `?`; groups, anchors, backreferences and Unicode
are unsupported. Fixed-width alternatives use parallel matching followed by
ordinary nonoverlapping match selection. Hash-map allocation stays inside the
growth branch. These are general library paths, not benchmark pattern handlers.

`build.py` compiles before measurement. `run.py` validates small/full workloads,
then runs randomized rounds of three sequential fresh processes. Elapsed time
includes startup, input, language/library work, complete stdout to a regular
file and teardown. Build time and output hashing are excluded. AC power with
Low Power Mode off is required. Concurrent compilers or power changes invalidate
a timing sample. Current Mech fastest controls retain the original arguments,
worker count and runtime settings. Each source's samples stay separate.

Results, reference checks and immutable pins are in `results/`. The explorer
keeps only the selected fastest/fewest-LOC endpoints and exposes their complete
sources, support files and timing samples. Earlier preflight/probe results stay
archived and do not enter the means.
