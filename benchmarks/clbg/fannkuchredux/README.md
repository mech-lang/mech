# Fannkuch-redux — v0.4 integration, experimental Cranelift AOT

The complete Mech algorithm lives in [`fannkuchredux.mec`](fannkuchredux.mec),
43 nonblank, noncomment lines. It defines factoradic initialization of any
starting permutation, incremental permutation generation, prefix flipping,
the alternating signed checksum and the maximum flip count. It processes all
permutations in the prescribed order within independently initialized ranges.
There is no fannkuch function in the Rust host or benchmark recognition in the
compiler. This is an experimental compiled subset of canonical Mech source;
these additions do not imply equivalent support in every resident engine.

The [specification](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/fannkuchredux.html)
requires all N! permutations, with output for N=12:

```text
3968050
Pfannkuchen(12) = 65
```

The measured workload contains **479,001,600 permutations**. Every admitted
implementation must match the official N=7 output and the full N=12 checksum
and maximum. Every timed result is checked again outside the timer.

## Measured results

The final comparison retains the three-sample broad sweep and a separate five-sample matched batch. It does not pool samples across batches. All results are local Apple M3 measurements for N=12, including process startup and full output.

| Language / implementation | Median elapsed | Counted LOC | Normalized gzip bytes |
| --- | ---: | ---: | ---: |
| Mech, even paired source | 2.736 s | 64 | 1205 |
| Rust #6, labeled NEON port + LTO | 2.461 s | 135 | 1250 |
| C #5, GCC | 5.106 s | 86 | 900 |
| C++ #1, GCC | 5.839 s | 121 | 1052 |
| Julia #2 | 4.744 s | 129 | 1057 |
| Go #2 | 4.578 s | 127 | 892 |
| Java #1, OpenJDK JIT | 5.817 s | 150 | 1278 |
| Chapel #2 | 7.209 s | 91 | 726 |

The compact Mech source takes **3.874 s at 43 LOC / 853 compressed bytes**, essentially tied with unmodified portable Rust #4 + LTO at 3.881 s (95 LOC / 1,010 bytes). The 64-line optimized Mech source takes about 29% less time than compact Mech. It takes about 11% more time than the SIMD Rust port.

Mech has a **line rather than a dot**: its fastest and smallest sources differ. It has the lowest LOC in this cohort; other languages have smaller compressed minimum sources. The [compressed-source chart](results/apple-m3-final-12.corner.png), [LOC chart](results/apple-m3-final-12.corner.loc.png), and their endpoint tables show both definitions of size. A dot for Chapel reflects one admitted source, rather than proof that no alternative tradeoff exists.

The [final paired raw measurements](results/final-paired-12.json) include both Rust SIMD translation policies, all portable Rust LTO controls, and the local fastest C/C++ candidates. The [combined report](results/apple-m3-final-12.json) identifies each row’s original batch, sample count and compiler snapshot. The unchanged x86 Rust #6 remains excluded; its [NEON translation](comparisons/fannkuchredux-rust6-neon.rs) is explicitly marked as modified source.

## Source variants and compiler policy

| Source / policy | LOC | Normalized gzip level 1 | Purpose |
| --- | ---: | ---: | --- |
| `fannkuchredux.mec`, SIMD | 43 | 853 bytes | Compact source, 4,096 permutations per range |
| Same source, scalar permutation lowering | 43 | 853 bytes | Compiler ablation |
| `fannkuchredux-coarse.mec` | 45 | 878 bytes | Approximately 128 ranges |
| `fannkuchredux-paired.mec` | 64 | 1,190 bytes | Two independent flip chains in one source loop |
| `fannkuchredux-cached.mec` | 64 | 1,218 bytes | Paired chains with scalar first-element state |
| `fannkuchredux-even.mec` | 64 | 1,205 bytes | Even starting indices simplify paired checksum |
| `fannkuchredux-aligned.mec` | 64 | 1,213 bytes | Approximately N × (N−1) factorial-aligned ranges |

The paired source generates consecutive permutations and executes their flip
chains together. A completed chain performs an identity prefix reversal while
the other chain finishes, and its flip count stops increasing. A lazy branch
handles an unpaired tail. The cached source loads each next first element
before reversing that permutation. The even-pair source removes the sign
selection because the range grain is even, all range starts are even, and
each pair advances the index by two. N=1 follows the single-permutation tail.
The aligned-range experiment uses N × (N−1) groups, with a checked nonzero
denominator and even grain; it did not improve the final median. These are explicit algorithm changes in
the counted Mech source, rather than hidden compiler substitutions.

The first measured executables and source archive are preserved in
`results/mech-initial-12.json`; the intermediate paired pilot is preserved in
`results/pilot-12.json`. The initial fixture predates signed formatting and
serves only as the historical N=7/N=12 baseline. Current sources correctly
print negative checksums as well, including N=2.

## General language and compiler pathways

- Closed typed Mech functions can return multiple values, including fixed
  byte columns; tuple definitions/destructuring carry those values through loops.
- `control/while(initial, predicate, step)` compiles pure source callbacks into
  native loop blocks. `control/if(condition, yes, no, state)` compiles a lazy
  branch; `control/select(condition, a, b)` selects already evaluated values.
- `sequence/iota-bytes(length)` and `sequence/zero-bytes(length)` construct
  fixed byte columns. `sequence/get` and `sequence/set` use checked one-based
  indices; set produces an immutable updated value and checks byte range.
- `sequence/reverse-prefix(array, length)` and
  `sequence/rotate-prefix(array, length, distance)` preserve the suffix.
  Prefix length zero is valid; rotation distance is reduced modulo nonempty
  prefix length. The admitted fixed array shapes span 1 through 64 bytes.
- `number/factorial` checks the unsigned 64-bit domain (0 through 20),
  `number/bounded` checks an explicit inclusive range, and `number/mod` and
  division check for a zero divisor. Ordinary unsigned arithmetic wraps.
- `parallel/reduce-ranges(start, end, grain, sourceFunction, arguments, reducers)`
  schedules half-open ranges and combines scalar outputs using general sum/max
  reductions. Captures, recursion, callback IO and nested scheduling are rejected
  by this subset. All source callback computation is native machine code.
- `io/argument-u64`, `text/format` and `io/write` provide general process IO.
  Formatting supports unsigned `{}` and signed interpretation `{:i64}`.

The compiler lowers fixed byte values into SSA SIMD registers. Prefix operations
on shapes up to 16 bytes use lookup masks generated from **every legal prefix
length and rotation distance for that shape**. Rotation by a constant one has a
smaller table. Larger shapes and the scalar ablation use checked native stack
copies and element loops. Constant element extraction uses a lane operation. Dynamic scalar byte reads
on ARM use a private vector store and indexed byte load; this shortens the
dependent instruction chain compared with vector lookup followed by a lane
transfer. Other architectures retain vector lookup. The general build policy
`MECH_CONTROL_BYTE_INDEX=auto|vector|stack` permits explicit ablation, and the
source-derived descriptor records the chosen lowering. This decision depends
on scalar result type, dynamic index and target architecture; it does not inspect
benchmark names or inputs. No permutation or flip loop calls a
Rust computation kernel, allocates a heap array or dispatches an interpreter.

Bounds analysis propagates conservative value domains through source operations,
function calls, tuple fields and loops. Iota establishes a byte range; reversal
and rotation preserve it; checked updates may widen it. Loop analysis widens
increasing byte domains to 255 and increasing integer domains to unknown.
Removing a get/set guard additionally requires a strictly positive index proof.
Addition is considered positive only when the operand bounds prove it cannot
wrap. Branch-local facts are restored before compiling a sibling branch.
Unproven cases retain checks. These decisions use types, operations and constants,
with no benchmark name or workload-size recognition.

Source functions are inlined with expression reuse within each scope, so reading
multiple tuple fields does not repeat their producer. Power-of-two division and
remainder lower to shifts/masks. The [emitted optimized assembly](results/native-even.asm)
shows ARM SIMD table permutations (`tbl`) and lane extraction (`umov`) in the
native callback. The only host import is the general range scheduler.

The generic Rust executable owns argument/result storage, Rayon scheduling,
sum/max combination and output formatting. LLVM builds that support executable;
**Cranelift compiles the permutation generation, flip loops and checksum work
expressed by Mech.** Support code is excluded from LOC just as other languages'
standard libraries and runtimes are excluded. The canonical parser, public AOT
build path and native scheduling infrastructure are shared with earlier work;
the fixed byte/control IR adds a reusable path for small stateful native loops.

## Techniques from other implementations

Portable Rust #4 uses Rayon ranges, factoradic starts, incremental generation,
and a flip loop that carries the first element separately while reversing only
the interior that still matters. The x86 Rust #6 submission packs a permutation
into SIMD bytes, uses shuffle tables, preloads its next first element and splits
independent work. C's SIMD submissions also interleave independent flip chains.
Mech adopts small SIMD values, prefix masks, explicit independent chains and
the scalar-head experiment through general operations. The x86 submissions
themselves are excluded on this ARM machine; their published timings are not
substituted for local measurements.

## Earlier benchmark regression checks

Fresh public AOT builds reproduce the complete nbody, Mandelbrot, k-nucleotide,
spectral-norm and reverse-complement outputs. Their emitted computation objects
are byte-for-byte identical to the latest preserved baselines. All policies use
identical source and runtime settings in the paired comparisons.

| Earlier benchmark / policy | Median time change |
| --- | ---: |
| Spectral norm, floating SIMD | +2.86% |
| nbody | +0.05% |
| Mandelbrot, compact | -0.46% |
| Mandelbrot, SIMD | +0.62% |
| Mandelbrot, SIMD + FMA | +1.19% |
| k-nucleotide, compact | -3.44% |
| k-nucleotide, parallel | -0.61% |
| Reverse complement | +0.70% |

The short eight-worker integer spectral-norm run initially differed by +22.2%,
with substantial overlapping sample ranges. A separate nine-sample recheck
reports +3.5% with eight workers and +0.03% with one worker. These observations,
together with identical computation objects, point to scheduling variability;
they do not establish exact multiworker timing parity. The original and repeated
measurements are retained in [the spectral recheck](results/spectral-drift-recheck.json).
Other comparisons remain between −3.4% and +2.9%.

Raw results: [nbody/spectral norm](results/nbody-spectral-regression.json),
[Mandelbrot/k-nucleotide](results/runtime-regression.json), and
[reverse-complement](results/revcomp-regression.json). Preparation completes before
all shuffled, sequential timings, with output validation excluded from timers.

## Validation and reproducibility

[`results/control-conformance.json`](results/control-conformance.json) records
2,544 passing checks against independent byte/range/permutation references:
scalar and SIMD shapes 1, 2, 15, 16, 17, 32, 33 and 64; every legal prefix length;
all 256 byte values; huge rotation distances; aliases; invalid indices/values;
branch scope and wrapped-index adversaries; unrelated range reductions with
different partition sizes and 1/3/8 workers. All eleven measured source/policy variants
also pass an independent recursive permutation generator for N=1 through 8;
[`results/source-conformance.json`](results/source-conformance.json) includes
admission rejection checks. Forty additional [constant-index checks](results/constant-conformance.json)
cover invalid literals and untaken lazy branches. Both vector and indexed-stack
lowering pass the 2,544 general checks independently. The labeled ARM Rust port
also passes 16 independent small-input checks. The scoped unsafe-boundary audit passes.

`manifest.json` pins all 30 listed submissions for C, C++, Rust, Chapel, Julia,
Go and Java, including starred entries. Sources and licenses are preserved.
C/C++ are attempted with GCC and LLVM native optimization and LTO; Rust uses
native optimization, one codegen unit, ordinary/panic-abort builds and
separate fat-LTO controls. LTO dependencies are rebuilt with LLVM bitcode
embedded; their pinned dependency manifest and lockfile are archived. This
machine cannot run unmodified x86-intrinsic sources. Boost-dependent C++ #2/#4
builds are excluded because their dependency is unavailable. Java is measured
as fresh-process OpenJDK JIT, rather than the published native AOT toolchain.
[`toolchains.json`](toolchains.json) records the exact installed versions.

Eight workers are requested through the applicable runtime settings; individual
sources may fix their own thread count. Chapel's runtime caps its request at
four workers (the [warning](results/chapel-worker-warning.txt) is preserved).
Julia runs with `--threads=8,0`: disabling its new interactive pool retains
the thread IDs assumed by the unchanged upstream sources.

Timing is a shuffled, sequential sweep of fresh processes from launch to exit,
with full output written to a regular file. It includes scheduling, CLI,
formatting and JIT startup, and excludes building, tests and output verification.
The blocking wait uses a POSIX deadline without timeout polling quantization.
Reports preserve each sample, command, source size/hash, native object hash,
build policy and a matching compiler-source archive. Endpoint selection uses
observed medians; near ties are exploratory desktop results, not a language
ranking or a paper-quality performance claim. LOC and compressed source size
are reported separately because formatting can change LOC substantially.

To prepare all current candidates and produce a new comparison report without
overwriting the preserved measurement batches:

```sh
RUSTFLAGS='-D warnings -C target-cpu=native' \
CARGO_TARGET_DIR=benchmarks/clbg/nbody/mech-driver/target \
cargo build --release --locked --offline --features cranelift_build

python3 benchmarks/clbg/fannkuchredux/fetch_sources.py
python3 benchmarks/clbg/fannkuchredux/run.py --prepare-only
python3 benchmarks/clbg/fannkuchredux/complete_preparation.py
MECH_CONTROL_BYTE_INDEX=vector python3 benchmarks/clbg/fannkuchredux/verify.py
MECH_CONTROL_BYTE_INDEX=stack python3 benchmarks/clbg/fannkuchredux/verify.py
python3 benchmarks/clbg/fannkuchredux/prepare_comparisons.py
python3 benchmarks/clbg/fannkuchredux/prepare_indexing.py
python3 benchmarks/clbg/fannkuchredux/prepare_final.py
python3 benchmarks/clbg/fannkuchredux/verify_extra.py
python3 benchmarks/clbg/fannkuchredux/verify_constants.py
python3 benchmarks/clbg/fannkuchredux/audit.py
python3 benchmarks/clbg/fannkuchredux/run.py --reuse-builds --samples 5 \
  --output benchmarks/clbg/fannkuchredux/results/new-comparison-12.json
python3 benchmarks/clbg/fannkuchredux/plot.py \
  benchmarks/clbg/fannkuchredux/results/new-comparison-12.json
```

Regression scripts are `extra_regressions.py`, the existing
`spectralnorm/regressions.py` with a fannkuch-local `--build-dir`,
`use_latest_baseline.py`, `revcomp_regression.py`, and `recheck_spectral.py`.
Use their preparation modes first, then `--reuse-builds` during timing. The
final chart report is built by `finalize.py` from its explicitly named raw
batches. Rebuilding alone does not reproduce the exact stored desktop samples.
