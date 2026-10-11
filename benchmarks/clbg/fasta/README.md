# FASTA: complete Mech sources and native execution

This is the seventh local Benchmarks Game study for the experimental v0.4
integration. Both programs contain their full generation and selection algorithm
in Mech. Cranelift emits the arithmetic, comparisons, source function calls and
native scan loops. Rust supplies general bounded IO, state storage and CLI
support; there is no host FASTA generator or nucleotide-selection kernel.

| Source | Noncomment LOC | Normalized gzip-1 bytes | Latest default median |
| --- | ---: | ---: | ---: |
| [Compact binary search](fasta.mec) | 33 | 827 | 1.823 s |
| [Expanded linear comparison count](fasta-optimized.mec) | 44 | 834 | 0.964 s |

The expanded program is 1.89 times faster with eleven additional source lines.
It is a line between two sources on the corner chart. Its `Count` helper is
written entirely in Mech. All fourteen IUB and three human threshold comparisons
are visible in the counted source. Compiler policies of one, four and eight
iterations per native scan loop are effectively tied in the final measurements.

## Required work and validation

At N = 25,000,000, each run produces 250,000,000 bases: 2N copies of the ALU
sequence, 3N IUB bases and 5N human bases. The random records share the prescribed
LCG state, beginning at seed 42. Every random base advances the generator and
uses direct f64 division and cumulative probabilities. Headers and 60-column
line wrapping are included.

All 59 builds in the latest cohort match the complete 254,166,745-byte reference output.
The small reference is the official N = 1000 output; the full reference comes
from the independently compiled C #1 generator already used for k-nucleotide.
Hashes, sizes and reference provenance are in [reference-pins.json](reference-pins.json).
The full SHA-256 is
`3fcf4f78104c8a65ef210fe1d469f4e473456c791225f2f1f9114f4986aa09fa`.

## Reusable language and compiler support

The new operations are `number/to-f64`, `bytes/from-text`, literal
`sequence/f64-values`, ordered `sequence/cumulative-sum`, readonly-column
`sequence/length` and `sequence/get`, literal `io/write`, and
`io/write-scan(count, state, sourceStep, width, separator, bufferSize)`.
The scan accepts up to 32 scalar integer, floating or Boolean state fields;
the source callback returns the next state and one byte. Buffering and line
wrapping are generic IO operations. The generator and search stay in the
native callback emitted from source. Source callbacks must be closed and pure.

The compiler works from typed graph operations and proven value domains:

- Literal indexed floating reads fold to their exact bit patterns.
- Independent Boolean floating comparisons followed by integer sums can pack
  into f64 SIMD comparisons. IEEE predicates and integer wrapping are preserved.
- Scan unrolling preserves state order and uses a scalar tail. The automatic
  policy avoids expanding callbacks with nested loops, where measurements show
  that expansion hurts the compact search.
- Inductive scan-state bounds and no-overflow proofs allow exact constant
  unsigned division through a low-product reciprocal and one shift.
- Readonly array checks disappear only when the index is positive and an exact
  array length proves its upper bound. Unknown or invalid indices retain guards.

These rules do not inspect benchmark names or recognize a particular RNG.
The reciprocal is derived from the divisor and proven dividend range. Its
correctness condition prevents crossing a quotient boundary, and the compiler
also proves that the widened product fits in 64 bits.

## October 8 streaming optimizations

The [five-sample matched confirmation](results/apple-m3-streaming-final-25000000.json)
measures expanded Mech at **0.964 s** and Rust #7 thin LTO at **0.963 s**,
within 0.2%. The preserved original expanded Mech measures 1.237 s in that same
batch: **22.0% less elapsed time**. Compact Mech falls from 3.010 s to **1.823 s**,
**39.4% less elapsed time**. C #2 LLVM measures 1.379 s. Both Mech sources are
unchanged, and every measured run matches the complete reference output.

Rust #7 preformats a cycle of the repeated ALU record and overlaps state
generation, byte mapping, line formatting and output using two workers.
Mech now supports those techniques through source proofs and generic storage:

- A one-state literal-table scan with `next = (state + 1) % tableLength` and
  lookup at `state + 1` proves an exact cycle. The emitted source callback
  generates that cycle once. Bounded IO builds a reusable block aligned to
  both the period and line width, then replays it and preserves final state.
- A one-integer-state scan whose byte depends only on the next state can split
  into two emitted native callbacks. A producer fills range-proved u32 state
  samples; a mapping worker selects bytes, wraps lines and writes in order.
  Two bounded buffers overlap these stages. The output session moves exclusively
  to the worker and is restored after joining, before later source effects.
- Errors re-evaluate the original pure callback for the failed block, preserving
  source error order and the already-written prefix. Unsupported shapes, small
  buffers, short scans and trivial mappings retain the ordinary scan.

The [cycle-replay study](results/apple-m3-periodic-matched-25000000.json)
reduces expanded elapsed time from 1.122 s to 1.039 s. The
[first pipeline study](results/apple-m3-pipeline-matched-25000000.json)
helps compact search but leaves expanded Mech near 1.04 s when formatting and
writing remain on the producer. Moving both into the mapping stage reaches
0.964 s in the [output-worker study](results/apple-m3-pipeline-output-worker-matched-25000000.json)
and the five-sample automatic-policy confirmation. Disabling the pipeline in
the final batch measures 1.045 s. These ablations distinguish cycle replay
from actual overlap of generation and IO.

The final compiler passes **6,181 checks**:
[1,567 existing checks](results/conformance-streaming-final-20261008.json),
[1,801 exact-comparison checks](results/conformance-exact-streaming-final-20261008.json),
[2,023 cycle checks](results/conformance-periodic-final-20261008.json), and
[790 pipeline checks](results/conformance-pipeline-final-20261008.json).
Coverage includes full FASTA boundary outputs, offsets, periods, final states,
multi-byte separators, block tails, arithmetic limits, fallback selection,
ordered records and original error status/prefix. The unsafe-boundary audit
passes. The automatic native objects are identical to the measured forced
pipeline policy; descriptors record the selected callbacks.

`MECH_CONTROL_SCAN_CACHE=0` disables cycle replay.
`MECH_CONTROL_SCAN_PIPELINE=0` disables splitting; `1` forces eligible mappings;
the default `auto` skips trivial mappings. Runtime scheduling also retains the
serial path for buffers below 1,024 bytes and scans below the greater of
131,072 bytes and two buffers. No rule recognizes FASTA, its alphabet or its RNG.

After building the compiler, run validation before timing:

```sh
python3 benchmarks/clbg/fasta/conformance.py --output benchmarks/clbg/fasta/results/local-conformance.json
python3 benchmarks/clbg/fasta/exact_compare_conformance.py --output benchmarks/clbg/fasta/results/local-exact.json
python3 benchmarks/clbg/fasta/periodic_scan_conformance.py --output benchmarks/clbg/fasta/results/local-periodic.json
python3 benchmarks/clbg/fasta/pipeline_scan_conformance.py --output benchmarks/clbg/fasta/results/local-pipeline.json
python3 benchmarks/clbg/fasta/optimize.py --stage final --prepare-only
python3 benchmarks/clbg/fasta/optimize.py --stage final --reuse-builds --samples 5 --output benchmarks/clbg/fasta/results/local-streaming.json
```

## Earlier October 8 arithmetic optimizations

The earlier five-sample matched run measured the compact source at **2.279 s**, down
from **3.006 s** (24.2% less elapsed time), and the expanded source at
**1.120 s**, down from **1.236 s** (9.4% less elapsed time). Rust #7 thin LTO
measures 0.962 s in the same batch, leaving the expanded source about 16.4%
slower. C #2 LLVM measures 1.375 s. Fixed unroll policies and the automatic
policy are closely grouped around 1.12 s; small differences are measurement
variation. The matched run uses the automatic compiler policy.

Both Mech sources are unchanged. The compiler now lowers an ordered comparison
of a bounded unsigned integer converted to f64 and divided by a positive finite
constant into an exact integer comparison. It finds the boundary by evaluating
the correctly rounded division over the proven integer domain, rather than
multiplying and rounding the threshold. Conversion must be exact (at most
2^53); unsupported domains, denominators and NaN thresholds retain floating
operations. Literal indexed thresholds use a derived boundary table with the
original index checks. This is exact strength reduction of source operations,
not an alternative source generator or nucleotide lookup algorithm.

Counts from these integer comparisons pack into four SIMD lanes, with pairwise reduction and
masked tails. Mixed floating predicates and wrapping integer sums retain their
semantics. Unchanged loop fields keep their initial values and literal-array
facts. Bounded quotient/remainder operations needing at most one subtraction
use a comparison and select; this especially helps the repeated ALU sequence.

The [policy study](results/apple-m3-exact-compare-policies-25000000.json) and
[five-sample default comparison](results/apple-m3-exact-compare-matched-25000000.json)
preserve the earlier executables and measure old Mech, new Mech, C #2 LLVM and
Rust #7 thin LTO in randomized sequential fresh processes. Complete output and
regular-file IO remain inside timing. Compilation, hashing and validation stay
outside timing. The
[new combined report](results/apple-m3-optimized-20261008-25000000.json) retains
the foreign-language results and normalizes each batch to its own C control.
The October 7 report and its charts remain available unchanged.

The expanded-source ablation with exact comparisons disabled measures 1.123 s,
effectively tied with the enabled policies. Most of the expanded-source gain
therefore comes from simpler bounded index arithmetic; it should not be
attributed entirely to replacing floating comparisons. The compact source also
benefits from retaining loop-invariant array facts and deriving indexed exact
comparison boundaries.

That arithmetic-only compiler stage passed
[1,801 targeted comparison checks](results/conformance-exact-20261008.json) and
[1,567 existing conformance checks](results/conformance-optimized-20261008.json).
Coverage includes rounding boundaries, unsigned limits, NaNs, infinities,
subnormal divisors, scalar fallbacks, mixed SIMD counts, tails and wrapping sums.
Indexed thresholds are checked over all 139,968 input values. Both previous
fannkuch variants retain identical native objects and pass their small and full
outputs in the [backend regression check](results/regressions-exact-20261008.json).
The scoped unsafe-boundary audit passes.

Use `MECH_CONTROL_EXACT_FLOAT_COMPARE=0` to disable comparison strength reduction
for a measured ablation. Reproduce the new checks and measurements with:

```sh
python3 benchmarks/clbg/fasta/exact_compare_conformance.py
python3 benchmarks/clbg/fasta/conformance.py --output benchmarks/clbg/fasta/results/conformance-optimized-20261008.json
python3 benchmarks/clbg/fasta/optimize.py --prepare-only
python3 benchmarks/clbg/fasta/optimize.py --reuse-builds --samples 3 --output benchmarks/clbg/fasta/results/apple-m3-exact-compare-policies-25000000.json
python3 benchmarks/clbg/fasta/optimize.py --reuse-builds --samples 5 --only mech-fasta-compact-before,mech-fasta-optimized-before,fasta-rust-7-thin-control,fasta-gcc-2-llvm-control,mech-fasta-exact-compact-auto-v2,mech-fasta-exact-expanded-auto-v2
python3 benchmarks/clbg/fasta/merge_optimized.py --stage exact
python3 benchmarks/clbg/fasta/plot.py benchmarks/clbg/fasta/results/apple-m3-optimized-20261008-25000000.json
```

## Why indiscriminate 32-bit narrowing loses

On this AArch64 backend, narrow arithmetic often acquires an explicit extension
when its result rejoins a u64 graph. Cranelift's original I32 high-multiply
remainder recipe also used two extensions and two dependent shifts. LLVM's C
version uses `umull`, one shift and `msub`.

The new exact reciprocal lowering removes the extra shift. Keeping values in
their existing registers avoids the extensions. Matched five-sample results
with the same array-check improvements are:

| Compiler policy, optimized source, unroll 8 | Median |
| --- | ---: |
| Wide arithmetic and original wide remainder | 1.235 s |
| Full range-proved 32-bit narrowing, improved remainder | 1.378 s |
| Selective narrowing, improved remainder | 1.237 s |
| C #2, LLVM | 1.375 s |

See [the matched width study](results/apple-m3-bounds-25000000.json). On AArch64,
selective narrowing is the automatic scan policy. Source u64 semantics remain
intact. `MECH_CONTROL_NARROW_UINT=0` disables narrowing;
`MECH_CONTROL_NARROW_UINT=1` enables the full experiment;
`MECH_CONTROL_NARROW_ARITHMETIC=0` retains wide arithmetic in that experiment.
Other targets retain the previous default pending measurements.

The October 7 matched default run measured Mech at 1.238 s and C #2 LLVM at 1.379 s,
about 10% less elapsed time for Mech. The initial expanded source measured
1.685 s. Bounds-check elimination accounts for most of the later improvement;
it should not be attributed entirely to integer width.

## Comparison cohorts and charts

[The current description](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/fasta.html)
prescribes cumulative probability lookup and restricts alternative generator
and lookup algorithms. [algorithm-audit.json](algorithm-audit.json) records a
conservative source-arithmetic cohort separately from the historical listings.
Both retain their measured full-output results.

The latest strict chart contains 18 builds. The full historical chart includes scaled
integer thresholds, floating reciprocal substitutions, lookup tables,
period reuse and noncumulative selection, each labeled in the audit. In
particular, the fastest listed Rust #7 uses scaled thresholds and measures
0.963 s in the latest matched run; the fastest admitted strict Rust source measures about 3.62 s.
The Mech-versus-C result above refers to the strict source cohort.

The final report combines the 56 foreign-language builds from the broad run
with three current Mech policies. Each measurement batch is normalized to its own
C #2 LLVM median; raw elapsed seconds and original reports remain available.

- [Latest combined report](results/apple-m3-streaming-20261008-25000000.json)
- [Strict LOC chart](results/apple-m3-streaming-20261008-25000000.corner.loc.png)
- [Strict compressed-source chart](results/apple-m3-streaming-20261008-25000000.corner.png)
- [Historical LOC chart](results/apple-m3-streaming-20261008-25000000.corner.historical.loc.png)
- [Historical compressed-source chart](results/apple-m3-streaming-20261008-25000000.corner.historical.png)
- [Strict endpoint table](results/apple-m3-streaming-20261008-25000000.corner.loc.table.md)
- [Preserved arithmetic-stage report](results/apple-m3-optimized-20261008-25000000.json)
- [Preserved October 7 combined report](results/apple-m3-final-25000000.json)
- [Seven-benchmark overview exports](../overview/mega-endpoints.json)

The pinned manifest contains 42 submissions. Ten build configurations are
explicitly excluded: C++ #9 needs unavailable Boost; C++ #7 does not compile;
C++ #4 needs Linux `fwrite_unlocked`; C #4 has malformed pinned source; and
Java #5/#6 fail the official small output. C/C++ exclusions apply to both GCC
and LLVM. Original diagnostics are retained. Sources and authors' comments
remain unmodified, and upstream licensing is preserved.

## Protocol, checks and reproduction

Measurements use an Apple M3, randomized sequential fresh processes, complete
output to a regular file, and three or five samples per build. Process startup,
CLI work, IO and JIT startup are included. Compilation, output hashing, tests
and rendering run outside timing. Eight workers are requested; source limits
vary, Chapel caps at four, and eligible Mech scans use two pipeline stages. Java runs
OpenJDK JIT rather than the site's native AOT configuration. These are local
exploratory results, with close differences treated as measurement variation.

The October 7 compiler passed [1,567 conformance checks](results/conformance.json),
including unsigned quotient/remainder limits, typed state, NaNs, infinities,
mixed SIMD predicates, block tails, output order, invalid indices, overflowing
indices and callback purity. The scoped unsafe-boundary audit passes.

[Eleven previous benchmark variants](results/regressions-final.json) produced
identical native objects and full outputs. A
[seven-sample focused recheck](results/regressions-focused.json) resolves the
initial timing outliers: spectral norm is within about 2%, and compact
fannkuch is essentially unchanged. No reproducible slowdown was identified.

Each raw report includes compiler-source archives, overlay hashes, native
object and executable hashes, build commands, environments and generated
project files. Before the final policy rebuild, existing Mech executables and
objects were preserved by hash in `.build/preserved/`; the later width
experiments have separate reports and binaries.

Build the compiler from this workspace:

```sh
RUSTFLAGS='-D warnings -C target-cpu=native' \
CARGO_TARGET_DIR=benchmarks/clbg/nbody/mech-driver/target \
cargo build --release --locked --offline --features cranelift_build
```

Then build pinned Rust dependencies, prepare competitors with `run.py
--prepare-only`, and prepare all Mech policies with `prepare_mech.py`. The
Rust dependency build requires `RUSTFLAGS='-Cembed-bitcode=yes'` for thin LTO.
Tool paths, exact flags and dependency locks are recorded in the scripts and
reports. `fetch_sources.py` refuses to overwrite an existing manifest.

```sh
python3 benchmarks/clbg/fasta/conformance.py
python3 benchmarks/clbg/fasta/audit.py
python3 benchmarks/clbg/fasta/run.py --reuse-builds --samples 5 \
  --output benchmarks/clbg/fasta/results/local-recheck.json
python3 benchmarks/clbg/fasta/merge_optimized.py
python3 benchmarks/clbg/fasta/plot.py benchmarks/clbg/fasta/results/apple-m3-streaming-20261008-25000000.json
python3 benchmarks/clbg/overview/build_gallery.py
python3 benchmarks/clbg/overview/plot_mega.py
```

Plotting requires matplotlib; the measurements themselves use Python's standard
library. Run timing alone. The original reports are preserved rather than
replaced by newly observed medians.
