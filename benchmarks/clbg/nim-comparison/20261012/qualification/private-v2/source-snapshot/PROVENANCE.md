Nim metadata: initial self-hosting release 0.6.0 was 2008-08-22, recorded in the
official historical web/news.txt at Nim commit
92c873154a36d34ff8e18de6b2d1a0eb1b611860. It is statically typed, compiled to
native code through C, and supports procedural/object-oriented/metaprogramming
styles. GitHub Linguist color is #ffc200 at commit
5fbdfcb8133be2bed88bf3ce62b2335f50474525. These are metadata, not timings.

Primary pinned community inventory:
https://github.com/def-/nim-benchmarksgame/tree/466e4b8dcc9d70a6e619c6145db92dbd4f921e63
contains all ten task families. This is a community collection of historical
Benchmarks Game ports; Nim is absent from the current official language catalog.

The second primary source collection is
https://github.com/hanabi1224/Programming-Language-Benchmarks/tree/ad18b203dd1769724f4eea94fc3ac1e99f6593e0/bench/algorithm
with Nim paths binarytrees/{1,2}.nim, fasta/{1,2}.nim, mandelbrot/1.nim,
nbody/2.nim, pidigits/1.nim, regex-redux/1.nim and spectral-norm/{1,2}.nim.
It has seven of our ten families; fannkuch, nucleotide and reverse complement
come from the complete community inventory. All local drafts are adaptations,
not byte-identical copies. No upstream physical line count is a validated LOC.

Trees use modern node-count checks, allocate leaves and retain the long-lived
tree; historical item checks and object pools are excluded. Optimized workers
each allocate/check their entire assigned trees without caching results.
Fannkuch uses the conventional descending factoradic initialization and ascending
prefix-rotation carry, global rank parity and actual repeated prefix reversals;
the Heap-order variant and shortcut flip-count branches are excluded.
Fasta keeps every float-CDF draw and the continuous LCG stream, replacing old
4096-entry lookup and integer thresholds; repeat output stops exactly at N.
Nucleotide reads exactly THREE with space/tab/end boundary, comments, CRLF/EOF,
and builds all seven growable maps, including lengths one and two; the old mmap
reader and first-window off-by-one are repaired. Reverse complement retains
headers/IUPAC symbols and writes bounded final lines rather than unchecked slices.
Mandelbrot emits full PBM bytes rather than an MD5 and keeps requested width,
including tails. Nbody uses ordinary exact sqrt with the standard pair/position
update order. Regex retains nine non-overlapping counts and five sequential
substitutions, reading stdin. Spectral norm computes each dense product for ten
power iterations; the optimized source bounds ordinary native workers explicitly.
Pi performs both actual endpoint divisions, with canonical ten-column output.

Generic package APIs are pinned independently: pure BigInts provides arithmetic
and checked toInt conversion. Optimized pi uses the public
FedeOmoto/nim-gmp 0.2.4 header declarations at
4e9a6a05bfc274abd02a241f2d7c9162b2300191
(https://github.com/FedeOmoto/nim-gmp/tree/4e9a6a05bfc274abd02a241f2d7c9162b2300191).
support/gmp_ops.nim provides generic names and the csize-to-csize_t compatibility
alias over that unchanged package; it contains no workload arithmetic or spigot.
Root's consistent policy excludes
this ordinary generic library support from application LOC, as for Lua's GMP
binding, while counting application workload helpers. No new native benchmark executor is
introduced. Qualification, runtime pins and audited LOC remain pending.
