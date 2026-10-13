# Crystal application preparation handoff

The research inventory supplies 21 pinned published candidates covering the
complete ten-workload suite. No application adaptation is qualified, built or
measured yet. Keep the upstream tree immutable. Each corrected or newly optimized
application must have its own source path, source pin, full work review and
canonical LOC audit. The research directory is independent of every running
benchmark's source and library pins.

Start with the current native Crystal 1.21.1 universal macOS package, whose exact
release metadata and source commit are in `research/inventory.json`. Bootstrap,
dependency inspection, compilation and every probe run under the common timing
mutex. Both source variants receive the same `--release --no-debug --mcpu native`
optimized LLVM build. Qualify actual floating-point output before introducing
different contraction or fast-math settings. Compiler threads are separate from
application compute workers.

| Workload | Compact basis | Meaningful optimized source pathway |
|---|---|---|
| nbody | Smallest faithful scalar/array published candidate after normalization | Scalar local body fields versus fixed StaticArray coordinates; exact sqrt/division and all ten interactions; use measured results to choose rather than assuming array syntax is faster. |
| mandelbrot | Published Kostya scalar PBM | Repair both Hanabi eight-lane value/pointer forms to exact dimensions/full PBM; cap ordinary row fibers at eight, join before ordered writes, preserve valid-lane escape reductions and zero tail bits. |
| spectralnorm | Smallest implicit-matrix source with nine-decimal output | Ordinary disjoint row fibers, denominator recurrence, several accumulation chains and scalar tail; always forty fresh coefficient products, no A materialization/cache. |
| binarytrees | Hanabi class Node/current counting workload | Same separately allocated internal nodes/leaves in capped independent depth jobs; release stretch scope and retain the real long-lived root until its final traversal. |
| fannkuchredux | Published StaticArray permutation/flips | Standard eight-fiber factoradic rank ranges, descending-width initialization, global checksum parity and actual prefix reversals; retain complete n! work and overflow-safe partitions. |
| fasta | Smallest faithful published ordered CDF source | Published array CDF binary search plus bounded writes; direct binary64 division on every draw, actual sequential carried LCG state, no integer-scaled thresholds/period replay/generated-letter cache. |
| knucleotide | Repair published full seven string-map source | Seven ordinary growable Hash(UInt64,UInt64) maps, packed rolling keys, seven fibers if helpful; first two counts remain maps and all seven maps include every overlapping window. |
| revcomp | Published general String reversal/IUPAC translation with parser fixes | Ordinary byte parser, case-inclusive complement table, in-place reverse and bounded writes; retain all records, ambiguity symbols, comments and 60-column wrapping. |
| regexredux | Hanabi current five-pass redux adapted to stdin | Standard PCRE2 library matches with at most eight independent count fibers and private match state; five replacements remain sequential and ordered. |
| pidigits | Published Hanabi two-extract general LFT | Pure Crystal endpoint-numerator basis or q/r/t scalar multiplier recurrence with two actual BigInt // operations per tested candidate; full padded rows/tails/counts. A distinct general mutable-GMP profile needs separate primitive/lifetime review and qualification. |

The default execution context must explicitly resize to the approved worker
cap. A parent waiting on channels is not an extra compute worker. Do not create
independent contexts whose combined capacity exceeds eight, or use unbounded
per-row fiber creation without a bounded ordinary work queue. Shared input is
immutable, each map/regex/tree allocation job has clear ownership, and output
order follows original benchmark order after the actual joins.

Required repairs are recorded per original candidate in `research/inventory.json`.
Keep explicitly invalid historical sources inspectable but excluded. In
particular, the older signed-item paired-tree algorithm and ambiguity-expansion
regex-dna source are different workloads; a renamed file does not fix them.
One-divmod/remainder-derived pi extraction is also excluded until both real
endpoint divisions are present in the application.

Bounded qualification should include exact PBM odd-width/tail cases, complete
FASTA boundary output, numerical nbody and spectral references, all permutations
and awkward worker tails for Fann, actual distinct-node/tree-lifetime audits,
THREE-prefix/comment/CRLF/EOF counterexamples, full IUPAC reverse complement,
all five ordered regex replacement counterexamples, and every pi tail through
97 digits plus zero. Instrumented unscored copies must independently reconcile
all seven actual nucleotide maps and both pi numerator/divisor/quotient values
for every candidate. The separate full canonical output check cannot be replaced
by the trace, nor can instrumented elapsed time become a timing sample.

The eventual sealed study uses fresh processes and the same timing boundary as
the existing suite, retaining startup, complete output and shutdown. Each chosen
compact/optimized source needs three actual canonical full records and freshly
matched current Mech AOT controls. Every result should report seconds, time
multiple against the explicitly identified Mech source and source LOC multiple
under the approved normalization rule. No ratios, speed estimates or LOC claims
are assigned during this preparation plan.
