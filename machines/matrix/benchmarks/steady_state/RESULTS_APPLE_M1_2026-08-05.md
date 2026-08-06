# Apple M1 steady-state matrix results

Date: 2026-08-05

Code base: `feat/native-application-builds` at `2ca07947b`, plus the benchmark
harness in this directory.

## Environment

- Mac mini (2020), Apple M1, 4 performance plus 4 efficiency cores, 8 GB RAM
- macOS 15.6, arm64
- Rust nightly 1.96.0 (2026-03-02)
- Python 3.14.6
- NumPy 2.5.1 using Apple's Accelerate BLAS/LAPACK backend
- Lua 5.5.1
- LuaJIT 2.1.1785763465

NumPy ran with `VECLIB_MAXIMUM_THREADS=1`, `OPENBLAS_NUM_THREADS=1`, and
`OMP_NUM_THREADS=1`. The native and NumPy values below are the means of two
independent benchmark runs. Python, Lua, and LuaJIT are single runs; each
reported value is still the median of nine timed samples.

## Protocol

- Dense `f64` square matrices and deterministic identical input formulas
- At least 250 ms of untimed warmup before measurement
- Nine samples with approximately 75 ms timed batches
- Persistent inputs, outputs, specialized Mech functions, and reactive plans
- Matrix multiplication reuses its output allocation
- Linear solve performs a fresh LU factorization each iteration in all runners
- Parsing, startup, construction, specialization, and validation are untimed
- Mech reactive runs dirty-cell scheduling on every iteration
- Python cyclic GC and Lua GC are paused only during timed samples
- Every runner validates multiplication output and solve correctness

`mech-kernel` is a virtual `solve_result()` call on the persistent specialized
function. `mech-reactive` additionally includes dirty-cell tracking and
reactive scheduling directly on a `ReactivePlan`; it is not a complete
`MechRuntime` transaction.

## Common size: 256 x 256

Lower is better. Times are milliseconds per operation.

| Runtime | Multiply | Solve |
| --- | ---: | ---: |
| raw Rust / nalgebra | 0.7180 | 1.2211 |
| Mech kernel | 0.7185 | 1.2200 |
| Mech reactive | 0.7189 | 1.2221 |
| NumPy / Accelerate | 0.09964 | 0.18066 |
| LuaJIT | 22.4518 | 8.2028 |
| Lua 5.5 | 650.231 | 211.513 |
| Python 3.14 | 1212.408 | 409.701 |

Relative to raw Rust at this size:

| Mech path | Multiply | Solve |
| --- | ---: | ---: |
| kernel | +0.069% | -0.093% |
| reactive | +0.119% | +0.079% |

The negative solve delta is measurement noise, not a faster implementation:
Mech and raw Rust execute the same nalgebra factorization kernel. At this size,
NumPy is 7.21x faster than Mech reactive for multiplication and 6.76x faster
for solve. LuaJIT is 31.2x and 6.71x slower than Mech reactive. The non-JIT
interpreters are hundreds to thousands of times slower.

## Large size: 1024 x 1024

| Runtime | Multiply (ms) | Solve (ms) |
| --- | ---: | ---: |
| raw Rust / nalgebra | 46.2419 | 77.7955 |
| Mech kernel | 46.2361 | 78.3007 |
| Mech reactive | 46.3464 | 78.1918 |
| NumPy / Accelerate | 8.4503 | 5.6542 |

The reactive multiplication path averages 0.226% over raw Rust. The direct
kernel path is indistinguishable from raw Rust. NumPy is 5.47x faster than raw
Rust for multiplication and 13.76x faster for solve.

Large solve timings have about one percent run-to-run allocator and system
noise because every iteration clones and factors an 8 MiB matrix. Mech kernel
measured +1.14% and +0.16% in the two runs; Mech reactive measured -0.11% and
+1.13%. These results do not support a measurable `Result`-handling cost in
the solve kernel.

## Retained runtime loop: source versus bytecode

This is the end-to-end steady-state Mech measurement. A retained runtime takes
a live scalar host input, opens a reactive transaction, updates the cell,
tracks dirty dependencies, runs a scaling node plus the requested matrix
kernel, validates the turn, and commits. The source and bytecode fixtures have
equal plan lengths and live-input binding counts.

The bytecode fixture is compiled and installed before warmup. Bytecode in this
branch is a serialized plan format: installation reconstructs the same
specialized runtime function nodes used by source execution. It is not a
separate bytecode VM that interprets matrix instructions on every turn.

The table contains the mean of three independent paired runs. Each run still
uses the median of nine samples and alternates source/bytecode sample order.

| Size | Source multiply | Bytecode multiply | Delta | Source solve | Bytecode solve | Delta |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 256 | 0.9850 ms | 0.9762 ms | -0.893% | 1.3200 ms | 1.3202 ms | +0.009% |
| 512 | 6.4800 ms | 6.4730 ms | -0.108% | 10.1330 ms | 10.1455 ms | +0.123% |
| 1024 | 49.3361 ms | 49.3547 ms | +0.038% | 77.2172 ms | 77.1804 ms | -0.048% |

The 64 and 128 results are intentionally omitted from the summary because
sub-millisecond turns are bimodal on this 4-performance/4-efficiency-core M1.
At compute-heavy sizes, source-installed and bytecode-installed steady-state
performance is equivalent.

For context, the source runtime loop versus a current raw-nalgebra kernel
control measured +37.0%, +13.7%, and +6.65% for multiplication at 256, 512,
and 1024. Solve measured +8.0%, +0.29%, and +0.01%. Multiplication includes
the extra O(n^2) scaling node needed to make the matrix input reactive, so
these are full-graph costs rather than isolated runtime overhead percentages.

## Rectangular follow-up: NumPy's rank-one boundary

Follow-up run date: 2026-08-06.

The rectangular sweep tested `M x K` by `K x N` products through raw nalgebra,
the persistent Mech function, direct reactive scheduling, complete retained
source and bytecode runtimes, and NumPy. All paths reuse their output buffers.
The same warmup, nine-sample median, single-thread environment, deterministic
inputs, and correctness checks apply.

`K = 1` exposes a sharp NumPy/Accelerate general-matmul boundary. To keep the
comparison honest, the table includes both `numpy.matmul(..., out=...)` and the
equivalent optimized `numpy.multiply(..., out=...)` broadcast formulation.
Times are milliseconds per operation.

| Shape | Raw nalgebra | Mech source runtime | Mech bytecode runtime | NumPy matmul | NumPy optimized outer |
| --- | ---: | ---: | ---: | ---: | ---: |
| 512 x 1 x 512 | 0.0272 | 0.5471 | 0.5172 | 1.2118 | 0.0969 |
| 1024 x 1 x 1024 | 0.2185 | 1.3009 | 1.3069 | 6.4649 | 0.4077 |
| 2048 x 1 x 2048 | 0.8819 | 4.3334 | 4.3637 | 34.7773 | 1.8089 |
| 3072 x 1 x 3072 | 2.0630 | 9.9527 | n/a | 47.2163 | 2.9732 |
| 4096 x 1 x 4096 | 3.6868 | 17.3846 | n/a | 139.2732 | 5.3459 |

Against NumPy's general matrix-multiplication API, the complete source runtime
is 2.2x faster at 512, 5.0x at 1024, 8.0x at 2048, 4.7x at 3072, and 8.0x at
4096. The 1024 result and representative rectangular one-million-element
outputs were reproduced in independent runs. This is a specific `K = 1`
dispatch weakness: at `1024 x 2 x 1024`, NumPy takes 0.249 ms while the Mech
source runtime takes 1.452 ms.

The specialized NumPy outer-product formulation reverses the runtime result.
It is 2.4x to 3.4x faster than the complete Mech turn from 1024 through 4096.
Raw nalgebra is still 1.4x to 2.1x faster than optimized NumPy at those sizes,
which locates the remaining gap above the multiplication kernel. The Mech turn
cost grows with the retained output state: at 4096, the kernel takes 3.69 ms
but the full source turn takes 17.38 ms.

Bytecode entries are unavailable at 3072 and 4096 because the dense `f64`
result alone is about 72 MiB and 128 MiB, respectively. Bytecode v1's default
read limit is 64 MiB (`67,108,864` bytes), and the retained result is serialized
into the program image. The source-only benchmark mode records these sizes
without weakening that safety limit.

### Retained-output rollback cost and fail-stop mode

The rank-one result exposed two complete copies of every executed function's
output during journal capture. Plain cell preflight cloned the payload once and
discarded it, then capture cloned it again for the actual rollback snapshot.
Map and set preflight can fail while constructing canonical hashed snapshots,
but an ordinary `Clone` has no recoverable error to probe. Making plain
preflight borrow-only removes the discarded copy while preserving the retained
before-state and rollback behavior.

The benchmark now includes `mech-journal-output`, which captures and drops only
the result matrix's rollback journal. Times below are milliseconds per turn.
The original atomic column is the pre-optimization source-runtime result above;
the optimized atomic and fail-stop columns are from the 2026-08-06 rerun.

| Shape | Kernel | Journal, two copies | Journal, one copy | Atomic original | Atomic optimized | Fail-stop |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1024 x 1 x 1024 | 0.2002 | 0.9191 | 0.4515 | 1.3009 | 0.8037 | 0.5794 |
| 2048 x 1 x 2048 | 0.8764 | 3.7866 | 1.8679 | 4.3334 | 2.4853 | 0.8279 |
| 3072 x 1 x 3072 | 2.0291 | 8.4253 | 4.2808 | 9.9527 | 5.5568 | 2.1886 |
| 4096 x 1 x 4096 | 3.6546 | 15.3897 | 7.5874 | 17.3846 | 9.7755 | 3.7634 |

Removing the redundant clone reduces the 4096 atomic turn by 43.8%. The
remaining atomic overhead is the one 128 MiB before-state snapshot required to
restore an output that the kernel mutates in place. Explicit fail-stop mode
skips program snapshots and poisons the runtime on any failed turn. At 4096 it
runs in 3.76 ms, 3.0% above the specialized Mech kernel and 3.7% above raw
nalgebra on this run.

Fail-stop source and bytecode installation are also equivalent: 0.5263 versus
0.4990 ms at 1024 and 0.9974 versus 1.0162 ms at 2048. The sub-millisecond
spread is consistent with the M1's performance/efficiency-core noise rather
than a different hot executor.

## Square graph follow-up: transpose and language runtimes

The 2026-08-06 follow-up added a live scalar scale followed by a materialized
transpose. Every runner performs both operations in the timed region. This
avoids timing NumPy's zero-copy transpose view against a Mech graph that must
actually produce a new matrix. Times below are single-run nine-sample medians
at 256; the Mech rows use the complete retained source runtime.

| Runtime | Multiply | Scale + transpose | Portable solve |
| --- | ---: | ---: | ---: |
| NumPy / Accelerate | 0.1002 ms | 0.0616 ms | 0.1816 ms |
| raw Rust / nalgebra | 0.7182 ms | 0.0694 ms | 1.2198 ms |
| Mech atomic | 0.9864 ms | 0.5608 ms | 1.3109 ms |
| Mech fail-stop | 0.9324 ms | 0.5641 ms | 1.3160 ms |
| LuaJIT | 20.3873 ms | 0.1344 ms | 7.1819 ms |
| Lua 5.5 | 651.100 ms | 3.1941 ms | 211.846 ms |
| Python 3.14 | 1212.733 ms | 6.3897 ms | 410.110 ms |

The 256 Mech transpose/runtime samples are bimodal, so their medians should not
be used as a precise overhead ratio. At 1024, raw Rust scale plus transpose is
2.085 ms, Mech atomic is 4.741 ms, Mech fail-stop is 3.544 ms, and NumPy is
1.393 ms. Source and bytecode remain equivalent for all three operations.

## Accelerate-backed solve

The portable nalgebra LU, not `solve_result` or the runtime scheduler, caused
the large solve gap. `nalgebra-lapack 0.27` can call the same single-threaded
Apple Accelerate backend as NumPy. Mech now exposes that path through the
opt-in `solve_accelerate` feature on macOS.

| Size | Portable Rust | Rust / Accelerate | Mech kernel / Accelerate | Mech reactive / Accelerate | NumPy / Accelerate |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 256 | 1.2198 ms | 0.1660 ms | 0.1707 ms | 0.1658 ms | 0.1816 ms |
| 1024 | 77.9412 ms | 5.5672 ms | 5.5245 ms | 5.5781 ms | 5.7597 ms |

At 1024, the complete retained source runtime is 5.7523 ms and bytecode is
5.7980 ms with Accelerate, effectively matching NumPy while still including
host-input admission, dirty scheduling, and atomic transaction work. Fail-stop
measured 5.8611 ms in this run; the small inversion is ordinary run-to-run
noise because the 8 MiB factorization dominates and rollback retains only the
vector output.

## Interpretation

The plan-level Mech execution machinery is not the matrix bottleneck. On a
shared kernel, `solve_result`, virtual dispatch, borrowing, and direct reactive
scheduling add no meaningful cost. Complete runtime transaction cost is visible
for shorter kernels but is amortized as square matrix work grows. Source and
bytecode installation converge on the same steady-state executor. NumPy's
optimized BLAS/LAPACK kernels dominate ordinary dense square work, and Mech
matches its large-solve performance once it selects the same Accelerate
backend. Portable nalgebra remains stronger on rank-one output generation. For
large retained outputs, atomic rollback copying can outweigh that kernel
advantage even though source and bytecode execution remain equivalent. The
opt-in fail-stop path demonstrates that the rest of the retained runtime loop
can run within a few percent of the kernel when restart-on-error semantics are
acceptable.
