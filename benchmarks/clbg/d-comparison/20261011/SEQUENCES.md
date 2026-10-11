# D sequence candidates

`sequence-manifest.json` freezes all ten candidates, adapted-source and executable
hashes, compiler commands, provenance, and exact small-output validation. Root
runs full validation and serial measurements; no timings in this preparation
were admitted as benchmark samples.

Historical sources are preserved from `kragen/shootout` commit
`71aa4ec4cd15940c59f1a1bb71ac1ff1572a55c2`; modern sources are preserved from
`hanabi1224/Programming-Language-Benchmarks` commit
`ad18b203dd1769724f4eea94fc3ac1e99f6593e0`. Each original has a `.provenance.json`
with its pinned URL. D no longer appears in the current Benchmark Game language
list; these must be described as historical/published sources and explicit local
adaptations, rather than current official D leaderboard submissions.

FASTA executes every seed recurrence, a binary64 `seed / 139968` division, and
comparisons with an unscaled binary64 CDF. The preserved old D1 FASTA source uses
float arithmetic and is historical evidence only: it was **not** selected or
compiled. The modern published form, compact adaptation, and buffered/static
probability-width adaptation all use binary64. No seed-period cache, integer
threshold scaling, or generated output cache is present.

Both k-nucleotide forms construct all seven complete growable maps, making
874,999,961 updates at the full 125-million-base THREE-record workload. The
compact form slices immutable string keys; the rolling form uses two-bit packed
keys in ordinary D associative arrays. Its five workers cover the requested
lengths while the main thread makes the one- and two-base complete maps. It does
not count only queried keys. The canonical generator naturally has modest map
cardinality; no implementation assumes or exploits that fact. An initial
1,000,020-base sample had 138,127 distinct 12-mers and 139,882 distinct 18-mers.
A modern `Nullable` hash-key incompatibility was resolved by retaining the same
`Code` payload directly. Arbitrary FASTA line lengths, lexical ties, and short
records are handled; `results/sequence-adversarial.json` records paired checks.

Reverse complement maps all allowed uppercase and lowercase IUB letters and
rewraps the full reversed records to 60 columns. The compact form is a D2
adaptation of the historical high-level D1 program. The optimized form performs
buffering and lookup-table reversal entirely in D source.

Regex redux uses actual D `std.regex` searches and replacements throughout.
The compact form compiles regexes at runtime; published forms use D compile-time
regex generation. All nine searches are case-sensitive and nonoverlapping,
matching the canonical current regex-redux requirements, and all five
replacements execute sequentially on their preceding complete result. Source
1 is serial, source2 uses appender replacement buffering, and source3 runs
substitutions in one worker while the main thread does the searches. The input
boundary was changed from a filename to canonical stdin and match arrays were
replaced by `walkLength`, preserving complete matching work.

`finalize_sequences.py` rebuilds the frozen adapted sources with native LDC
`-O3 -release -boundscheck=off -mcpu=native`, regenerates hashes, and verifies small
stdout. It never generates final timing measurements. LOC includes every D
application operation; there are no benchmark-specific host kernels.
