# Swift comparison

Full official CPU workloads on Apple M3, AC power and Low Power Mode off.
The installed compiler is Apple Swift 5.10. The measurement page uses Swift
6.0.3 on Linux; its timings are used only to identify candidates, never as local
measurements. Build flags match the published optimized builds:
`-Ounchecked -whole-module-optimization`.

`upstream/` pins the measurement listing, all 37 linked program pages and their
original sources. `manifest.json` records URLs and hashes. `sources/` contains
Darwin import ports with independent packed statements separated. All algorithm
code counts toward nonblank/noncomment LOC; comments and blank lines do not.
The lexer also checks that no unclassified tokens were silently dropped.

`corrections.json` records ten separate Swift corrections, each retaining its
upstream ID, source hash and detailed changes:

- K-nucleotide #1 replaces obsolete `String.characters` with the modern API.
- FASTA #3 replaces integer-scaled cutoffs, explicitly prohibited by the current
  [FASTA description](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/fasta.html).
- FASTA #8 uses cumulative probabilities rather than subtracting probabilities
  from the sample. FASTA #1/#2 use explicit binary64 division to align with this
  comparison's work. The latter changes are cohort comparability decisions;
  they are not blanket claims that every source violates the published rules.
- Tree versions release stretch storage before long-lived allocation and keep
  the long-lived root until after all temporary-tree work. These enforce the
  [tree lifecycle](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/binarytrees.html).

`source-audit.json` binds required-work review to each exact source hash.
Optimized IR in `results/*.ll` confirms separate leaf/interior allocation,
recursive construction, every temporary-tree iteration, stretch release and
final long-lived checks. An indirect enum's `Node(Empty,Empty)` is a separately
allocated leaf; these sources do not omit leaves. Original asynchronous versions
#2/#3/#4 own the long-lived tree only in a queued check closure and do not ensure
it survives all temporary work. Original #1/#5 keep stretch storage globally
rooted without clearing it. Corrections are labelled as local adaptations.

All admitted candidates run sequentially in three shuffled rounds, alongside
one unchanged current fastest Mech AOT control per benchmark. Each also receives
independent small and full output checks. Fresh-process elapsed times include
startup, complete input, computation, complete output to a regular file and
process teardown. Compilation and output hashing are outside timing. Swift
Dispatch retains the original scheduling on eight logical CPUs; task counts can
exceed core counts. Active compiler or power changes invalidate timed samples.

GMP supplies general unbounded arithmetic through `support/gmp/module.modulemap`;
all pi-digit spigot control and output remain in Swift. Build commands, actual
driver/frontend hashes, linked GMP binary, sources and output hashes are pinned.
Build failures and output failures stay in the report with diagnostics; they
never enter smallest/fastest endpoint selection.

Reproduction order: `fetch.py`, `prepare.py`, `build.py`, optimized IR emission,
`audit.py`, `run.py`, `summarize.py`. Fetch requires Pygments 2.19.2 for Swift
comment/string-aware statement normalization. The runner resumes its immutable
source/build snapshot. `results/summary.json` selects the fastest median and the
fewest LOC from admitted sources; it records fresh controls separately from
the existing website cohort. No compressed-source metric is used.
