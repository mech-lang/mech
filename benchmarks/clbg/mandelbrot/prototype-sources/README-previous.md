# Mandelbrot: local speed and source-size comparison

**Source-size status:** The 17–36 line Mech sources are numerical kernels,
not complete benchmark programs. Their coordinate formulas, recurrence,
escape test, iteration state and stopping predicate are compiled from Mech
to native code. The host grid driver supplies row/column traversal,
worker dispatch and PBM serialization; the CLI also selects the stopping
and result outputs. Competitors include more of this orchestration in their
counted source. The timings remain measurements of complete executables,
but these kernel-only sizes do not establish a fair complete-program size
advantage. A replacement comparison must express the program's traversal
and output composition in counted Mech source, using general runtime facilities.

This is the second Benchmarks Game workload for the blog evaluation, following
[n-body](../nbody/README.md). It uses the v0.4 integration base
`c4777b7015fe8ff47fdfa48d18606c49aace7d97` with the IROS Cranelift backend
and the experimental fixed-shape f64 native extension in this worktree.
The ordinary `mech build` command is not connected to this extension yet.


The latest controlled Rust/Mech comparison puts Mech at **0.684 s** and Rust #4
at **0.662 s**: Mech takes **3.4% longer** on this Apple M3. These are medians
of five sequential, shuffled fresh-process runs at N=16000 with eight workers.
Every complete image is byte-identical, including all 256 million pixels.
The winning Mech source is [mandelbrot-unrolled-add.mec](mandelbrot-unrolled-add.mec),
26 noncomment lines and 338 normalized gzip-1 bytes. The native path uses SIMD,
five-step unrolling, recurrence FMA, integer Boolean masks and a packed return.

[Native optimization chart](results/apple-m3-peeled-native-16000.native.svg)
· [A/B samples and hashes](results/apple-m3-peeled-native-16000.json)
· [Complete baseline speed/size plot](results/apple-m3-16000.svg)
· [Complete baseline endpoint table](results/apple-m3-16000.table.md)

The [updated full corner chart](results/apple-m3-improved-corner-16000.svg)
retains all eight languages and adds the improved Mech builds: 77 accepted
configurations across 42 sources. It normalizes each timing batch to its own
unmodified Rust #4 control; each row retains its actual median, batch identifier
and control median in the [derived report](results/apple-m3-improved-corner-16000.json).
This combines the complete baseline cohort with the later Mech studies. The
dashed red line shows the previous Mech endpoints. It is not a new full-cohort
timing run, and the removed first-turn peeling experiments are excluded.
The updated Mech endpoints are 17 LOC / 197 gzip bytes / 5.142× same-batch Rust
and 26 LOC / 338 gzip bytes / 1.034× same-batch Rust. Recreate the figure with
`python3 benchmarks/clbg/mandelbrot/corner_chart.py` followed by
`plot.py benchmarks/clbg/mandelbrot/results/apple-m3-improved-corner-16000.json`.

The complete baseline attempted all 45 pinned upstream submissions. It accepted
72 compiler configurations spanning 37 upstream sources and three Mech sources;
53 configurations were excluded. All 72 accepted configurations produced the
same complete N=16000 image. Its SHA256 is
`609262469ee6a0262ccd03932e557f745c9e7b997ad17835a02a0232a64807be`.
The baseline uses the compiler before the Boolean-mask/packed-return changes.
It is retained as the complete submitted-source tradeoff cohort. Its fastest
endpoints were:

| Language | Fastest local build | Median seconds | Source LOC | Gzip bytes |
| --- | --- | ---: | ---: | ---: |
| Rust | #4 | 0.603 | 146 | 1277 |
| C++ | #0, GCC fast-math | 0.768 | 308 | 2708 |
| C | #9, LLVM fast-math | 0.806 | 49 | 686 |
| Mech | Five-step SIMD, loop FMA | 0.814 | 26 | 321 |
| Julia | #7 | 0.861 | 59 | 751 |
| Java | #6 source, OpenJDK JIT | 1.822 | 57 | 798 |
| Chapel | #3 | 1.944 | 47 | 571 |
| Go | #4, FMA disabled | 2.120 | 102 | 896 |

The [five-sample endpoint confirmation](results/apple-m3-confirm-16000.json)
remeasured the fastest baseline build in each language and all five baseline
Mech builds. It measured Rust at 0.662 s and the original Mech build at 0.795 s.
Chapel chunk sizes 64/512/4096 and Mech demand scheduling did not improve their
medians. This selected study does not remeasure the smallest competitor sources.

The final native study compares preserved baseline binaries with newly generated
Mech binaries in the same sampling pass:

| Native configuration | Median seconds | Observed range |
| --- | ---: | ---: |
| Original Mech SIMD/unrolling/FMA | 0.804 | 0.783–0.817 |
| Mech + integer Boolean masks | 0.747 | 0.729–0.782 |
| Mech + packed Boolean return | 0.689 | 0.654–0.698 |
| Mech + doubling by addition | **0.684** | **0.683–0.690** |
| Rust #4, unchanged | **0.662** | **0.651–0.670** |

Boolean masks remove floating-point truth conversions and reductions. Packed
returns avoid eight double stores followed by runtime bit packing. Doubling by
addition replaces `2.0 * zr` with `zr + zr`, matching Rust #4's expression.
Ten-step unrolling was slower (0.782 s in its separate five-sample study), as
was first-turn peeling (0.698 s for the addition variant in the final study).
Peeling was removed from the live compiler; its exact implementation and tests
remain in that study's compiler archive. Winning native object/runtime hashes
were verified unchanged after removing it; the [final verification](results/apple-m3-final-verification.json)
retains the current compiler archive and validation results.

Rust #4 additionally precomputes the horizontal coordinates once, initializes
`z = c` to account for the first iteration, and performs four-step batches. It
checks early divergence only when the previous output byte is zero, skipping
some escape checks in likely interior regions. These are remaining implementation
differences, not a measured attribution of the final 3.4% gap. Establishing their
individual costs requires further ablations or profiling.

Mech is still a line in the observed tradeoff plot: the smallest source is 17
lines / 197 gzip bytes and the fastest is 26 lines / 338 gzip bytes. The current
mask-only compact build measured 3.402 s in the final A/B study, about 5.0× the
fastest build's time. The smaller program's compression advantage is 1.72×.
This selected Rust/Mech study does not replace the complete language plot.

A [one-worker baseline study](results/apple-m3-one-worker-16000.json) validated
all 12 selected controlled configurations with identical full images. Before
these Boolean optimizations, Mech took 4.443 s and Rust #4 took 3.916 s, a 13.5%
gap. It uses the earlier compiler and is kept separate from the final A/B data.
The full baseline's nine worker-policy label corrections changed metadata only;
[the raw report](results/apple-m3-16000.raw.json), corrections and original hash
are retained. Commands, samples and measured binaries were unchanged.

The [task specification](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/mandelbrot.html)
requires a complete N×N binary PBM of the complex plane from -1.5-i to 0.5+i,
with an escape radius of 2 and a maximum of 50 iterations. The full input is
N=16000: 256 million pixels and 32,000,015 output bytes. Every accepted build
must first match all 5,011 bytes of the official N=200 image. Its pinned SHA256
is `97610473750700638fc63d13cfa49d339b67c18e7f26b3f9c9acb61e746472d5`.

All numerical work is in Mech. [mandelbrot-compact.mec](mandelbrot-compact.mec)
computes one pixel at a time. [mandelbrot.mec](mandelbrot.mec) computes eight
pixels together and stops when all eight have escaped or 50 iterations have
completed. [mandelbrot-unrolled.mec](mandelbrot-unrolled.mec) performs five
recurrence steps between checks, following the technique used by several fast
upstream submissions. None uses an analytic cardioid/bulb test, image symmetry,
or a precomputed answer.

| Mech source | Nonblank noncomment LOC | Normalized gzip-1 bytes |
| --- | ---: | ---: |
| One pixel | 17 | 197 |
| Eight pixels | 18 | 255 |
| Eight pixels, five steps per loop | 26 | 321 |
| Five steps, doubling by addition | 26 | 338 |
| Ten steps, doubling by addition | 36 | 437 |

Only the `.mec` program is counted for Mech. Initialization, coordinates,
recurrence, classification, iteration count and termination are included.
Compiler, runtime and CLI are excluded as requested. Competitor sizes include
their complete pinned submissions, including their authored threading and
serialization code. This difference in source scope is explicit in every plot;
it is an experimental kernel-size comparison with runtime facilities supplied
for Mech. It does not establish a complete-program source-size advantage.

The native extension lowers the canonical v0.4 artifact to a Cranelift function
that resets state, reads immutable parameters, loops until the Mech Boolean
termination output is true, and returns the selected output. Arithmetic,
comparisons and stopping all execute in that native function. Scalar and
structured f64x2 SIMD code are generated by Cranelift. LLVM/Clang only compiles
the generic PBM/thread driver and links the object; it does not generate the
Mandelbrot recurrence. The same native map API is tested with unrelated source
programs. Boolean scalar matrix access was also added to resident execution so
the Mech source has an authoritative resident counterpart.

The generic grid runtime receives width, row and column indices, invokes a
compiled map, packs Boolean results into PBM bytes, and schedules rows across
pthread workers. An optional packed Boolean ABI returns up to 64 output bits
in one register; eight-lane maps write the returned byte directly. It contains no Mandelbrot coordinate formula, escape limit,
iteration limit or recurrence. Row workers use independent input/output arrays
and write disjoint bitmap rows.

FMA contraction is explicit. Full contraction changes coordinate rounding and
flips pixel (150,0) in the official N=200 image on this host. That configuration
is rejected. The loop-only FMA option preserves immutable input expressions,
including coordinate rounding, and contracts only state-dependent arithmetic.
Larger images can still differ at boundary pixels when recurrence rounding
changes; the report retains complete-image hashes and differing-pixel counts.
Passing N=200 alone is not described as byte equality at N=16000. An additional
plot filters to complete images that are byte-identical to the local Rust #4
reference.

The manifest pins all 45 submissions listed for Chapel, Julia, Rust, Go, Java
GraalVM AOT, C and C++ in the
[upstream table](https://benchmarksgame-team.pages.debian.net/benchmarksgame/performance/mandelbrot.html),
including starred submissions. All seven listed Rust sources are included in
the build/validation attempt. Sources stay unchanged. C/C++ uses GCC and LLVM
with native O3 and with fast-math, unrolling and LTO; LLVM additionally tests
fast-math with contraction disabled. Go uses default compilation and a compiler
configuration disabling FMA (`-gcflags=-d=fmahash=n`), which is documented in
this installed Go compiler's `base/hashdebug.go` and `ssa/func.go`. Dependency
versions for legacy Rust programs are locked separately. Local Java uses
OpenJDK JIT with the GraalVM submission source; it is labeled accordingly.

Architecture-dependent x86 submissions are retained in the manifest and
explicitly excluded on this Apple M3. Builds that fail the official bitmap
comparison are excluded from timing. All errors and diagnostics are retained.
The plots report both the entire accepted cohort and a cohort excluding
upstream starred submissions. An upstream star is used as the filter; a
high-level tuple or compiler-generated SIMD is not automatically classified
as heroic.

The main run requests eight workers through the Mech CLI and the Chapel,
Julia, Rayon and OpenMP runtime controls. Some unchanged sources fix their
worker count or use automatic hardware discovery. Those source policies are
recorded per build. Java's processor count is controlled, but some sources
create twice that many worker threads. Serial submissions remain serial.
Consequently the main chart compares complete submitted programs under their
recorded worker policies. A separate one-worker run isolates selected programs
whose execution can be controlled; it does not substitute a one-worker timing
for an uncontrolled submission.

Timing starts immediately before each fresh process and ends after it exits.
It includes initialization, JIT compilation where applicable, computation and
complete PBM output to an identical temporary regular-file sink. AOT builds,
hashing and validation are outside the clock. After one discarded warmup per
build, three or more independent samples run sequentially in seeded shuffled
order; median and range are retained. No compilation or tests run concurrently
with timed samples.

The size metric removes comments, collapses whitespace, encodes UTF-8 and
compresses with gzip level 1 and timestamp zero. It follows the deck's
compressed-source approach but is a local metric, not a claim to reproduce
published `gz` byte counts. Nonblank noncomment physical LOC is also reported.
Each language's line connects the fastest accepted build to the smallest
accepted source (ties broken by runtime), using independently normalized size
and time on logarithmic axes. A coincident pair appears as a dot. This measures
the observed submitted implementations, not an inherent limit of a language.

The JSON reports preserve every command, toolchain version, image hash, timing
sample, worker policy, exclusion and endpoint choice. A deterministic compiler
archive retains the exact source/runtime/driver/harness and lockfiles used on
the integration base; upstream submissions are separately hashed in the
manifest. Local machine performance is not the published Benchmarks Game
ranking and one workload is not an aggregate language ranking.

To reproduce on this configured Apple M3 host:

```sh
python3 benchmarks/clbg/mandelbrot/run.py --size 16000 --threads 8 --samples 3 \
  --output benchmarks/clbg/mandelbrot/results/new-full-run.json
python3 benchmarks/clbg/mandelbrot/plot.py \
  benchmarks/clbg/mandelbrot/results/new-full-run.json
```

The harness accepts executable paths for Julia, Java, GCC, g++ and Clang.
On macOS it uses the installed SDK, the installed OpenMP runtime for LLVM,
and the same SDK compatibility header as the n-body harness for GCC 16.
These build adaptations do not edit benchmark source or arithmetic.

Validation includes the complete official bitmap, independent scalar reference
images at multiple sizes, scalar/SIMD agreement, recurrence FMA, resident/native
agreement, repeated map reset with changing parameters, IEEE comparison edge
cases, invalid termination outputs, and immutable-product rounding across FMA.
The resident Boolean-access unit check verifies malformed Boolean payloads and
invalid indices leave the destination untouched. The n-body native regression
suite remains part of verification for the shared compiler changes.

To reproduce an archived study exactly, restore its compiler archive into the
pinned integration checkout before building. The current compiler has the
winning optimizations and the full harness includes the additional Mech variants,
so a current full run is a new cohort. `native_study.py --extended` compares
preserved baseline binaries against the current compiler using unique artifact
names. The removed peeling experiment is reproducible from the
`apple-m3-peeled-native-16000.compiler.tar.gz` archive, which retains its harness,
compiler, Mech sources and tests. Every report stores source/object/runtime
hashes and its sampling order.
