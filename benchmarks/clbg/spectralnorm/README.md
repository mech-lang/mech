# Spectral norm: complete compiled Mech source

The current strict-comparison source is
[spectralnorm-faithful.mec](spectralnorm-faithful.mec): **12 nonblank,
noncomment LOC**, including four separate functions (`coefficient`, `apply`,
`apply-transpose`, and `step`). The previous 9-line application rewrite
omitted the required source structure and is retired from the comparison.
Historical sources and reports remain available for provenance.

The program uses ordinary matrix comprehensions, multiplication, transpose,
and function application. Its typed integer coefficient arithmetic is
converted explicitly to `f64`. It uses the public
`mech build --backend cranelift` path on the v0.4 integration worktree.
LOC includes the complete Mech program and excludes general libraries,
the compiler, and the generic CLI, consistently with the other languages.

The [required-work audit](../work-audit/20261009/audit.json) and
[five-run timing report](../work-audit/20261009/timings.json) record a full-size
median of **0.088515 seconds**, with sample standard deviation **0.003140
seconds**. All small and full output checks passed. General scalar helper calls
inside matrix comprehensions are now supported; 51 native-sequence tests pass,
and 13 unchanged benchmark sources emit identical native compute objects.

The [official specification](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/spectralnorm.html)
requires the same algorithm and four separate functions, an N=100 output check,
and N=5500 for performance. Mech applies AtA 19 times to obtain v, then once
more to obtain u: exactly the same 20 successive power-method applications as
ten pairs in the reference. Every run therefore performs 40 matrix-vector
products and **1,210,000,000 weighted contributions**. It stores no matrix,
uses no answer shortcut, and does not reduce the iteration count.

## General language and compiler pathways

The engine's canonical-source admission now supports closed, typed source
functions as arguments to native array operations:

| Operation | Semantics |
|---|---|
| `sequence/fill(n, x)` | Construct an f64 vector. |
| `sequence/length(v)` | Return its length. |
| `A ** v` | Multiply a shaped coefficient-producing matrix by a vector. |
| `A' ** v` | Transpose that matrix and multiply by a vector. |
| `sequence/iterate(seed, count, f)` | Apply a pure Mech-defined function repeatedly. |
| `stats/dot(a,b)` | Sum corresponding element products. |
| `math/sqrt(x)` | Compute a scalar square root. |

The matrix is implicit and can be rectangular. Coefficient functions accept
either two f64 or two u64 indices and return f64. They support scalar f64
arithmetic and square root, checked u64 arithmetic and explicit integer
conversion, including inlined
Mech helpers. Iteration supports f64 scalars and vectors, including nested
source-defined iterations. Immutable matrix captures are supported; recursive
coefficient helpers, mutable function bodies and callable IO are rejected.

Cranelift emits the coefficient formula, row/column traversal, weighted sums,
counted iteration loop, dot product and square root. Runtime code owns array
storage, closure handle scopes, IO and disjoint-row scheduling. It does not
interpret source function graphs or implement a spectral-norm algorithm.
The runtime's closure bridge calls a native function body emitted in the same
object; its loop control is also native.

## How optimization decisions are made

All eligibility checks inspect operation types, constants, dependencies or
ownership. They do not inspect benchmark names or recognize an input size.

- **Exact binary division:** one shared engine predicate replaces division by
  a power of two with multiplication by its finite, exactly representable
  reciprocal. Both the array IR and the fused backend used by n-body and
  Mandelbrot call this predicate. Nonbinary divisors and reciprocals that
  overflow are left as divisions.
- **Common expressions:** pure scalar coefficient nodes with identical
  operations and input IDs share a computed value.
- **Integer range checks:** native eligibility code propagates conservative
  unsigned intervals through the coefficient graph, using the actual row and
  column bounds for each call. A successful proof removes repeated overflow
  and zero-divisor checks in the inner loop. Failed proofs call a checked
  native kernel; errors prevent publication of its partial output. Empty
  reductions write zeros without evaluating the coefficient function. Scalar
  u64 arithmetic also uses checked native kernels, matching Mech math semantics.
- **Loop invariants:** argument-dependency masks identify expressions that
  do not depend on the varying column. Those expressions move outside the
  column loop.
- **Integer induction:** the coefficient DAG identifies varying integer
  multiplication cones that are polynomials of degree at most two. With the
  unsigned range proof successful, the kernel carries each polynomial's value
  and first difference through the column loop. A fixed second difference
  updates them with additions. SIMD offsets come from the same polynomial
  state. Divisions, conversions and floating arithmetic terminate polynomial
  eligibility; checked fallback kernels retain the original operations.
  Auxiliary differences use u64 ring arithmetic, while the guard proves every
  observable source operation valid under Mech's checked integer semantics.
- **SIMD:** independent evaluations use f64x2 lanes; remaining columns use
  scalar tails. Separate accumulators expose instruction-level parallelism.
  SIMD groups reduction additions differently; `--scalar` preserves increasing
  column accumulation order. Array SIMD and packed-symbol SIMD are separate
  policies, preserving the earlier k-nucleotide tuning.
- **Row tiling:** a matrix operation's contract gives independent output rows.
  The backend can compute two or four rows together, sharing input loads and
  column indices while keeping a separate reduction for every row. Incomplete
  row tiles use an emitted one-row fallback; incomplete column tiles use scalar
  tails. This transformation works with any admitted coefficient function.
- **Buffer reuse:** native graph last-use releases mark dead values. The
  runtime recycles numeric buffers only when `Arc::try_unwrap` proves exclusive
  ownership. Existing vector arithmetic uses this allocator too.
- **Row scheduling:** small disjoint row chunks allow Rayon work stealing on
  heterogeneous processors. This is independent of histogram partitioning.

Spectral norm reuses the typed function/dataflow admission, scheduling,
standard-library catalog, native arithmetic, and public AOT build machinery
from the sequence revisions. The array and earlier fused backends still have
separate loop lowering implementations; the shared reciprocal predicate is
an explicit cross-backend rule. Byte-window packing, grid escape masks and
reciprocal-square-root refinement remain available to eligible programs.

Eligibility is automatic, but profitability is not fully automatic: SIMD,
accumulator count, row tile size and task grain are explicit policies. The
measurements sweep these policies; there is currently no graph cost model
automatically predicting the best settings or adapting them to measured input.
General transformations are implemented in the compiler, rather than in a
benchmark-specific wrapper. Generality alone is not proof of a speedup on every
program: each numerical path needs its own measurements.

Compiler policy `MECH_NUMERIC_UNROLL=1|2|4|8` chooses SIMD accumulators, and
`MECH_NUMERIC_ROWS=1|2|4` chooses the row tile size.
`MECH_NUMERIC_INDUCTION=0|1` disables/enables eligible integer induction (default 1).
`MECH_ARRAY_GRAIN=1..1024` selects target tasks per row partition, and
`RAYON_NUM_THREADS` limits pool workers. These are compiler/runtime settings,
with no extra benchmark algorithm hidden in them.

## Validation and reproducibility

The current array revision checks **85,293 independently calculated results**
across sixteen unrelated coefficient formulas, rectangular/empty arrays, odd
tails, thirteen scalar/SIMD and row-tiling configurations, three partition
counts and three grains. It includes NaN,
infinity and signed-zero coefficients, zero-count and nested iterations,
scalar/vector closure signatures, and mismatched-dot rejection. Spectral norm
is independently checked at N=1,2,3,5,7,8,9,15,16,17,31,100.
Unsigned overflow and division errors are checked in scalar expressions and
matrix coefficients, with one and eight workers. A deliberately conservative
interval case exercises the checked fallback. Empty reductions skip even an
overflowing coefficient. `verify_induction.py` checks another 240 results with
induction enabled and disabled: descending polynomials, large unsigned values,
transpose, tails, and a cubic expression containing a quadratic subgraph.
Earlier 28,665-result floating and 69,849-result unsigned reports are preserved.

Compiler tests reject captures, IO, recursion and incompatible callables,
check binary-reciprocal rounding at normal/subnormal boundaries, and retain
compact/expanded k-nucleotide object equivalence. Fused regression tests cover
both scalar and SIMD use of the shared rule. N-body's 19 fused tests and
Mandelbrot's 12 native-map tests pass after sharing the reciprocal rule.

`regressions.py` additionally preserves earlier complete-source Mandelbrot
and k-nucleotide executables, builds their identical Mech sources with the
current public compiler, checks official small and full outputs, then measures
old and new binaries in one shuffled batch with identical runtime settings.

## Diagnosing the Rust gap

The floating-denominator full cohort measures Mech at 0.166 s and Rust #5 at
0.127 s. This elapsed-time gap fluctuates between batches on this host. The
single-worker diagnostic removes the extra ambiguity from worker scheduling:

| Program | Wall seconds | Instructions, billions | Cycles, billions |
|---|---:|---:|---:|
| Rust #5, original integer denominator | 0.662 | 11.212 | 1.388 |
| Mech, floating denominator, row tile 4 / unroll 4 | 0.807 | 6.856 | 1.711 |
| Rust #5, diagnostic floating denominator | 1.285 | 14.539 | 2.729 |

These are medians of seven sequential fresh processes at N=5500, collected
with macOS `time -lp`; collector overhead is included. Every output matches.
Changing only Rust's coefficient arithmetic to floating point substantially
increases its time and cycles. Mech retires fewer instructions than integer
Rust but consumes more cycles, pointing to the arithmetic mix rather than
instruction count alone. This is evidence for implementing integer coefficient
graphs, not a claim that changing the arithmetic will explain every difference.

The fastest floating Mech matrix loops contain no stack loads/stores inside
the column loop; saved registers in their prologue are not per-element spills.
Native sampling at N=11000 / one worker places the worker's work in emitted
matrix loops, with the main thread waiting for the worker. Some row-index
conversions are rematerialized in machine code despite their position outside
the loop in the compiler IR, so IR hoisting does not guarantee register reuse.
Rust's hot loop also replaces the quadratic integer products with carried
additions through LLVM loop strength reduction. The initial Mech integer path
still multiplies per coefficient. The general induction rule addresses this
second compiler difference, rather than recognizing a spectral-norm formula.
It does not eliminate every multiplication: some SIMD offset expressions use
small constant multiply-add instructions, and scalar tails retain the source
formula. The tiled fast loops inspected here have no per-element stack spills;
the checked fallback has additional checks and register pressure.

The revised same-batch worker diagnostic measures:

| Program | One worker, seconds | Four workers, seconds |
|---|---:|---:|
| Mech floating denominator, tile 4 / unroll 4 | 0.808 | 0.225 |
| Mech integer denominator, tile 4 / unroll 4, induction off | 0.638 | 0.180 |
| Mech integer denominator, tile 4 / unroll 2, induction on | 0.658 | 0.189 |
| Rust #5 | 0.665 | 0.198 |

The integer source cuts one-worker elapsed time by 21% and measured cycles
from 1.711 to 1.339 billion. This closes the stable compute gap to the portable
Rust submissions. These diagnostic times include collector overhead and are
separate from the complete-language corner-chart run.

An additional matched on/off test confirms that induction is not an automatic
win on the M3. At tile 4 / unroll 2 it gives a small improvement (0.663 to
0.660 seconds with one worker); at tile 4 / unroll 4 it adds about 2% (0.638
to 0.654 seconds). The transformation changes dependencies and temporary
values as well as multiplication count. Both policies are retained and timed;
no general profitability model is claimed. Eight-worker desktop times fluctuate
substantially, so a ranking from that configuration alone is weaker evidence
than the consistent one- and four-worker compute results.

## Updated full cohort

The [current report](results/apple-m3-optimized-5500.json) contains 99 successful
builds: 66 Mech policies over three complete sources, and 33 builds from the
portable upstream submissions. Every build matches both official outputs.
All Mech builds use the same current public compiler. Compilation, correctness
checks and previous-benchmark regressions finish before the five shuffled,
sequential fresh-process timing rounds.

| Endpoint | Elapsed seconds | LOC | gzip bytes |
|---|---:|---:|---:|
| Mech fastest: integer denominator, SIMD, unroll 4, tile 4, induction off, grain 4 | 0.124092 | 13 | 306 |
| Mech smallest: floating denominator, SIMD, unroll 4, tile 4, grain 16 | 0.163125 | 13 | 300 |
| Rust fastest and smallest: #4, panic abort | 0.118299 | 60 | 798 |

The observed fastest-Mech median is about 5% above Rust, compared with 31%
in the earlier floating-only full cohort. In this batch it is 24% below the
floating endpoint. These are observed policy-sweep medians, not proof of a
5% underlying performance difference: the eight-worker samples have large
outliers and rankings fluctuate between batches. The independent one-worker
diagnostics provide the stronger evidence that integer arithmetic closes the
compute gap.

Runtime grain 4 targets 32 chunks with eight partitions; grain 16 targets 128.
The integer grain variants have exactly the same native object hash. Smaller
chunks offer more work stealing, but can put a larger fraction of rows through
the one-row tail kernel. This is a scheduling tradeoff with arbitrary array
sizes, with no recognized benchmark name or special input bound. Grain 4 is
the measured setting for this report, not an automatic universal choice.

Mech is a dot on the [LOC chart](results/apple-m3-optimized-5500.corner.loc.png):
its fastest source also has the minimum 13 lines. The
[compressed-size chart](results/apple-m3-optimized-5500.corner.png) has a short
Mech line between the 300-byte floating and 306-byte integer sources.

Rust's fastest and smallest endpoints coincide in this batch: #4 with panic
abort is 60 LOC and 798 compressed bytes. Six portable Rust sources were
measured, and the faster-source ranking has changed between batches. C++ also
has coincident endpoints, but only the serial #1 source (35 LOC, 445 bytes)
built unchanged here, with four GCC/LLVM flag combinations. The other three
pinned C++ sources require x86 intrinsics or Linux affinity APIs. Its dot
therefore reflects limited portable-source coverage, not evidence that an
optimized parallel C++ implementation needs no additional source.

The [updated earlier-program comparison](results/integer-backend-regression.json)
preserves official small/full outputs for Mandelbrot and k-nucleotide. All
three Mandelbrot native objects are byte-identical to their previous versions.
The k-nucleotide objects change because numeric kernel symbols now record
their scalar type; the old/current timing comparison shows no clear slowdown.

To reproduce after building the public compiler with `cranelift_build`:

```sh
python3 benchmarks/clbg/spectralnorm/verify.py --output benchmarks/clbg/spectralnorm/results/array-induction-conformance.json
python3 benchmarks/clbg/spectralnorm/verify_induction.py
python3 benchmarks/clbg/spectralnorm/run.py --prepare-only
python3 benchmarks/clbg/spectralnorm/regressions.py --prepare-only --build-dir benchmarks/clbg/spectralnorm/.build/integer-regressions
python3 benchmarks/clbg/spectralnorm/regressions.py --reuse-builds --build-dir benchmarks/clbg/spectralnorm/.build/integer-regressions --output benchmarks/clbg/spectralnorm/results/integer-backend-regression.json
python3 benchmarks/clbg/spectralnorm/run.py --reuse-builds --samples 5 --output benchmarks/clbg/spectralnorm/results/apple-m3-optimized-5500.json
python3 benchmarks/clbg/spectralnorm/plot.py benchmarks/clbg/spectralnorm/results/apple-m3-optimized-5500.json
```

The timing commands must run without simultaneous builds, tests, audits or
plot generation. `profile.py --report ... --only ... --workers 1,4 --output ...`
reproduces the selected-program hardware-counter diagnostics separately.

[Arithmetic ablation](results/integer-coefficient-ablation.json),
[worker scaling and counters](results/worker-scaling-counters.json), and
the extracted Mech/Rust assembly listings in `results` retain the diagnostic
evidence. The altered Rust source is kept separately in `experiments` and
excluded from source-size chart endpoints.
[Revised worker counters](results/induction-worker-counters.json) and
[matched induction on/off counters](results/induction-paired-counters.json)
retain every sample behind the revised diagnosis.

`fetch_sources.py` pins all 30 listed submissions for C, C++, Chapel, Julia,
Rust, Go and Java with complete comments, build notes and SHA256 hashes.
`run.py` attempts every source, native GCC/LLVM and fast-math variants, and
portable Rust panic-abort variants. Unmodified x86 and Linux-only programs
are explicitly excluded on the Apple M3. Rust 1–3 use edition 2018 to preserve
their original closure-capture semantics without editing source. Java runs
as OpenJDK JIT and is not labeled as GraalVM AOT.

All final timing samples run sequentially in fresh processes with identical
output validation, including CLI/IO and Julia/Java JIT startup. Builds, tests
and source audits finish before timing. Reports retain individual samples,
commands, policies, output hashes, source metrics, executable/object hashes,
toolchain versions, exclusions and an exact compiler/runtime/source archive.
