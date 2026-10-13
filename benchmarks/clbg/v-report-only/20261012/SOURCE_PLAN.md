# V report-only source plan

These are editable application drafts, not qualified results. V is not added to
any explorer, chart, language list, builder or website. `profiles.json` names
twenty core sources, compact and optimized for all ten workloads, plus one
separately labelled optional public big-integer-library candidate. Source
hashes, canonical LOC, compiler validation and timing samples are deliberately
absent until the root's locked preparation and review.

The nine published V files under `upstream/hanabi` came from
[Programming-Language-Benchmarks](https://github.com/hanabi1224/Programming-Language-Benchmarks/tree/ad18b203dd1769724f4eea94fc3ac1e99f6593e0),
commit `ad18b203dd1769724f4eea94fc3ac1e99f6593e0`, covering eight workloads.
Reverse-complement and nucleotide sources are authored locally. Every profile
identifies its exact ancestry and the algorithmic changes. The source generator
only writes V text and metadata; it never builds or executes benchmarks.

The planned compiler is official V0.5.2, source commit
`7647ce1c6fad63b5578bc07883139906de74b2f8`. Both forms receive the same optimized
C backend and Clang22 compiler flags: `-prod`, native CPU optimization and
`-ffp-contract=off`. Source-level optimizations differ. A later FMA experiment
must independently preserve complete PBM output and the nbody numerical oracle.
No fast-math, approximate reciprocal, source-specific runtime executor or
precomputed result is proposed.

The compact nbody coordinate displacement is `[dx,dy,dz]!`, the release's
documented fixed-array literal, rather than a dynamic allocation per pair.
This keeps the same algorithm and source-line count while avoiding an incidental
heap allocation. The exact release documentation is saved as
`upstream/v-api/docs.md` (fixed-size array section), with the original upstream
[documentation](https://github.com/vlang/v/blob/7647ce1c6fad63b5578bc07883139906de74b2f8/doc/docs.md#fixed-size-arrays).

| Workload | Compact draft | Optimized draft | Required work review |
|---|---|---|---|
| nbody | Nested V arrays and coordinate loops | Published cached scalar body fields and fixed five-body loops | Every step updates all ten unordered interactions and all positions; exact square root/division; before/after energies and published constants. |
| mandelbrot | Scalar escape loop and PBM packing | Eight row workers with uniform eight-lane arrays and published five-iteration escape checks | Original dimensions, up to50 actual recurrences per pixel, complete binary PBM, zero tail bits; no MD5-only output or dimension rounding. |
| spectralnorm | One implicit-matrix application with transpose flag | Eight row workers, exact denominator recurrence and four accumulation chains | Ten power iterations with40 fresh implicit products; no cached or materialized A matrix; complete scalar tail. |
| binarytrees | Distinct heap Node recursion and checks | The same allocations/traversals in at most eight depth workers | Both leaves and internal nodes are allocated separately; scoped stretch tree and retained long-lived root; no flat/index-only tree, cached subtree or checksum formula. |
| fannkuchredux | Published sequential fixed-array rotations | Eight global-rank ranges, factoradic initialization and ordinary prefix reversals | Every rank in n!, descending factoradic width order, global checksum parity and actual flips; no permutation/flip cache; empty worker ranges permitted. |
| fasta | Ordered binary64 CDFs and complete output buffers | Binary search over the same CDFs and bounded output buffers | Direct binary64 division for every RNG draw; one sequential LCG state carried from TWO to THREE; exactly2n/3n/5n bases,60-column output, no integer-scaled table or period cache. |
| revcomp | Record/line parsing and IUPAC complement lookup | Byte parser,256-entry alphabet table, in-place reversal and bulk output | All records, upper/lowercase IUPAC including U and ambiguity codes, comments, empty records, missing final newline and60-column wrapping. |
| knucleotide | Seven growable string-key maps | Seven growable packed-key maps in seven standard V workers | Read only the complete THREE record, ignore semicolon comments, normalize ASCII case; all overlapping windows for1/2/3/4/6/12/18; lengths1/2 remain maps; queries use the computed maps. |
| regexredux | Standard V regex counts and ordered replacement arrays | Eight independent standard-regex count workers, ninth after joins | Complete stdin, all nine nonoverlapping counts and five sequential replacements in the required order; identical generic cleanup semantics. |
| pidigits | Standard math.big q/r/t LFT | Published standard math.big endpoint-numerator basis | Two genuine arbitrary-precision divisions of distinct endpoint numerators for every tested candidate; sequential spigot, complete row/count/tail output and zero-digit empty output. |

## Published corrections and boundaries

Mandelbrot's original MD5 payload and rounded width are not the benchmark PBM.
Both drafts output the true dimensions and image. The optimized source computes
padded lanes only as ordinary vector padding, excludes them from the escape
reduction and never sets their output bits. Grid coordinates use the same
`scale=2/n; coordinate*scale-offset` binary64 order in both forms.

FASTA uses explicit ordered symbol/CDF arrays. This removes dependence on map
iteration order. The repeat loop produces exactly n requested characters even
when n is divisible by the ALU length or60; the original extra outer iteration
cannot append another character. There is no lookup table of generated random
letters and no LCG-cycle replay.

The nucleotide reader ignores semicolon comment lines before interpreting a
header. An embedded `>` in a comment cannot terminate the target sequence.
The target identifier is THREE, with a space, tab or end-of-header boundary.
Selected sequence data must be A/C/G/T, as in the required generated input.
Neither form replaces complete maps by the five requested-pattern counters.

Regex uses V's general regex implementation for cleanup, all counting patterns
and all ordered replacements. The cleanup must be checked independently against
`>[^\n]*\n|\n`, including multiple records and a final unterminated header.
The release's regex source documents dot as excluding newline, but that is not
a substitute for actual output validation. Replacement arrays fix the published
map-order dependency. Every worker owns its regex state and reads immutable
sequence data; the ninth count starts only after the first eight workers join.

For pi, V math.big `/` is truncating division. All actual denominators and
endpoint numerators in these spigots are nonnegative, so it equals floor here.
In the q/r/t form, production retains `3q+r>=0`; consumption transforms that
lower endpoint to `(7k+2)q+(2k+1)r>=(k-1)q>=0`. In the endpoint-basis form both
production remainders are nonnegative and consumption is a nonnegative linear
combination for k>=1. Both divisions are explicit; no second quotient is inferred
from a remainder or skipped after the first result. Untimed tracing should
independently verify both actual numerators, common denominator, quotients,
accepted digits and candidate count at a bounded input before full qualification.

The optional published `hanabi1224.biginteger` dependency is
[vlang-big-integer](https://github.com/hanabi1224/vlang-big-integer/tree/dae1213e3b7576b40bf84a5c45a25748a74ef3c0),
commit `dae1213e3b7576b40bf84a5c45a25748a74ef3c0`. It is a general **pure V**
32-bit-limb library, not GMP. Original implementation, tests, MIT license and
pi example are saved under `upstream/vlang-big-integer`. Its older option/error
syntax may need an isolated compatibility copy for V0.5.2; any such changes
need separate source pins and primitive tests. Its performance is unknown.
No package is installed globally. A future GMP profile would need a separately
reviewed ordinary public arithmetic/lifetime binding and an honest library
label; it is not present in these core drafts.

The Fann rank domain uses signed64-bit factorials, supports n<=20 and partitions
without multiplying total by the worker count, avoiding overflow at large n.
Canonical n12 still evaluates all479001600 permutations. Small n0/1 returns
the empty/identity permutation's zero-flip checksum. Binary-tree minimum maximum
depth is six, as in the common benchmark specification. Mandel/FASTA accept
nonnegative sizes and spectral norm requires a positive size.

## Qualification and timing handoff

The root owns bootstrap, compiler flags, source normalization, actual build,
references, primitive/source audits, guarded sampler and full studies. Planned
small/edge checks include Mandel widths1/2/7/8/9/17 and exact padding; spectral
sizes1/2/3/5/9/100; full Fann output and rank/flip traces for n1..8 plus awkward
worker boundaries; FASTA0/1/29/30/31 and sizes divisible by ALU length; mixed-case,
CRLF, comment, empty-record and EOF DNA cases; all five ordered replacement
counterexamples; and pi0..21/27/30/97 plus complete10000-digit output. Actual
tree allocation/lifetime and at-most-eight compute-worker ownership must also
be audited under the common mutex. The optional library needs its own general
signed arithmetic, construction, comparison and division tests.

Once both selected forms qualify, each workload's isolated sealed study has
three rotating rounds of compact V, optimized V and the freshly measured current
Mech AOT fast control, nine actual processes. Source/library experiments require
their own matched records rather than reusing a different library's samples.
Reports must retain seconds, time ratios against that named Mech source and
canonical LOC ratios, failures, output hashes, worker/PGID ownership, runtime
pins and timing boundaries. No planning estimate, partial set or scaled result
becomes a reported score. V remains report-only through completion.
