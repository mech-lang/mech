# k-nucleotide: v0.4 compiled comparison

**Evaluation status:** This is an API-composition prototype, not yet a fair
cross-language implementation-size comparison. The eight-line Mech program
calls newly added high-level operations that implement window traversal,
histogram counting, record extraction and frequency/lookup formatting. The
other submissions include much of that helper logic in their counted source.
The lack of hard-coded DNA constants and the generic conformance tests do not
by themselves establish an appropriate Mech compiler/library boundary. The
measured timings remain valid for this prototype pipeline; the eight-line/dot
result is withdrawn as a language-size claim pending that correction.
General counting and sequence operations are permitted standard-library support,
including new support developed for this evaluation, just as combinations is
for n-body. Their implementation language is not the deciding factor. The
required boundary is a Mech program composed of compiler-visible operations
with reusable semantics and native lowering, rather than a benchmark wrapper
dispatching its algorithm to an opaque host pipeline. Benchmark-specific
selection, queries and output composition belong in counted Mech source.

This is a local blog benchmark on the v0.4 integration checkout
`c4777b7015fe8ff47fdfa48d18606c49aace7d97`. It adds an experimental typed
collection backend; these operations are not yet installed in the general
resident compiler/catalog. It does not establish stock v0.4 support or a
language-wide performance ranking.

The [Benchmarks Game task](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/knucleotide.html)
requires counting **all** overlapping windows for lengths 1, 2, 3, 4, 6,
12 and 18 using a library hash table. Every window performs a lookup and
count update. Counting only the five queried strings would change the work.

## Input and validation

The pinned, unchanged FASTA C program generates input with N=25,000,000.
The complete input is 254,166,745 bytes, with 125,000,000 letters in THREE.
Every program performs 874,999,961 window updates in total, plus any
source-specific table merging. No code exploits the generator's periodicity.
The exact full input is hashed in `input25000000.json`; generating N=1000
with the same executable also reproduces the official input byte-for-byte.

The complete official N=1000 input and output are in `inputs/`.
Full output is compared byte-for-byte with unchanged Rust #7 and with the
published full output. All locally measured configurations must match the
complete full output. The official small test is checked separately.

Some unchanged upstream sources fail the small reference despite passing
full input: Rust #4/#5, Julia #1, and Java #4/#5 assume absent query keys
exist; Julia #7 divides frequencies by one fewer window than it counts.
The full-cohort chart flags these sources. The strict chart additionally
requires passing the official small reference. This distinction matters:
filtering small-test failures can remove a language's smallest implementation.

`verify.py` independently checks complete all-key frequency tables and
lookups for 287 histograms. The packing study checks nine configurations
(2,583 tables); the runtime study checks three packing configurations and
six runtime policies (5,166 tables); the SIMD study checks four packing
configurations and twelve runtime policies (13,776 tables). Cases
include empty/short inputs, batch tails, 64-bit keys, CRLF, lowercase input,
missing queries, frequency ties, later records, and non-DNA alphabets.
A separate case verifies fused record extraction still consumes stdin
through EOF before a later read. Native workers also check that the sum
of all table counts equals the number of admitted windows.
The source-admission tests reject unsupported syntax, wrong argument kinds,
mutable assignments and window shapes that exceed a 64-bit packed key.

## Mech sources and execution

- `knucleotide-compact.mec`: eight nonblank source lines; nests each histogram in its
  formatting/output expression.
- `knucleotide.mec`: fifteen nonblank source lines; starts the seven
  independent histogram computations, then prints them in required order.
  Longer windows start first, as in Rust #7.

The expanded form is redundant for performance with the new scheduler. Its
seven additional lines merely name the seven histogram values. The compact
form has eight physical, nonblank lines: one input definition and seven output
statements, containing 31 function calls in total. It is not eight primitive
operations. These are the complete eight lines:

```mech
sequence := bytes/encode(text/record(io/read(), ">THREE", ">"), "ACGT", "ignore-ascii-case")
io/write(format/frequencies(map/histogram(sequence/packed-windows(sequence, 2, 1)), "ACGT", 100, 3))
io/write(format/frequencies(map/histogram(sequence/packed-windows(sequence, 2, 2)), "ACGT", 100, 3))
io/write(format/lookup(map/histogram(sequence/packed-windows(sequence, 2, 3)), "GGT", "ACGT", "\t"))
io/write(format/lookup(map/histogram(sequence/packed-windows(sequence, 2, 4)), "GGTA", "ACGT", "\t"))
io/write(format/lookup(map/histogram(sequence/packed-windows(sequence, 2, 6)), "GGTATT", "ACGT", "\t"))
io/write(format/lookup(map/histogram(sequence/packed-windows(sequence, 2, 12)), "GGTATTTTAATT", "ACGT", "\t"))
io/write(format/lookup(map/histogram(sequence/packed-windows(sequence, 2, 18)), "GGTATTTTAATTTATAGT", "ACGT", "\t"))
```

Line count is sensitive to formatting and abstraction level, so the chart also
reports normalized compressed source: 245 bytes for the compact form and
294 for the expanded form. The named bindings add source text but produce no
additional executable work in the scheduled program.

The source selects THREE, specifies the alphabet and case handling,
requests every window length and histogram, and specifies output queries,
percent scaling, precision and separators. There are no nucleotide letters,
window lengths or query strings in the generic host implementation.

`hosts/gpu/src/sequence` uses Mech's canonical syntax parser, checks an
admitted immutable call graph, and emits a native entry point and packed
window kernels with Cranelift 0.131.3. An AOT object is emitted and linked
**before** timing. The executable does not parse or compile Mech at runtime.
Small serialized constants from the compiled source are loaded at startup.

Generic runtime services implement record extraction, alphabet encoding,
hashbrown 0.15.5 storage, asynchronous histogram tasks, sorting and text
formatting. Optional runtime policies use FxHash, narrowed key/counter
types and either hashbrown or the standard-library HashMap. The Rust runtime is compiled by rustc/LLVM with native CPU
features. Cranelift generates the program calls and rolling-key traversal;
the hash-table library and bulk increment service are ordinary runtime code.
This division is recorded explicitly in each Mech measurement.

The backend streams a single-consumer record selection instead of storing
unwanted records, releases dead intermediates, carries rolling integer keys
between batches, and avoids constructing a full vector of all window keys.
Each batch still performs the same library lookup/update for every key.
Tables begin at the library's small default capacity and grow normally.

As requested, source-size measurements include **Mech code only**. Generic
compiler/runtime/CLI code and library dependencies are excluded. This relies
on the new generic collection API: it should be described as an extension
prototype in the blog and implemented in the maintained catalog before
making a claim about the standard language in a paper. Compiler/library
abstraction boundaries also affect cross-language source-size comparisons.

## Competitors and timing

`manifest.json` pins all 28 currently listed sources for C, C++, Chapel,
Julia, Rust, Go and Java, including starred entries. Sources and their author
comments remain unchanged. Separate dependency pins cover khash, fastutil
and locked Rust crates. Local Java uses OpenJDK JIT with the published
GraalVM AOT submission sources; it must be labelled accordingly.

C/C++ builds test native O3 and fast-math/unrolling/LTO with GCC and LLVM.
Clang can use the GNU pb_ds sources with GCC's headers/libstdc++ and
emulated TLS. Two std::async LLVM LTO configurations fail to link against
this host's GCC TLS runtime; their errors are retained, and separate
fast-math/unrolling configurations without LTO are measured.
The SDK compatibility header only adjusts unused system-library declarations.

Timing uses fresh processes with identical regular-file stdin and stdout
sinks. Reading, encoding, every hash-table update, sorting, output, startup
and Julia/Java JIT work are timed. AOT builds, input generation and validation
are excluded. Full input is checked once before randomized timed rounds;
medians, min/max, all raw samples, execution order and executable hashes
are saved. No compiler builds or tests run alongside measured executions.

Runtime environment controls request eight workers. Unchanged source
policies may use different thread counts, per-frame partitions or serial
execution; every row records its policy. The original lowering serialized
the compact form and started seven histogram workers for the expanded form.
The new immutable dependency scheduler launches independent histograms in
both forms before waiting for their results, while preserving read/write
effect dependencies. Equivalent forms emit identical constant data, AOT
objects and executable bytes; the admission tests verify the object equality.

## Optimizations and measurement provenance

The baseline shows parallel Mech ahead of Julia and Chapel, with Rust ahead
of Mech. Inspecting the pinned implementations explains several available
optimizations: Rust #6 partitions each histogram across cores and uses
u8/u16/u32/u64 keys with u32 counters and FxHash; Rust #7 launches independent
histograms using hashbrown; Julia #6 uses packed integer keys, a fast hash,
one-lookup updates and histogram tasks; Chapel #4 builds task-local maps
and combines them, with an identity hash on packed keys.

The new Mech support applies the following generic optimizations:

- Schedule the immutable dependency graph so compact expressions can expose
  the same parallelism as named intermediate values.
- Pack eight two-bit symbols with wide integer operations. Optionally use
  two-lane integer SIMD to form and store the resulting keys. All other bit
  widths and every boundary/tail retain a scalar path.
- Choose key widths from the admitted packed width and choose u32 counters
  only when the full number of windows fits; otherwise use u64 counters.
  FxHash and the map storage remain existing library implementations.
- Partition disjoint window starts into local library maps and merge counts.
  Input slices overlap only to finish boundary windows. A bounded Rayon pool
  does the counting; waiting histogram coordinators are recorded separately.
- Fuse a single-consumer record extraction with encoding, including EOF
  consumption, to avoid retaining and rescanning the selected text.

The driver exposes these generic compiler/runtime policies through build and
runtime environment flags. Both source forms receive the same policies;
no DNA constants, query-only counter, custom hash table or periodicity shortcut
is moved into the host. Each native traversal still performs the full library
lookup/update workload. All maps start at their library's default capacity.

The raw baseline, scheduling/packing, runtime, and SIMD/map studies each have
a compiler snapshot, executable hashes, all fresh-process samples and full
output checks. `combine.py` takes the latest measurement for repeated
configurations. It divides times by the Rust #7 control measured in the same
batch; raw medians remain available. Within a batch, byte-identical Mech
executables with identical runtime policies share a median pooled from their
samples. That treats the compact/expanded comparison as the same executable,
rather than mistaking timing noise for a source-size penalty.

The combined chart selects each language's fastest and smallest implementations
across 57 configurations: all 28 upstream sources plus 16 builds of the two
Mech forms using the current compiler/runtime revision. All earlier Mech
revisions, including faster estimates, are retained separately in the raw
reports and `historical_programs`; compiler revisions are not mixed into the
current Mech corner. Its gzip and LOC charts select size
endpoints independently. Small-reference failures remain marked, with separate
strict-cohort charts. This is an implementation sample for the blog, and its
Mech source-size result depends on the experimental generic collection API.

## Results on Apple M3

The baseline experimental lowering measured 3.966 seconds for the eight-line
compact form and 1.277 seconds for the fifteen-line expanded form. The final
confirmation measured 0.946 seconds for both forms, pooled from 22 runs of
byte-identical executables under the same policy. The fastest Rust #6 build
with `panic=abort` measured 0.916 seconds over eleven runs in that batch.
The point estimate is a 3.3% Mech time penalty; a bootstrap of paired randomized
rounds gives a local 95% interval of 0.964–1.062 for the Mech/Rust time ratio.
These samples do not establish a clear difference in execution speed.

The fastest baseline Julia #6 measured 2.673 seconds and Chapel #4 measured
3.081 seconds. Cross-batch plots use a same-batch Rust #7 control rather than
comparing unadjusted times from separate runs.

The best current Mech profile uses DAG scheduling, eight-symbol **wide integer**
packing, fused extraction/encoding, narrow keys/counters, FxHash,
standard-library HashMap storage, and four partitions per histogram in an
eight-worker Rayon pool. NEON packing was tested and verified but was slower;
it remains an optional experiment, not the selected profile. Native assembly
for both packing paths is saved alongside the results.

Within this API-composition experiment, the compact call site is both fastest
and smallest, so the plotted line collapses to a dot. This does not establish
an eight-line implementation-size advantage over the other languages. The expanded source
is an optional readability form, not a separate performance implementation.
The result depends on the generic collection API and the prototype compiler
extension, which must remain explicit in the blog.

- [Current corner chart](results/apple-m3-improved-corner-25000000.png)
- [LOC chart](results/apple-m3-improved-corner-25000000.loc.png)
- [Full endpoint table](results/apple-m3-improved-corner-25000000.table.md)
- [Strict small-reference chart](results/apple-m3-improved-corner-25000000.strict.png)
- [Combined data and provenance](results/apple-m3-improved-corner-25000000.json)
- [Confirmation and timing uncertainty](results/confirmation-analysis.json)

## Reproduce

```sh
# Pin only on first setup; refuses to refresh an existing manifest.
python3 benchmarks/clbg/knucleotide/fetch_sources.py

# Full input was produced with the pinned FASTA source:
# compile upstream/fasta-gcc-1.c with native O3 and strict contraction,
# then run .build/fasta 25000000 > .build/knucleotide-input25000000.txt.

RUSTFLAGS='-D warnings -C target-cpu=native' \
CARGO_TARGET_DIR=benchmarks/clbg/nbody/mech-driver/target \
cargo test --release --locked \
  --manifest-path benchmarks/clbg/knucleotide/mech-driver/Cargo.toml \
  --test admission
python3 benchmarks/clbg/knucleotide/verify.py
python3 benchmarks/clbg/knucleotide/verify.py --runtime
python3 benchmarks/clbg/knucleotide/verify.py --runtime --simd

# Build/check all candidates, then time without further compilation.
python3 benchmarks/clbg/knucleotide/run.py --prepare-only
python3 benchmarks/clbg/knucleotide/run.py --reuse-builds --samples 3

# Prepare the selected current profile (both source forms).
python3 benchmarks/clbg/knucleotide/run.py --prepare-only \
  --only mech-compact-final-std1-simd0-p4,mech-parallel-final-std1-simd0-p4
# Confirmation requires Rust controls to have been prepared as well.
python3 benchmarks/clbg/knucleotide/run.py --reuse-builds --samples 11 \
  --seed 20261008 \
  --only knucleotide-rust-6,knucleotide-rust-6-panicabort,knucleotide-rust-7,mech-compact-final-std1-simd0-p4,mech-parallel-final-std1-simd0-p4 \
  --output benchmarks/clbg/knucleotide/results/replay-confirmation.json

# Rebuild the combined view from the recorded batches.
python3 benchmarks/clbg/knucleotide/combine.py \
  benchmarks/clbg/knucleotide/results/apple-m3-25000000.json \
  benchmarks/clbg/knucleotide/results/apple-m3-scheduling-packing-25000000.json \
  benchmarks/clbg/knucleotide/results/apple-m3-runtime-25000000.json \
  benchmarks/clbg/knucleotide/results/apple-m3-simd-map-25000000.json \
  benchmarks/clbg/knucleotide/results/apple-m3-confirmation-25000000.json \
  --output benchmarks/clbg/knucleotide/results/apple-m3-improved-corner-25000000.json

# Python environment requires matplotlib; generates gzip and LOC charts,
# both full-input and strict-small-reference cohorts, SVG/PNG and tables.
python3 benchmarks/clbg/knucleotide/plot.py \
  benchmarks/clbg/knucleotide/results/apple-m3-improved-corner-25000000.json
```

The source-only metric removes comments, collapses whitespace and applies
gzip level 1 with a deterministic timestamp. This is a local compressed
metric, not a claim to reproduce the website's `gz` values. The LOC chart
selects the smallest source by nonblank, noncomment lines independently of
the gzip chart; each joins the fastest and smallest endpoints. A coincident
point only describes the sampled implementations, not an absence of a
possible size/performance tradeoff in the language.
