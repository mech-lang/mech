# Mojo and Futhark CLBG CPU comparison

Mojo covers all ten Computer Language Benchmarks Game workloads in the
Mech benchmark explorer. They use native CPU execution on Apple M3, at most eight
workers, and the same full inputs and output references as the existing cohort.
This cohort completes distinct compact and performance candidates for every
benchmark in both languages: twenty attempted pairs. After allocation review,
nineteen pairs and fifty sources are admitted, including earlier valid baselines.
Futhark covers nine workloads; its tree arrays fail the required per-node
allocation rule and are excluded. `profiles.json` records the techniques
attempted for each pair. Endpoint selection uses the complete measured pool;
an optimization attempt that loses does not replace a faster baseline.
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
executors. Mojo's tree programs use a thin generic binding to the existing Apache
APR memory pool, the same library allocator used by the C reference. Every node
is allocated separately in Mojo. Application LOC excludes this general library
and IO support.

LOC counts nonblank, noncomment lines. Independent packed statements are not
permitted; structural array literals, tuples and loop-state patterns remain
single language constructs. Endpoint labels mean the fastest or fewest-LOC
candidate actually measured here, not the global best possible program.

`audit_sources.py` pins every source, compiled binary, compiler and support file
and documents required work. In particular, spectral norm evaluates all
coefficients in all forty products, nucleotide performs all seven counting
passes and every hash update, binary trees separately allocates and traverses every node,
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

The extended tests cover 848 signed arithmetic cases for each Futhark integer
library, including 16,384-bit divisors and one-limb quotient boundaries. The
fast library estimates a quotient from leading limbs, then corrects it using
exact integer comparisons; wider quotients retain unbounded long division.
Reserved map batches are tested at five unrelated sizes with signed deltas.
Mojo tests exercise owned copies, signed operators, every destination alias
of fused multiply/add, 5,002 map updates, shrinking and expanding regex
replacements, and arbitrary line widths. Aggregate test pins are recorded in
`results/stdlib-tests.json`.

Generated tree code is reviewed separately in `results/tree-work-audit.json`.
An early Mojo candidate built and checked one tree then multiplied the result
by its iteration count. That binary and its machine code are archived under
`snapshots/rejected-tree-loop/` and excluded. Later flat/indexed-array trees
were also rejected despite rebuilding all node contents: they omit required
allocator calls. Their five sources, binaries and original timing report remain
in `snapshots/pre-allocation-audit/`. The corrected two Mojo sources call
`apr_palloc(16)` for every interior and leaf node, write both child pointers,
walk each tree, and clear its pool after each iteration. Stretch storage is
destroyed before the long-lived tree is allocated. The algorithm remains in
Mojo; APR only allocates and releases memory.

`test_tree_allocation.py` verifies the generic pool's 34,816 allocations over
seventeen clear/reuse rounds. Test-only instrumentation then checks exact tree
allocation and clear counts at depths six and ten in both compiled sources;
every node, including every leaf, receives sixteen bytes. Instrumentation is
absent from timed runs. Futhark's flattened-array model does not provide the
per-node heap-reference tree required by this test; no native executor is used
to fill that gap. See the [allocation requirements](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/binarytrees.html).

`build.py` compiles before measurement. `run.py` validates small/full workloads,
then runs randomized rounds of three sequential fresh processes. Elapsed time
includes startup, input, language/library work, complete stdout to a regular
file and teardown. Build time and output hashing are excluded. AC power with
Low Power Mode off is required. Concurrent compilers or power changes invalidate
a timing sample. Current Mech fastest controls retain the original arguments,
worker count and runtime settings. Each source's samples stay separate.
The final admitted report retains 171 valid samples for nine unchanged workloads,
verifying identical source, binary and used-library hashes. Corrected Mojo trees
and their Mech control add nine fresh samples, for 180 total. The reuse procedure
and snapshot hash are recorded in `prepare_allocation_remeasurement.py` and the
final timing report. Futhark enters means only over the shared nine tests.

Results, reference checks and immutable pins are in `results/`. The explorer
keeps only the selected fastest/fewest-LOC endpoints and exposes their complete
sources, support files and timing samples. Earlier preflight/probe results stay
archived and do not enter the means.
