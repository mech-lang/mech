# Reverse-complement — v0.4 integration, experimental Cranelift AOT

This benchmark extends the existing managed Benchmarks Game worktree. It is for
blog exploration; these desktop measurements are not yet a paper-quality claim
about language rankings. The compiler path is an experimental admitted subset
of canonical Mech source, exposed through `mech build --backend cranelift`.
It does not imply that these operations already work in every resident engine.

The complete, readable program is [`revcomp.mec`](revcomp.mec): **10 nonblank,
noncomment lines**. It defines both cases of every complement, reversal,
translation, wrapping, record traversal and output. The earlier five-line
formatting was replaced with explicit intermediate definitions. Source size is
also reported as whitespace-normalized gzip level 1, so line formatting alone
cannot establish a compactness advantage.

## Measured results

On this Apple M3, the final paired seven-sample comparison transforms all
250 million bases and writes the complete 254 MB output:

| Implementation | Median elapsed | Counted LOC | Normalized gzip bytes |
| --- | ---: | ---: | ---: |
| C #9, LLVM | 0.138 s | 143 | 1,523 |
| Rust #1, portable ARM fallback | 0.164 s | 257 | 2,729 |
| Mech, Cranelift SIMD AOT | 0.218 s | 10 | 301 |

The [paired raw measurements](results/paired-optimizations.json) put Mech at
1.58 times C's elapsed time and 1.33 times Rust's. The preserved first Mech
implementation takes 0.354 s in the same paired batch: current Mech takes
**38.4% less time**, with the same source algorithm. Dense formatting in that
preserved executable does not represent an additional compact implementation.

The [complete corner chart](results/apple-m3-final-25000000.corner.png) uses
normalized gzip size; [the LOC chart](results/apple-m3-final-25000000.corner.loc.png)
and their endpoint tables report physical compactness separately. Mech's
fastest and smallest admitted source coincide. This is a result for this
benchmark and these implementations; our newly implemented general byte/record
library has a substantial role in the compact source. It does not establish
that Mech programs in general will have this relationship.

Compiler ablations use the same complete source, so these numbers measure
policy effects rather than comparing different Mech algorithms:

| Mech policy | Median elapsed |
| --- | ---: |
| SIMD, fusion, exclusive reuse, four partitions | 0.218 s |
| Scalar transformation and library wrapping | 0.315 s |
| Reverse/translate fusion disabled | 0.226 s |
| Exclusive-buffer reuse disabled | 0.245 s |
| One transformation partition | 0.246 s |
| Eager buffered record input | 0.229 s |

These effects are not additive. A separate [instrumented diagnostic](results/record-phase-profile.txt) reports
about 131 ms reading/parsing/joining records, 8 ms in native transformation,
and 58 ms formatting/writing. Instrumentation changes the timed path; the
ordinary end-to-end measurements above remain the comparison. The next useful
optimization is to reduce line compaction and output memory passes, rather
than focusing only on the already fast byte transformation.

`revcomp-buffered.mec` is the same transform with eager record parsing instead
of explicit record streaming. Compiler policy ablations use the same complete
source; they are not distinct algorithms and are not counted as extra language
implementations. There is no benchmark-named function, complement table,
FASTA header, 60-column constant or workload-size test in the host backend.

## General language/library operations

- `bytes/table(from, to, missing)` constructs a 256-entry checked byte map.
  Byte strings must have equal length; conflicting duplicate keys are errors.
  Missing entries either error or preserve their input (`identity`). This is a
  byte operation; UTF-8 strings provide their encoded bytes.
- `sequence/reverse(bytes)` reverses an immutable byte sequence.
- `bytes/translate(bytes, table)` performs checked byte-to-byte substitution.
- `text/records(text, prefix, separator)` groups lines by a nonempty header
  prefix. It preserves each header and joins body lines with the supplied
  separator; CRLF terminators are stripped. Stored records are repeatable.
- `io/read-records(prefix, separator)` provides an explicit one-shot input
  stream using growing buffered reads. It checks line-prefix boundaries even
  when marker bytes occur inside body lines; it does not stat/preallocate input.
- `records/map(records, function)` is a lazy mapping to text. The function's
  signature is `(string, byte sequence) -> string`; its body is compiled Mech.
  Captures, recursion and callback IO are rejected by this admitted subset.
- `text/wrap(bytes, width, separator)` inserts a trailing separator after every
  full or partial nonempty line; empty input produces empty text. Width is
  positive. Immutable formatting views materialize only for contiguous text
  consumers; IO can serialize them directly.
- `io/write-each(stream)` runs the emitted traversal and writes mapped texts
  in order. Stream errors may follow previously written successful records.

Rust library support owns storage, parses general records, and writes buffers.
**Reversal, translation, source callback bodies, mapping traversal and eligible
wrapping loops execute in emitted Cranelift machine code.** The generic Rust
application links that object and its source-derived constant table; its LLVM
build is not the source algorithm's execution engine.

## Compiler decisions and techniques adopted

The backend reasons about operation types, use counts, constant values and
runtime ownership. It does not recognize benchmark names or inputs.

1. A reversal with one translation consumer is fused; a protected function
   output or another consumer prevents removal. Liveness is recomputed after
   fusion, and stable handle allocation is preserved with an unused placeholder.
2. A last use permits buffer consumption; mutation still requires exclusive
   `Arc` ownership. Shared inputs are copied. Native workers own disjoint
   front/back byte pairs, including at most one middle byte, and join before
   a successful result is published.
3. Constant tables are inspected across **all 256 inputs**. Identical ASCII
   case pairs allow folding; a nonzero output alphabet spanning at most 32
   folded keys admits two SIMD table lookups and byte reversal. Invalid bytes
   are still checked. Arbitrary/dynamic tables and scalar tails use checked
   native lookup loops. This is a general lowering of byte permutations.
4. Constant wrapping widths 1–256 and separators up to 8 bytes admit native
   vector/scalar copies into approximately 256 KiB output spans. Other shapes
   retain the general formatter. Empty and partial final lines are handled.
5. Generic record input grows through buffered spans, validates actual header
   prefixes and joins body lines. Source requests explicit streaming to release
   one record before reading/transforming the next.

The portable C #9 program uses a two-byte lookup table, cache-sized transformed
chunks with ordered writes, and a fast path that preserves suitably positioned
input newlines. Rust #1 has cfg-gated x86 SSSE3 and a portable fallback; the
fallback is measured on this ARM host. Rust #2/#3 retain a full transformed
sequence and use scalar lookup with parallel byte ranges. These are concrete
implementation differences, not evidence that C is intrinsically faster.

Mech adopts SIMD substitution, traversal fusion, exclusive-buffer reuse,
bounded formatting buffers and native formatting copies. It currently strips
input line terminators and reconstructs output lines, whereas C can preserve
regular newline layouts and combine transformation with output chunks. This
remaining distinction accounts for extra work on the Mech path. A validated general
strided/line-layout view could remove these passes; it is not implemented here.

## Validation and protocol

`input-pins.json` records the official N=1000 input/output hashes. The 254,166,745
byte full input is the exact existing pinned FASTA N=25,000,000 input from
k-nucleotide. A separate Python implementation derived from the specification
produces the full reference. Every included build must pass both official small
and full byte-for-byte references and every timed output hash. All three
sequences, all 250 million bases, every header and the final partial lines are
processed.

`manifest.json` pins all 30 listed upstream submissions for C, C++, Rust,
Chapel, Julia, Go and native-AOT Java, including starred and unmeasured entries.
Unchanged sources are attempted. C/C++ use native GCC and LLVM builds; Rust
uses native optimization with ordinary and panic-abort builds. Conditional x86
Rust code with a portable fallback is included. Architecture/build/output
exclusions remain explicit. Local Java runs use fresh-process OpenJDK **JIT**,
not the published native AOT toolchain.

Timing is shuffled and sequential, from fresh process launch to exit, reading
one regular stdin file and writing complete output to a regular file. It
includes IO, runtime/CLI and JIT startup. Builds, tests, reference generation,
hashing and plotting finish outside timing. Worker environment limits are
recorded; source-created pthread/std::thread policies may have their own limits.
Desktop timing and endpoint selection remain exploratory.

The first measurements used Python's timeout wait with regular output files,
which polls and can add roughly 50 ms to short processes. Those raw reports
remain historical evidence of source coverage and correctness. Current results
use blocking child completion with a POSIX alarm deadline; successful exit,
nonzero exit, deadline termination/reaping and signal cleanup are independently
checked in [process-wait-validation.json](results/process-wait-validation.json).

All 30 upstream source submissions were attempted, yielding 46 runnable builds
including Mech policy variants and four architecture exclusions. The corrected
competitive cohort contains 42 builds: four C++ builds already measured at
22–28 seconds, each dominated in both size metrics and time by another included
C++ build, are explicitly skipped. All fastest/smallest endpoint candidates
remain. The [final report](results/apple-m3-final-25000000.json) combines two
separately labeled sequential batches: five samples per competitive build,
then seven for the current Mech policies and fastest C/Rust controls. Each row
records its measurement batch and sample count. The matching compiler source
archives remain attached to the original raw reports; this is not one combined
42-program run. Near-tied Rust build policies can exchange positions between
these batches without establishing a performance difference.

`verify.py` independently checks all byte values, unrelated constant and dynamic
maps, NUL outputs, error/identity policies, alias preservation, empty/odd/vector
and large partition tails, generic markers/joining, CRLF, empty records,
wrapping boundaries and rejected callback signatures/IO. `audit.py` applies the
repository unsafe-boundary checker to production sources, allowlisted fixtures
and new benchmark fixtures; generated projects and unrelated upstream imports
are excluded. Previous benchmark programs and executables are preserved.

The [general byte/record suite](results/byte-record-conformance.json) passes
**12,356 independent checks** across scalar/SIMD, fused/unfused and shared/copied
paths. [Two additional compiled compositions](results/byte-domain-composition.json)
verify scalar and SIMD reversal/substitution feeding the earlier packed-window
and histogram operations. Reversal preserves the input encoding domain;
substitution infers its output domain from its table. This metadata is needed
for the previous sequence optimizations to remain reusable. The scoped
[unsafe-boundary audit](results/unsafe-audit.txt) and 19 existing native nbody
semantic tests also pass.

Previous programs pass their official small and full output checks when rebuilt.
The [Mandelbrot/k-nucleotide regression comparison](results/runtime-regression.json)
puts paired median changes between −3.6% and +2.0%. All three Mandelbrot native
objects are identical to their preserved versions. The nbody and two spectral
norm native objects are also identical. The initial eight-worker
[nbody/spectral comparison](results/nbody-spectral-regression.json) has substantial
desktop scheduling variance, with spectral medians increasing 3.5% and 5.5%.
A [controlled one-worker repeat](results/nbody-spectral-regression-one-worker.json)
puts all three old/current medians within 0.03%. This finds no clear compiler or
runtime regression; the noisy parallel data remains visible.

## Reproduction

From the worktree root:

```sh
RUSTFLAGS='-D warnings -C target-cpu=native' \
CARGO_TARGET_DIR=benchmarks/clbg/nbody/mech-driver/target \
cargo build --release --locked --offline --features cranelift_build
python3 benchmarks/clbg/revcomp/run.py --prepare-only
python3 benchmarks/clbg/revcomp/verify.py
python3 benchmarks/clbg/revcomp/verify_composition.py
python3 benchmarks/clbg/revcomp/audit.py
python3 benchmarks/clbg/revcomp/run.py --reuse-builds --samples 5 \
  --output benchmarks/clbg/revcomp/results/reproduced-25000000.json
python3 benchmarks/clbg/revcomp/plot.py \
  benchmarks/clbg/revcomp/results/reproduced-25000000.json
```

`MECH_BYTE_FUSION=0` and `MECH_BYTE_REUSE=0` are build-time diagnostic policies;
`--scalar` disables byte SIMD and native vector wrapping. Partition count is a
runtime policy. Compiler cost decisions are not automatically tuned: eligibility
is general, but profitability must be established by recorded measurements.

Primary sources: [benchmark specification](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/revcomp.html),
[upstream table](https://benchmarksgame-team.pages.debian.net/benchmarksgame/performance/revcomp.html),
[C #9](https://benchmarksgame-team.pages.debian.net/benchmarksgame/program/revcomp-gcc-9.html),
[Rust #1](https://benchmarksgame-team.pages.debian.net/benchmarksgame/program/revcomp-rust-1.html).
