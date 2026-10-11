# k-nucleotide: compiled Mech source comparison

The revised program defines frequency and query functions in Mech. It selects
the input record, requests every histogram, decodes keys, sorts counts with
lexical tie breaking, calculates percentages, formats rows and prints queries.
The former `format/frequencies` and `format/lookup` host operations have been
removed from the active catalog and runtime. Their eight-line size claim remains
withdrawn; previous sources and reports are retained as history.

This work adds pathways to the v0.4 integration base
`c4777b7015fe8ff47fdfa48d18606c49aace7d97`:

- Engine-owned typed sequence IR and canonical source admission, including typed
  Mech statement functions and scalar/vector arithmetic.
- Standard-library catalog for general sequences, maps, sorting, numeric
  formatting, string composition and IO.
- Cranelift AOT arithmetic and packed-window traversal, with scheduling and
  extraction/encoding fusion. Library hash maps, sorting, text and IO remain
  ordinary reusable runtime services.
- `mech build --backend cranelift`, enabled by the `cranelift_build` feature.
  The benchmark build driver calls the same sequence emission API.

The sequence IR is an additional native compiler pathway; it is distinct from
the reactive resident `ProgramArtifact`. These changes describe the language
extension implemented in this worktree, rather than the previously released
v0.4 distribution.

## Counted source

[Compact source](knucleotide-compact.mec): **17 LOC / 502 gzip bytes**.
[Expanded source](knucleotide.mec): **24 LOC / 536 gzip bytes**.
Both include the authored `frequencies` and `lookup` function bodies. The seven
extra bindings in the expanded source name histogram values. Under identical
scheduling policies the two forms emit identical constants and native objects;
there is no required performance penalty for the compact form.

```mech
frequencies(h<{u64:u64}>, width<u64>) = (output<string>) :=
  keys := sequence/decode(map/keys(h), 2, width, "ACGT")
  counts := map/values(h)
  order := sequence/sort-indices(counts, "desc", keys, "asc")
  percentages := 100.0 * number/to-f64(sequence/select(counts, order)) / number/to-f64(stats/sum(counts))
  output := string/concat(text/join-terminated(text/zip(sequence/select(keys, order), number/format(percentages, 3), " "), "\n"), "\n").
lookup(counts<{u64:u64}>, query<string>) = (output<string>) :=
  key := sequence/pack(bytes/encode(query, "ACGT", "exact"), 2)
  output := string/concat(number/format(map/get(counts, key), 0), string/concat("\t", string/concat(query, "\n"))).
sequence := bytes/encode(text/record(io/read(), ">THREE", ">"), "ACGT", "ignore-ascii-case")
io/write(frequencies(map/histogram(sequence/packed-windows(sequence, 2, 1)), 1))
io/write(frequencies(map/histogram(sequence/packed-windows(sequence, 2, 2)), 2))
io/write(lookup(map/histogram(sequence/packed-windows(sequence, 2, 3)), "GGT"))
io/write(lookup(map/histogram(sequence/packed-windows(sequence, 2, 4)), "GGTA"))
io/write(lookup(map/histogram(sequence/packed-windows(sequence, 2, 6)), "GGTATT"))
io/write(lookup(map/histogram(sequence/packed-windows(sequence, 2, 12)), "GGTATTTTAATT"))
io/write(lookup(map/histogram(sequence/packed-windows(sequence, 2, 18)), "GGTATTTTAATTTATAGT"))
```

The size metric counts complete Mech source and excludes generic compiler,
standard library, runtime and CLI support, as requested. Competitors count their
unchanged complete submissions. New abstraction boundaries are explicit: general
histogram and sequencing library support is permitted; benchmark-specific
frequency/query composition is counted source. Both noncomment LOC and
whitespace-normalized gzip-1 bytes with timestamp zero are reported.

## Work and correctness

The [task](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/knucleotide.html)
requires **all** overlapping windows at k=1,2,3,4,6,12,18 to receive a library
hash-table lookup/update, including lengths used only for queries. The unchanged
pinned FASTA generator produces 254,166,745 input bytes, with 125,000,000 letters
in THREE. Every candidate performs **874,999,961** window updates plus any table
merging. There is no query-only counter or generator-period shortcut.

All timed candidates must match the complete full output from unchanged Rust #7
and the published output. The official N=1000 test is checked separately. Some
unchanged submissions fail that test despite passing full input; the full chart
marks them and the strict chart excludes them.

The revised generic conformance program defines its own Mech frequency/query
functions. It independently checks 287 all-key histograms per policy against
Python Counter, including non-DNA alphabets, empty/short inputs, CRLF, lowercase,
absent keys, ties, packing boundaries and full 64-bit keys. The runtime study
checks three packing configurations and six policies: **5,166 histograms**.
It also checks invalid encoding rejection and that fused record extraction
consumes through EOF. Admission tests cover functions, native arithmetic,
invalid shapes, recursion, duplicate parameters and rejection of removed host
wrappers. Earlier conformance reports are preserved separately.

## Optimization and timing

The selected profile uses immutable dependency scheduling, eight-symbol wide
integer packing, fused record extraction/encoding, narrow keys/counters,
FxHash, standard-library HashMap and four partitions per histogram in an
eight-worker Rayon pool. Each partition uses an ordinary growing library map;
start ranges are disjoint and source slices overlap only to complete windows.
NEON packing remains an optional verified experiment; prior measurements did
not favor it. Both authored source forms receive the same compiler policies.

The pinned cohort contains 28 upstream sources across C, C++, Chapel, Julia,
Rust, Go and Java. C/C++ tests GCC and LLVM. Java uses OpenJDK JIT with the
published GraalVM-submission source and is labeled accordingly. Architecture,
linking and correctness exclusions are recorded rather than editing sources.

Fresh-process timing includes reading identical regular-file stdin, encoding,
every count, merging, sorting, complete output and JIT startup where applicable.
AOT builds and output validation are excluded. Samples run sequentially in
seeded shuffled rounds with eight requested workers; source-specific worker
policies are retained. No builds or tests run during timed samples.

New complete-source measurements are saved separately from prototype timings.
The revised combined chart retains the upstream cohort and uses only the new
Mech compiler revision. Each batch is divided by its own Rust #7 control;
raw medians, every sample, source/executable/object hashes and exact compiler
archives remain available. Earlier Mech revisions remain in history.

- [Revised corner chart](results/apple-m3-revised-corner-25000000.png)
- [LOC chart](results/apple-m3-revised-corner-25000000.loc.png)
- [Endpoint table](results/apple-m3-revised-corner-25000000.table.md)
- [Strict chart](results/apple-m3-revised-corner-25000000.strict.png)
- [New raw measurements](results/apple-m3-revised-25000000.json)

## Revised results on Apple M3

Five fresh-process samples at full input measured **1.005 s** for the compact
program and **0.999 s** for the expanded program. The fastest unchanged Rust #6
with panic=abort measured **0.899 s** in the same batch: Mech takes about
**11–12% longer**. The two Mech native objects and constant tables are identical;
the public build gives them separate application binaries. The small observed
time difference does not establish a benefit from seven extra bindings.
These are fresh complete-source measurements, not the previous prototype score.

## Reproduce

```sh
cargo build --release --features cranelift_build --bin mech
# The program itself contains all benchmark-specific behavior.
mech build knucleotide-compact.mec --backend cranelift -o knucleotide --offline
RAYON_NUM_THREADS=8 ./knucleotide < input.fa

# Independent checks and benchmark preparation, from repository root:
python3 benchmarks/clbg/knucleotide/verify.py --runtime
python3 benchmarks/clbg/knucleotide/run.py --prepare-only \
  --only knucleotide-rust-6-panicabort,knucleotide-rust-7,mech-compact-final-std1-simd0-p4,mech-parallel-final-std1-simd0-p4
python3 benchmarks/clbg/knucleotide/run.py --reuse-builds --samples 5 \
  --only knucleotide-rust-6-panicabort,knucleotide-rust-7,mech-compact-final-std1-simd0-p4,mech-parallel-final-std1-simd0-p4 \
  --output benchmarks/clbg/knucleotide/results/new-revision.json
```

A dot describes the two observed endpoints under this library/compiler revision;
it does not prove that the language has no possible size/performance tradeoff.
The blog can describe this as implementation work plus a local evaluation.
A paper should specify the added language/library surface and measurement scope.
